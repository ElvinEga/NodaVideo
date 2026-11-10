//! NodaVideo timeline engine crate.
//!
//! Provides the multitrack editing model, tracks, clips, frame-accurate timing,
//! splitting, trimming, ripple editing, snapping, and selection state.
//!
//! As specified in `AGENTS.md`, this crate is pure and deterministic:
//! it has zero dependencies on FFmpeg, GPUI, wall-clock time, or random IDs.

pub mod clip;
pub mod error;
pub mod id;
pub mod selection;
pub mod sequence;
pub mod snap;
pub mod track;

pub use clip::{Clip, ClipAudioParams, ClipTransform};
pub use error::TimelineError;
pub use id::{ClipId, SequenceId, TrackId};
pub use selection::TimelineSelection;
pub use sequence::Sequence;
pub use snap::{SnapResult, SnapTarget, snap_clip_move, snap_frame};
pub use track::{Track, TrackType};

#[cfg(test)]
mod tests {
    use super::*;
    use nodavideo_project::AssetId;

    fn test_clip(
        id: ClipId,
        asset_id: AssetId,
        track_id: TrackId,
        start: i64,
        source_in: i64,
        source_out: i64,
    ) -> Clip {
        Clip::new(
            id,
            asset_id,
            track_id,
            "Test Clip",
            start,
            source_in,
            source_out,
        )
        .expect("valid clip parameters")
    }

    #[test]
    fn test_clip_invariants_and_durations() {
        let asset_id = AssetId::new();
        let track_id = TrackId::new();
        let clip_id = ClipId::new();

        // Valid clip
        let clip = Clip::new(clip_id, asset_id, track_id, "Clip", 10, 0, 100).unwrap();
        assert_eq!(clip.duration_frames(), 100);
        assert_eq!(clip.end_frame(), 110);
        assert!(clip.contains_timeline_frame(10));
        assert!(clip.contains_timeline_frame(109));
        assert!(!clip.contains_timeline_frame(9));
        assert!(!clip.contains_timeline_frame(110));

        // Mapping timeline to source
        assert_eq!(clip.timeline_to_source_frame(10), Some(0));
        assert_eq!(clip.timeline_to_source_frame(35), Some(25));
        assert_eq!(clip.timeline_to_source_frame(115), None);

        // Negative start frame rejected
        assert!(matches!(
            Clip::new(clip_id, asset_id, track_id, "Clip", -1, 0, 100),
            Err(TimelineError::InvalidStartFrame(-1))
        ));

        // Inverted source in/out rejected
        assert!(matches!(
            Clip::new(clip_id, asset_id, track_id, "Clip", 0, 50, 50),
            Err(TimelineError::InvalidTrim { .. })
        ));
        assert!(matches!(
            Clip::new(clip_id, asset_id, track_id, "Clip", 0, 50, 40),
            Err(TimelineError::InvalidTrim { .. })
        ));
    }

    #[test]
    fn test_clip_trimming() {
        let asset_id = AssetId::new();
        let track_id = TrackId::new();
        let clip_id = ClipId::new();

        let mut clip = Clip::new(clip_id, asset_id, track_id, "Clip", 50, 10, 60).unwrap();
        assert_eq!(clip.duration_frames(), 50);
        assert_eq!(clip.end_frame(), 100);

        // Trim in from 10 to 20: delta = +10, start_frame shifts to 60, end_frame remains 100
        clip.trim_in(20).unwrap();
        assert_eq!(clip.source_in, 20);
        assert_eq!(clip.start_frame, 60);
        assert_eq!(clip.duration_frames(), 40);
        assert_eq!(clip.end_frame(), 100);

        // Trim out from 60 to 50: duration shrinks to 30, start_frame remains 60, end_frame becomes 90
        clip.trim_out(50).unwrap();
        assert_eq!(clip.source_out, 50);
        assert_eq!(clip.start_frame, 60);
        assert_eq!(clip.duration_frames(), 30);
        assert_eq!(clip.end_frame(), 90);

        // Invalid trim out <= source_in
        assert!(clip.trim_out(20).is_err());
    }

