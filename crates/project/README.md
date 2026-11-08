# `nodavideo-project`

Persistent project document model, media asset references, and schema migrations for **NodaVideo**.

## Architectural Contract

- **Canonical Truth:** The project state is stored as a versioned JSON document (`ProjectDocument`).
- **Schema Versioning:** Every document carries a `schema_version` (currently `1`). Schema changes include deterministic migrations.
- **Safety & Atomicity:** Project saves write first to a temporary file (`.{filename}.tmp`) and atomically rename into place to prevent corruption.
- **Pure Domain & Timing:** Media duration and sequence timings are stored as integer frames or exact rational time (`TimeRational`), never floating-point seconds.
- **Portability:** Holds references to original media files without embedding large binary assets or blobs. Missing assets are detected upon load and marked as `MediaAssetStatus::Missing`.

## Key Types

- `ProjectId`, `AssetId`: Strongly typed, opaque UUID newtypes.
- `TimeRational`: Exact rational time representation (`numerator: i64`, `denominator: u32`).
- `FrameRate`: Rational video frame rate with standard constants (`FPS_24`, `FPS_25`, `FPS_29_97`, `FPS_30`, `FPS_50`, `FPS_59_94`, `FPS_60`).
- `MediaType`: `Video`, `Audio`, `Image` enumeration with extension inference.
- `MediaAsset`: Metadata record for an imported media source.
- `ProjectDocument`: Root project entity with `save_to_file`, `load_from_file`, and asset management methods.
