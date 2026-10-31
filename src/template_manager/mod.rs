//! Template management functionality for slopctl

mod agents;
mod doctor;
mod list;
mod merge;
mod models;
mod partial_update;
mod remove;
mod smart;
mod update;
mod verify;

#[cfg(test)] mod integration_tests;

use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf}
};

pub use merge::MergeOptions;
use owo_colors::OwoColorize;

use crate::{
    Result,
    agent_defaults::AGENT_DEFAULTS_FILE,
    download_manager::DownloadManager,
    file_tracker::{self, FileTracker, SLOPCTL_DIR},
    model_defaults::MODEL_DEFAULTS_FILE,
    utils::copy_dir_all
};

/// Manages template files for coding agent instructions
///
/// The `TemplateManager` handles all operations related to template storage,
/// verification, and synchronization. Templates are stored in the global
/// cache directory (`$XDG_CACHE_HOME/slopctl/templates`, or
/// `$HOME/.cache/slopctl/templates` — same on all platforms).
pub struct TemplateManager
{
    pub(crate) config_dir: PathBuf
}

impl TemplateManager
{
    /// Creates a new TemplateManager instance
    ///
    /// Initializes path to the global cache directory via
    /// [`crate::utils::global_cache_dir`].
    ///
    /// # Errors
    ///
    /// Returns an error if the cache directory cannot be determined
    pub fn new() -> Result<Self>
    {
        let config_dir = crate::utils::global_cache_dir()?;

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
    /// templates are downloaded. For local paths, templates are copied. The cache is
    /// replaced as a whole, see [`fetch_catalog`].
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
        fetch_catalog(source, &self.config_dir)
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

        let catalogs = crate::agent_overlay::load_effective_catalogs(&self.config_dir, workspace)?;
        let adopted = tracker.adopt_untracked_files_from_catalog(workspace, &catalogs.agents)?;
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

    /// Returns the `skills/<name>` root directory containing a skill path.
    pub(crate) fn skill_root_from_path(path: &Path) -> Option<PathBuf>
    {
        path.ancestors().find(|candidate| candidate.parent().and_then(Path::file_name) == Some(OsStr::new("skills"))).map(Path::to_path_buf)
    }
}

/// Fetches a template catalog from `source` into `dest`, replacing its previous content
///
/// The catalog is staged in a sibling directory and swapped in only after the fetch succeeded,
/// so a failed fetch leaves the old cache intact and files dropped upstream do not linger.
/// Agent and model defaults and the legacy tracker have their own update paths and are carried
/// over when the new catalog does not ship them.
///
/// # Errors
///
/// Returns an error if the source is invalid, the fetch fails, or the cache cannot be replaced
pub(crate) fn fetch_catalog(source: &str, dest: &Path) -> Result<()>
{
    let parent = dest.parent().ok_or_else(|| anyhow::anyhow!("Cache directory {} has no parent", dest.display()))?;
    fs::create_dir_all(parent)?;
    let staging = tempfile::Builder::new().prefix(".slopctl-staging-").tempdir_in(parent)?;
    fetch_catalog_into(source, staging.path())?;

    if dest.exists() == false
    {
        fs::rename(staging.path(), dest)?;
        let _ = staging.keep();
        return Ok(());
    }

    for preserved in [dest.join(AGENT_DEFAULTS_FILE), dest.join(MODEL_DEFAULTS_FILE), file_tracker::legacy_tracker_path(dest)]
    {
        if let Some(name) = preserved.file_name() &&
            preserved.is_file() == true &&
            staging.path().join(name).exists() == false
        {
            fs::copy(&preserved, staging.path().join(name))?;
        }
    }

    // Move the old cache aside first so a failed swap can be rolled back; the retired dir is
    // deleted when it goes out of scope.
    let retired = tempfile::Builder::new().prefix(".slopctl-retired-").tempdir_in(parent)?;
    let previous = retired.path().join("previous");
    fs::rename(dest, &previous)?;
    if let Err(e) = fs::rename(staging.path(), dest)
    {
        fs::rename(&previous, dest)?;
        return Err(e.into());
    }
    let _ = staging.keep();
    Ok(())
}

/// Downloads (URL) or copies (local path) a validated catalog into an empty directory
fn fetch_catalog_into(source: &str, dest: &Path) -> Result<()>
{
    if source.starts_with("http://") || source.starts_with("https://")
    {
        println!("{} Downloading templates from URL...", "→".blue());
        DownloadManager::new(dest.to_path_buf()).download_templates_from_url(source)
    }
    else
    {
        let source_path = Path::new(source);
        require!(source_path.exists() == true, Err(anyhow::anyhow!("Source path does not exist: {}", source)));

        let source_config = source_path.join("templates.yml");
        require!(
            source_config.exists() == true,
            Err(anyhow::anyhow!("Source path '{}' does not contain a templates.yml; a template source must include one", source))
        );
        serde_yaml::from_str::<crate::bom::TemplateConfig>(&fs::read_to_string(&source_config)?)
            .map_err(|e| anyhow::anyhow!("Invalid templates.yml in source '{}': {}", source, e))?;

        println!("{} Copying templates from local path...", "→".blue());
        let copied = copy_dir_all(source_path, dest)?;
        println!("{} Copied {} template file(s) from {}", "✓".green(), copied, source_path.display().to_string().yellow());
        Ok(())
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
    fn test_download_or_copy_templates_local_missing_templates_yml_errors() -> anyhow::Result<()>
    {
        let source = tempfile::TempDir::new()?;
        let config_dir = tempfile::TempDir::new()?;
        fs::write(source.path().join("stray.md"), "# not a catalog\n")?;

        let manager = TemplateManager { config_dir: config_dir.path().to_path_buf() };
        let result = manager.download_or_copy_templates(&source.path().to_string_lossy());

        assert!(result.is_err() == true);
        assert!(result.unwrap_err().to_string().contains("templates.yml") == true);
        assert!(config_dir.path().join("stray.md").exists() == false, "nothing must be copied when validation fails");
        Ok(())
    }

    #[test]
    fn test_download_or_copy_templates_local_invalid_yaml_errors() -> anyhow::Result<()>
    {
        let source = tempfile::TempDir::new()?;
        let config_dir = tempfile::TempDir::new()?;
        fs::write(source.path().join("templates.yml"), "languages: [not, a, map\n")?;

        let manager = TemplateManager { config_dir: config_dir.path().to_path_buf() };
        let result = manager.download_or_copy_templates(&source.path().to_string_lossy());

        assert!(result.is_err() == true);
        assert!(result.unwrap_err().to_string().contains("Invalid templates.yml") == true);
        Ok(())
    }

    #[test]
    fn test_download_or_copy_templates_local_valid_source_copies() -> anyhow::Result<()>
    {
        let source = tempfile::TempDir::new()?;
        let config_dir = tempfile::TempDir::new()?;
        fs::write(source.path().join("templates.yml"), "version: 5\nlanguages: {}\n")?;
        fs::write(source.path().join("AGENTS.md"), "# Template\n")?;

        let manager = TemplateManager { config_dir: config_dir.path().to_path_buf() };
        manager.download_or_copy_templates(&source.path().to_string_lossy())?;

        assert!(config_dir.path().join("templates.yml").exists() == true);
        assert!(config_dir.path().join("AGENTS.md").exists() == true);
        Ok(())
    }

    #[test]
    fn test_download_or_copy_templates_local_refetch_drops_stale_files() -> anyhow::Result<()>
    {
        let source = tempfile::TempDir::new()?;
        let cache_root = tempfile::TempDir::new()?;
        let config_dir = cache_root.path().join("templates");
        fs::write(source.path().join("templates.yml"), "version: 5\nlanguages: {}\n")?;
        fs::create_dir_all(source.path().join("skills/review"))?;
        fs::write(source.path().join("skills/review/SKILL.md"), "# Review\n")?;
        fs::write(source.path().join("skills/review/old.md"), "# Dropped upstream\n")?;

        let manager = TemplateManager { config_dir: config_dir.clone() };
        manager.download_or_copy_templates(&source.path().to_string_lossy())?;
        fs::write(config_dir.join(crate::agent_defaults::AGENT_DEFAULTS_FILE), "version: 1\nagents: []\n")?;

        fs::remove_file(source.path().join("skills/review/old.md"))?;
        manager.download_or_copy_templates(&source.path().to_string_lossy())?;

        assert!(config_dir.join("skills/review/SKILL.md").exists() == true);
        assert!(config_dir.join("skills/review/old.md").exists() == false, "files dropped upstream must leave the cache");
        assert!(config_dir.join(crate::agent_defaults::AGENT_DEFAULTS_FILE).exists() == true, "agent defaults have their own update path");
        assert_eq!(fs::read_dir(cache_root.path())?.count(), 1, "no staging leftovers next to the cache");
        Ok(())
    }

    #[test]
    fn test_download_or_copy_templates_failed_fetch_keeps_existing_cache() -> anyhow::Result<()>
    {
        let source = tempfile::TempDir::new()?;
        let cache_root = tempfile::TempDir::new()?;
        let config_dir = cache_root.path().join("templates");
        fs::write(source.path().join("templates.yml"), "version: 5\nlanguages: {}\n")?;

        let manager = TemplateManager { config_dir: config_dir.clone() };
        manager.download_or_copy_templates(&source.path().to_string_lossy())?;
        fs::write(source.path().join("templates.yml"), "languages: [broken\n")?;

        assert!(manager.download_or_copy_templates(&source.path().to_string_lossy()).is_err() == true);
        assert_eq!(fs::read_to_string(config_dir.join("templates.yml"))?, "version: 5\nlanguages: {}\n");
        Ok(())
    }

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

    #[test]
    fn test_skill_root_from_nested_path()
    {
        let path = PathBuf::from("/project/.bogus/skills/my-skill/references/topic.md");
        assert_eq!(TemplateManager::skill_root_from_path(&path), Some(PathBuf::from("/project/.bogus/skills/my-skill")));
    }

    #[test]
    fn test_skill_root_from_non_skill_path()
    {
        let path = PathBuf::from("/project/.bogus/commands/my-prompt.md");
        assert_eq!(TemplateManager::skill_root_from_path(&path), None);
    }
}
