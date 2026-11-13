use crate::editor::Editor;
use crate::explorer_file_entry::FileTreeEntryKind;
use anyhow::{Context as _, Result};
use gpui::Context;
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

const THUMBNAIL_WIDTH: u32 = 320;

// Public APIs only
impl Editor {
    /// Queues videos that have no thumbnail yet. Generation runs one
    /// file at a time on the background executor, never on the UI thread.
    pub(crate) fn schedule_media_thumbnails(&mut self, cx: &mut Context<Self>) {
        let videos = self
            .explorer
            .imported_media
            .iter()
            .chain(self.explorer.file_tree.iter())
            .filter(|entry| entry.kind == FileTreeEntryKind::Video)
            .map(|entry| entry.absolute_path.clone())
            .collect::<Vec<_>>();
        let mut queued_any = false;
        for path in videos {
            if self.explorer.thumbnail_cache.contains_key(&path)
                || self.explorer.thumbnail_failed.contains(&path)
                || self.explorer.thumbnail_queue.iter().any(|queued| queued == &path)
            {
                continue;
            }
            if let Ok(cached_path) = thumbnail_cache_path(&path)
                && cached_path.is_file()
            {
                self.explorer.thumbnail_cache.insert(path, cached_path);
                continue;
            }
            self.explorer.thumbnail_queue.push_back(path);
            queued_any = true;
        }
        if queued_any {
            self.start_thumbnail_generation(cx);
        }
    }
}

// Private APIs only
impl Editor {
    fn start_thumbnail_generation(&mut self, cx: &mut Context<Self>) {
        if self.explorer.thumbnail_task.is_some() || self.explorer.thumbnail_queue.is_empty() {
            return;
        }
        let task = cx.spawn(async move |editor, cx| {
            loop {
                let next =
                    editor.update(cx, |editor, _| editor.explorer.thumbnail_queue.pop_front());
                let path = match next {
                    Ok(Some(path)) => path,
                    Ok(None) => break,
                    Err(error) => {
                        log::debug!("editor released: {error:?}");
                        return;
                    }
                };
                let source = path.clone();
                let result = cx
                    .background_executor()
                    .spawn(async move {
                        let destination = thumbnail_cache_path(&source)?;
                        if !destination.is_file() {
                            let directory = destination
                                .parent()
                                .context("thumbnail cache path has no parent")?;
                            fs::create_dir_all(directory)
                                .context("creating thumbnail cache directory")?;
                            // 先写临时文件再改名：中断不会留下被当作缓存命中的半截 PNG。
                            let temporary = destination
                                .with_file_name(format!("{}.tmp", ulid::Ulid::generate()));
                            let written = engine::thumbnail::video_thumbnail(
                                &source,
                                &temporary,
                                THUMBNAIL_WIDTH,
                            )
                            .and_then(|()| {
                                fs::rename(&temporary, &destination)
                                    .context("installing thumbnail cache file")
                            });
                            if let Err(error) = written {
                                let _ = fs::remove_file(&temporary);
                                return Err(error);
                            }
                        }
                        Ok::<_, anyhow::Error>(destination)
                    })
                    .await;
                let updated = editor.update(cx, |editor, cx| {
                    match result {
                        Ok(destination) => {
                            editor.explorer.thumbnail_cache.insert(path, destination);
                        }
                        Err(error) => {
                            log::error!(
                                "Could not generate thumbnail for {}: {error:?}",
                                path.display()
                            );
                            editor.explorer.thumbnail_failed.insert(path);
                        }
                    }
                    cx.notify();
                });
                if updated.is_err() {
                    return;
                }
            }
            if let Err(error) = editor.update(cx, |editor, cx| {
                editor.explorer.thumbnail_task = None;
                cx.notify();
            }) {
                log::debug!("editor released: {error:?}");
            }
        });
        self.explorer.thumbnail_task = Some(task);
    }
}

pub(crate) fn thumbnail_cache_dir() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".opencut").join("thumbnails")
    } else if let Ok(profile) = std::env::var("USERPROFILE") {
        PathBuf::from(profile).join(".opencut").join("thumbnails")
    } else {
        std::env::temp_dir().join("opencut").join("thumbnails")
    }
}

/// Cache key covers the source path, modification time and size, so edited media
/// regenerates.
pub(crate) fn thumbnail_cache_path(source: &Path) -> Result<PathBuf> {
    let metadata = fs::metadata(source).context("reading video metadata")?;
    let modified = metadata.modified().context("reading video modification time")?;
    let mut hasher = DefaultHasher::new();
    source.hash(&mut hasher);
    modified.hash(&mut hasher);
    metadata.len().hash(&mut hasher);
    let directory = thumbnail_cache_dir();
    Ok(directory.join(format!("{:016x}.png", hasher.finish())))
}
