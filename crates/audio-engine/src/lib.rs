//! Audio engine for multitrack mixing, level metering, and playback synchronization.

pub mod buffer;
pub mod clock;
pub mod error;
pub mod meter;
pub mod mixer;

pub use buffer::{
    AudioBuffer, AudioFormat, CHANNELS_MONO, CHANNELS_STEREO, SAMPLE_RATE_44K1, SAMPLE_RATE_48K,
    audio_sample_to_timeline_frame, samples_per_video_frame, timeline_frame_to_audio_sample,
};
pub use clock::AudioClock;
pub use error::AudioError;
pub use meter::{AudioLevels, AudioMeter, linear_to_dbfs};
pub use mixer::{
    AudioSampleSource, SequenceAudioMixer, SilentSource, SineToneSource, db_to_linear,
    pan_to_stereo_gains,
};

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use nodavideo_project::{AssetId, FrameRate};
    use nodavideo_timeline::{Clip, ClipAudioParams, ClipId, Sequence, TrackType};

    use super::*;

    #[test]
    fn test_audio_format_and_buffer_lifecycle() {
        let fmt = AudioFormat::new(48_000, 2).unwrap();
        assert_eq!(fmt.sample_rate, 48_000);
        assert_eq!(fmt.channels, 2);

        let mut buf = AudioBuffer::new(fmt, 100);
        assert_eq!(buf.frame_count(), 100);
        assert_eq!(buf.data.len(), 200);

        buf.set_sample(10, 0, 0.75);
        buf.set_sample(10, 1, -0.5);
        assert_eq!(buf.get_sample(10, 0), 0.75);
        assert_eq!(buf.get_sample(10, 1), -0.5);

        buf.add_sample(10, 0, 0.5);
        assert_eq!(buf.get_sample(10, 0), 1.25);

        buf.clamp(-1.0, 1.0);
        assert_eq!(buf.get_sample(10, 0), 1.0);

        buf.clear();
        assert_eq!(buf.get_sample(10, 0), 0.0);
    }

    #[test]
    fn test_timing_conversions_24fps() {
        let fps = FrameRate::FPS_24;
        let sample_rate = 48_000;

        let samples_per_frame = samples_per_video_frame(sample_rate, fps);
        assert_eq!(samples_per_frame, 2000);

        let sample_at_frame_24 = timeline_frame_to_audio_sample(24, sample_rate, fps);
        assert_eq!(sample_at_frame_24, 48_000);

        let frame_at_sample = audio_sample_to_timeline_frame(48_000, sample_rate, fps);
        assert_eq!(frame_at_sample, 24);

        let frame_at_partial_sample = audio_sample_to_timeline_frame(50_000, sample_rate, fps);
        assert_eq!(frame_at_partial_sample, 25);
    }

    #[test]
    fn test_audio_clock_and_drift() {
        let fps = FrameRate::FPS_24;
        let mut clock = AudioClock::new(48_000);
        assert_eq!(clock.current_sample, 0);
        assert!(!clock.is_playing);

        clock.play();
        assert!(clock.is_playing);

        // Advance by 1 second (48,000 samples = 24 frames)
        clock.advance_samples(48_000);
        assert_eq!(clock.current_sample, 48_000);
        assert_eq!(clock.current_timeline_frame(fps), 24);
        assert_eq!(clock.current_time_seconds(), 1.0);

        // Test drift calculation
        // If visual is at frame 26 while audio clock is at frame 24, visual is +2 frames ahead
        let drift = clock.drift_frames(26, fps);
        assert_eq!(drift, 2);

        // Seek to frame 48
        clock.seek_to_frame(48, fps);
        assert_eq!(clock.current_sample, 96_000);
        assert_eq!(clock.current_timeline_frame(fps), 48);

        clock.pause();
        assert!(!clock.is_playing);
        clock.advance_samples(1000); // Should not advance when paused
        assert_eq!(clock.current_sample, 96_000);
    }

    #[test]
    fn test_db_and_pan_conversions() {
        assert_eq!(db_to_linear(0.0), 1.0);
        assert!((db_to_linear(-6.0) - 0.501_187_2).abs() < 1e-4);
        assert_eq!(db_to_linear(-65.0), 0.0); // Mute threshold

        let (left_c, right_c) = pan_to_stereo_gains(0.0);
        assert_eq!(left_c, 1.0);
        assert_eq!(right_c, 1.0);

        let (left_hard, right_hard) = pan_to_stereo_gains(-1.0);
        assert_eq!(left_hard, 1.0);
        assert_eq!(right_hard, 0.0);

        let (left_r, right_r) = pan_to_stereo_gains(1.0);
        assert_eq!(left_r, 0.0);
        assert_eq!(right_r, 1.0);
    }

    #[test]
    fn test_audio_meter_calculation() {
        let fmt = AudioFormat::stereo_48k();
        let mut buf = AudioBuffer::new(fmt, 100);

        // Left channel: full scale 1.0 (0 dBFS)
        // Right channel: half scale 0.5 (~ -6 dBFS)
        for i in 0..100 {
            buf.set_sample(i, 0, 1.0);
            buf.set_sample(i, 1, 0.5);
        }

        let levels = AudioMeter::compute_levels(&buf);
        assert!((levels.peak_db_left - 0.0).abs() < 0.1);
        assert!((levels.peak_db_right - (-6.02)).abs() < 0.1);
        assert!((levels.rms_db_left - 0.0).abs() < 0.1);
        assert!((levels.rms_db_right - (-6.02)).abs() < 0.1);
    }

    #[test]
    fn test_sequence_audio_mixer_multitrack_and_gating() {
        let mut sequence = Sequence::default_1080p24();
        // Default sequence has tracks[0] = V2, tracks[1] = V1, tracks[2] = A1, tracks[3] = A2
        let a1_id = sequence.tracks[2].id;
        let a2_id = sequence.tracks[3].id;
        assert_eq!(sequence.tracks[2].track_type, TrackType::Audio);
        assert_eq!(sequence.tracks[3].track_type, TrackType::Audio);

        let clip1_id = ClipId::new();
        let mut clip1 = Clip::new(clip1_id, AssetId::new(), a1_id, "Dialogue", 0, 0, 48).unwrap();
        clip1.audio_params = ClipAudioParams {
            volume_db: 0.0,
            pan: -1.0, // Hard Left
        };
        sequence.add_clip(a1_id, clip1).unwrap();

        let clip2_id = ClipId::new();
        let mut clip2 = Clip::new(clip2_id, AssetId::new(), a2_id, "Music", 0, 0, 48).unwrap();
        clip2.audio_params = ClipAudioParams {
            volume_db: -6.0,
            pan: 1.0, // Hard Right
        };
        sequence.add_clip(a2_id, clip2).unwrap();

        let tone_a1 = Arc::new(SineToneSource::new(440.0, 48_000, 1.0));
        let tone_a2 = Arc::new(SineToneSource::new(880.0, 48_000, 1.0));

        let source_provider = move |id: ClipId| {
            if id == clip1_id {
                Some(tone_a1.clone() as Arc<dyn AudioSampleSource>)
            } else if id == clip2_id {
                Some(tone_a2.clone() as Arc<dyn AudioSampleSource>)
            } else {
                None
            }
        };

        let audio_mixer = SequenceAudioMixer::default();

        // 1. Mix frames 0..24 (1 second) with both tracks active
        let mixed = audio_mixer
            .mix_timeline_range(&sequence, 0, 24, &source_provider)
            .unwrap();
        assert_eq!(mixed.frame_count(), 48_000);

        let levels = AudioMeter::compute_levels(&mixed);
        // Left should have Dialogue tone (~0 dBFS), Right should have Music tone (~ -6 dBFS)
        assert!(levels.peak_db_left > -1.0);
        assert!(levels.peak_db_right > -8.0 && levels.peak_db_right < -4.0);

        // 2. Test Mute Gating: Mute A1
        sequence.tracks[2].muted = true;
        let mixed_muted = audio_mixer
            .mix_timeline_range(&sequence, 0, 24, &source_provider)
            .unwrap();
        let levels_muted = AudioMeter::compute_levels(&mixed_muted);
        assert_eq!(levels_muted.peak_db_left, -100.0); // A1 muted and A2 is hard right!
        assert!(levels_muted.peak_db_right > -8.0);

        // 3. Test Solo Gating: Unmute A1, Solo A1
        sequence.tracks[2].muted = false;
        sequence.tracks[2].soloed = true;
        let mixed_solo = audio_mixer
            .mix_timeline_range(&sequence, 0, 24, &source_provider)
            .unwrap();
        let levels_solo = AudioMeter::compute_levels(&mixed_solo);
        assert!(levels_solo.peak_db_left > -1.0); // A1 is heard
        assert_eq!(levels_solo.peak_db_right, -100.0); // A2 is silenced by solo gating
    }
}
