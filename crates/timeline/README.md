# `nodavideo-timeline`

Pure, deterministic multitrack timeline domain model, tracks, clips, trimming, splitting, ripple editing, and magnetic snapping for **NodaVideo**.

## Architectural Contract

- **Zero Non-Deterministic Dependencies:** Zero wall-clock queries, zero random generation, zero floating-point accumulation, zero FFmpeg, and zero GPUI dependencies.
- **Deterministic Pure Domain:** Given `(sequence, operation)`, state transitions are 100% reproducible and testable in headless unit test environments.
- **Frame-Accurate Time:** All timeline positions, in/out points, and durations are stored as exact integer frames (`i64`) and rational frame rates (`FrameRate`), never floating-point seconds.
- **Single Source of Truth:** Clips reference imported media assets by [`AssetId`](../project) without duplicating media data.

## Key Types

- [`SequenceId`](src/id.rs), [`TrackId`](src/id.rs), [`ClipId`](src/id.rs): Strongly typed opaque newtypes wrapping UUIDs.
- [`TrackType`](src/track.rs): `Video` and `Audio` track classifications with track compatibility rules.
- [`Track`](src/track.rs): Ordered list of clips, mute, solo, lock, and visibility states.
- [`Clip`](src/clip.rs): Placed segment with `start_frame`, `source_in`, `source_out`, and derived `duration_frames = source_out - source_in`.
- [`Sequence`](src/sequence.rs): Multitrack sequence coordinating track layout, duration calculation, razor split, trimming, moving, and project JSON serialization.
- [`snap_frame`](src/snap.rs), [`snap_clip_move`](src/snap.rs): Magnetic snapping engine anchoring to sequence start, playhead, and clip edges.
- [`TimelineSelection`](src/selection.rs): Playhead frame position and single/multi-clip selection tracking.

## Editing Invariants

- `start_frame >= 0`
- `source_in >= 0`
- `source_out > source_in` (clip duration must be >= 1 frame)
- Locked tracks reject all editing operations.
- Trimming the in-point shifts `start_frame` by delta to keep the clip tail stationary.
- Splitting at a timeline frame divides one clip into two adjacent clips without altering temporal continuity.
