//! Headless composition rendering engine.
//!
//! Compositions render natively: the template's SVG source is parameter-substituted
//! and rasterized with resvg, producing transparent RGBA frames with no external
//! browser or process. The HTML document export is retained for interop.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use image::RgbaImage;

use crate::composition::Composition;
use crate::error::HyperframesError;
use crate::template::TemplateRegistry;

/// Headless renderer producing rendered video or image assets for compositions.
#[derive(Debug, Clone, Default)]
pub struct CompositionRenderer;

// Public APIs only
impl CompositionRenderer {
    /// Create a new composition renderer instance.
    pub fn new() -> Self {
        Self
    }

    /// Render a composition to an output HTML artifact file.
    pub fn render_html_document(
        &self,
        composition: &Composition,
        registry: &TemplateRegistry,
        output_dir: &Path,
    ) -> Result<PathBuf, HyperframesError> {
        let template = registry
            .get(&composition.template_id)
            .ok_or_else(|| HyperframesError::TemplateNotFound(composition.template_id.clone()))?;

        template.validate_parameters(&composition.parameters)?;
        let inner_markup = template.render_html(&composition.parameters);

        let full_doc = format!(
            r#"<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<style>
* {{ box-sizing: border-box; margin: 0; padding: 0; }}
body {{ width: {width}px; height: {height}px; background: transparent; overflow: hidden; }}
</style>
</head>
<body>
{inner_markup}
</body>
</html>"#,
            width = composition.width,
            height = composition.height
        );

        fs::create_dir_all(output_dir)?;
        let out_path = output_dir.join(format!("comp_{}.html", composition.id));
        fs::write(&out_path, full_doc)?;
        Ok(out_path)
    }

    /// Build the composition's standalone SVG document (transparent background).
    pub fn render_svg_string(
        &self,
        composition: &Composition,
        registry: &TemplateRegistry,
    ) -> Result<String, HyperframesError> {
        self.svg_document(composition, registry, composition.width, composition.height)
    }

    /// Rasterize a composition at a reduced size (thumbnails), preserving aspect via viewBox.
    pub fn render_thumbnail_rgba(
        &self,
        composition: &Composition,
        registry: &TemplateRegistry,
        width: u32,
        height: u32,
    ) -> Result<RgbaImage, HyperframesError> {
        let svg = self.svg_document(composition, registry, width, height)?;
        Self::rasterize_svg(&svg, width, height)
    }

    /// Render a composition to an output SVG artifact file.
    pub fn render_svg_document(
        &self,
        composition: &Composition,
        registry: &TemplateRegistry,
        output_dir: &Path,
    ) -> Result<PathBuf, HyperframesError> {
        let svg = self.render_svg_string(composition, registry)?;
        fs::create_dir_all(output_dir)?;
        let out_path = output_dir.join(format!("comp_{}.svg", composition.id));
        fs::write(&out_path, svg)?;
        Ok(out_path)
    }

    /// Rasterize a composition to a transparent RGBA frame at its native size.
    pub fn render_frame_rgba(
        &self,
        composition: &Composition,
        registry: &TemplateRegistry,
    ) -> Result<RgbaImage, HyperframesError> {
        let svg = self.render_svg_string(composition, registry)?;
        Self::rasterize_svg(&svg, composition.width, composition.height)
    }

    /// Render a composition to a PNG media asset for timeline attachment.
    pub fn render_media_asset(
        &self,
        composition: &mut Composition,
        registry: &TemplateRegistry,
        output_dir: &Path,
    ) -> Result<PathBuf, HyperframesError> {
        self.render_svg_document(composition, registry, output_dir)?;
        let frame = self.render_frame_rgba(composition, registry)?;

        let asset_path = output_dir.join(format!("comp_{}.png", composition.id));
        frame
            .save(&asset_path)
            .map_err(|error| HyperframesError::RenderFailed(error.to_string()))?;

        composition.rendered_asset_path = Some(asset_path.clone());
        Ok(asset_path)
    }
}

// Private APIs only
impl CompositionRenderer {
    fn svg_document(
        &self,
        composition: &Composition,
        registry: &TemplateRegistry,
        width: u32,
        height: u32,
    ) -> Result<String, HyperframesError> {
        let template = registry
            .get(&composition.template_id)
            .ok_or_else(|| HyperframesError::TemplateNotFound(composition.template_id.clone()))?;

        template.validate_parameters(&composition.parameters)?;
        if template.svg_template.is_empty() {
            return Err(HyperframesError::RenderFailed(format!(
                "template '{}' has no SVG source",
                template.id
            )));
        }

        let inner_markup = template.render_svg(&Self::escaped_parameters(&composition.parameters));
        Ok(format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {view_width} {view_height}">{inner_markup}</svg>"#,
            view_width = composition.width,
            view_height = composition.height
        ))
    }

    fn escaped_parameters(
        params: &HashMap<String, serde_json::Value>,
    ) -> HashMap<String, serde_json::Value> {
        params
            .iter()
            .map(|(key, value)| {
                let escaped = match value {
                    serde_json::Value::String(text) => {
                        serde_json::Value::String(Self::escape_xml(text))
                    }
                    other => other.clone(),
                };
                (key.clone(), escaped)
            })
            .collect()
    }

    fn escape_xml(text: &str) -> String {
        text.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&apos;")
    }

    fn rasterize_svg(svg: &str, width: u32, height: u32) -> Result<RgbaImage, HyperframesError> {
        if width > 16384 || height > 16384 {
            return Err(HyperframesError::RenderFailed(
                "SVG dimensions exceed 16384".into(),
            ));
        }
        let mut fontdb = resvg::usvg::fontdb::Database::new();
        fontdb.load_system_fonts();
        let options = resvg::usvg::Options {
            fontdb: std::sync::Arc::new(fontdb),
            ..Default::default()
        };
        let tree = resvg::usvg::Tree::from_data(svg.as_bytes(), &options)
            .map_err(|error| HyperframesError::RenderFailed(error.to_string()))?;
        let Some(mut pixmap) = resvg::tiny_skia::Pixmap::new(width, height) else {
            return Err(HyperframesError::RenderFailed(
                "cannot allocate SVG raster".into(),
            ));
        };
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::default(),
            &mut pixmap.as_mut(),
        );

        let mut bytes = pixmap.take();
        for pixel in bytes.as_chunks_mut::<4>().0 {
            let alpha = pixel[3] as u32;
            for channel in &mut pixel[..3] {
                *channel = (*channel as u32 * 255 + alpha / 2)
                    .checked_div(alpha)
                    .unwrap_or(0)
                    .min(255) as u8;
            }
        }
        RgbaImage::from_raw(width, height, bytes)
            .ok_or_else(|| HyperframesError::RenderFailed("RGBA buffer size mismatch".into()))
    }
}
