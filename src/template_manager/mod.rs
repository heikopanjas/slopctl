//! Template management functionality for slopctl

mod agents;
mod doctor;
mod list;
mod merge;
mod models;
mod remove;
mod smart;
mod update;
mod verify;

use std::{
    ffi::OsStr,
    fs, io,
    path::{Path, PathBuf}
};

pub use merge::MergeOptions;
use owo_colors::OwoColorize;

use crate::{
    Result,
    download_manager::DownloadManager,
    file_tracker::{self, FileTracker, SLOPCTL_DIR},
    utils::copy_dir_all
};

/// Manages template files for coding agent instructions
///
/// The `TemplateManager` handles all operations related to template storage,
/// verification, and synchronization. Templates are stored in the
/// local data directory (e.g., `$HOME/.local/share/slopctl/templates` on Linux,
/// `$HOME/Library/Application Support/slopctl/templates` on macOS).
pub struct TemplateManager
{
    pub(crate) config_dir: PathBuf
}

impl TemplateManager
{
    /// Creates a new TemplateManager instance
    ///
    /// Initializes path to local data directory using the `dirs` crate.
    /// Templates are stored in the local data directory.
    ///
    /// # Errors
    ///
    /// Returns an error if the local data directory cannot be determined
    pub fn new() -> Result<Self>
    {
        let data_dir = dirs::data_local_dir().ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Could not determine local data directory"))?;

        let config_dir = data_dir.join("slopctl/templates");

        Ok(Self { config_dir })
    }

    /// Checks if global templates exist
    ///
    /// Returns true if the global template directory exists and contains templates.yml
    pub fn has_global_templates(&self) -> bool
    {
        self.config_dir.exists() && self.config_dir.join("templates.yml").exists()
    }

    /// Returns the path to the global template directory
    pub fn get_config_dir(&self) -> &Path
    {
        &self.config_dir
    }

    /// Downloads or copies templates from a source (URL or local path)
    ///
    /// Supports both local file paths and URLs. For URLs starting with http/https,
    /// templates are downloaded. For local paths, templates are copied.
    ///
    /// # Arguments
    ///
    /// * `source` - Path or URL to download/copy templates from
    ///
    /// # Errors
    ///
    /// Returns an error if download or copy operation fails
    pub fn download_or_copy_templates(&self, source: &str) -> Result<()>
    {
        if source.starts_with("http://") || source.starts_with("https://")
        {
            // Download from URL using DownloadManager
            println!("{} Downloading templates from URL...", "→".blue());
            let download_manager = DownloadManager::new(self.config_dir.clone());
            download_manager.download_templates_from_url(source)?;
        }
        else
        {
            // Copy from local path
            let source_path = Path::new(source);
            if source_path.exists() == false
            {
                return Err(anyhow::anyhow!("Source path does not exist: {}", source));
            }

            println!("{} Copying templates from local path...", "→".blue());
            fs::create_dir_all(&self.config_dir)?;
            copy_dir_all(source_path, &self.config_dir)?;
        }

        Ok(())
    }

    /// Returns the path to the workspace-local `.slopctl/` directory
    pub fn slopctl_dir(workspace: &Path) -> PathBuf
    {
        workspace.join(SLOPCTL_DIR)
    }

    /// Returns true if the workspace has a local tracker file
    pub fn is_workspace_initialized(workspace: &Path) -> bool
    {
        Self::slopctl_dir(workspace).join("tracker.yml").exists()
    }

    /// Attempt migration from the global tracker and adopt untracked files
    ///
    /// If `.slopctl/tracker.yml` does not exist, runs two passes:
    /// 1. Migrates matching entries from the global `installed_files.json`
    /// 2. Scans the workspace for known slopctl-managed files (instructions, skills, commands) and adopts any that are not yet tracked
    ///
    /// Returns the total number of entries migrated + adopted.
    pub fn try_migrate_tracker(&self, workspace: &Path) -> Result<usize>
    {
        if Self::is_workspace_initialized(workspace) == true
        {
            return Ok(0);
        }

        let mut tracker = FileTracker::new(workspace)?;
        let mut total = 0usize;

        let global_path = file_tracker::legacy_tracker_path(&self.config_dir);
        if global_path.exists() == true
        {
            let migrated = tracker.migrate_from_global(&global_path)?;
            if migrated > 0
            {
                println!("{} Migrated {} tracked file(s) from global tracker", "→".blue(), migrated);
                total += migrated;
            }
        }

        let agent_catalog = crate::agent_defaults::load_agent_catalog_from_dir(&self.config_dir)?;
        let adopted = tracker.adopt_untracked_files_from_catalog(workspace, &agent_catalog)?;
        if adopted > 0
        {
            println!("{} Adopted {} existing file(s) into .slopctl/", "→".blue(), adopted);
            total += adopted;
        }

        Ok(total)
    }