    #[test]
    fn test_clip_splitting() {
        let asset_id = AssetId::new();
        let track_id = TrackId::new();
        let clip_id = ClipId::new();
        let new_clip_id = ClipId::new();

        let clip = Clip::new(clip_id, asset_id, track_id, "Clip", 100, 0, 50).unwrap();
        assert_eq!(clip.end_frame(), 150);

        // Split at timeline frame 120
        let (left, right) = clip.split(120, new_clip_id).unwrap();
        assert_eq!(left.id, clip_id);
        assert_eq!(left.start_frame, 100);
        assert_eq!(left.source_in, 0);
        assert_eq!(left.source_out, 20);
        assert_eq!(left.duration_frames(), 20);
        assert_eq!(left.end_frame(), 120);

        assert_eq!(right.id, new_clip_id);
        assert_eq!(right.start_frame, 120);
        assert_eq!(right.source_in, 20);
        assert_eq!(right.source_out, 50);
        assert_eq!(right.duration_frames(), 30);
        assert_eq!(right.end_frame(), 150);

        // Boundary splits must fail
        assert!(clip.split(100, new_clip_id).is_err());
        assert!(clip.split(150, new_clip_id).is_err());
        assert!(clip.split(90, new_clip_id).is_err());
    }

    #[test]
    fn test_track_operations_and_ripple() {
        let track_id = TrackId::new();
        let asset_id = AssetId::new();
        let mut track = Track::new(track_id, "V1", TrackType::Video, 0);

        let c1 = test_clip(ClipId::new(), asset_id, track_id, 0, 0, 100);
        let c2 = test_clip(ClipId::new(), asset_id, track_id, 100, 0, 50);
        let c3 = test_clip(ClipId::new(), asset_id, track_id, 200, 0, 50);

        track.add_clip(c1).unwrap();
        track.add_clip(c2).unwrap();
        track.add_clip(c3).unwrap();
        assert_eq!(track.duration_frames(), 250);

        // Ripple remove c2 (duration 50, ends at 150)
        let c2_id = track.clips[1].id;
        track.ripple_remove_clip(c2_id).unwrap();

        assert_eq!(track.clips.len(), 2);
        // c1 remains at 0
        assert_eq!(track.clips[0].start_frame, 0);
        // c3 ripples from 200 - 50 = 150
        assert_eq!(track.clips[1].start_frame, 150);
        assert_eq!(track.duration_frames(), 200);
    }

    #[test]
    fn test_sequence_multitrack_composition() {
        let mut seq = Sequence::default_1080p24();
        assert_eq!(seq.tracks.len(), 4);
        assert_eq!(seq.duration_frames(), 0);

        let v1_id = seq.tracks[1].id;
        let a1_id = seq.tracks[2].id;
        let asset1 = AssetId::new();
        let asset2 = AssetId::new();

        let v_clip = test_clip(ClipId::new(), asset1, v1_id, 0, 0, 120);
        let a_clip = test_clip(ClipId::new(), asset2, a1_id, 24, 0, 96);

        seq.add_clip(v1_id, v_clip).unwrap();
        seq.add_clip(a1_id, a_clip).unwrap();

        assert_eq!(seq.duration_frames(), 120);
        assert_eq!(seq.duration_timecode(), "00:00:05:00");

        // Move audio clip
        let audio_clip_id = seq.tracks[2].clips[0].id;
        let a2_id = seq.tracks[3].id;
        seq.move_clip(audio_clip_id, a2_id, 48).unwrap();
        assert_eq!(seq.tracks[3].clips.len(), 1);
        assert_eq!(seq.tracks[3].clips[0].start_frame, 48);
    }

    #[test]
    fn test_snapping_engine() {
        let mut seq = Sequence::default_1080p24();
        let v1_id = seq.tracks[1].id;
        let clip1_id = ClipId::new();
        let clip1 = test_clip(clip1_id, AssetId::new(), v1_id, 100, 0, 50); // [100, 150)
        seq.add_clip(v1_id, clip1).unwrap();

        // Snap candidate at 98 within 5 frames threshold -> snaps to 100 (ClipStart)
        let snap1 = snap_frame(98, &seq, Some(240), None, 5);
        assert_eq!(snap1.snapped_frame, 100);
        assert!(matches!(snap1.target, Some(SnapTarget::ClipStart { .. })));

        // Snap candidate at 2 within 5 frames -> snaps to 0 (SequenceStart)
        let snap0 = snap_frame(2, &seq, None, None, 5);
        assert_eq!(snap0.snapped_frame, 0);
        assert_eq!(snap0.target, Some(SnapTarget::SequenceStart));

        // Snap candidate at 238 within 5 frames of playhead (240) -> snaps to 240
        let snap_ph = snap_frame(238, &seq, Some(240), None, 5);
        assert_eq!(snap_ph.snapped_frame, 240);
        assert_eq!(snap_ph.target, Some(SnapTarget::Playhead(240)));

        // Candidate at 70 (too far from 0 and 100) -> no snap
        let snap_none = snap_frame(70, &seq, None, None, 5);
        assert_eq!(snap_none.snapped_frame, 70);
        assert_eq!(snap_none.target, None);
    }

