//! Central FFmpeg / FFprobe executable resolution.
//!
//! A GUI application launched from Finder, Explorer or a desktop launcher does not
//! inherit a login shell's `PATH`. On macOS, Homebrew installs FFmpeg into
//! `/opt/homebrew/bin`, which is absent from the GUI environment, so a bare
//! `Command::new("ffmpeg")` fails with `ENOENT` even though FFmpeg is installed.
//!
//! Every FFmpeg invocation in `media-engine` must go through [`ffmpeg_command`] or
//! [`ffprobe_command`] so resolution is consistent, cached and overridable.
//!
//! Resolution order:
//! 1. `NODAVIDEO_FFMPEG` / `NODAVIDEO_FFPROBE` — explicit absolute path wins.
//! 2. Every directory on `PATH`.
//! 3. Well-known installation directories (Homebrew, `/usr/local`, system, Windows).
//!
//! No platform-specific APIs are used: `.exe` is simply an additional candidate
//! filename on every platform.

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use crate::error::MediaEngineError;

/// Explicit override for the FFmpeg executable.
pub const FFMPEG_ENV_VAR: &str = "NODAVIDEO_FFMPEG";
/// Explicit override for the FFprobe executable.
pub const FFPROBE_ENV_VAR: &str = "NODAVIDEO_FFPROBE";

/// Candidate filenames for a binary stem. `*.exe` is included on every platform so
/// Windows installs resolve without a `cfg` branch in a library crate.
fn binary_names(stem: &str) -> Vec<OsString> {
    vec![OsString::from(stem), OsString::from(format!("{stem}.exe"))]
}

/// Directories that commonly contain FFmpeg but are missing from a GUI `PATH`.
fn common_install_dirs() -> Vec<PathBuf> {
    [
        "/opt/homebrew/bin",
        "/usr/local/bin",
        "/usr/bin",
        "/bin",
        "/opt/local/bin",
        "/snap/bin",
        "C:\\Program Files\\ffmpeg\\bin",
        "C:\\ffmpeg\\bin",
    ]
    .iter()
    .map(PathBuf::from)
    .collect()
}

fn is_existing_file(path: &Path) -> bool {
    path.is_file()
}

/// Pure resolution logic. Kept separate from the process environment so it can be
/// unit-tested with temporary directories and without mutating global state.
///
/// `env_override` wins unconditionally: if it is set but invalid, resolution fails
/// instead of silently falling back to a different binary.
fn resolve_from(
    env_override: Option<OsString>,
    path_var: Option<&OsStr>,
    extra_dirs: &[PathBuf],
    names: &[OsString],
) -> Option<PathBuf> {
    if let Some(override_path) = env_override.filter(|v| !v.is_empty()) {
        let candidate = PathBuf::from(override_path);
        return is_existing_file(&candidate).then_some(candidate);
    }

    let mut search_dirs: Vec<PathBuf> = path_var
        .map(std::env::split_paths)
        .map(Iterator::collect)
        .unwrap_or_default();
    search_dirs.extend(extra_dirs.iter().cloned());

    search_dirs
        .iter()
        .flat_map(|dir| names.iter().map(move |name| dir.join(name)))
        .find(|candidate| is_existing_file(candidate))
}

fn ffmpeg_names() -> Vec<OsString> {
    binary_names("ffmpeg")
}

fn ffprobe_names() -> Vec<OsString> {
    binary_names("ffprobe")
}

fn resolve_ffmpeg() -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH");
    resolve_from(
        std::env::var_os(FFMPEG_ENV_VAR),
        path_var.as_deref(),
        &common_install_dirs(),
        &ffmpeg_names(),
    )
}

fn resolve_ffprobe() -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH");
    resolve_from(
        std::env::var_os(FFPROBE_ENV_VAR),
        path_var.as_deref(),
        &common_install_dirs(),
        &ffprobe_names(),
    )
}

/// Resolve and cache the absolute path to the FFmpeg executable.
pub fn ffmpeg_path() -> Result<&'static Path, MediaEngineError> {
    static CACHE: OnceLock<Option<PathBuf>> = OnceLock::new();
    CACHE
        .get_or_init(resolve_ffmpeg)
        .as_deref()
        .ok_or(MediaEngineError::FfmpegNotFound)
}

/// Resolve and cache the absolute path to the FFprobe executable.
pub fn ffprobe_path() -> Result<&'static Path, MediaEngineError> {
    static CACHE: OnceLock<Option<PathBuf>> = OnceLock::new();
    CACHE
        .get_or_init(resolve_ffprobe)
        .as_deref()
        .ok_or(MediaEngineError::FfprobeNotFound)
}

/// Build a `Command` for FFmpeg using the resolved executable path.
pub fn ffmpeg_command() -> Result<Command, MediaEngineError> {
    Ok(Command::new(ffmpeg_path()?))
}

