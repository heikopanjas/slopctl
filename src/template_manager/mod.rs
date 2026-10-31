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
    config::LayerSpec,
    download_manager::DownloadManager,
    file_tracker::{self, FileTracker, SLOPCTL_DIR},
    git,
    model_defaults::MODEL_DEFAULTS_FILE,
    template_engine::load_template_config,
    template_layer::{self, SourceManifest},
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
        self.download_or_copy_templates_from(&CatalogSource::uri(source))
    }

    /// Fetches the baseline templates from a source with optional git `ref` and subdirectory `path`
    ///
    /// # Errors
    ///
    /// Returns an error if the fetch fails or the source holds no valid `templates.yml`
    pub fn download_or_copy_templates_from(&self, source: &CatalogSource) -> Result<()>
    {
        fetch_catalog(source, &self.config_dir)
    }

    /// Fetches the template layers configured for a workspace
    ///
    /// `templates --update` refreshes every layer; `init` passes `only_missing` to fetch just the
    /// layers that were never fetched or whose configuration changed since.
    ///
    /// # Arguments
    ///
    /// * `workspace` - Workspace whose (global + workspace) config lists the layers
    /// * `only_missing` - Skip layers whose cache matches their configuration
    /// * `dry_run` - Only report what would be fetched
    ///
    /// # Errors
    ///
    /// Returns an error if the layer configuration is invalid or a fetch fails
    pub fn fetch_layers(&self, workspace: &Path, only_missing: bool, dry_run: bool) -> Result<()>
    {
        for (spec, scope) in crate::agent_overlay::configured_layers(workspace)?
        {
            let dir = template_layer::layer_cache_dir(&self.config_dir, &spec.id);
            let current = SourceManifest::read(&dir).ok().flatten().is_some_and(|manifest| manifest.matches(&spec) == true);
            if only_missing == true && current == true
            {
                println!("{} Template layer '{}' is cached", "✓".green(), spec.id.green());
            }
            else if dry_run == true
            {
                println!("{} Dry run: would fetch template layer '{}' ({}) from {}", "→".blue(), spec.id.green(), scope, spec.uri.yellow());
            }
            else
            {
                self.fetch_layer(&spec)?;
            }
        }
        Ok(())
    }

    /// Fetches a configured template layer into its cache directory
    ///
    /// # Errors
    ///
    /// Returns an error if the fetch fails or the layer's `templates.yml` declares another id
    pub fn fetch_layer(&self, spec: &LayerSpec) -> Result<()>
    {
        println!("{} Fetching template layer '{}'...", "→".blue(), spec.id.green());
        fetch_catalog(&CatalogSource::layer(spec), &template_layer::layer_cache_dir(&self.config_dir, &spec.id))
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

/// Where a template catalog is fetched from
///
/// `uri` is a local path, a GitHub tree URL, or a git URL; `ref` (git only) and `path` select a
/// branch/tag and the subdirectory holding `templates.yml`. Layers also carry the id their
/// `templates.yml` must declare.
#[derive(Debug, Clone, Copy)]
pub struct CatalogSource<'a>
{
    pub uri:         &'a str,
    pub git_ref:     Option<&'a str>,
    pub path:        Option<&'a str>,
    pub expected_id: Option<&'a str>
}

impl<'a> CatalogSource<'a>
{
    /// A plain source URI (local path or URL), as used for the baseline
    pub fn uri(uri: &'a str) -> Self
    {
        Self { uri, git_ref: None, path: None, expected_id: None }
    }

    /// The source of a configured template layer
    pub fn layer(spec: &'a LayerSpec) -> Self
    {
        Self { uri: &spec.uri, git_ref: spec.git_ref.as_deref(), path: spec.path.as_deref(), expected_id: Some(&spec.id) }
    }
}

/// Fetches a template catalog from `source` into `dest`, replacing its previous content
///
/// The catalog is staged in a sibling directory and swapped in only after the fetch succeeded,
/// so a failed fetch leaves the old cache intact and files dropped upstream do not linger.
/// Agent and model defaults and the legacy tracker have their own update paths and are carried
/// over when the new catalog does not ship them. Every fetch writes a [`SourceManifest`].
///
/// # Errors
///
/// Returns an error if the source is invalid, the fetch fails, the catalog declares the wrong id,
/// or the cache cannot be replaced
pub(crate) fn fetch_catalog(source: &CatalogSource, dest: &Path) -> Result<()>
{
    let parent = dest.parent().ok_or_else(|| anyhow::anyhow!("Cache directory {} has no parent", dest.display()))?;
    fs::create_dir_all(parent)?;
    let staging = tempfile::Builder::new().prefix(".slopctl-staging-").tempdir_in(parent)?;
    fetch_catalog_into(source, staging.path())?;

    if let Some(expected) = source.expected_id
    {
        let declared = load_template_config(staging.path())?.id;
        require!(
            declared.as_deref() == Some(expected),
            Err(anyhow::anyhow!(
                "templates.yml from '{}' declares id '{}', but it is configured as template layer '{}'",
                source.uri,
                declared.unwrap_or_default(),
                expected
            ))
        );
    }

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

/// Fetches a validated catalog into an empty directory and writes its manifest
///
/// Git URLs are cloned with the user's `git`, other `http(s)` URLs are GitHub tree URLs downloaded
/// over HTTP, and anything else is a local directory. URL-based skills are cached in every case.
fn fetch_catalog_into(source: &CatalogSource, dest: &Path) -> Result<()>
{
    let (commit, url_skills) = if git::is_git_url(source.uri) == true
    {
        println!("{} Cloning {}...", "→".blue(), source.uri.yellow());
        let checkout = tempfile::TempDir::new()?;
        let clone = checkout.path().join("repo");
        git::clone_shallow(source.uri, source.git_ref, &clone)?;
        let commit = git::head_commit(&clone)?;
        fs::remove_dir_all(clone.join(".git"))?;
        let config = copy_validated_catalog(&catalog_root(&clone, source.path)?, dest, source.uri)?;
        (Some(commit), DownloadManager::new(dest.to_path_buf()).cache_url_skills(&config)?)
    }
    else if source.uri.starts_with("http://") == true || source.uri.starts_with("https://") == true
    {
        require!(
            source.git_ref.is_none() == true && source.path.is_none() == true,
            Err(anyhow::anyhow!("'ref' and 'path' only apply to git sources; a GitHub tree URL already names its branch and directory"))
        );
        println!("{} Downloading templates from URL...", "→".blue());
        (None, DownloadManager::new(dest.to_path_buf()).download_templates_from_url(source.uri)?)
    }
    else
    {
        require!(source.git_ref.is_none() == true, Err(anyhow::anyhow!("'ref' only applies to git sources, not to the local path '{}'", source.uri)));
        let config = copy_validated_catalog(&catalog_root(Path::new(source.uri), source.path)?, dest, source.uri)?;
        (None, DownloadManager::new(dest.to_path_buf()).cache_url_skills(&config)?)
    };

    SourceManifest {
        uri: source.uri.to_string(),
        git_ref: source.git_ref.map(str::to_string),
        path: source.path.map(str::to_string),
        commit,
        fetched: chrono::Utc::now().to_rfc3339(),
        url_skills
    }
    .write(dest)
}

/// Resolves the catalog directory inside a checkout or local source
fn catalog_root(root: &Path, path: Option<&str>) -> Result<PathBuf>
{
    match path
    {
        // `.` names the root itself, which `safe_relative_path` rejects as naming nothing
        | Some(path) if Path::new(path).components().all(|component| component == std::path::Component::CurDir) == false =>
            Ok(root.join(crate::utils::safe_relative_path(path)?)),
        | _ => Ok(root.to_path_buf())
    }
}

/// Validates the `templates.yml` in `source_path` and copies the catalog into `dest`
fn copy_validated_catalog(source_path: &Path, dest: &Path, uri: &str) -> Result<crate::bom::TemplateConfig>
{
    require!(source_path.exists() == true, Err(anyhow::anyhow!("Source path does not exist: {}", source_path.display())));
    let source_config = source_path.join("templates.yml");
    require!(
        source_config.exists() == true,
        Err(anyhow::anyhow!(
            "Source '{}' has no templates.yml at {}; a template source must include one (set its 'path' if the catalog lives in a subdirectory)",
            uri,
            source_path.display()
        ))
    );
    let config = serde_yaml::from_str::<crate::bom::TemplateConfig>(&fs::read_to_string(&source_config)?)
        .map_err(|e| anyhow::anyhow!("Invalid templates.yml in source '{}': {}", uri, e))?;

    println!("{} Copying templates from {}...", "→".blue(), uri.yellow());
    let copied = copy_dir_all(source_path, dest)?;
    println!("{} Copied {} template file(s)", "✓".green(), copied);
    Ok(config)
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
