//! Skill registry for discovering, inspecting, and combining SKILL.md packages.

pub mod discovery;
pub mod error;
pub mod skill;

pub use discovery::SkillRegistry;
pub use error::SkillError;
pub use skill::SkillPackage;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_parse_skill_frontmatter() {
        let content = r#"---
name: "Auto Captions"
id: "auto-captions"
description: "Automatically transcribe audio and generate subtitle clips"
version: "1.2.0"
tools: [get_timeline, add_clip]
---

# Instructions

When user requests subtitles, transcribe audio tracks and create title overlays.
"#;
        let pkg =
            SkillPackage::parse_str(content, Path::new("skills/auto-captions/SKILL.md")).unwrap();
        assert_eq!(pkg.id, "auto-captions");
        assert_eq!(pkg.name, "Auto Captions");
        assert_eq!(pkg.version, "1.2.0");
        assert_eq!(pkg.tools, vec!["get_timeline", "add_clip"]);
        assert!(pkg.instructions.contains("transcribe audio tracks"));
    }

    #[test]
    fn test_registry_enable_and_combine() {
        let mut registry = SkillRegistry::new();

        let s1 = SkillPackage::parse_str(
            "---\nname: Skill A\nid: skill-a\n---\nDo task A.",
            Path::new("skill-a/SKILL.md"),
        )
        .unwrap();

        let s2 = SkillPackage::parse_str(
            "---\nname: Skill B\nid: skill-b\n---\nDo task B.",
            Path::new("skill-b/SKILL.md"),
        )
        .unwrap();

        registry.register(s1);
        registry.register(s2);

        assert_eq!(registry.list().len(), 2);

        registry.set_enabled("skill-a", true);
        assert!(registry.is_enabled("skill-a"));
        assert!(!registry.is_enabled("skill-b"));

        let instructions = registry.build_combined_instructions();
        assert!(instructions.contains("Skill A"));
        assert!(instructions.contains("Do task A."));
        assert!(!instructions.contains("Skill B"));
    }
}
