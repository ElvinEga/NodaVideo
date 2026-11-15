use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

use anyhow::Context as _;
use gpui::Context;
use nodavideo_hyperframes_bridge::{Composition, CompositionRenderer, template_registry};
use timeline::Clip;

use crate::editor::Editor;
use crate::explorer::{CompositionThumbJob, MediaCategoryFilter};
use crate::media_thumbnail::thumbnail_cache_dir;

const THUMBNAIL_WIDTH: u32 = 320;
const THUMBNAIL_HEIGHT: u32 = 180;

// Public APIs only
impl Editor {
    /// Queues composition and gallery-template thumbnails that are not cached yet.
    /// Rasterization runs one job at a time on the background executor, never on the UI thread.
    pub(crate) fn schedule_composition_thumbnails(&mut self, cx: &mut Context<Self>) {
        let show_compositions = matches!(
            self.explorer.category,
            MediaCategoryFilter::All | MediaCategoryFilter::Motion
        );
        let mut jobs: Vec<CompositionThumbJob> = Vec::new();
        if show_compositions {
            for asset in &self.explorer.compositions {
                jobs.push(CompositionThumbJob::Composition(asset.id));
            }
        }
        // 已放置到时间线的合成无论当前分类都要有缓存，预览叠加层复用同一缩略图。
        if let Some(timeline) = self.timeline.as_ref() {
            for clip in &timeline.editing_state.clips {
                if let Clip::Composition(composition) = clip {
                    jobs.push(CompositionThumbJob::Composition(composition.composition_id));
                }
            }
        }
        if let Some(gallery) = &self.explorer.template_gallery {
            for template in &gallery.templates {
                jobs.push(CompositionThumbJob::Template(template.clone()));
            }
        }

        let mut queued_any = false;
        for job in jobs {
            if self.composition_thumb_cached(&job) || self.composition_thumb_queued(&job) {
                continue;
            }
            let key = self.composition_thumb_key(&job);
            let Some(key) = key else {
                continue;
            };
            let cached = composition_thumb_cache_path(&key);
            if cached.is_file() {
                self.insert_composition_thumb(&job, cached);
                continue;
            }
            self.explorer.composition_thumb_queue.push_back(job);
            queued_any = true;
        }
        if queued_any {
            self.start_composition_thumbnail_generation(cx);
        }
    }
}

// Private APIs only
impl Editor {
    fn composition_thumb_key(&self, job: &CompositionThumbJob) -> Option<String> {
        match job {
            CompositionThumbJob::Composition(id) => {
                let asset = self
                    .explorer
                    .compositions
                    .iter()
                    .find(|asset| asset.id == *id)?;
                Some(format!(
                    "{}:{}:{}x{}",
                    asset.template_id,
                    serde_json::to_string(&asset.parameters).unwrap_or_default(),
                    asset.width,
                    asset.height
                ))
            }
            CompositionThumbJob::Template(template) => Some(format!("template:{}", template.id)),
        }
    }

    fn composition_thumb_cached(&self, job: &CompositionThumbJob) -> bool {
        match job {
            CompositionThumbJob::Composition(id) => {
                self.explorer.composition_thumbs.contains_key(id)
                    || self.explorer.composition_thumb_failed.contains(id)
            }
            CompositionThumbJob::Template(template) => {
                self.explorer.gallery_thumbs.contains_key(&template.id)
                    || self.explorer.gallery_thumb_failed.contains(&template.id)
            }
        }
    }

    fn composition_thumb_queued(&self, job: &CompositionThumbJob) -> bool {
        self.explorer
            .composition_thumb_queue
            .iter()
            .any(|queued| queued == job)
    }

    fn insert_composition_thumb(&mut self, job: &CompositionThumbJob, path: PathBuf) {
        match job {
            CompositionThumbJob::Composition(id) => {
                self.explorer.composition_thumbs.insert(*id, path);
            }
            CompositionThumbJob::Template(template) => {
                self.explorer
                    .gallery_thumbs
                    .insert(template.id.clone(), path);
            }
        }
    }

