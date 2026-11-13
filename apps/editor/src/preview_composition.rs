use crate::editor::Editor;
use gpui::prelude::*;
use gpui::{AvailableSpace, Context, Entity, ObjectFit, canvas, div, img, px};
use player_ui::timeline_player::TimelinePlayer;
use player_ui::timeline_player_view::TIMELINE_CONTROLS_HEIGHT;
use std::path::PathBuf;
use timeline::Clip;

impl Editor {
    /// Composites placed motion-graphics compositions over the footage.
    ///
    /// The engine skips composition clips, so the editor draws them here from the
    /// shared composition thumbnail cache. Rasterization happens on the background
    /// executor (see `schedule_composition_thumbnails`); this only paints cached PNGs
    /// and never blocks input. The canvas geometry matches `render_frame` so the
    /// overlay aligns with the footage underneath.
    pub(crate) fn preview_composition_overlay(
        &self,
        player: &Entity<TimelinePlayer>,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let Some(timeline) = self.timeline.as_ref() else {
            return div().into_any_element();
        };
        let frame = player.read(cx).backend().preview_frame();
        let frame_width = frame.width;
        let frame_height = frame.height;
        let playhead = timeline.playhead();
        let frame_rate = timeline.editing_state.settings.frame_rate;
        let images = timeline
            .editing_state
            .clips
            .iter()
            .filter_map(|clip| {
                let Clip::Composition(composition) = clip else {
                    return None;
                };
                if playhead < clip.timeline_start() || playhead >= clip.timeline_end(frame_rate) {
                    return None;
                }
                let track = timeline
                    .editing_state
                    .tracks
                    .iter()
                    .find(|track| track.id == composition.track_id)?;
                if !track.visible {
                    return None;
                }
                self.explorer
                    .composition_thumbs
                    .get(&composition.composition_id)
                    .cloned()
            })
            .collect::<Vec<PathBuf>>();
        if images.is_empty() {
            return div().into_any_element();
        }
        div()
            .id("preview-composition-overlay")
            .absolute()
            .top_0()
            .left_0()
            .right_0()
            .bottom(px(TIMELINE_CONTROLS_HEIGHT))
            .overflow_hidden()
            .child(
                canvas(
                    move |bounds, window, cx| {
                        let scale = (f32::from(bounds.size.width) / frame_width as f32)
                            .min(f32::from(bounds.size.height) / frame_height as f32);
                        let mut layer = div().absolute().inset_0();
                        if scale.is_finite() && scale > 0.0 {
                            let canvas_width = frame_width as f32 * scale;
                            let canvas_height = frame_height as f32 * scale;
                            let left = (f32::from(bounds.size.width) - canvas_width) / 2.0;
                            let top = (f32::from(bounds.size.height) - canvas_height) / 2.0;
                            for path in &images {
                                layer = layer.child(
                                    img(path.clone())
                                        .absolute()
                                        .left(px(left))
                                        .top(px(top))
                                        .w(px(canvas_width))
                                        .h(px(canvas_height))
                                        .object_fit(ObjectFit::Fill),
                                );
                            }
                        }
                        let mut element = layer.into_any_element();
                        element.prepaint_as_root(
                            bounds.origin,
                            bounds.size.map(AvailableSpace::Definite),
                            window,
                            cx,
                        );
                        element
                    },
                    |_, mut element, window, cx| element.paint(window, cx),
                )
                .size_full(),
            )
            .into_any_element()
    }
}
