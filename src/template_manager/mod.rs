//! Template management functionality for slopctl

mod doctor;
mod list;
mod merge;
mod purge;
mod remove;
mod smart;
mod update;

use std::{
    ffi::OsStr,
    fs, io,
    path::{Path, PathBuf}
};

pub use merge::MergeOptions;
use owo_colors::OwoColorize;

use crate::{Result, download_manager::DownloadManager, utils::copy_dir_all};

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

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn test_extract_skill_name_from_cursor_path()
    {
        let path = PathBuf::from("/home/user/project/.cursor/skills/my-skill/SKILL.md");
        assert_eq!(TemplateManager::extract_skill_name_from_path(&path), Some("my-skill".to_string()));
    }

    #[test]
    fn test_extract_skill_name_from_claude_path()
    {
        let path = PathBuf::from("/home/user/project/.claude/skills/code-review/SKILL.md");
        assert_eq!(TemplateManager::extract_skill_name_from_path(&path), Some("code-review".to_string()));
    }

    #[test]
    fn test_extract_skill_name_nested_file()
    {
        let path = PathBuf::from("/project/.cursor/skills/my-skill/scripts/setup.sh");
        assert_eq!(TemplateManager::extract_skill_name_from_path(&path), Some("my-skill".to_string()));
    }

    #[test]
    fn test_extract_skill_name_no_skills_segment()
    {
        let path = PathBuf::from("/project/.cursor/commands/my-prompt.md");
        assert_eq!(TemplateManager::extract_skill_name_from_path(&path), None);
    }

    #[test]
    fn test_extract_skill_name_skills_as_last_component()
    {
        let path = PathBuf::from("/project/.cursor/skills");
        assert_eq!(TemplateManager::extract_skill_name_from_path(&path), None);
    }
}
