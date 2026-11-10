//! Multitrack audio mixing, volume gain, stereo panning, and track gating.

use std::sync::Arc;

use nodavideo_timeline::{ClipId, Sequence, TrackType};

use crate::buffer::{AudioBuffer, AudioFormat, timeline_frame_to_audio_sample};
use crate::error::AudioError;

/// Convert volume in decibels (dB) into linear amplitude multiplier.
///
/// Any value at or below -60.0 dB is treated as silence (0.0).
#[must_use]
pub fn db_to_linear(volume_db: f64) -> f32 {
    if volume_db <= -60.0 {
        0.0
    } else {
        10.0_f64.powf(volume_db / 20.0) as f32
    }
}

/// Calculate Left and Right stereo channel gains from a stereo pan parameter in `[-1.0, 1.0]`.
///
/// - `-1.0` is 100% hard Left (`1.0, 0.0`).
/// - `0.0` is Center (`1.0, 1.0`).
/// - `1.0` is 100% hard Right (`0.0, 1.0`).
#[must_use]
pub fn pan_to_stereo_gains(pan: f64) -> (f32, f32) {
    let p = pan.clamp(-1.0, 1.0);
    let left = if p <= 0.0 { 1.0 } else { (1.0 - p) as f32 };
    let right = if p >= 0.0 { 1.0 } else { (1.0 + p) as f32 };
    (left, right)
}

/// Abstract provider interface for reading audio PCM samples for an asset or clip.
pub trait AudioSampleSource: Send + Sync {
    /// Read `frame_count` audio sample frames starting from `sample_offset`.
    ///
    /// For stereo formats, the returned slice is interleaved `[L, R, L, R, ...]`.
    fn read_samples(&self, sample_offset: i64, frame_count: usize, channels: usize) -> Vec<f32>;
}

/// Silent audio source producing zero-amplitude samples.
#[derive(Debug, Clone, Copy, Default)]
pub struct SilentSource;

impl AudioSampleSource for SilentSource {
    fn read_samples(&self, _sample_offset: i64, frame_count: usize, channels: usize) -> Vec<f32> {
        vec![0.0; frame_count * channels]
    }
}

/// Synthetic sine wave audio source for diagnostics, tone generation, and testing.
#[derive(Debug, Clone)]
pub struct SineToneSource {
    /// Tone frequency in Hertz (e.g. 440.0 for musical note A4).
    pub frequency_hz: f32,
    /// Sample rate in Hertz (e.g. 48000).
    pub sample_rate: u32,
    /// Peak amplitude `[0.0, 1.0]`.
    pub amplitude: f32,
}

impl SineToneSource {
    /// Create a new sine wave source.
    #[must_use]
    pub fn new(frequency_hz: f32, sample_rate: u32, amplitude: f32) -> Self {
        Self {
            frequency_hz,
            sample_rate: sample_rate.max(1),
            amplitude: amplitude.clamp(0.0, 1.0),
        }
    }

    /// Standard 440 Hz concert pitch test tone at 48 kHz.
    #[must_use]
    pub fn concert_a4_48k() -> Self {
        Self::new(440.0, 48_000, 0.5)
    }
}

impl AudioSampleSource for SineToneSource {
    fn read_samples(&self, sample_offset: i64, frame_count: usize, channels: usize) -> Vec<f32> {
        let total_samples = frame_count * channels;
        let mut data = Vec::with_capacity(total_samples);
        let sample_rate = self.sample_rate as f64;
        let freq = self.frequency_hz as f64;
        let two_pi = std::f64::consts::PI * 2.0;

        for frame in 0..frame_count {
            let current_sample = sample_offset + frame as i64;
            let time = current_sample as f64 / sample_rate;
            let val = (time * freq * two_pi).sin() as f32 * self.amplitude;
            for _ in 0..channels {
                data.push(val);
            }
        }

        data
    }
}

/// Multitrack audio mixer for timeline playback and preview generation.
#[derive(Debug, Clone, Copy)]
pub struct SequenceAudioMixer {
    /// Target output audio format.
    pub format: AudioFormat,
}

impl Default for SequenceAudioMixer {
    fn default() -> Self {
        Self {
            format: AudioFormat::stereo_48k(),
        }
    }
}

impl SequenceAudioMixer {
    /// Create a mixer with the specified output audio format.
    #[must_use]
    pub const fn new(format: AudioFormat) -> Self {
        Self { format }
    }

