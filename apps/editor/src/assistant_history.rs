use crate::assistant::AssistantEntry;
use anyhow::{Context as _, Result};
use chrono::{DateTime, Datelike, Local, TimeZone};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use ulid::Ulid;

#[derive(Clone, Deserialize, Serialize)]
pub(crate) struct AssistantSession {
    pub(crate) id: Ulid,
    pub(crate) provider_id: String,
    /// Model id the transcript ran with; None means the provider default.
    #[serde(default)]
    pub(crate) model: Option<String>,
    /// Reasoning-effort variant the transcript ran with; None means the model default.
    #[serde(default)]
    pub(crate) effort: Option<String>,
    pub(crate) created_at_ms: u64,
    pub(crate) updated_at_ms: u64,
    pub(crate) title: String,
    pub(crate) entries: Vec<AssistantEntry>,
}

#[derive(Clone)]
pub(crate) struct AssistantSessionMeta {
    pub(crate) id: Ulid,
    pub(crate) provider_id: String,
    pub(crate) updated_at_ms: u64,
    pub(crate) title: String,
    pub(crate) message_count: usize,
}

// Public APIs only
impl AssistantSession {
    pub(crate) fn meta(&self) -> AssistantSessionMeta {
        AssistantSessionMeta {
            id: self.id,
            provider_id: self.provider_id.clone(),
            updated_at_ms: self.updated_at_ms,
            title: self.title.clone(),
            message_count: self.entries.len(),
        }
    }
}

pub(crate) fn now_ms() -> u64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_millis() as u64,
        Err(_) => 0,
    }
}

pub(crate) fn session_title(entries: &[AssistantEntry]) -> String {
    for entry in entries {
        if let AssistantEntry::User(message) = entry {
            let trimmed = message.trim();
            if trimmed.is_empty() {
                continue;
            }
            return match trimmed.char_indices().nth(48) {
                Some((cut, _)) => format!("{}…", &trimmed[..cut]),
                None => trimmed.to_string(),
            };
        }
    }
    "New chat".to_string()
}

pub(crate) fn load_session_metas(project_root: &Path) -> Vec<AssistantSessionMeta> {
    let directory = sessions_dir(project_root);
    let Ok(read_dir) = fs::read_dir(&directory) else {
        return Vec::new(); // 目录不存在 = 还没有会话，属于预期空态。
    };
    let mut metas = Vec::new();
    for entry in read_dir.flatten() {
        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
            continue;
        }
        let contents = match fs::read_to_string(&path) {
            Ok(contents) => contents,
            Err(error) => {
                log::warn!("Skipping unreadable assistant session {}: {error:?}", path.display());
                continue;
            }
        };
        let session: AssistantSession = match serde_json::from_str(&contents) {
            Ok(session) => session,
            Err(error) => {
                log::warn!("Skipping invalid assistant session {}: {error:?}", path.display());
                continue;
            }
        };
        metas.push(session.meta());
    }
    metas.sort_by_key(|meta| std::cmp::Reverse(meta.updated_at_ms));
    metas
}

pub(crate) fn load_session(project_root: &Path, id: Ulid) -> Result<AssistantSession> {
    let path = session_path(project_root, id);
    let contents = fs::read_to_string(&path)
        .with_context(|| format!("could not read assistant session {}", path.display()))?;
    serde_json::from_str(&contents)
        .with_context(|| format!("could not parse assistant session {}", path.display()))
}

pub(crate) fn save_session(project_root: &Path, session: &AssistantSession) -> Result<()> {
    let directory = sessions_dir(project_root);
    fs::create_dir_all(&directory)
        .with_context(|| format!("could not create {}", directory.display()))?;
    let json =
        serde_json::to_string_pretty(session).context("could not serialize assistant session")?;
    let path = session_path(project_root, session.id);
    fs::write(&path, format!("{json}\n"))
        .with_context(|| format!("could not write {}", path.display()))
}

pub(crate) fn session_day_label(updated_at_ms: u64) -> String {
    let Some(updated) = local_datetime(updated_at_ms) else {
        return "Unknown".to_string();
    };
    let today = Local::now().date_naive();
    let day = updated.date_naive();
    if day == today {
        return "Today".to_string();
    }
    let yesterday = match today.pred_opt() {
        Some(yesterday) => yesterday,
        None => today,
    };
    if day == yesterday {
        return "Yesterday".to_string();
    }
    if updated.year() == Local::now().year() {
        updated.format("%b %d").to_string()
    } else {
        updated.format("%b %d, %Y").to_string()
    }
}

fn sessions_dir(project_root: &Path) -> PathBuf {
    project_root.join(".opencut/assistant")
}

fn session_path(project_root: &Path, id: Ulid) -> PathBuf {
    sessions_dir(project_root).join(format!("{id}.json"))
}

fn local_datetime(ms: u64) -> Option<DateTime<Local>> {
    Local.timestamp_millis_opt(ms as i64).single()
}