//! Skill package metadata and parsed instructions.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::error::SkillError;

/// Parsed SKILL.md package defining an AI editing capability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillPackage {
    /// Unique identifier (e.g. "auto-captions", "smart-cuts").
    pub id: String,

    /// Human-readable title of the skill.
    pub name: String,

    /// Description of the capability provided by this skill.
    pub description: String,

    /// Package version string.
    pub version: String,

    /// Editor tools allowed or required by this skill.
    pub tools: Vec<String>,

    /// Full system instructions extracted from SKILL.md markdown body.
    pub instructions: String,

    /// File path to the SKILL.md package.
    pub path: PathBuf,
}

impl SkillPackage {
    /// Parse a SKILL.md document containing YAML frontmatter and markdown instructions.
    pub fn parse_file(path: &std::path::Path) -> Result<Self, SkillError> {
        let content = std::fs::read_to_string(path).map_err(|e| SkillError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;

        Self::parse_str(&content, path)
    }

    /// Parse SKILL.md text content.
    pub fn parse_str(content: &str, path: &std::path::Path) -> Result<Self, SkillError> {
        let fallback_id = path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|f| f.to_str())
            .unwrap_or("custom-skill")
            .to_string();

        if !content.starts_with("---") {
            // Document without frontmatter: treat entire content as instructions
            return Ok(Self {
                id: fallback_id.clone(),
                name: fallback_id,
                description: "Custom AI editing skill".into(),
                version: "1.0.0".into(),
                tools: Vec::new(),
                instructions: content.trim().to_string(),
                path: path.to_path_buf(),
            });
        }

        // Split frontmatter from markdown body
        let rest = &content[3..];
        let Some(end_idx) = rest.find("---") else {
            return Err(SkillError::InvalidDocument {
                path: path.to_path_buf(),
                reason: "Unclosed YAML frontmatter delimiter '---'".into(),
            });
        };

        let frontmatter = &rest[..end_idx];
        let body = rest[end_idx + 3..].trim();

        let mut id = fallback_id.clone();
        let mut name = fallback_id;
        let mut description = "Video production skill".to_string();
        let mut version = "1.0.0".to_string();
        let mut tools = Vec::new();

        for line in frontmatter.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((k, v)) = line.split_once(':') {
                let key = k.trim();
                let val = v.trim().trim_matches('"').trim_matches('\'');
                match key {
                    "id" | "name" if key == "id" => id = val.to_string(),
                    "title" | "name" => name = val.to_string(),
                    "description" => description = val.to_string(),
                    "version" => version = val.to_string(),
                    "tools" => {
                        // Comma separated or bracket list
                        let cleaned = val.trim_matches('[').trim_matches(']');
                        for t in cleaned.split(',') {
                            let t = t.trim().trim_matches('"').trim_matches('\'');
                            if !t.is_empty() {
                                tools.push(t.to_string());
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(Self {
            id,
            name,
            description,
            version,
            tools,
            instructions: body.to_string(),
            path: path.to_path_buf(),
        })
    }
}
