//! Skill discovery engine scanning directories for SKILL.md packages.

use std::collections::HashMap;
use std::path::Path;

use crate::error::SkillError;
use crate::skill::SkillPackage;

/// Registry holding discovered skills and tracking enabled capabilities.
#[derive(Debug, Clone, Default)]
pub struct SkillRegistry {
    skills: HashMap<String, SkillPackage>,
    enabled: Vec<String>,
}

impl SkillRegistry {
    /// Create a new empty skill registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Discover skills recursively within a given root folder (e.g. `./skills/`).
    pub fn discover_from_dir(&mut self, root: &Path) -> Result<usize, SkillError> {
        if !root.exists() {
            return Ok(0);
        }

        let mut discovered = 0;
        self.scan_recursive(root, &mut discovered)?;
        Ok(discovered)
    }

    fn scan_recursive(&mut self, dir: &Path, count: &mut usize) -> Result<(), SkillError> {
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return Ok(()),
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                    if file_name.eq_ignore_ascii_case("SKILL.md") {
                        if let Ok(pkg) = SkillPackage::parse_file(&path) {
                            self.skills.insert(pkg.id.clone(), pkg);
                            *count += 1;
                        }
                    }
                }
            } else if path.is_dir() {
                // Check if directory name starts with dot or target
                let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if !dir_name.starts_with('.') && dir_name != "target" {
                    self.scan_recursive(&path, count)?;
                }
            }
        }

        Ok(())
    }

    /// Add a skill package directly into the registry.
    pub fn register(&mut self, skill: SkillPackage) {
        self.skills.insert(skill.id.clone(), skill);
    }

    /// Retrieve a skill package by ID.
    pub fn get(&self, id: &str) -> Option<&SkillPackage> {
        self.skills.get(id)
    }

    /// List all discovered skills.
    pub fn list(&self) -> Vec<&SkillPackage> {
        self.skills.values().collect()
    }

    /// Toggle enabled state for a skill.
    pub fn set_enabled(&mut self, id: &str, enabled: bool) {
        if enabled {
            if !self.enabled.iter().any(|e| e == id) {
                self.enabled.push(id.to_string());
            }
        } else {
            self.enabled.retain(|e| e != id);
        }
    }

    /// Check if a skill is enabled.
    pub fn is_enabled(&self, id: &str) -> bool {
        self.enabled.iter().any(|e| e == id)
    }

    /// Get list of enabled skill IDs.
    pub fn enabled_skills(&self) -> &[String] {
        &self.enabled
    }

    /// Build combined instructions from all currently enabled skills.
    pub fn build_combined_instructions(&self) -> String {
        use std::fmt::Write;
        let mut out = String::new();
        for id in &self.enabled {
            if let Some(skill) = self.get(id) {
                let _ = writeln!(out, "## Skill: {}", skill.name);
                out.push_str(&skill.instructions);
                out.push_str("\n\n");
            }
        }
        out
    }
}
