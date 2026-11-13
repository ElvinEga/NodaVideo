//! Pure, frame-exact time conversions used at the FFmpeg boundary.
//!
//! The timeline model stores time as integer frames. Floating-point seconds exist
//! only in FFmpeg arguments, where the filtergraph requires them. Keeping every
//! conversion in this module makes it possible to test frame-rate math without
//! spawning a process, and prevents sequence fps / asset fps confusion from
//! spreading across preview and export.

use nodavideo_project::FrameRate;
use nodavideo_timeline::Clip;

/// Convert a frame count at `fps` into seconds for an FFmpeg argument.
///
/// Frame counts are non-negative in a valid clip; the sign is preserved so an
/// invalid caller is visible rather than silently clamped.
pub fn frames_to_seconds(frames: i64, fps: FrameRate) -> f64 {
    (frames as f64) / fps.to_f64().max(1.0)
}

/// Format seconds for an FFmpeg filtergraph argument.
///
/// Six decimals keep rounding error well below one frame even at 60 fps; four
/// decimals were enough to drift by a visible frame on longer timelines.
pub fn format_seconds(seconds: f64) -> String {
    format!("{seconds:.6}")
}

/// Rate correction that maps a source stream onto the sequence timeline.
///
/// `1.0` means the asset and sequence run at the same rate (the common case).
/// A 30 fps source on a 24 fps sequence yields `1.25`: the source must play
/// 25% faster to occupy the same number of timeline frames.
pub fn retiming_factor(source_fps: FrameRate, sequence_fps: FrameRate) -> f64 {
    source_fps.to_f64().max(1.0) / sequence_fps.to_f64().max(1.0)
}

/// Map a timeline frame to the corresponding source frame of a clip.
///
/// When the asset and sequence frame rates differ, the timeline delta is
/// retimed into source frames. This keeps `source_in` (source frames) and
/// `start_frame` (timeline frames) in their own units instead of adding them
/// together as if they were the same unit.
pub fn timeline_to_source_frame(
    clip: &Clip,
    timeline_frame: i64,
    source_fps: FrameRate,
    sequence_fps: FrameRate,
) -> i64 {
    let timeline_delta = timeline_frame - clip.start_frame;
    let factor = retiming_factor(source_fps, sequence_fps);
    let source_delta = (timeline_delta as f64 * factor).round() as i64;
    clip.source_in + source_delta
}

#[cfg(test)]
mod tests {
    use super::*;
    use nodavideo_timeline::{Clip, ClipId, TrackId};

    #[test]
    fn frames_to_seconds_is_exact_at_integer_rates() {
        assert_eq!(frames_to_seconds(24, FrameRate::FPS_24), 1.0);
        assert_eq!(frames_to_seconds(0, FrameRate::FPS_24), 0.0);
        assert!((frames_to_seconds(12, FrameRate::FPS_24) - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn frames_to_seconds_handles_fractional_rates() {
        let sec = frames_to_seconds(30000, FrameRate::FPS_29_97);
        assert!((sec - 1001.0).abs() < 1e-9, "29.97 fps: got {sec}");
    }

    #[test]
    fn format_seconds_keeps_subframe_precision() {
        assert_eq!(format_seconds(1.0 / 24.0), "0.041667");
        assert_eq!(format_seconds(0.0), "0.000000");
    }

    #[test]
    fn retiming_factor_is_one_for_matching_rates() {
        assert_eq!(retiming_factor(FrameRate::FPS_24, FrameRate::FPS_24), 1.0);
    }

    #[test]
    fn retiming_factor_scales_30fps_source_onto_24fps_sequence() {
        assert_eq!(retiming_factor(FrameRate::FPS_30, FrameRate::FPS_24), 1.25);
        assert_eq!(retiming_factor(FrameRate::FPS_24, FrameRate::FPS_30), 0.8);
    }

    #[test]
    fn source_frame_mapping_is_identity_when_rates_match() {
        let clip = Clip::new(
            ClipId::new(),
            nodavideo_project::AssetId::new(),
            TrackId::new(),
            "clip",
            100,
            10,
            60,
        )
        .unwrap();

        assert_eq!(
            timeline_to_source_frame(&clip, 100, FrameRate::FPS_24, FrameRate::FPS_24),
            10
        );
        assert_eq!(
            timeline_to_source_frame(&clip, 112, FrameRate::FPS_24, FrameRate::FPS_24),
            22
        );
    }

    #[test]
    fn source_frame_mapping_retimes_when_rates_differ() {
        let clip = Clip::new(
            ClipId::new(),
            nodavideo_project::AssetId::new(),
            TrackId::new(),
            "clip",
            0,
            0,
            120,
        )
        .unwrap();

        // 24 timeline frames at 24fps == 1 second == 30 source frames at 30fps.
        assert_eq!(
            timeline_to_source_frame(&clip, 24, FrameRate::FPS_30, FrameRate::FPS_24),
            30
        );
    }
}
