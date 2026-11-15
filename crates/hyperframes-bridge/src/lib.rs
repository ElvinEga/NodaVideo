//! HyperFrames motion graphics composition bridge for NodaVideo.
//!
//! Provides templates and rendering for motion graphics, lower thirds, titles,
//! animated subtitles, and transitions. The Rust timeline owns timing and placement.

pub mod composition;
pub mod error;
pub mod id;
pub mod project;
pub mod renderer;
pub mod template;

pub use composition::Composition;
pub use error::HyperframesError;
pub use id::CompositionId;
pub use project::{
    CompositionAsset, load_compositions, save_composition, template_registry, unique_folder,
};
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

    #[test]
    fn test_template_directory_loading() {
        let dir = tempdir().unwrap();
        let sub = dir.path().join("sub_templates");
        std::fs::create_dir(&sub).unwrap();

        let json_content = r#"{
            "id": "custom-title",
            "name": "Custom Title Card",
            "category": "Titles",
            "default_duration_frames": 72,
            "parameters": [
                {
                    "name": "headline",
                    "label": "Headline Text",
                    "description": "Main title",
                    "param_type": "text",
                    "default_value": "Welcome"
                }
            ],
            "html_template": "<h1>{{headline}}</h1>"
        }"#;

        std::fs::write(sub.join("custom.json"), json_content).unwrap();

        let mut registry = TemplateRegistry::default();
        let count = registry.load_from_directory(dir.path()).unwrap();
        assert_eq!(count, 1);
        assert!(registry.get("custom-title").is_some());
        assert_eq!(
            registry.get("custom-title").unwrap().name,
            "Custom Title Card"
        );
    }

    #[test]
    fn test_load_workspace_templates_directory() {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let templates_path = std::path::Path::new(&manifest_dir).join("../../templates");
        if templates_path.exists() {
            let mut registry = TemplateRegistry::default();
            let count = registry.load_from_directory(&templates_path).unwrap();
            assert!(count >= 8, "Expected at least 8 templates, found {count}");
            assert!(registry.get("title-minimal").is_some());
            assert!(registry.get("kinetic-headline").is_some());
            assert!(registry.get("lower-third-modern").is_some());
            assert!(registry.get("broadcast-badge").is_some());
            assert!(registry.get("social-caption-card").is_some());
            assert!(registry.get("quote-callout").is_some());
            assert!(registry.get("branded-wipe").is_some());
            assert!(registry.get("geometric-iris").is_some());
        }
    }
}
