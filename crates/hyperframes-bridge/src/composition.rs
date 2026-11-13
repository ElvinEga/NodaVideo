//! Composition instance representation for motion graphics on the timeline.

use std::collections::HashMap;
use std::path::PathBuf;

use timeline::FrameRate;
use serde::{Deserialize, Serialize};

use crate::id::CompositionId;
use crate::template::TemplateDefinition;

/// A configured instance of a motion graphics composition ready for rendering.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Composition {
    pub id: CompositionId,
    pub template_id: String,
    pub name: String,
    pub duration_frames: i64,
    pub fps: FrameRate,
    pub width: u32,
    pub height: u32,
    pub parameters: HashMap<String, serde_json::Value>,
    pub rendered_asset_path: Option<PathBuf>,
}

impl Composition {
    /// Create a new composition from a template with default settings.
    pub fn new(template: &TemplateDefinition, name: impl Into<String>) -> Self {
        let mut params = HashMap::new();
        for p in &template.parameters {
            params.insert(p.name.clone(), p.default_value.clone());
        }

        Self {
            id: CompositionId::new(),
            template_id: template.id.clone(),
            name: name.into(),
            duration_frames: template.default_duration_frames,
            fps: FrameRate::new(24, 1),
            width: 1920,
            height: 1080,
            parameters: params,
            rendered_asset_path: None,
        }
    }

    /// Set or update a template parameter value.
    pub fn set_parameter(&mut self, key: impl Into<String>, val: serde_json::Value) {
        self.parameters.insert(key.into(), val);
    }
}