    fn start_composition_thumbnail_generation(&mut self, cx: &mut Context<Self>) {
        if self.explorer.composition_thumb_task.is_some()
            || self.explorer.composition_thumb_queue.is_empty()
        {
            return;
        }
        let task = cx.spawn(async move |editor, cx| {
            loop {
                let next = editor.update(cx, |editor, _| {
                    editor.explorer.composition_thumb_queue.pop_front()
                });
                let job = match next {
                    Ok(Some(job)) => job,
                    Ok(None) => break,
                    Err(error) => {
                        log::debug!("editor released: {error:?}");
                        return;
                    }
                };
                let key = match editor.update(cx, |editor, _| editor.composition_thumb_key(&job)) {
                    Ok(Some(key)) => key,
                    Ok(None) => continue,
                    Err(error) => {
                        log::debug!("editor released: {error:?}");
                        return;
                    }
                };
                let composition =
                    match editor.update(cx, |editor, _| editor.composition_for_thumb(&job)) {
                        Ok(Some(composition)) => composition,
                        Ok(None) => continue,
                        Err(error) => {
                            log::debug!("editor released: {error:?}");
                            return;
                        }
                    };

                let result = cx
                    .background_executor()
                    .spawn(async move {
                        let destination = composition_thumb_cache_path(&key);
                        if !destination.is_file() {
                            let directory = destination
                                .parent()
                                .context("composition thumbnail cache path has no parent")?;
                            fs::create_dir_all(directory)
                                .context("creating composition thumbnail cache directory")?;
                            let registry = template_registry();
                            let renderer = CompositionRenderer::new();
                            let frame = renderer.render_thumbnail_rgba(
                                &composition,
                                &registry,
                                THUMBNAIL_WIDTH,
                                THUMBNAIL_HEIGHT,
                            )?;
                            // 先写临时文件再改名：中断不会留下被当作缓存命中的半截 PNG。
                            let temporary = destination
                                .with_file_name(format!("{}.tmp", ulid::Ulid::generate()));
                            frame
                                .save(&temporary)
                                .context("writing composition thumbnail")?;
                            fs::rename(&temporary, &destination)
                                .context("installing composition thumbnail cache file")?;
                        }
                        Ok::<_, anyhow::Error>(destination)
                    })
                    .await;

                let updated = editor.update(cx, |editor, cx| {
                    match result {
                        Ok(destination) => editor.insert_composition_thumb(&job, destination),
                        Err(error) => {
                            log::error!("Could not render composition thumbnail: {error:?}");
                            match &job {
                                CompositionThumbJob::Composition(id) => {
                                    editor.explorer.composition_thumb_failed.insert(*id);
                                }
                                CompositionThumbJob::Template(template) => {
                                    editor
                                        .explorer
                                        .gallery_thumb_failed
                                        .insert(template.id.clone());
                                }
                            }
                        }
                    }
                    cx.notify();
                });
                if updated.is_err() {
                    return;
                }
            }
            if let Err(error) = editor.update(cx, |editor, cx| {
                editor.explorer.composition_thumb_task = None;
                cx.notify();
            }) {
                log::debug!("editor released: {error:?}");
            }
        });
        self.explorer.composition_thumb_task = Some(task);
    }

    fn composition_for_thumb(&self, job: &CompositionThumbJob) -> Option<Composition> {
        match job {
            CompositionThumbJob::Composition(id) => self
                .explorer
                .compositions
                .iter()
                .find(|asset| asset.id == *id)
                .map(|asset| asset.to_composition()),
            CompositionThumbJob::Template(template) => {
                Some(Composition::new(template, &template.name))
            }
        }
    }
}

fn composition_thumb_cache_path(key: &str) -> PathBuf {
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    thumbnail_cache_dir()
        .join("compositions")
        .join(format!("{:016x}.png", hasher.finish()))
}
