# nodavideo-skill-registry

`nodavideo-skill-registry` provides package discovery, metadata parsing, and instruction composition for video-production skills.

## Core Responsibilities

- **`SKILL.md` parser**: Reads YAML frontmatter (name, id, description, version, tags, recommended tools) and Markdown instruction body.
- **Recursive Discovery**: Scans project and global skill directories (e.g. `./skills/`) without indexing hidden folders or build directories.
- **Dynamic Skill Composition**: Allows selecting and enabling capabilities (e.g. Auto Captions, Smart Cuts, Motion Graphics) and aggregates instructions for agent prompting.
