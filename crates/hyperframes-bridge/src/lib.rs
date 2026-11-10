//! HyperFrames motion graphics composition bridge for NodaVideo.
//!
//! Provides templates and rendering for motion graphics, lower thirds, titles,
//! animated subtitles, and transitions. The Rust timeline owns timing and placement.

pub mod composition;
pub mod error;
pub mod id;
pub mod renderer;
pub mod template;

pub use composition::Composition;
pub use error::HyperframesError;
pub use id::CompositionId;
pub use renderer::CompositionRenderer;
pub use template::{ParameterType, TemplateDefinition, TemplateParameter, TemplateRegistry};

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_builtin_template_registry() {
        let registry = TemplateRegistry::builtin();
        let templates = registry.list();
        assert_eq!(templates.len(), 4);
        assert!(registry.get("lower-third-modern").is_some());
        assert!(registry.get("title-minimal").is_some());
        assert!(registry.get("social-caption-card").is_some());
        assert!(registry.get("branded-transition").is_some());
    }

    #[test]
    fn test_template_rendering_and_parameter_substitution() {
        let registry = TemplateRegistry::builtin();
        let template = registry.get("lower-third-modern").unwrap();

        let mut comp = Composition::new(template, "Speaker Card");
        comp.set_parameter("title", serde_json::json!("Dr. Jane Doe"));
        comp.set_parameter("subtitle", serde_json::json!("Chief Scientist"));
        comp.set_parameter("accent_color", serde_json::json!("#10b981"));

        let renderer = CompositionRenderer::new();
        let dir = tempdir().unwrap();
        let html_path = renderer
            .render_html_document(&comp, &registry, dir.path())
            .unwrap();

        assert!(html_path.exists());
        let content = std::fs::read_to_string(&html_path).unwrap();
        assert!(content.contains("Dr. Jane Doe"));
        assert!(content.contains("Chief Scientist"));
        assert!(content.contains("#10b981"));
    }

    #[test]
    fn test_parameter_validation_error() {
        let registry = TemplateRegistry::builtin();
        let template = registry.get("lower-third-modern").unwrap();

        let mut comp = Composition::new(template, "Bad Params");
        // Color must start with '#'
        comp.set_parameter("accent_color", serde_json::json!("invalid-color"));

        let renderer = CompositionRenderer::new();
        let dir = tempdir().unwrap();
        let res = renderer.render_html_document(&comp, &registry, dir.path());
        assert!(matches!(
            res,
            Err(HyperframesError::InvalidParameter { .. })
        ));
    }
}
