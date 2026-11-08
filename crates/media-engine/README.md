# `nodavideo-media-engine`

Media inspection, stream probing, thumbnail generation, audio waveform extraction, and SQLite-backed caching for **NodaVideo**.

## Architectural Contract

- **FFmpeg & Pure-Rust Pipeline:** Uses FFprobe and FFmpeg for video and multi-stream decoding, with pure-Rust image readers for fast image probing and scaling.
- **Off-Thread Processing:** All heavy media I/O, probing, thumbnailing, and waveform generation run in background tasks off the main UI thread.
- **SQLite Media Cache:** Maintains a SQLite database (`media_cache.db`) indexing probed metadata, thumbnail references, and waveform peak data. Invalidation is tracked via source file modification time (`mtime`) and file size.
- **Thumbnails:** Scaled down maintaining aspect ratio (max 320px) and cached to disk as JPEG.
- **Waveforms:** 8kHz mono PCM decoded via FFmpeg, bucketed into normalized peak points (`WaveformData`, 100 points/second) and cached as `.wave` files on disk.

## Key APIs

- `probe_media_file(path: &Path) -> Result<MediaAsset, MediaEngineError>`
- `generate_thumbnail(source: &Path, id: AssetId, type: MediaType, duration: Option<TimeRational>, cache_dir: &Path) -> Result<PathBuf, MediaEngineError>`
- `generate_waveform(source: &Path, id: AssetId, duration: Option<TimeRational>, cache_dir: &Path) -> Result<(WaveformData, PathBuf), MediaEngineError>`
- `MediaCacheDb`: Thread-safe SQLite index cache with `get_by_path`, `put`, and `invalidate`.
- `MediaProcessor`: High-level coordinator combining cache lookups, probing, thumbnailing, and waveform extraction.