    #[test]
    fn test_sequence_project_serialization_roundtrip() {
        let mut seq = Sequence::default_1080p24();
        let v1_id = seq.tracks[1].id;
        let clip = test_clip(ClipId::new(), AssetId::new(), v1_id, 0, 0, 120);
        seq.add_clip(v1_id, clip).unwrap();

        let mut doc = nodavideo_project::ProjectDocument::new("Test Timeline Project");
        seq.attach_to_project(&mut doc).unwrap();

        // Roundtrip to temp file
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("project_with_timeline.nodavideo");
        doc.save_to_file(&file_path).unwrap();

        let loaded_doc = nodavideo_project::ProjectDocument::load_from_file(&file_path).unwrap();
        let loaded_seq = Sequence::extract_from_project(&loaded_doc)
            .unwrap()
            .unwrap();

        assert_eq!(loaded_seq.id, seq.id);
        assert_eq!(loaded_seq.name, "Main Sequence");
        assert_eq!(loaded_seq.tracks.len(), 4);
        assert_eq!(loaded_seq.tracks[1].clips.len(), 1);
        assert_eq!(loaded_seq.tracks[1].clips[0].duration_frames(), 120);
    }

    #[test]
    fn test_clip_transform_and_audio_parameters() {
        let mut clip = test_clip(ClipId::new(), AssetId::new(), TrackId::new(), 0, 0, 48);
        assert!(clip.transform.is_identity());
        assert!(clip.audio_params.is_default());
        assert_eq!(clip.transform.opacity, 1.0);
        assert_eq!(clip.transform.scale_x, 1.0);
        assert_eq!(clip.audio_params.volume_db, 0.0);

        // Update transform
        let transform = ClipTransform {
            position_x: 120.0,
            position_y: -45.0,
            scale_x: 1.5,
            scale_y: 1.5,
            rotation_degrees: 45.0,
            opacity: 0.85,
        };
        clip.set_transform(transform);
        assert!(!clip.transform.is_identity());
        assert_eq!(clip.transform.position_x, 120.0);
        assert_eq!(clip.transform.position_y, -45.0);
        assert_eq!(clip.transform.opacity, 0.85);

        // Clamping check
        let unclamped = ClipTransform {
            position_x: 0.0,
            position_y: 0.0,
            scale_x: 0.0,
            scale_y: -1.0,
            rotation_degrees: 390.0,
            opacity: 1.5,
        };
        clip.set_transform(unclamped);
        assert_eq!(clip.transform.opacity, 1.0);
        assert_eq!(clip.transform.scale_x, 0.001);
        assert_eq!(clip.transform.rotation_degrees, 30.0);

        // Reset transform
        clip.reset_transform();
        assert!(clip.transform.is_identity());

        // Update audio params
        let audio = ClipAudioParams {
            volume_db: -6.0,
            pan: -0.5,
        };
        clip.set_audio_params(audio);
        assert!(!clip.audio_params.is_default());
        assert_eq!(clip.audio_params.volume_db, -6.0);
        assert_eq!(clip.audio_params.pan, -0.5);

        // Audio clamping
        clip.set_audio_params(ClipAudioParams {
            volume_db: 50.0,
            pan: -2.0,
        });
        assert_eq!(clip.audio_params.volume_db, 24.0);
        assert_eq!(clip.audio_params.pan, -1.0);

        clip.reset_audio_params();
        assert!(clip.audio_params.is_default());

        // Splitting preserves transform and audio params
        clip.set_transform(transform);
        clip.set_audio_params(audio);
        let (left, right) = clip.split(24, ClipId::new()).unwrap();
        assert_eq!(left.transform, transform);
        assert_eq!(right.transform, transform);
        assert_eq!(left.audio_params, audio);
        assert_eq!(right.audio_params, audio);
    }
}
