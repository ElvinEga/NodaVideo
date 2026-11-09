//! Frame-accurate deterministic timeline snapping engine.

use crate::id::ClipId;
use crate::sequence::Sequence;

/// Classification of snap targets that magnetic snapping can attach to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapTarget {
    /// Beginning of the timeline (frame 0).
    SequenceStart,
    /// Current playhead position.
    Playhead(i64),
    /// Start frame of another clip on any track.
    ClipStart { clip_id: ClipId, frame: i64 },
    /// End frame of another clip on any track.
    ClipEnd { clip_id: ClipId, frame: i64 },
}

/// Result of a snapping calculation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnapResult {
    /// Snapped frame position (or original candidate frame if no snap occurred).
    pub snapped_frame: i64,
    /// Snap target hit, if within threshold.
    pub target: Option<SnapTarget>,
    /// Frame delta applied: `snapped_frame - original_candidate_frame`.
    pub delta: i64,
}

/// Snap an individual candidate frame to the nearest anchor point in the sequence.
pub fn snap_frame(
    candidate_frame: i64,
    sequence: &Sequence,
    playhead_frame: Option<i64>,
    excluded_clip: Option<ClipId>,
    threshold_frames: i64,
) -> SnapResult {
    if threshold_frames <= 0 {
        return SnapResult {
            snapped_frame: candidate_frame,
            target: None,
            delta: 0,
        };
    }

    let mut best_target: Option<SnapTarget> = None;
    let mut best_diff = threshold_frames + 1;
    let mut best_frame = candidate_frame;

    // 1. Sequence start (frame 0)
    let diff_0 = candidate_frame.abs();
    if diff_0 <= threshold_frames && diff_0 < best_diff {
        best_diff = diff_0;
        best_target = Some(SnapTarget::SequenceStart);
        best_frame = 0;
    }

    // 2. Playhead frame
    if let Some(ph) = playhead_frame {
        let diff_ph = (candidate_frame - ph).abs();
        if diff_ph <= threshold_frames && diff_ph < best_diff {
            best_diff = diff_ph;
            best_target = Some(SnapTarget::Playhead(ph));
            best_frame = ph;
        }
    }

    // 3. Clip boundaries across all tracks
    for track in &sequence.tracks {
        for clip in &track.clips {
            if Some(clip.id) == excluded_clip {
                continue;
            }

            // Clip start
            let diff_start = (candidate_frame - clip.start_frame).abs();
            if diff_start <= threshold_frames && diff_start < best_diff {
                best_diff = diff_start;
                best_target = Some(SnapTarget::ClipStart {
                    clip_id: clip.id,
                    frame: clip.start_frame,
                });
                best_frame = clip.start_frame;
            }

            // Clip end
            let end = clip.end_frame();
            let diff_end = (candidate_frame - end).abs();
            if diff_end <= threshold_frames && diff_end < best_diff {
                best_diff = diff_end;
                best_target = Some(SnapTarget::ClipEnd {
                    clip_id: clip.id,
                    frame: end,
                });
                best_frame = end;
            }
        }
    }

    SnapResult {
        snapped_frame: best_frame,
        target: best_target,
        delta: best_frame - candidate_frame,
    }
}

/// Snap a moving clip by evaluating both its leading edge (`start_frame`) and trailing edge (`end_frame`).
pub fn snap_clip_move(
    candidate_start_frame: i64,
    clip_duration: i64,
    sequence: &Sequence,
    playhead_frame: Option<i64>,
    moving_clip_id: ClipId,
    threshold_frames: i64,
) -> SnapResult {
    let candidate_start = candidate_start_frame.max(0);
    let candidate_end = candidate_start + clip_duration;

    // Test leading edge
    let snap_leading = snap_frame(
        candidate_start,
        sequence,
        playhead_frame,
        Some(moving_clip_id),
        threshold_frames,
    );

    // Test trailing edge
    let snap_trailing = snap_frame(
        candidate_end,
        sequence,
        playhead_frame,
        Some(moving_clip_id),
        threshold_frames,
    );

    match (snap_leading.target, snap_trailing.target) {
        (Some(_), None) => snap_leading,
        (None, Some(_)) => SnapResult {
            snapped_frame: (snap_trailing.snapped_frame - clip_duration).max(0),
            target: snap_trailing.target,
            delta: snap_trailing.delta,
        },
        (Some(_), Some(_)) => {
            if snap_leading.delta.abs() <= snap_trailing.delta.abs() {
                snap_leading
            } else {
                SnapResult {
                    snapped_frame: (snap_trailing.snapped_frame - clip_duration).max(0),
                    target: snap_trailing.target,
                    delta: snap_trailing.delta,
                }
            }
        }
        (None, None) => SnapResult {
            snapped_frame: candidate_start,
            target: None,
            delta: 0,
        },
    }
}
