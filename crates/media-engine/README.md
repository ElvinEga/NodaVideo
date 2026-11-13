# `nodavideo-media-engine`

Media inspection, stream probing, thumbnail generation, audio waveform extraction, real-time timeline frame preview, broadcast-standard MP4 export, and SQLite-backed caching for **NodaVideo**.

## Architectural Contract

- **FFmpeg & Pure-Rust Pipeline:** Uses FFprobe and FFmpeg for video and multi-stream decoding, frame extraction, and export encoding, with pure-Rust image readers for fast image probing and scaling.
- **Central binary resolution:** Every FFmpeg/FFprobe invocation goes through `binaries::ffmpeg_command()` / `binaries::ffprobe_command()`. A GUI app launched from Finder/Explorer does not inherit a login-shell `PATH`, so a bare `Command::new("ffmpeg")` fails there. Resolution order is `NODAVIDEO_FFMPEG` / `NODAVIDEO_FFPROBE` → `PATH` → well-known install dirs (`/opt/homebrew/bin`, `/usr/local/bin`, Windows locations), cached with `OnceLock`. An explicit override that is invalid fails rather than silently falling back.
- **Off-Thread Processing:** All heavy media I/O, probing, thumbnailing, waveform generation, frame extraction, and export rendering run in background tasks off the main GPUI UI thread.
- **SQLite Media Cache:** Maintains a SQLite database (`media_cache.db`) indexing probed metadata, thumbnail references, and waveform peak data. Invalidation is tracked via source file modification time (`mtime`) and file size.
- **Thumbnails & Preview Frames:** Scaled down maintaining aspect ratio and cached to disk as JPEG for instant scrubbing and playback recall. Cache keys include the frame rate; writes use a temp file plus atomic rename; zero-byte cache files are rejected.
- **Waveforms:** 8kHz mono PCM decoded via FFmpeg, bucketed into normalized peak points (`WaveformData`, 100 points/second) and cached as `.wave` files on disk.
- **Multitrack Export:** Dynamic FFmpeg filtergraph composing a black background canvas, frame-accurate clip trimming, video scaling/padding, track layering (e.g. V2 over V1), audio delay alignment (`adelay`), and audio mixing (`amix=...:normalize=0`), outputting `libx264` (`yuv420p` + `+faststart`) and `aac` audio, with an explicit output `fps` filter and CFR mode.
- **Frame-exact time boundaries:** `time_math` holds pure helpers (`frames_to_seconds`, `format_seconds`, `retiming_factor`, `timeline_to_source_frame`). Sequence fps is used for timeline frames and asset fps for source frames; the retiming factor is applied to `setpts` (video) and `atempo` (audio). Floating-point seconds exist only at the FFmpeg CLI boundary, never in the project model.

## Key APIs

- `probe_media_file(path: &Path) -> Result<MediaAsset, MediaEngineError>`
- `generate_thumbnail(source: &Path, id: AssetId, type: MediaType, duration: Option<TimeRational>, cache_dir: &Path) -> Result<PathBuf, MediaEngineError>`
- `generate_waveform(source: &Path, id: AssetId, duration: Option<TimeRational>, cache_dir: &Path) -> Result<(WaveformData, PathBuf), MediaEngineError>`
- `resolve_active_video_clip(sequence: &Sequence, frame: i64) -> Option<(TrackId, Clip)>`
- `resolve_renderable_video_clip(sequence: &Sequence, project: &ProjectDocument, frame: i64) -> Option<(TrackId, Clip)>` — skips text/title/caption overlays, disabled clips and assets without a renderable video stream, so an overlay-only frame returns `None` instead of blanking the preview.
- `extract_preview_frame(asset: &MediaAsset, source_frame: i64, cache_dir: &Path) -> Result<PathBuf, MediaEngineError>`
- `render_sequence_frame(sequence: &Sequence, project: &ProjectDocument, timeline_frame: i64, cache_dir: &Path) -> Result<Option<PathBuf>, MediaEngineError>`
- `export_sequence(sequence: &Sequence, project: &ProjectDocument, settings: &ExportSettings) -> Result<ExportReport, MediaEngineError>`
- `binaries::{ffmpeg_command, ffprobe_command, ffmpeg_path, ffprobe_path, is_ffmpeg_available, is_ffprobe_available}` and the `NODAVIDEO_FFMPEG` / `NODAVIDEO_FFPROBE` overrides.
- `time_math::{frames_to_seconds, format_seconds, retiming_factor, timeline_to_source_frame}`.
- `MediaCacheDb`: Thread-safe SQLite index cache with `get_by_path`, `put`, and `invalidate`.
- `MediaProcessor`: High-level coordinator combining cache lookups, probing, thumbnailing, waveform extraction, preview rendering, and sequence export.

## Known limitations

`render_sequence_frame` spawns one FFmpeg process and writes one JPEG per
requested frame, and renders only the topmost renderable video clip. It is a
still-frame preview, **not** a real-time playback engine. There is no audio
output, no A/V sync, and no compositing of multiple tracks, transitions, titles,
captions or transforms; export builds a separate FFmpeg filtergraph and therefore
does not match the preview. See `docs/PLAYBACK_ENGINE_DESIGN.md` (status:
PROPOSED) and the "Reality check" in `AGENTS.md`.
