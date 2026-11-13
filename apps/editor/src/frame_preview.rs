use crate::editor::Editor;
use crate::media_thumbnail::thumbnail_cache_dir;
use crate::timeline::{TimelineFrameIndex, TimelineRuntimeState};
use crate::timeline_clip::Clip;
use crate::timeline_document::resolve_asset_path;
use crate::track::TrackKind;
use ::timeline::MediaKind;
use anyhow::{Context as _, Result};
use gpui::{Context, RenderImage, Task};
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::Arc;

/// 解码宽度与媒体库缩略图一致；悬停预览与胶片条单元格都小于该宽度。
const FRAME_WIDTH: u32 = 320;
const MAX_CACHED_FRAMES: usize = 512;
const MAX_QUEUED_FRAMES: usize = 256;

/// 一帧源画面：素材绝对路径 + 源时间（微秒，AV_TIME_BASE）。键不含时间线位置，
/// 同一素材被多个片段引用时共享缓存。
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct FrameKey {
    pub(crate) path: PathBuf,
    pub(crate) at_micros: i64,
}

/// 悬停位置要显示的内容：视频帧、图片原图，或空轨道的黑场。
pub(crate) enum HoverPreview {
    Video { key: FrameKey, aspect: f32 },
    Image { path: PathBuf, aspect: f32 },
    Empty,
}

pub(crate) struct FramePreviewCache {
    frames: HashMap<FrameKey, Arc<RenderImage>>,
    order: VecDeque<FrameKey>, // 插入顺序；超出上限时淘汰最旧帧，内存有界。
    failed: HashSet<FrameKey>, // 本次会话内不再重试。
    queue: VecDeque<FrameRequest>,
    task: Option<Task<()>>,
}

struct FrameRequest {
    key: FrameKey,
    hover: bool,
}

// Public APIs only
impl FramePreviewCache {
    pub(crate) fn new() -> Self {
        Self {
            frames: HashMap::new(),
            order: VecDeque::new(),
            failed: HashSet::new(),
            queue: VecDeque::new(),
            task: None,
        }
    }
}

// Private APIs only
impl FramePreviewCache {
    fn insert(&mut self, key: FrameKey, image: Arc<RenderImage>) {
        if self.frames.insert(key.clone(), image).is_some() {
            return;
        }
        self.order.push_back(key);
        if self.order.len() > MAX_CACHED_FRAMES
            && let Some(oldest) = self.order.pop_front()
        {
            self.frames.remove(&oldest);
        }
    }
}

// Public APIs only
impl Editor {
    /// 请求一帧源画面。生成在后台执行器上逐帧进行，绝不阻塞 UI 线程；
    /// 悬停请求取代排队中的旧悬停请求，快速移动指针时不积压解码。
    pub(crate) fn request_frame(&mut self, key: FrameKey, hover: bool, cx: &mut Context<Self>) {
        let cache = &mut self.frame_previews;
        if cache.frames.contains_key(&key)
            || cache.failed.contains(&key)
            || cache.queue.iter().any(|request| request.key == key)
        {
            return;
        }
        if hover {
            cache.queue.retain(|request| !request.hover);
        }
        if cache.queue.len() >= MAX_QUEUED_FRAMES {
            cache.queue.pop_front();
        }
        cache.queue.push_back(FrameRequest { key, hover });
        self.start_frame_generation(cx);
    }

    pub(crate) fn frame_image(&self, key: &FrameKey) -> Option<Arc<RenderImage>> {
        self.frame_previews.frames.get(key).cloned()
    }

    pub(crate) fn frame_failed(&self, key: &FrameKey) -> bool {
        self.frame_previews.failed.contains(key)
    }
}

