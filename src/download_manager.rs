//! Download management functionality for vibe-cop
//!
//! Handles downloading templates from GitHub repositories.

use std::{
    fs,
    io::{self, Write},
    path::PathBuf
};

use owo_colors::OwoColorize;

use crate::{Result, bom::TemplateConfig, github};

/// Manages downloading templates from remote sources
///
/// The `DownloadManager` handles all operations related to downloading
/// templates from GitHub repositories.
pub struct DownloadManager
{
    config_dir: PathBuf
}

impl DownloadManager
{
    /// Creates a new DownloadManager instance
    ///
    /// # Arguments
    ///
    /// * `config_dir` - Path to the global template storage directory
    pub fn new(config_dir: PathBuf) -> Self
    {
        Self { config_dir }
    }

    /// Downloads templates from a GitHub URL
    ///
    /// Downloads template files from a GitHub repository based on templates.yml configuration.
    ///
    /// # Arguments
    ///
    /// * `url` - GitHub URL to download from
    ///
    /// # Errors
    ///
    /// Returns an error if URL parsing or download fails
    pub fn download_templates_from_url(&self, url: &str) -> Result<()>
    {
        let parsed =
            github::parse_github_url(url).ok_or_else(|| anyhow::anyhow!("Invalid GitHub URL format. Expected: https://github.com/owner/repo/tree/branch/path"))?;

        println!("{} Repository: {}/{} (branch: {})", "→".blue(), parsed.owner.green(), parsed.repo.green(), parsed.branch.yellow());

        // Build base raw URL
        let base_url = format!("https://raw.githubusercontent.com/{}/{}/{}", parsed.owner, parsed.repo, parsed.branch);
        let url_path = if parsed.path.is_empty() == false
        {
            format!("/{}", parsed.path)
        }
        else
        {
            String::new()
        };

        fs::create_dir_all(&self.config_dir)?;

        // Load template configuration
        let config = self.load_template_config(&base_url, &url_path)?;

        // Helper closure to download a file entry
        let download_entry = |source: &str| -> Result<()> {
            let file_url = format!("{}{}/{}", base_url, url_path, source);
            let dest_path = self.config_dir.join(source);

            print!("{} Downloading {}... ", "→".blue(), source.yellow());
            io::stdout().flush()?;

            match github::download_file(&file_url, &dest_path)
            {
                | Ok(_) => println!("{}", "✓".green()),
                | Err(_) => println!("{} (skipped)", "✗".red())
            }
            Ok(())
        };

        // Download main AGENTS.md template if present
        if let Some(main) = &config.main
        {
            download_entry(&main.source)?;
        }

        for entry in &config.principles
        {
            download_entry(&entry.source)?;
        }

        for entry in &config.mission
        {
            download_entry(&entry.source)?;
        }

        // Download shared file groups (used by language includes)
        for shared_files in config.shared.values()
        {
            for file_entry in shared_files
            {
                download_entry(&file_entry.source)?;
            }
        }

        // Download language templates
        for lang_config in config.languages.values()
        {
            for file_entry in &lang_config.files
            {
                download_entry(&file_entry.source)?;
            }
        }

        for integration_config in config.integration.values()
        {
            for file_entry in &integration_config.files
            {
                download_entry(&file_entry.source)?;
            }
        }

        for agent_config in config.agents.values()
        {
            for entry in agent_config.instructions.iter().chain(&agent_config.prompts)
            {
                download_entry(&entry.source)?;
            }
        }

        println!("{} Templates downloaded successfully", "✓".green());

        Ok(())
    }

    /// Loads template configuration from templates.yml
    ///
    /// Downloads templates.yml from the remote URL.
    ///
    /// # Arguments
    ///
    /// * `base_url` - Base URL for downloading templates.yml from GitHub
    /// * `url_path` - Path within the repository
    ///
    /// # Errors
    ///
    /// Returns an error if templates.yml cannot be loaded or parsed
    fn load_template_config(&self, base_url: &str, url_path: &str) -> Result<TemplateConfig>
    {
        let config_path = self.config_dir.join("templates.yml");
        let config_url = format!("{}{}/templates.yml", base_url, url_path);

        print!("{} Downloading templates.yml... ", "→".blue());
        io::stdout().flush()?;

        match github::download_file(&config_url, &config_path)
        {
            | Ok(_) => println!("{}", "✓".green()),
            | Err(e) =>
            {
                println!("{}", "✗".red());
                return Err(anyhow::anyhow!("Failed to download templates.yml: {}", e));
            }
        }

        let content = fs::read_to_string(&config_path)?;
        let config: TemplateConfig = serde_yaml::from_str(&content)?;
        Ok(config)
    }
}
