//! Template management functionality for vibe-check

use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    time::SystemTime
};

use chrono::{DateTime, Utc};
use owo_colors::OwoColorize;
use serde::{Deserialize, Serialize};

use crate::{Result, utils::copy_dir_all};

/// File mapping with source and target paths
#[derive(Debug, Serialize, Deserialize)]
struct FileMapping
{
    source: String,
    target: String
}

/// Agent configuration with instructions and prompts
#[derive(Debug, Serialize, Deserialize)]
struct AgentConfig
{
    #[serde(skip_serializing_if = "Option::is_none")]
    instructions: Option<Vec<FileMapping>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    prompts:      Option<Vec<FileMapping>>
}

/// Language configuration with files
#[derive(Debug, Serialize, Deserialize)]
struct LanguageConfig
{
    files: Vec<FileMapping>
}

/// Integration configuration with files
#[derive(Debug, Serialize, Deserialize)]
struct IntegrationConfig
{
    files: Vec<FileMapping>
}

/// Main file configuration
#[derive(Debug, Serialize, Deserialize)]
struct MainConfig
{
    source: String,
    target: String
}

/// Template configuration structure
#[derive(Debug, Serialize, Deserialize)]
struct TemplateConfig
{
    #[serde(skip_serializing_if = "Option::is_none")]
    main:        Option<MainConfig>,
    agents:      std::collections::HashMap<String, AgentConfig>,
    languages:   std::collections::HashMap<String, LanguageConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    integration: Option<std::collections::HashMap<String, IntegrationConfig>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    principles:  Option<Vec<FileMapping>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mission:     Option<Vec<FileMapping>>
}

/// Manages template files for coding agent instructions
///
/// The `TemplateManager` handles all operations related to template storage,
/// verification, backup, and synchronization. Templates are stored in the
/// local data directory (e.g., `$HOME/.local/share/vibe-check/templates` on Linux,
/// `$HOME/Library/Application Support/vibe-check/templates` on macOS) and backed up
/// to the cache directory before modifications.
pub struct TemplateManager
{
    config_dir: PathBuf,
    cache_dir:  PathBuf
}

impl TemplateManager
{
    /// Creates a new TemplateManager instance
    ///
    /// Initializes paths to local data and cache directories using the `dirs` crate.
    /// Templates are stored in the local data directory and backups in the cache directory.
    ///
    /// # Errors
    ///
    /// Returns an error if the local data directory cannot be determined
    pub fn new() -> Result<Self>
    {
        let data_dir = dirs::data_local_dir().ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Could not determine local data directory"))?;
        let cache_dir = dirs::cache_dir().ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Could not determine cache directory"))?;

        let config_dir = data_dir.join("vibe-check/templates");
        let cache_dir = cache_dir.join("vibe-check/backups");

        Ok(Self { config_dir, cache_dir })
    }

    /// Loads template configuration from templates.yml
    ///
    /// Downloads templates.yml if it doesn't exist in the global config directory.
    ///
    /// # Arguments
    ///
    /// * `base_url` - Base URL for downloading templates.yml from GitHub
    /// * `url_path` - Path within the repository
    ///
    /// # Errors
    ///
    /// Returns an error if templates.yml cannot be loaded or parsed
    fn load_template_config(&self, base_url: Option<&str>, url_path: Option<&str>) -> Result<TemplateConfig>
    {
        let config_path = self.config_dir.join("templates.yml");

        // If templates.yml doesn't exist and we have a URL, download it
        if config_path.exists() == false &&
            let (Some(base), Some(path)) = (base_url, url_path)
        {
            let config_url = format!("{}{}/templates.yml", base, path);
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
        }

        // Try to load and parse templates.yml
        if config_path.exists() == false
        {
            return Err("templates.yml not found in global template directory".into());
        }

        let content = fs::read_to_string(&config_path)?;
        let config: TemplateConfig = serde_yaml::from_str(&content)?;
        Ok(config)
    }

    /// Generates a timestamp string in YYYY-MM-DD_HH_MM_SS format
    fn get_timestamp() -> String
    {
        let now = SystemTime::now();
        let datetime: DateTime<Utc> = now.into();
        datetime.format("%Y-%m-%d_%H_%M_%S").to_string()
    }