    /// Mix audio across all active sequence tracks for the given timeline video frame range.
    ///
    /// Respects track mute, solo gating, clip enable status, volume gain (`volume_db`), and stereo pan.
    pub fn mix_timeline_range<F>(
        &self,
        sequence: &Sequence,
        start_frame: i64,
        end_frame: i64,
        source_provider: &F,
    ) -> Result<AudioBuffer, AudioError>
    where
        F: Fn(ClipId) -> Option<Arc<dyn AudioSampleSource>>,
    {
        if end_frame <= start_frame {
            return Ok(AudioBuffer::new(self.format, 0));
        }

        let start_sample =
            timeline_frame_to_audio_sample(start_frame, self.format.sample_rate, sequence.fps);
        let end_sample =
            timeline_frame_to_audio_sample(end_frame, self.format.sample_rate, sequence.fps);
        let total_sample_frames = (end_sample - start_sample).max(0) as usize;

        let mut output = AudioBuffer::new(self.format, total_sample_frames);
        if total_sample_frames == 0 {
            return Ok(output);
        }

        // Determine if any audio track is soloed
        let any_soloed = sequence
            .tracks
            .iter()
            .any(|t| t.track_type == TrackType::Audio && t.soloed);

        for track in &sequence.tracks {
            // Process only audio tracks
            if track.track_type != TrackType::Audio {
                continue;
            }
            // Honor track mute and solo gating
            if track.muted || (any_soloed && !track.soloed) {
                continue;
            }

            for clip in &track.clips {
                if !clip.enabled {
                    continue;
                }
                let clip_start = clip.start_frame;
                let clip_end = clip.end_frame();

                // Check intersection with requested timeline window
                let overlap_start = start_frame.max(clip_start);
                let overlap_end = end_frame.min(clip_end);

                if overlap_end <= overlap_start {
                    continue;
                }

                // Convert clip window to audio sample frames
                let overlap_start_sample = timeline_frame_to_audio_sample(
                    overlap_start,
                    self.format.sample_rate,
                    sequence.fps,
                );
                let overlap_end_sample = timeline_frame_to_audio_sample(
                    overlap_end,
                    self.format.sample_rate,
                    sequence.fps,
                );
                let overlap_frames = (overlap_end_sample - overlap_start_sample).max(0) as usize;

                if overlap_frames == 0 {
                    continue;
                }

                // Calculate source sample offset
                let clip_start_sample = timeline_frame_to_audio_sample(
                    clip_start,
                    self.format.sample_rate,
                    sequence.fps,
                );
                let source_in_sample = timeline_frame_to_audio_sample(
                    clip.source_in,
                    self.format.sample_rate,
                    sequence.fps,
                );
                let source_offset = source_in_sample + (overlap_start_sample - clip_start_sample);

                // Fetch source samples or fallback to silence
                let source = source_provider(clip.id).unwrap_or_else(|| Arc::new(SilentSource));
                let samples =
                    source.read_samples(source_offset, overlap_frames, self.format.channels);

                // Calculate gains
                let volume_gain = db_to_linear(clip.audio_params.volume_db);
                let (pan_l, pan_r) = pan_to_stereo_gains(clip.audio_params.pan);
                let left_gain = volume_gain * pan_l;
                let right_gain = volume_gain * pan_r;

                // Destination offset in the output buffer
                let dest_offset_frames = (overlap_start_sample - start_sample).max(0) as usize;

                // Mix into output buffer
                for f in 0..overlap_frames {
                    let dest_frame = dest_offset_frames + f;
                    if dest_frame >= total_sample_frames {
                        break;
                    }

                    if self.format.channels == 2 {
                        let src_l = samples.get(f * 2).copied().unwrap_or(0.0);
                        let src_r = samples.get(f * 2 + 1).copied().unwrap_or(src_l);

                        output.add_sample(dest_frame, 0, src_l * left_gain);
                        output.add_sample(dest_frame, 1, src_r * right_gain);
                    } else {
                        let src = samples.get(f).copied().unwrap_or(0.0);
                        output.add_sample(dest_frame, 0, src * volume_gain);
                    }
                }
            }
        }

        // Clamp to prevent digital distortion clipping
        output.clamp(-1.0, 1.0);
        Ok(output)
    }
}
