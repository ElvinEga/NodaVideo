//! HyperFrames motion graphics composition tools for MCP agents.
//!
//! Compositions are reusable template instances persisted in the project's
//! `compositions/` directory, shared with the editor. These tools create, inspect,
//! edit, validate and render them. Timeline placement is a separate approved tool.

use std::path::Path;

use nodavideo_hyperframes_bridge::{
    CompositionAsset, CompositionRenderer, TemplateDefinition, load_compositions, save_composition,
    template_registry, unique_folder,
};
use serde_json::{Value, json};
use ulid::Ulid;

use crate::error::McpError;
use crate::executor::{invalid, optional_str, required_str, required_ulid};

pub(crate) fn list_templates() -> Value {
    let mut templates = template_registry()
        .list()
        .into_iter()
        .cloned()
        .collect::<Vec<_>>();
    templates.sort_by(|a, b| a.name.cmp(&b.name));
    json!({
        "templates": templates.iter().map(template_json).collect::<Vec<_>>(),
    })
}

pub(crate) fn list_compositions(project_root: &Path) -> Result<Value, McpError> {
    let compositions = load_compositions(project_root)?;
    Ok(json!({
        "compositions": compositions.iter().map(composition_json).collect::<Vec<_>>(),
    }))
}

pub(crate) fn create_composition(project_root: &Path, args: &Value) -> Result<Value, McpError> {
    let template_id = required_str("create_composition", args, "template_id")?;
    let registry = template_registry();
    let Some(template) = registry.get(template_id) else {
        return Err(invalid(
            "create_composition",
            format!("template '{template_id}' not found"),
        ));
    };
    let name = optional_str(args, "name").unwrap_or(template.name.as_str());
    let folder = unique_folder(project_root, name);
    let asset = CompositionAsset::new_from_template(template, name, folder);
    save_composition(project_root, &asset)?;
    Ok(json!({ "status": "created", "composition": composition_json(&asset) }))
}

pub(crate) fn inspect_composition(project_root: &Path, args: &Value) -> Result<Value, McpError> {
    let composition_id = required_ulid("inspect_composition", args, "composition_id")?;
    let asset = find_composition("inspect_composition", project_root, composition_id)?;
    Ok(json!({ "composition": composition_json(&asset) }))
}

pub(crate) fn edit_composition(project_root: &Path, args: &Value) -> Result<Value, McpError> {
    let composition_id = required_ulid("edit_composition", args, "composition_id")?;
    let Some(parameters) = args.get("parameters").and_then(Value::as_object) else {
        return Err(invalid("edit_composition", "parameters must be an object"));
    };
    let mut asset = find_composition("edit_composition", project_root, composition_id)?;
    for (key, value) in parameters {
        asset.parameters.insert(key.clone(), value.clone());
    }
    let registry = template_registry();
    let Some(template) = registry.get(&asset.template_id) else {
        return Err(invalid(
            "edit_composition",
            format!("template '{}' not found", asset.template_id),
        ));
    };
    // 先校验参数再落盘，校验失败时磁盘上的旧版本保持不变。
    template.validate_parameters(&asset.parameters)?;
    save_composition(project_root, &asset)?;
    Ok(json!({ "status": "updated", "composition": composition_json(&asset) }))
}

pub(crate) fn render_composition(project_root: &Path, args: &Value) -> Result<Value, McpError> {
    let composition_id = required_ulid("render_composition", args, "composition_id")?;
    let asset = find_composition("render_composition", project_root, composition_id)?;
    let registry = template_registry();
    let mut composition = asset.to_composition();
    let output_dir = asset.directory(project_root);
    let renderer = CompositionRenderer::new();
    let asset_path = renderer.render_media_asset(&mut composition, &registry, &output_dir)?;
    Ok(json!({
        "status": "rendered",
        "composition_id": composition_id.to_string(),
        "asset_path": asset_path.to_string_lossy(),
        "width": asset.width,
        "height": asset.height,
        "duration_frames": asset.duration_frames,
    }))
}

pub(crate) fn validate_composition(project_root: &Path, args: &Value) -> Result<Value, McpError> {
    let composition_id = required_ulid("validate_composition", args, "composition_id")?;
    let asset = find_composition("validate_composition", project_root, composition_id)?;
    let registry = template_registry();
    let Some(template) = registry.get(&asset.template_id) else {
        return Err(invalid(
            "validate_composition",
            format!("template '{}' not found", asset.template_id),
        ));
    };
    template.validate_parameters(&asset.parameters)?;
    Ok(json!({ "valid": true, "composition_id": composition_id.to_string() }))
}

pub(crate) fn find_composition(
    tool: &str,
    project_root: &Path,
    composition_id: Ulid,
) -> Result<CompositionAsset, McpError> {
    let compositions = load_compositions(project_root)?;
    compositions
        .into_iter()
        .find(|asset| asset.id == composition_id)
        .ok_or_else(|| invalid(tool, format!("composition {composition_id} does not exist")))
}

fn composition_json(asset: &CompositionAsset) -> Value {
    json!({
        "id": asset.id.to_string(),
        "name": asset.name,
        "template_id": asset.template_id,
        "parameters": asset.parameters,
        "width": asset.width,
        "height": asset.height,
        "duration_frames": asset.duration_frames,
        "fps": {
            "numerator": asset.fps.numerator,
            "denominator": asset.fps.denominator,
        },
        "folder": asset.folder,
    })
}

fn template_json(template: &TemplateDefinition) -> Value {
    json!({
        "id": template.id,
        "name": template.name,
        "category": template.category,
        "default_duration_frames": template.default_duration_frames,
        "parameters": template
            .parameters
            .iter()
            .map(|parameter| {
                json!({
                    "name": parameter.name,
                    "label": parameter.label,
                    "type": parameter.param_type,
                    "default": parameter.default_value,
                })
            })
            .collect::<Vec<_>>(),
    })
}