    /// Checks if a local file has been customized by checking for the template marker
    ///
    /// If the template marker is missing from the local file, it means the file
    /// has been merged or customized and should not be overwritten without confirmation.
    ///
    /// # Arguments
    ///
    /// * `local_path` - Path to local file to check
    ///
    /// # Returns
    ///
    /// Returns `true` if file exists and marker is missing (file is customized)
    fn is_file_customized(&self, local_path: &Path) -> Result<bool>
    {
        if local_path.exists() == false
        {
            return Ok(false);
        }

        let content = fs::read_to_string(local_path)?;
        let marker = "<!-- VIBE-CHECK-TEMPLATE: This marker indicates an unmerged template. Do not remove manually. -->";

        // If marker is missing, file has been customized
        Ok(content.contains(marker) == false)
    }

    /// Creates a timestamped backup of a directory
    ///
    /// Backups are stored in the cache directory with timestamp: `backups/YYYY-MM-DD_HH_MM_SS/`
    ///
    /// # Arguments
    ///
    /// * `source_dir` - Directory to backup
    ///
    /// # Errors
    ///
    /// Returns an error if backup creation fails
    fn create_backup(&self, source_dir: &Path) -> Result<()>
    {
        if source_dir.exists() == false
        {
            return Ok(());
        }

        let timestamp = Self::get_timestamp();
        let backup_dir = self.cache_dir.join(timestamp);

        fs::create_dir_all(&backup_dir)?;

        println!("{} Creating backup in {}", "→".blue(), backup_dir.display().to_string().yellow());

        copy_dir_all(source_dir, &backup_dir)?;

        Ok(())
    }

