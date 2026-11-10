//! Motion graphics template schemas and built-in template library.

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::HyperframesError;

/// Supported parameter types for template customization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParameterType {
    Text,
    Color,
    Number,
    Choice(Vec<String>),
    Boolean,
}

/// Description of a customizable parameter exposed by a template.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemplateParameter {
    pub name: String,
    pub label: String,
    pub description: String,
    pub param_type: ParameterType,
    pub default_value: serde_json::Value,
}

/// A motion graphics template specification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemplateDefinition {
    pub id: String,
    pub name: String,
    pub category: String,
    pub default_duration_frames: i64,
    pub parameters: Vec<TemplateParameter>,
    pub html_template: String,
}

impl TemplateDefinition {
    /// Validate that all required parameters are supplied and fit their declared schemas.
    pub fn validate_parameters(
        &self,
        params: &HashMap<String, serde_json::Value>,
    ) -> Result<(), HyperframesError> {
        for p in &self.parameters {
            if let Some(val) = params.get(&p.name) {
                match &p.param_type {
                    ParameterType::Text => {
                        if !val.is_string() {
                            return Err(HyperframesError::InvalidParameter {
                                name: p.name.clone(),
                                reason: "Expected string text".into(),
                            });
                        }
                    }
                    ParameterType::Color => {
                        let valid = val.as_str().is_some_and(|s| s.starts_with('#'));
                        if !valid {
                            return Err(HyperframesError::InvalidParameter {
                                name: p.name.clone(),
                                reason: "Expected hex color string (e.g. #3b82f6)".into(),
                            });
                        }
                    }
                    ParameterType::Number => {
                        if !val.is_number() {
                            return Err(HyperframesError::InvalidParameter {
                                name: p.name.clone(),
                                reason: "Expected numeric value".into(),
                            });
                        }
                    }
                    ParameterType::Choice(options) => {
                        let selected = val.as_str().unwrap_or("");
                        if !options.iter().any(|o| o == selected) {
                            return Err(HyperframesError::InvalidParameter {
                                name: p.name.clone(),
                                reason: format!("Value must be one of: {options:?}"),
                            });
                        }
                    }
                    ParameterType::Boolean => {
                        if !val.is_boolean() {
                            return Err(HyperframesError::InvalidParameter {
                                name: p.name.clone(),
                                reason: "Expected boolean value".into(),
                            });
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Substitute parameter values into the template HTML/SVG markup.
    pub fn render_html(&self, params: &HashMap<String, serde_json::Value>) -> String {
        let mut html = self.html_template.clone();
        for p in &self.parameters {
            let val = params.get(&p.name).unwrap_or(&p.default_value);

            let val_str = match val {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::Bool(b) => b.to_string(),
                other => other.to_string(),
            };

            let token = format!("{{{{{}}}}}", p.name);
            html = html.replace(&token, &val_str);
        }
        html
    }
}

/// Repository of motion graphics templates.
#[derive(Debug, Clone, Default)]
pub struct TemplateRegistry {
    templates: HashMap<String, TemplateDefinition>,
}

impl TemplateRegistry {
    /// Create a new registry populated with NodaVideo's built-in broadcast templates.
    pub fn builtin() -> Self {
        let mut reg = Self::default();

        // 1. Lower Third Modern
        reg.register(TemplateDefinition {
            id: "lower-third-modern".into(),
            name: "Lower Third (Modern)".into(),
            category: "Lower Thirds".into(),
            default_duration_frames: 96, // 4.0s @ 24fps
            parameters: vec![
                TemplateParameter {
                    name: "title".into(),
                    label: "Speaker Name".into(),
                    description: "Main speaker or subject name".into(),
                    param_type: ParameterType::Text,
                    default_value: serde_json::json!("Alex Rivera"),
                },
                TemplateParameter {
                    name: "subtitle".into(),
                    label: "Title / Role".into(),
                    description: "Speaker designation or organization".into(),
                    param_type: ParameterType::Text,
                    default_value: serde_json::json!("Lead Director & Editor"),
                },
                TemplateParameter {
                    name: "accent_color".into(),
                    label: "Accent Color".into(),
                    description: "Brand accent bar color".into(),
                    param_type: ParameterType::Color,
                    default_value: serde_json::json!("#3b82f6"),
                },
            ],
            html_template: r#"<div style="position: absolute; bottom: 80px; left: 80px; display: flex; align-items: center; gap: 16px; font-family: sans-serif;">
  <div style="width: 6px; height: 56px; background-color: {{accent_color}}; border-radius: 3px;"></div>
  <div style="display: flex; flex-direction: column;">
    <span style="font-size: 28px; font-weight: bold; color: #ffffff; text-shadow: 0 2px 4px rgba(0,0,0,0.6);">{{title}}</span>
    <span style="font-size: 16px; font-weight: 500; color: #94a3b8; text-shadow: 0 1px 2px rgba(0,0,0,0.6);">{{subtitle}}</span>
  </div>
</div>"#.into(),
        });

        // 2. Title Minimal
        reg.register(TemplateDefinition {
            id: "title-minimal".into(),
            name: "Title (Minimal Fade)".into(),
            category: "Titles".into(),
            default_duration_frames: 72, // 3.0s @ 24fps
            parameters: vec![
                TemplateParameter {
                    name: "headline".into(),
                    label: "Headline".into(),
                    description: "Main presentation or chapter title".into(),
                    param_type: ParameterType::Text,
                    default_value: serde_json::json!("CHAPTER 01"),
                },
                TemplateParameter {
                    name: "subheadline".into(),
                    label: "Subheadline".into(),
                    description: "Descriptive context".into(),
                    param_type: ParameterType::Text,
                    default_value: serde_json::json!("The Awakening"),
                },
            ],
            html_template: r#"<div style="position: absolute; inset: 0; display: flex; flex-direction: column; align-items: center; justify-content: center; font-family: sans-serif; text-align: center;">
  <h1 style="font-size: 48px; font-weight: 800; letter-spacing: 0.15em; color: #ffffff; margin: 0; text-shadow: 0 4px 8px rgba(0,0,0,0.8);">{{headline}}</h1>
  <p style="font-size: 20px; font-weight: 400; letter-spacing: 0.05em; color: #cbd5e1; margin-top: 12px; text-shadow: 0 2px 4px rgba(0,0,0,0.8);">{{subheadline}}</p>
</div>"#.into(),
        });

        // 3. Social Caption Card
        reg.register(TemplateDefinition {
            id: "social-caption-card".into(),
            name: "Social Caption Card (High Contrast)".into(),
            category: "Captions".into(),
            default_duration_frames: 36, // 1.5s @ 24fps
            parameters: vec![
                TemplateParameter {
                    name: "text".into(),
                    label: "Subtitle Text".into(),
                    description: "Punchy dialogue sentence for vertical format".into(),
                    param_type: ParameterType::Text,
                    default_value: serde_json::json!("THIS CHANGES EVERYTHING"),
                },
                TemplateParameter {
                    name: "highlight_color".into(),
                    label: "Highlight Color".into(),
                    description: "Background container highlight color".into(),
                    param_type: ParameterType::Color,
                    default_value: serde_json::json!("#facc15"),
                },
            ],
            html_template: r#"<div style="position: absolute; bottom: 240px; width: 100%; display: flex; justify-content: center; font-family: Impact, sans-serif;">
  <div style="background-color: {{highlight_color}}; padding: 8px 20px; border-radius: 8px; transform: rotate(-1deg); box-shadow: 0 8px 16px rgba(0,0,0,0.5);">
    <span style="font-size: 36px; font-weight: 900; color: #000000; letter-spacing: 0.05em;">{{text}}</span>
  </div>
</div>"#.into(),
        });

        // 4. Branded Transition Stinger
        reg.register(TemplateDefinition {
            id: "branded-transition".into(),
            name: "Branded Stinger Transition".into(),
            category: "Transitions".into(),
            default_duration_frames: 24, // 1.0s @ 24fps
            parameters: vec![
                TemplateParameter {
                    name: "brand_name".into(),
                    label: "Brand Name".into(),
                    description: "Centered text during stinger wipe".into(),
                    param_type: ParameterType::Text,
                    default_value: serde_json::json!("NODACON"),
                },
                TemplateParameter {
                    name: "fill_color".into(),
                    label: "Fill Color".into(),
                    description: "Background fill wipe color".into(),
                    param_type: ParameterType::Color,
                    default_value: serde_json::json!("#0f172a"),
                },
            ],
            html_template: r#"<div style="position: absolute; inset: 0; background-color: {{fill_color}}; display: flex; align-items: center; justify-content: center; font-family: sans-serif;">
  <span style="font-size: 64px; font-weight: 900; letter-spacing: 0.2em; color: #ffffff;">{{brand_name}}</span>
</div>"#.into(),
        });

        reg
    }

    /// Add a template definition.
    pub fn register(&mut self, template: TemplateDefinition) {
        self.templates.insert(template.id.clone(), template);
    }

    /// Fetch a template by ID.
    pub fn get(&self, id: &str) -> Option<&TemplateDefinition> {
        self.templates.get(id)
    }

    /// List all registered templates.
    pub fn list(&self) -> Vec<&TemplateDefinition> {
        self.templates.values().collect()
    }

    /// Load a template definition from a JSON file and register it.
    pub fn load_from_file(&mut self, path: impl AsRef<Path>) -> Result<(), HyperframesError> {
        let content = std::fs::read_to_string(path)?;
        let template: TemplateDefinition = serde_json::from_str(&content)?;
        self.register(template);
        Ok(())
    }

    /// Recursively scan a directory for JSON template definitions and register them.
    pub fn load_from_directory(
        &mut self,
        dir: impl AsRef<Path>,
    ) -> Result<usize, HyperframesError> {
        let mut count = 0;
        let dir_path = dir.as_ref();
        if !dir_path.exists() {
            return Ok(0);
        }

        Self::scan_dir(dir_path, self, &mut count)?;
        Ok(count)
    }

    fn scan_dir(dir: &Path, reg: &mut Self, count: &mut usize) -> Result<(), HyperframesError> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                Self::scan_dir(&path, reg, count)?;
            } else if path.extension().is_some_and(|ext| ext == "json") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(template) = serde_json::from_str::<TemplateDefinition>(&content) {
                        reg.register(template);
                        *count += 1;
                    }
                }
            }
        }
        Ok(())
    }
}
