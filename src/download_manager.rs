//! Download management functionality for vibe-check
//!
//! Handles downloading templates from GitHub repositories.

use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf}
};

use owo_colors::OwoColorize;

use crate::{Result, bom::TemplateConfig};

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
        let (owner, repo, branch, path) = self.parse_github_url(url).ok_or("Invalid GitHub URL format. Expected: https://github.com/owner/repo/tree/branch/path")?;

        println!("{} Repository: {}/{} (branch: {})", "→".blue(), owner.green(), repo.green(), branch.yellow());

        // Build base raw URL
        let base_url = format!("https://raw.githubusercontent.com/{}/{}/{}", owner, repo, branch);
        let url_path = if path.is_empty() == false
        {
            format!("/{}", path)
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

            match self.download_file(&file_url, &dest_path)
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

        // Download principles templates if present
        if let Some(principles_entries) = &config.principles
        {
            for entry in principles_entries
            {
                download_entry(&entry.source)?;
            }
        }

        // Download mission templates if present
        if let Some(mission_entries) = &config.mission
        {
            for entry in mission_entries
            {
                download_entry(&entry.source)?;
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

        // Download integration templates
        if let Some(integration_map) = &config.integration
        {
            for integration_config in integration_map.values()
            {
                for file_entry in &integration_config.files
                {
                    download_entry(&file_entry.source)?;
                }
            }
        }

        // Download agent templates (if agents section exists)
        if let Some(agents) = &config.agents
        {
            for agent_config in agents.values()
            {
                if let Some(instructions) = &agent_config.instructions
                {
                    for instruction in instructions
                    {
                        download_entry(&instruction.source)?;
                    }
                }

                if let Some(prompts) = &agent_config.prompts
                {
                    for prompt in prompts
                    {
                        download_entry(&prompt.source)?;
                    }
                }

                if let Some(skills) = &agent_config.skills
                {
                    for skill in skills
                    {
                        download_entry(&skill.source)?;
                    }
                }
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

        match self.download_file(&config_url, &config_path)
        {
            | Ok(_) => println!("{}", "✓".green()),
            | Err(e) =>
            {
                println!("{}", "✗".red());
                return Err(format!("Failed to download templates.yml: {}", e).into());
            }
        }

        let content = fs::read_to_string(&config_path)?;
        let config: TemplateConfig = serde_yaml::from_str(&content)?;
        Ok(config)
    }

    /// Converts a GitHub tree URL to raw content URLs
    ///
    /// Converts URLs like:
    /// `https://github.com/owner/repo/tree/branch/path`
    /// to:
    /// `https://raw.githubusercontent.com/owner/repo/branch/path`
    ///
    /// # Arguments
    ///
    /// * `url` - GitHub tree URL
    ///
    /// # Returns
    ///
    /// Returns base raw URL and path components, or None if URL is not a GitHub tree URL
    fn parse_github_url(&self, url: &str) -> Option<(String, String, String, String)>
    {
        // Parse URLs like: https://github.com/owner/repo/tree/branch/path
        if url.contains("github.com") == false
        {
            return None;
        }

        let parts: Vec<&str> = url.split('/').collect();

        // Find the indices for owner, repo, tree, branch
        let github_idx = parts.iter().position(|&p| p == "github.com")?;

        if parts.len() < github_idx + 5
        {
            return None;
        }

        let owner = parts[github_idx + 1];
        let repo = parts[github_idx + 2];
        let tree_or_blob = parts[github_idx + 3];

        if tree_or_blob != "tree" && tree_or_blob != "blob"
        {
            return None;
        }

        let branch = parts[github_idx + 4];
        let path = if parts.len() > github_idx + 5
        {
            parts[github_idx + 5..].join("/")
        }
        else
        {
            String::new()
        };

        Some((owner.to_string(), repo.to_string(), branch.to_string(), path))
    }

    /// Downloads a file from a URL
    ///
    /// # Arguments
    ///
    /// * `url` - URL to download from
    /// * `dest_path` - Destination file path
    ///
    /// # Errors
    ///
    /// Returns an error if download or file write fails
    fn download_file(&self, url: &str, dest_path: &Path) -> Result<()>
    {
        let response = reqwest::blocking::get(url)?;

        if response.status().is_success() == false
        {
            return Err(format!("Failed to download {}: HTTP {}", url, response.status()).into());
        }

        let content = response.bytes()?;

        if let Some(parent) = dest_path.parent()
        {
            fs::create_dir_all(parent)?;
        }

        fs::write(dest_path, content)?;

        Ok(())
    }
}