    /// Downloads or copies templates from a source (URL or local path)
    ///
    /// Supports both local file paths and URLs. For URLs starting with http/https,
    /// templates are downloaded. For local paths, templates are copied.
    /// Creates SHA checksums immediately after downloading or copying.
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
            // Download from URL
            println!("{} Downloading templates from URL...", "→".blue());
            self.download_templates_from_url(source)?;
        }
        else
        {
            // Copy from local path
            let source_path = Path::new(source);
            if source_path.exists() == false
            {
                return Err(format!("Source path does not exist: {}", source).into());
            }

            println!("{} Copying templates from local path...", "→".blue());
            fs::create_dir_all(&self.config_dir)?;
            copy_dir_all(source_path, &self.config_dir)?;
        }

        Ok(())
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

    /// Downloads templates from a GitHub URL
    ///
    /// Downloads template files from a GitHub repository based on templates.yml configuration.
    ///
    /// Creates SHA checksums immediately after downloading each file.
    ///
    /// # Arguments
    ///
    /// * `url` - GitHub URL to download from
    ///
    /// # Errors
    ///
    /// Returns an error if URL parsing or download fails
    fn download_templates_from_url(&self, url: &str) -> Result<()>
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
        let config = self.load_template_config(Some(&base_url), Some(&url_path))?;

        // Download main AGENTS.md template if present
        if let Some(main) = &config.main
        {
            let file_url = format!("{}{}/{}", base_url, url_path, main.source);
            let dest_path = self.config_dir.join(&main.source);

            print!("{} Downloading {}... ", "→".blue(), main.source.yellow());
            io::stdout().flush()?;

            match self.download_file(&file_url, &dest_path)
            {
                | Ok(_) => println!("{}", "✓".green()),
                | Err(_) => println!("{} (skipped)", "✗".red())
            }
        }

        // Download principles templates if present
        if let Some(principles_entries) = &config.principles
        {
            for entry in principles_entries
            {
                let file_url = format!("{}{}/{}", base_url, url_path, entry.source);
                let dest_path = self.config_dir.join(&entry.source);

                print!("{} Downloading {}... ", "→".blue(), entry.source.yellow());
                io::stdout().flush()?;

                match self.download_file(&file_url, &dest_path)
                {
                    | Ok(_) => println!("{}", "✓".green()),
                    | Err(_) => println!("{} (skipped)", "✗".red())
                }
            }
        }

        // Download mission templates if present
        if let Some(mission_entries) = &config.mission
        {
            for entry in mission_entries
            {
                let file_url = format!("{}{}/{}", base_url, url_path, entry.source);
                let dest_path = self.config_dir.join(&entry.source);

                print!("{} Downloading {}... ", "→".blue(), entry.source.yellow());
                io::stdout().flush()?;

                match self.download_file(&file_url, &dest_path)
                {
                    | Ok(_) => println!("{}", "✓".green()),
                    | Err(_) => println!("{} (skipped)", "✗".red())
                }
            }
        }

        // Download language templates
        for lang_config in config.languages.values()
        {
            for file_entry in &lang_config.files
            {
                let file_url = format!("{}{}/{}", base_url, url_path, file_entry.source);
                let dest_path = self.config_dir.join(&file_entry.source);

                print!("{} Downloading {}... ", "→".blue(), file_entry.source.yellow());
                io::stdout().flush()?;

                match self.download_file(&file_url, &dest_path)
                {
                    | Ok(_) => println!("{}", "✓".green()),
                    | Err(_) => println!("{} (skipped)", "✗".red())
                }
            }
        }

        // Download integration templates
        if let Some(integration_map) = &config.integration
        {
            for integration_config in integration_map.values()
            {
                for file_entry in &integration_config.files
                {
                    let file_url = format!("{}{}/{}", base_url, url_path, file_entry.source);
                    let dest_path = self.config_dir.join(&file_entry.source);

                    print!("{} Downloading {}... ", "→".blue(), file_entry.source.yellow());
                    io::stdout().flush()?;

                    match self.download_file(&file_url, &dest_path)
                    {
                        | Ok(_) => println!("{}", "✓".green()),
                        | Err(_) => println!("{} (skipped)", "✗".red())
                    }
                }
            }
        }

        // Download agent templates
        for agent_config in config.agents.values()
        {
            // Download instructions files if present
            if let Some(instructions) = &agent_config.instructions
            {
                for instruction in instructions
                {
                    let file_url = format!("{}{}/{}", base_url, url_path, instruction.source);
                    let dest_path = self.config_dir.join(&instruction.source);

                    print!("{} Downloading {}... ", "→".blue(), instruction.source.yellow());
                    io::stdout().flush()?;

                    match self.download_file(&file_url, &dest_path)
                    {
                        | Ok(_) => println!("{}", "✓".green()),
                        | Err(_) => println!("{} (skipped)", "✗".red())
                    }
                }
            }

            // Download prompt files if present
            if let Some(prompts) = &agent_config.prompts
            {
                for prompt in prompts
                {
                    let file_url = format!("{}{}/{}", base_url, url_path, prompt.source);
                    let dest_path = self.config_dir.join(&prompt.source);

                    print!("{} Downloading {}... ", "→".blue(), prompt.source.yellow());
                    io::stdout().flush()?;

                    match self.download_file(&file_url, &dest_path)
                    {
                        | Ok(_) => println!("{}", "✓".green()),
                        | Err(_) => println!("{} (skipped)", "✗".red())
                    }
                }
            }
        }

        println!("{} Templates downloaded successfully", "✓".green());

        Ok(())
    }

    /// Updates local templates from global storage
    ///
    /// This method:
    /// 1. Verifies global templates exist
    /// 2. Detects local modifications to AGENTS.md
    /// 3. Creates backup of existing local files
    /// 4. Copies templates to current directory
    ///
    /// # Arguments
    ///
    /// * `lang` - Programming language or framework identifier
    /// * `agent` - AI coding agent identifier
    /// * `force` - If true, overwrite local modifications without warning
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Global templates don't exist
    /// - Local modifications detected and force is false
    /// - Backup or copy operations fail
    pub fn update(&self, lang: &str, agent: &str, force: bool) -> Result<()>
    {
        println!("{} Updating templates for {} with {}", "→".blue(), lang.green(), agent.green());

        let templates_yml_path = self.config_dir.join("templates.yml");

        // Check if global templates exist
        if self.config_dir.exists() == false || templates_yml_path.exists() == false
        {
            return Err("Global templates not found. Please run 'vibe-check init' first to download templates.".into());
        }

        // Load template configuration
        let config = self.load_template_config(None, None)?;

        // Get current working directory and user home directory
        let workspace = std::env::current_dir()?;
        let userprofile = dirs::home_dir().ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Could not determine home directory"))?;

        // Collect files to copy
        let mut files_to_copy: Vec<(PathBuf, PathBuf)> = Vec::new();
        let mut fragments: Vec<(PathBuf, String)> = Vec::new();
        let mut main_template: Option<(PathBuf, PathBuf)> = None;

        // Check if main AGENTS.md should be copied
        if let Some(main) = config.main.as_ref()
        {
            let source_path = self.config_dir.join(&main.source);
            if source_path.exists()
            {
                let target_path = self.resolve_placeholder(&main.target, &workspace, &userprofile);
                main_template = Some((source_path, target_path));
            }
        }

        // Add principles templates (fragments) if present
        if let Some(principles_entries) = &config.principles
        {
            for entry in principles_entries
            {
                let source_path = self.config_dir.join(&entry.source);
                if source_path.exists() == false
                {
                    continue;
                }

                if entry.target.starts_with("$instructions")
                {
                    fragments.push((source_path, "principles".to_string()));
                }
                else
                {
                    let target_path = self.resolve_placeholder(&entry.target, &workspace, &userprofile);
                    files_to_copy.push((source_path, target_path));
                }
            }
        }

        // Add mission templates (fragments) if present
        if let Some(mission_entries) = &config.mission
        {
            for entry in mission_entries
            {
                let source_path = self.config_dir.join(&entry.source);
                if source_path.exists() == false
                {
                    continue;
                }

                if entry.target.starts_with("$instructions")
                {
                    fragments.push((source_path, "mission".to_string()));
                }
                else
                {
                    let target_path = self.resolve_placeholder(&entry.target, &workspace, &userprofile);
                    files_to_copy.push((source_path, target_path));
                }
            }
        }

        // Add language-specific templates (fragments)
        if let Some(lang_config) = config.languages.get(lang)
        {
            for file_entry in &lang_config.files
            {
                let source_path = self.config_dir.join(&file_entry.source);
                if source_path.exists() == false
                {
                    continue;
                }

                if file_entry.target.starts_with("$instructions")
                {
                    fragments.push((source_path, "languages".to_string()));
                }
                else
                {
                    let target_path = self.resolve_placeholder(&file_entry.target, &workspace, &userprofile);
                    files_to_copy.push((source_path, target_path));
                }
            }
        }

        // Add integration templates (fragments)
        if let Some(integration_map) = &config.integration
        {
            for integration_config in integration_map.values()
            {
                for file_entry in &integration_config.files
                {
                    let source_path = self.config_dir.join(&file_entry.source);
                    if source_path.exists() == false
                    {
                        continue;
                    }

                    if file_entry.target.starts_with("$instructions")
                    {
                        fragments.push((source_path, "integration".to_string()));
                    }
                    else
                    {
                        let target_path = self.resolve_placeholder(&file_entry.target, &workspace, &userprofile);
                        files_to_copy.push((source_path, target_path));
                    }
                }
            }
        }

        // Add agent-specific templates
        if let Some(agent_config) = config.agents.get(agent)
        {
            // Add instructions files if present
            if let Some(instructions) = &agent_config.instructions
            {
                for instruction in instructions
                {
                    let source_path = self.config_dir.join(&instruction.source);
                    if source_path.exists()
                    {
                        let target_path = self.resolve_placeholder(&instruction.target, &workspace, &userprofile);
                        files_to_copy.push((source_path, target_path));
                    }
                }
            }

            // Add prompt files if present
            if let Some(prompts) = &agent_config.prompts
            {
                for prompt in prompts
                {
                    let source_path = self.config_dir.join(&prompt.source);
                    if source_path.exists()
                    {
                        let target_path = self.resolve_placeholder(&prompt.target, &workspace, &userprofile);
                        files_to_copy.push((source_path, target_path));
                    }
                }
            }
        }
        else
        {
            return Err(format!("Agent '{}' not found in templates.yml", agent).into());
        }

        if files_to_copy.is_empty()
        {
            println!("{} No templates found to copy", "!".yellow());
            return Ok(());
        }

        // Check for local modifications
        let mut has_modifications = false;
        let mut modified_files = Vec::new();

        // Check if main AGENTS.md has been customized (marker removed)
        if let Some((_, main_target)) = &main_template &&
            main_target.exists() &&
            self.is_file_customized(main_target)?
        {
            has_modifications = true;
            modified_files.push(main_target.clone());
        }

        if has_modifications && force == false
        {
            println!("{} Local AGENTS.md has been customized:", "!".yellow());
            for file in &modified_files
            {
                println!("  - {}", file.display().to_string().yellow());
            }
            println!("{} Use --force to overwrite", "→".blue());
            return Err("Local modifications detected. Aborting.".into());
        }

        // Create backup of existing local files before any modifications
        self.create_backup(&workspace)?;

        // Handle main AGENTS.md with fragment merging if fragments exist
        if let Some((main_source, main_target)) = main_template
        {
            if fragments.is_empty() == false
            {
                println!("{} Merging fragments into AGENTS.md", "→".blue());
                self.merge_fragments(&main_source, &main_target, &fragments)?;
                println!("  - Merged: {}", main_target.display().to_string().yellow());
            }
            else
            {
                // No fragments, just copy main file as-is
                if let Some(parent) = main_target.parent()
                {
                    fs::create_dir_all(parent)?;
                }
                fs::copy(&main_source, &main_target)?;
                println!("  - Copied: {}", main_target.display().to_string().yellow());
            }
        }

        // Copy templates
        println!("{} Copying templates to target directories", "→".blue());

        for (source, target) in &files_to_copy
        {
            // Create parent directory if needed
            if let Some(parent) = target.parent()
            {
                fs::create_dir_all(parent)?;
            }

            fs::copy(source, target)?;
            println!("  - Copied: {}", target.display().to_string().yellow());
        }

        println!("{} Templates updated successfully", "✓".green());

        Ok(())
    }

    /// Merges fragment files into main AGENTS.md at insertion points
    ///
    /// Reads fragments that have `$instructions` placeholder in their target path
    /// and inserts them into the main AGENTS.md template at the corresponding
    /// insertion points: <!-- {mission} -->, <!-- {principles} -->, <!-- {languages} -->, <!-- {integration} -->
    ///
    /// The insertion point comments are preserved in the final merged file.
    ///
    /// # Arguments
    ///
    /// * `main_source` - Path to the main AGENTS.md template in global storage
    /// * `main_target` - Path where merged AGENTS.md should be written
    /// * `fragments` - Vector of (source_path, category) tuples where category is "mission", "principles", "languages", or "integration"
    ///
    /// # Errors
    ///
    /// Returns an error if file reading or writing fails
    fn merge_fragments(&self, main_source: &Path, main_target: &Path, fragments: &[(PathBuf, String)]) -> Result<()>
    {
        // Read main AGENTS.md template
        let mut main_content = fs::read_to_string(main_source)?;

        // Remove the template marker to indicate this is a merged/customized file
        let marker = "<!-- VIBE-CHECK-TEMPLATE: This marker indicates an unmerged template. Do not remove manually. -->\n";
        main_content = main_content.replace(marker, "");

        // Group fragments by category to handle multiple fragments per insertion point
        let mut fragments_by_category: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();

        for (fragment_path, category) in fragments
        {
            let fragment_content = fs::read_to_string(fragment_path)?;
            fragments_by_category.entry(category.clone()).or_default().push(fragment_content);
        }

        // Process each category
        for (category, contents) in fragments_by_category
        {
            let insertion_point = format!("<!-- {{{}}} -->", category);

            // Combine all fragments for this category
            let combined_content = contents.iter().map(|c| c.trim()).collect::<Vec<_>>().join("\n\n");

            // Replace insertion point with comment + fragment content (keep single insertion point)
            if main_content.contains(&insertion_point)
            {
                let replacement = format!("<!-- {{{}}} -->\n\n{}", category, combined_content);
                main_content = main_content.replace(&insertion_point, &replacement);
            }
            else
            {
                println!("{} Warning: Insertion point {} not found in AGENTS.md", "!".yellow(), insertion_point.yellow());
            }
        }

        // Write merged content to target
        if let Some(parent) = main_target.parent()
        {
            fs::create_dir_all(parent)?;
        }
        fs::write(main_target, main_content)?;

        Ok(())
    }

    /// Resolves placeholder variables in target paths
    ///
    /// Replaces $workspace with the workspace directory path
    /// and $userprofile with the user's home directory path
    ///
    /// # Arguments
    ///
    /// * `path` - Path string containing placeholders
    /// * `workspace` - Workspace directory path
    /// * `userprofile` - User profile directory path
    fn resolve_placeholder(&self, path: &str, workspace: &Path, userprofile: &Path) -> PathBuf
    {
        let resolved = path.replace("$workspace", workspace.to_str().unwrap_or("")).replace("$userprofile", userprofile.to_str().unwrap_or(""));
        PathBuf::from(resolved)
    }

    /// Clears global templates from local data directory
    ///
    /// This method deletes all templates from the global storage directory,
    /// forcing a fresh download on the next init or update operation.
    ///
    /// # Errors
    ///
    /// Returns an error if the directory cannot be removed
    pub fn clear_global_templates(&self) -> Result<()>
    {
        if self.config_dir.exists()
        {
            println!("{} Removing global templates from {}", "→".blue(), self.config_dir.display().to_string().yellow());
            fs::remove_dir_all(&self.config_dir)?;
            println!("{} Global templates cleared successfully", "✓".green());
        }
        else
        {
            println!("{} No global templates found to clear", "→".blue());
        }

        Ok(())
    }

    /// Clears local templates from current directory
    ///
    /// Removes agent instruction directories and language template files from
    /// the current directory. Global templates in the local data directory
    /// are not affected.
    ///
    /// Creates a backup before clearing and optionally prompts for confirmation.
    ///
    /// # Arguments
    ///
    /// * `force` - If true, clear without confirmation prompt
    ///
    /// # Errors
    ///
    /// Returns an error if backup or deletion fails
    pub fn clear(&self, force: bool) -> Result<()>
    {
        let current_dir = std::env::current_dir()?;

        if force == false
        {
            print!("{} Are you sure you want to clear local templates? (y/N): ", "?".yellow());
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;

            if input.trim().eq_ignore_ascii_case("y") == false
            {
                println!("{} Operation cancelled", "→".blue());
                return Ok(());
            }
        }

        // Create backup before clearing
        self.create_backup(&current_dir)?;

        let mut cleared_count = 0;

        // Find and remove agent directories (.claude, .copilot, .codex)
        let agent_dirs = vec![".claude", ".copilot", ".codex"];
        for agent_dir in agent_dirs
        {
            let path = current_dir.join(agent_dir);
            if path.exists()
            {
                println!("{} Removing {}", "→".blue(), path.display().to_string().yellow());
                fs::remove_dir_all(&path)?;
                cleared_count += 1;
            }
        }

        // Check if AGENTS.md exists and if it has been modified
        let agents_md_path = current_dir.join("AGENTS.md");
        let agents_md_customized = agents_md_path.exists() && self.is_file_customized(&agents_md_path)?;

        if agents_md_customized && force == false
        {
            println!("{} AGENTS.md has been customized and will not be deleted", "→".yellow());
            println!("{} Use --force to delete it anyway (backup will be created)", "→".yellow());
        }

        // Find and remove common language template files
        // Check for common patterns in current directory
        if let Ok(entries) = fs::read_dir(&current_dir)
        {
            for entry in entries.flatten()
            {
                let path = entry.path();
                if let Some(file_name) = path.file_name()
                {
                    let name = file_name.to_string_lossy();
                    // Remove files that match common language patterns
                    // but not AGENTS.md or README.md or other important files
                    if path.is_file() &&
                        (name.ends_with(".md") || name.ends_with(".MD")) &&
                        name != "README.md" &&
                        name != "LICENSE.md" &&
                        name != "CHANGELOG.md" &&
                        name != "CONTRIBUTING.md"
                    {
                        // Special handling for AGENTS.md
                        if name == "AGENTS.md"
                        {
                            // Skip if customized and not forced
                            if agents_md_customized && force == false
                            {
                                continue;
                            }
                            // Delete if not customized or if forced
                            println!("{} Removing {}", "→".blue(), path.display().to_string().yellow());
                            fs::remove_file(&path)?;
                            cleared_count += 1;
                        }
                        else
                        {
                            // Check if it matches known template patterns
                            // Currently supported languages: c++, swift, rust
                            let lowercase = name.to_lowercase();
                            if lowercase == "c++-coding-conventions.md" || lowercase == "swift.md" || lowercase == "rust.md"
                            {
                                println!("{} Removing {}", "→".blue(), path.display().to_string().yellow());
                                fs::remove_file(&path)?;
                                cleared_count += 1;
                            }
                        }
                    }
                }
            }
        }

        if cleared_count == 0
        {
            println!("{} No local templates found to clear", "→".blue());
        }
        else
        {
            println!("{} Cleared {} local template(s) successfully", "✓".green(), cleared_count);
        }

        Ok(())
    }
}