// Private APIs only
impl Editor {
    fn start_frame_generation(&mut self, cx: &mut Context<Self>) {
        if self.frame_previews.task.is_some() || self.frame_previews.queue.is_empty() {
            return;
        }
        let task = cx.spawn(async move |editor, cx| {
            loop {
                let next = editor.update(cx, |editor, _| editor.frame_previews.queue.pop_front());
                let request = match next {
                    Ok(Some(request)) => request,
                    Ok(None) => break,
                    Err(error) => {
                        log::debug!("editor released: {error:?}");
                        return;
                    }
                };
                let key = request.key;
                let decode_key = key.clone();
                let result = cx
                    .background_executor()
                    .spawn(async move {
                        let destination = frame_cache_path(&decode_key)?;
                        if !destination.is_file() {
                            let directory = destination
                                .parent()
                                .context("frame cache path has no parent")?;
                            fs::create_dir_all(directory)
                                .context("creating frame cache directory")?;
                            // 先写临时文件再改名：中断不会留下被当作缓存命中的半截 PNG。
                            let temporary = destination
                                .with_file_name(format!("{}.tmp", ulid::Ulid::generate()));
                            let written = engine::thumbnail::video_thumbnail_at(
                                &decode_key.path,
                                &temporary,
                                FRAME_WIDTH,
                                decode_key.at_micros,
                            )
                            .and_then(|()| {
                                fs::rename(&temporary, &destination)
                                    .context("installing frame cache file")
                            });
                            if let Err(error) = written {
                                let _ = fs::remove_file(&temporary);
                                return Err(error);
                            }
                        }
                        let pixels = image::open(&destination)
                            .context("decoding cached frame")?
                            .to_rgba8();
                        Ok::<_, anyhow::Error>(Arc::new(RenderImage::new(vec![image::Frame::new(
                            pixels,
                        )])))
                    })
                    .await;
                let updated = editor.update(cx, |editor, cx| {
                    match result {
                        Ok(image) => editor.frame_previews.insert(key, image),
                        Err(error) => {
                            log::error!(
                                "Could not generate frame preview for {} at {}us: {error:?}",
                                key.path.display(),
                                key.at_micros
                            );
                            editor.frame_previews.failed.insert(key);
                        }
                    }
                    cx.notify();
                });
                if updated.is_err() {
                    return;
                }
            }
            if let Err(error) = editor.update(cx, |editor, cx| {
                editor.frame_previews.task = None;
                cx.notify();
            }) {
                log::debug!("editor released: {error:?}");
            }
        });
        self.frame_previews.task = Some(task);
    }
}

/// 悬停位置最上层视频轨片段的预览内容；没有可预览片段时为空。
pub(crate) fn hover_preview_at(
    timeline: &TimelineRuntimeState,
    position: TimelineFrameIndex,
) -> HoverPreview {
    let state = &timeline.editing_state;
    let Some(timeline_directory) = timeline.path.parent() else {
        return HoverPreview::Empty;
    };
    for track in &state.tracks {
        if track.kind != TrackKind::Video {
            continue;
        }
        for clip in &state.clips {
            if clip.track_id() != track.id || timeline.clip_is_offline(clip) {
                continue;
            }
            let start = clip.timeline_start();
            let end = start + clip.frame_length(state.settings.frame_rate);
            if position < start || position >= end {
                continue;
            }
            let Clip::Video(media) = clip else {
                continue;
            };
            let Some(asset) = state.asset(media.asset_id) else {
                continue;
            };
            let path = resolve_asset_path(timeline_directory, asset.path());
            let aspect = asset.width() as f32 / asset.height().max(1) as f32;
            match asset.kind() {
                MediaKind::Image => return HoverPreview::Image { path, aspect },
                MediaKind::Video => {
                    let source_frame = media.source_in + (position - start);
                    let at_micros = (state.seconds(source_frame) * 1_000_000.0) as i64;
                    return HoverPreview::Video {
                        key: FrameKey { path, at_micros },
                        aspect,
                    };
                }
                MediaKind::Audio => continue,
            }
        }
    }
    HoverPreview::Empty
}

/// 缓存键覆盖源路径、修改时间、大小与源时间，素材被编辑后重新生成。
fn frame_cache_path(key: &FrameKey) -> Result<PathBuf> {
    let metadata = fs::metadata(&key.path).context("reading video metadata")?;
    let modified = metadata
        .modified()
        .context("reading video modification time")?;
    let mut hasher = DefaultHasher::new();
    key.path.hash(&mut hasher);
    modified.hash(&mut hasher);
    metadata.len().hash(&mut hasher);
    key.at_micros.hash(&mut hasher);
    Ok(thumbnail_cache_dir()
        .join("frames")
        .join(format!("{:016x}.png", hasher.finish())))
}
