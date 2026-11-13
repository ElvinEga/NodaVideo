//! Project-level composition assets and their on-disk persistence.
//!
//! A composition asset is a reusable template instance kept in the project, shared
//! by the editor and the MCP tool executor so both agree on one model. Rendering
//! lives in [`crate::renderer`]; this module owns the project folder layout.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use timeline::FrameRate;
use ulid::Ulid;

use crate::composition::Composition;
use crate::error::HyperframesError;
use crate::id::CompositionId;
use crate::template::{TemplateDefinition, TemplateRegistry};

/// A reusable motion-graphics composition kept in the project, not an imported file.
///
/// It is an instance of a template: the template supplies the SVG source, the asset
/// supplies the parameter snapshot. Placing it copies these into a timeline clip.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct CompositionAsset {
    pub id: Ulid,
    pub name: String,
    pub template_id: String,
    pub parameters: HashMap<String, serde_json::Value>,
    pub width: u32,
    pub height: u32,
    pub duration_frames: i64,
    pub fps: FrameRate,
    /// On-disk directory name under `compositions/`; fixed at creation so renaming
    /// the display name never moves the folder.
    pub folder: String,
}

// Public APIs only
impl CompositionAsset {
    pub fn new_from_template(
        template: &TemplateDefinition,
        name: impl Into<String>,
        folder: String,
    ) -> Self {
        let mut parameters = HashMap::new();
        for parameter in &template.parameters {
            parameters.insert(parameter.name.clone(), parameter.default_value.clone());
        }
        Self {
            id: Ulid::generate(),
            name: name.into(),
            template_id: template.id.clone(),
            parameters,
            width: 1920,
            height: 1080,
            duration_frames: template.default_duration_frames,
            fps: FrameRate::new(24, 1),
            folder,
        }
    }

    /// The bridge composition used for rendering; the id is derived from the asset id
    /// so rendered artifacts stay stable across renders.
    pub fn to_composition(&self) -> Composition {
        Composition {
            id: CompositionId(uuid::Uuid::from_bytes(self.id.to_bytes())),
            template_id: self.template_id.clone(),
            name: self.name.clone(),
            duration_frames: self.duration_frames,
            fps: self.fps,
            width: self.width,
            height: self.height,
            parameters: self.parameters.clone(),
            rendered_asset_path: None,
        }
    }

    pub fn directory(&self, project_root: &Path) -> PathBuf {
        project_root.join("compositions").join(&self.folder)
    }
}

/// Build the template registry: built-in templates plus the workspace `templates/`
/// directory. A workspace load failure is logged and the built-ins remain available.
pub fn template_registry() -> TemplateRegistry {
    let mut registry = TemplateRegistry::builtin();
    let templates = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../templates");
    if let Err(error) = registry.load_from_directory(&templates) {
        log::error!("Could not load motion graphics templates: {error:?}");
    }
    registry
}

pub fn load_compositions(project_root: &Path) -> Result<Vec<CompositionAsset>, HyperframesError> {
    let directory = project_root.join("compositions");
    let entries = match fs::read_dir(&directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    };

    let mut compositions = Vec::new();
    for entry in entries {
        let entry = entry?;
        let path = entry.path().join("composition.json");
        if !path.is_file() {
            continue;
        }
        let contents = fs::read(&path)?;
        let asset: CompositionAsset = match serde_json::from_slice(&contents) {
            Ok(asset) => asset,
            Err(error) => {
                return Err(HyperframesError::CompositionParse {
                    path: path.display().to_string(),
                    reason: error.to_string(),
                });
            }
        };
        compositions.push(asset);
    }
    compositions.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(compositions)
}

pub fn save_composition(
    project_root: &Path,
    asset: &CompositionAsset,
) -> Result<(), HyperframesError> {
    let directory = asset.directory(project_root);
    fs::create_dir_all(&directory)?;
    let contents = serde_json::to_vec_pretty(asset)?;
    let temporary_path = directory.join("composition.json.tmp");
    fs::write(&temporary_path, contents)?;
    fs::rename(temporary_path, directory.join("composition.json"))?;
    Ok(())
}

/// A unique, filesystem-safe directory name for a new composition.
pub fn unique_folder(project_root: &Path, name: &str) -> String {
    let base = sanitize_folder(name);
    let mut candidate = base.clone();
    let mut suffix = 2;
    while project_root.join("compositions").join(&candidate).exists() {
        candidate = format!("{base}-{suffix}");
        suffix += 1;
    }
    candidate
}

// Private APIs only
fn sanitize_folder(name: &str) -> String {
    let mut slug = String::new();
    for character in name.chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character.to_ascii_lowercase());
        } else if !slug.is_empty() && !slug.ends_with('-') {
            slug.push('-');
        }
    }
    while slug.ends_with('-') {
        slug.pop();
    }
    if slug.is_empty() {
        slug.push_str("composition");
    }
    slug
}
