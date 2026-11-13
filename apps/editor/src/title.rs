use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::time::Duration;
use timeline::TextClipProperties;
use ulid::Ulid;

/// A reusable title kept in the project, not an imported file. It carries the same
/// properties as a timeline text clip; placing it copies these into a new clip.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub(crate) struct TextTitle {
    pub(crate) id: Ulid,
    pub(crate) name: String,
    pub(crate) properties: TextClipProperties,
    pub(crate) duration: Duration,
}

// Public APIs only
impl TextTitle {
    /// The title created by New Text: the model defaults (text "Text", white, Sans, 3s).
    pub(crate) fn new_default() -> Self {
        Self {
            id: Ulid::generate(),
            name: "Text".to_string(),
            properties: TextClipProperties::default(),
            duration: Duration::from_secs(3),
        }
    }
}

pub(crate) fn load_titles(project_root: &Path) -> Result<Vec<TextTitle>> {
    let contents = match fs::read(project_root.join(".opencut/titles.json")) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    };
    Ok(serde_json::from_slice(&contents)?)
}

pub(crate) fn save_titles(project_root: &Path, titles: &[TextTitle]) -> Result<()> {
    let contents = serde_json::to_vec_pretty(titles)?;
    let directory = project_root.join(".opencut");
    fs::create_dir_all(&directory)?;
    let temporary_path = directory.join("titles.json.tmp");
    fs::write(&temporary_path, contents)?;
    fs::rename(temporary_path, directory.join("titles.json"))?;
    Ok(())
}