    /// Extract a skill name from an installed skill file path
    ///
    /// Looks for a `/skills/<name>/` segment in the path and returns the name.
    pub(crate) fn extract_skill_name_from_path(path: &Path) -> Option<String>
    {
        let components: Vec<&OsStr> = path.components().map(|c| c.as_os_str()).collect();

        for (i, component) in components.iter().enumerate()
        {
            if *component == "skills" && i + 1 < components.len()
            {
                return Some(components[i + 1].to_string_lossy().to_string());
            }
        }

        None
    }
}

/// Serializes tests that call `std::env::set_current_dir` (process-global state).
/// Shared across all `template_manager` submodule tests.
#[cfg(test)]
pub(crate) static CWD_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// One-time-cached "known good" cwd for test isolation.
///
/// Initialised by `cwd_test_guard()` on its first invocation. Used as the
/// recovery target whenever a previous test panicked while inside a tempdir
/// that has since been removed (which would otherwise make `current_dir()`
/// fail with ENOENT and cascade across the suite).
#[cfg(test)]
static ORIGINAL_CWD: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();

/// RAII guard that holds `CWD_LOCK` and restores cwd on drop
#[cfg(test)]
pub(crate) struct CwdTestGuard
{
    _lock: std::sync::MutexGuard<'static, ()>
}

#[cfg(test)]
impl Drop for CwdTestGuard
{
    fn drop(&mut self)
    {
        if let Some(orig) = ORIGINAL_CWD.get()
        {
            let _ = std::env::set_current_dir(orig);
        }
    }
}

/// Test helper: locks `CWD_LOCK`, restores cwd to a known-good directory, and
/// returns a guard that re-restores cwd on drop.
///
/// Use at the top of any test that calls `std::env::set_current_dir`. The
/// helper recovers transparently if a previous test panicked inside a tempdir
/// that has been cleaned up (cwd points at a deleted inode), by resetting cwd
/// to the cached startup directory before the new test runs.
#[cfg(test)]
pub(crate) fn cwd_test_guard() -> CwdTestGuard
{
    let lock = CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let orig = ORIGINAL_CWD.get_or_init(|| std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))));
    let _ = std::env::set_current_dir(orig);
    CwdTestGuard { _lock: lock }
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn test_extract_skill_name_from_bogus_path()
    {
        let path = PathBuf::from("/home/user/project/.bogus/skills/my-skill/SKILL.md");
        assert_eq!(TemplateManager::extract_skill_name_from_path(&path), Some("my-skill".to_string()));
    }

    #[test]
    fn test_extract_skill_name_from_fake_path()
    {
        let path = PathBuf::from("/home/user/project/.fake/skills/code-review/SKILL.md");
        assert_eq!(TemplateManager::extract_skill_name_from_path(&path), Some("code-review".to_string()));
    }

    #[test]
    fn test_extract_skill_name_nested_file()
    {
        let path = PathBuf::from("/project/.bogus/skills/my-skill/scripts/setup.sh");
        assert_eq!(TemplateManager::extract_skill_name_from_path(&path), Some("my-skill".to_string()));
    }

    #[test]
    fn test_extract_skill_name_no_skills_segment()
    {
        let path = PathBuf::from("/project/.bogus/commands/my-prompt.md");
        assert_eq!(TemplateManager::extract_skill_name_from_path(&path), None);
    }

    #[test]
    fn test_extract_skill_name_skills_as_last_component()
    {
        let path = PathBuf::from("/project/.bogus/skills");
        assert_eq!(TemplateManager::extract_skill_name_from_path(&path), None);
    }
}