/// Build a `Command` for FFprobe using the resolved executable path.
pub fn ffprobe_command() -> Result<Command, MediaEngineError> {
    Ok(Command::new(ffprobe_path()?))
}

/// Verify that a resolved binary can actually be executed (`-version` succeeds).
fn is_usable(path: &Path) -> bool {
    Command::new(path)
        .arg("-version")
        .output()
        .is_ok_and(|out| out.status.success())
}

/// Whether a usable FFmpeg executable is available.
pub fn is_ffmpeg_available() -> bool {
    ffmpeg_path().is_ok_and(is_usable)
}

/// Whether a usable FFprobe executable is available.
pub fn is_ffprobe_available() -> bool {
    ffprobe_path().is_ok_and(is_usable)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsStr;
    use std::fs;

    fn touch(dir: &Path, name: &str) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, b"").unwrap();
        path
    }

    #[test]
    fn env_override_wins_over_path() {
        let dir = tempfile::tempdir().unwrap();
        let override_dir = dir.path().join("override");
        let path_dir = dir.path().join("onpath");
        fs::create_dir_all(&override_dir).unwrap();
        fs::create_dir_all(&path_dir).unwrap();

        let custom = touch(&override_dir, "my-ffmpeg");
        touch(&path_dir, "ffmpeg");

        let resolved = resolve_from(
            Some(custom.as_os_str().to_os_string()),
            Some(path_dir.as_os_str()),
            &[],
            &ffmpeg_names(),
        );
        assert_eq!(resolved, Some(custom));
    }

    #[test]
    fn invalid_override_does_not_silently_fall_back() {
        let dir = tempfile::tempdir().unwrap();
        let path_dir = dir.path().join("onpath");
        fs::create_dir_all(&path_dir).unwrap();
        touch(&path_dir, "ffmpeg");

        let missing = dir.path().join("does-not-exist");
        let resolved = resolve_from(
            Some(missing.as_os_str().to_os_string()),
            Some(path_dir.as_os_str()),
            &[],
            &ffmpeg_names(),
        );
        assert_eq!(resolved, None);
    }

    #[test]
    fn finds_binary_on_explicit_path() {
        let dir = tempfile::tempdir().unwrap();
        let expected = touch(dir.path(), "ffprobe");

        let resolved = resolve_from(None, Some(dir.path().as_os_str()), &[], &ffprobe_names());
        assert_eq!(resolved, Some(expected));
    }

    #[test]
    fn falls_back_to_common_install_dirs_when_gui_path_is_minimal() {
        let dir = tempfile::tempdir().unwrap();
        let fake_homebrew = dir.path().join("opt/homebrew/bin");
        fs::create_dir_all(&fake_homebrew).unwrap();
        let expected = touch(&fake_homebrew, "ffmpeg");

        // GUI apps often get only /usr/bin:/bin, which has no FFmpeg.
        let gui_path = OsString::from("/usr/bin:/bin");
        let resolved = resolve_from(
            None,
            Some(gui_path.as_os_str()),
            &[fake_homebrew],
            &ffmpeg_names(),
        );
        assert_eq!(resolved, Some(expected));
    }

    #[test]
    fn path_takes_precedence_over_common_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let path_dir = dir.path().join("onpath");
        let common_dir = dir.path().join("common");
        fs::create_dir_all(&path_dir).unwrap();
        fs::create_dir_all(&common_dir).unwrap();
        let expected = touch(&path_dir, "ffmpeg");
        touch(&common_dir, "ffmpeg");

        let resolved = resolve_from(
            None,
            Some(path_dir.as_os_str()),
            &[common_dir],
            &ffmpeg_names(),
        );
        assert_eq!(resolved, Some(expected));
    }

    #[test]
    fn resolves_windows_exe_filename() {
        let dir = tempfile::tempdir().unwrap();
        let expected = touch(dir.path(), "ffmpeg.exe");

        let resolved = resolve_from(None, Some(dir.path().as_os_str()), &[], &ffmpeg_names());
        assert_eq!(resolved, Some(expected));
    }

    #[test]
    fn returns_none_when_nothing_is_installed() {
        let dir = tempfile::tempdir().unwrap();
        let empty = dir.path().join("empty");
        fs::create_dir_all(&empty).unwrap();

        let resolved = resolve_from(None, Some(empty.as_os_str()), &[], &ffmpeg_names());
        assert_eq!(resolved, None);
    }

    #[test]
    fn names_include_plain_and_exe_variants() {
        let names = binary_names("ffmpeg");
        assert!(names.contains(&OsString::from("ffmpeg")));
        assert!(names.contains(&OsString::from("ffmpeg.exe")));
        assert_eq!(OsStr::new("ffmpeg"), names[0].as_os_str());
    }
}
