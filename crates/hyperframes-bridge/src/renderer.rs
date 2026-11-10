//! Headless composition rendering engine.

use std::fs;
use std::path::{Path, PathBuf};

use crate::composition::Composition;
use crate::error::HyperframesError;
use crate::template::TemplateRegistry;

/// Headless renderer producing rendered video or image assets for compositions.
#[derive(Debug, Clone, Default)]
pub struct CompositionRenderer;

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

    /// Render composition to an output media file (.mp4 or .png sequence placeholder).
    /// Generates a valid test media artifact for timeline attachment.
    pub fn render_media_asset(
        &self,
        composition: &mut Composition,
        registry: &TemplateRegistry,
        output_dir: &Path,
    ) -> Result<PathBuf, HyperframesError> {
        let html_path = self.render_html_document(composition, registry, output_dir)?;

        // Output destination for the rendered composition asset
        let asset_path = output_dir.join(format!("comp_{}.png", composition.id));

        // Create a 1x1 or transparent placeholder frame representing the rendered composition
        let sample_svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}">
  <!-- HyperFrames Rendered Composition: {} -->
</svg>"#,
            composition.width, composition.height, composition.name
        );
        fs::write(&asset_path, sample_svg)?;

        composition.rendered_asset_path = Some(asset_path.clone());
        let _ = html_path; // retained for inspector preview
        Ok(asset_path)
    }
}
