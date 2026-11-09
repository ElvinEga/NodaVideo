# `nodavideo-media-engine`

Media inspection, stream probing, thumbnail generation, audio waveform extraction, real-time timeline frame preview, broadcast-standard MP4 export, and SQLite-backed caching for **NodaVideo**.

## Architectural Contract

- **FFmpeg & Pure-Rust Pipeline:** Uses FFprobe and FFmpeg for video and multi-stream decoding, frame extraction, and export encoding, with pure-Rust image readers for fast image probing and scaling.
- **Off-Thread Processing:** All heavy media I/O, probing, thumbnailing, waveform generation, frame extraction, and export rendering run in background tasks off the main GPUI UI thread.
- **SQLite Media Cache:** Maintains a SQLite database (`media_cache.db`) indexing probed metadata, thumbnail references, and waveform peak data. Invalidation is tracked via source file modification time (`mtime`) and file size.
- **Thumbnails & Preview Frames:** Scaled down maintaining aspect ratio and cached to disk as JPEG for instant scrubbing and playback recall.
- **Waveforms:** 8kHz mono PCM decoded via FFmpeg, bucketed into normalized peak points (`WaveformData`, 100 points/second) and cached as `.wave` files on disk.
- **Multitrack Export:** Dynamic FFmpeg filtergraph composing a black background canvas, frame-accurate clip trimming, video scaling/padding, track layering (e.g. V2 over V1), audio delay alignment (`adelay`), and audio mixing (`amix`), outputting `libx264` (`yuv420p` + `+faststart`) and `aac` audio.

## Key APIs

- `probe_media_file(path: &Path) -> Result<MediaAsset, MediaEngineError>`
- `generate_thumbnail(source: &Path, id: AssetId, type: MediaType, duration: Option<TimeRational>, cache_dir: &Path) -> Result<PathBuf, MediaEngineError>`
- `generate_waveform(source: &Path, id: AssetId, duration: Option<TimeRational>, cache_dir: &Path) -> Result<(WaveformData, PathBuf), MediaEngineError>`
- `resolve_active_video_clip(sequence: &Sequence, frame: i64) -> Option<(TrackId, Clip)>`
- `extract_preview_frame(asset: &MediaAsset, source_frame: i64, cache_dir: &Path) -> Result<PathBuf, MediaEngineError>`
- `render_sequence_frame(sequence: &Sequence, project: &ProjectDocument, timeline_frame: i64, cache_dir: &Path) -> Result<Option<PathBuf>, MediaEngineError>`
- `export_sequence(sequence: &Sequence, project: &ProjectDocument, settings: &ExportSettings) -> Result<ExportReport, MediaEngineError>`
- `MediaCacheDb`: Thread-safe SQLite index cache with `get_by_path`, `put`, and `invalidate`.
- `MediaProcessor`: High-level coordinator combining cache lookups, probing, thumbnailing, waveform extraction, preview rendering, and sequence export.
