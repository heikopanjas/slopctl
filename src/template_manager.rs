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
use sha2::{Digest, Sha256};

use crate::{Result, utils::copy_dir_all};

/// File mapping with source and target paths
#[derive(Debug, Serialize, Deserialize)]
struct FileMapping
{
    source: String,
    target: String
}

/// Agent instruction configuration
#[derive(Debug, Serialize, Deserialize)]
struct AgentInstruction
{
    source: String,
    target: String
}

/// Agent configuration with instruction and prompts
#[derive(Debug, Serialize, Deserialize)]
struct AgentConfig
{
    #[serde(skip_serializing_if = "Option::is_none")]
    instruction: Option<AgentInstruction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    prompts:     Option<Vec<FileMapping>>
}

/// Language configuration with files
#[derive(Debug, Serialize, Deserialize)]
struct LanguageConfig
{
    files: Vec<FileMapping>
}

/// Template configuration structure
#[derive(Debug, Serialize, Deserialize)]
struct TemplateConfig
{
    agents:    std::collections::HashMap<String, AgentConfig>,
    languages: std::collections::HashMap<String, LanguageConfig>,
    general:   Vec<FileMapping>
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
        if config_path.exists() == false
        {
            if let (Some(base), Some(path)) = (base_url, url_path)
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
        }

        // Try to load and parse templates.yml
        if config_path.exists() == true
        {
            let content = fs::read_to_string(&config_path)?;
            let config: TemplateConfig = serde_yaml::from_str(&content)?;
            Ok(config)
        }
        else
        {
            // Return default configuration if file doesn't exist
            let mut agents = std::collections::HashMap::new();
            agents.insert("claude".to_string(), AgentConfig {
                instruction: Some(AgentInstruction { source: "claude/CLAUDE.md".to_string(), target: "$workspace/CLAUDE.md".to_string() }),
                prompts:     Some(vec![FileMapping {
                    source: "claude/commands/init-session.md".to_string(),
                    target: "$workspace/.claude/commands/init-session.md".to_string()
                }])
            });
            agents.insert("copilot".to_string(), AgentConfig {
                instruction: Some(AgentInstruction {
                    source: "copilot/copilot-instructions.md".to_string(),
                    target: "$workspace/.github/copilot-instructions.md".to_string()
                }),
                prompts:     Some(vec![FileMapping {
                    source: "copilot/prompts/init-session.prompt.md".to_string(),
                    target: "$workspace/.github/prompts/init-session.prompt.md".to_string()
                }])
            });

            let mut languages = std::collections::HashMap::new();
            languages.insert("c++".to_string(), LanguageConfig { files: vec![FileMapping { source: "c++.md".to_string(), target: "$workspace/c++.md".to_string() }] });

            let general = vec![
                FileMapping { source: "AGENTS.md".to_string(), target: "$workspace/AGENTS.md".to_string() },
                FileMapping { source: "cmake.md".to_string(), target: "$workspace/cmake.md".to_string() },
                FileMapping { source: "general.md".to_string(), target: "$workspace/general.md".to_string() },
                FileMapping { source: "git.md".to_string(), target: "$workspace/git.md".to_string() },
            ];

            Ok(TemplateConfig { agents, languages, general })
        }
    }

    /// Generates a timestamp string in YYYY-MM-DD_HH_MM_SS format
    fn get_timestamp() -> String
    {
        let now = SystemTime::now();
        let datetime: DateTime<Utc> = now.into();
        datetime.format("%Y-%m-%d_%H_%M_%S").to_string()
    }

    /// Calculates SHA-256 checksum for a file
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to the file to checksum
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read
    fn calculate_checksum(&self, file_path: &Path) -> Result<String>
    {
        let content = fs::read(file_path)?;
        let mut hasher = Sha256::new();
        hasher.update(&content);
        Ok(hex::encode(hasher.finalize()))
    }

    /// Checks if local file has been modified compared to global template
    ///
    /// Compares SHA-256 checksums of local and global files.
    ///
    /// # Arguments
    ///
    /// * `local_path` - Path to local file
    /// * `global_path` - Path to global template
    ///
    /// # Returns
    ///
    /// Returns `true` if files differ, `false` if identical or local doesn't exist
    fn has_local_modifications(&self, local_path: &Path, global_path: &Path) -> Result<bool>
    {
        if local_path.exists() == false
        {
            return Ok(false);
        }

        let local_checksum = self.calculate_checksum(local_path)?;
        let global_checksum = self.calculate_checksum(global_path)?;

        Ok(local_checksum != global_checksum)
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

    /// Downloads or copies templates from a source
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
    fn download_or_copy_templates(&self, source: &str) -> Result<()>
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

            // Create checksums for all copied files
            println!("{} Creating checksums for copied templates...", "→".blue());
            self.create_checksums_for_directory(&self.config_dir)?;
        }

        Ok(())
    }

    /// Creates checksums for all template files in a directory
    ///
    /// Recursively walks through the directory and creates .sha files
    /// for all .md files found.
    ///
    /// # Arguments
    ///
    /// * `dir` - Directory to create checksums for
    ///
    /// # Errors
    ///
    /// Returns an error if checksum creation fails
    fn create_checksums_for_directory(&self, dir: &Path) -> Result<()>
    {
        if dir.exists() == false
        {
            return Ok(());
        }

        for entry in fs::read_dir(dir)?
        {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir()
            {
                // Recursively process subdirectories
                self.create_checksums_for_directory(&path)?;
            }
            else if path.is_file()
            {
                // Create checksum for .md files
                if let Some(ext) = path.extension()
                {
                    if ext == "md"
                    {
                        let checksum = self.calculate_checksum(&path)?;
                        let checksum_path = path.with_extension("sha");
                        fs::write(&checksum_path, checksum)?;
                    }
                }
            }
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

        // Download general templates
        for entry in &config.general
        {
            let file_url = format!("{}{}/{}", base_url, url_path, entry.source);
            let dest_path = self.config_dir.join(&entry.source);

            print!("{} Downloading {}... ", "→".blue(), entry.source.yellow());
            io::stdout().flush()?;

            match self.download_file(&file_url, &dest_path)
            {
                | Ok(_) =>
                {
                    println!("{}", "✓".green());
                    // Create checksum immediately after download
                    let checksum = self.calculate_checksum(&dest_path)?;
                    let checksum_path = dest_path.with_extension("sha");
                    fs::write(&checksum_path, checksum)?;
                }
                | Err(_) => println!("{} (skipped)", "✗".red())
            }
        }

        // Download language templates
        for (_lang_name, lang_config) in &config.languages
        {
            for file_entry in &lang_config.files
            {
                let file_url = format!("{}{}/{}", base_url, url_path, file_entry.source);
                let dest_path = self.config_dir.join(&file_entry.source);

                print!("{} Downloading {}... ", "→".blue(), file_entry.source.yellow());
                io::stdout().flush()?;

                match self.download_file(&file_url, &dest_path)
                {
                    | Ok(_) =>
                    {
                        println!("{}", "✓".green());
                        // Create checksum immediately after download
                        let checksum = self.calculate_checksum(&dest_path)?;
                        let checksum_path = dest_path.with_extension("sha");
                        fs::write(&checksum_path, checksum)?;
                    }
                    | Err(_) => println!("{} (skipped)", "✗".red())
                }
            }
        }

        // Download agent templates
        for (_agent_name, agent_config) in &config.agents
        {
            // Download instruction file if present
            if let Some(instruction) = &agent_config.instruction
            {
                let file_url = format!("{}{}/{}", base_url, url_path, instruction.source);
                let dest_path = self.config_dir.join(&instruction.source);

                print!("{} Downloading {}... ", "→".blue(), instruction.source.yellow());
                io::stdout().flush()?;

                match self.download_file(&file_url, &dest_path)
                {
                    | Ok(_) =>
                    {
                        println!("{}", "✓".green());
                        // Create checksum immediately after download
                        let checksum = self.calculate_checksum(&dest_path)?;
                        let checksum_path = dest_path.with_extension("sha");
                        fs::write(&checksum_path, checksum)?;
                    }
                    | Err(_) => println!("{} (skipped)", "✗".red())
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
                        | Ok(_) =>
                        {
                            println!("{}", "✓".green());
                            // Create checksum immediately after download
                            let checksum = self.calculate_checksum(&dest_path)?;
                            let checksum_path = dest_path.with_extension("sha");
                            fs::write(&checksum_path, checksum)?;
                        }
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
    /// 1. Downloads/copies templates from source if global templates don't exist
    /// 2. Verifies global template existence and integrity
    /// 3. Creates missing checksums for global templates
    /// 4. Detects local modifications
    /// 5. Creates backup of existing local files
    /// 6. Copies templates to current directory
    ///
    /// # Arguments
    ///
    /// * `lang` - Programming language or framework identifier
    /// * `agent` - AI coding agent identifier
    /// * `force` - If true, overwrite local modifications without warning
    /// * `from` - Optional path or URL to copy/download templates from
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Template files don't exist and can't be downloaded
    /// - Local modifications detected and force is false
    /// - Backup or copy operations fail
    pub fn update(&self, lang: &str, agent: &str, force: bool, from: Option<&str>) -> Result<()>
    {
        println!("{} Updating templates for {} with {}", "→".blue(), lang.green(), agent.green());

        let templates_yml_path = self.config_dir.join("templates.yml");

        // Check if global templates exist, if not download/copy them
        if self.config_dir.exists() == false || templates_yml_path.exists() == false
        {
            let source = from.unwrap_or("https://github.com/heikopanjas/vibe-check/tree/feature/template-management/templates");
            println!("{} Global templates not found, downloading from {}", "→".blue(), source.yellow());
            self.download_or_copy_templates(source)?;
        }

        // Load template configuration
        let config = self.load_template_config(None, None)?;

        // Get current working directory and user home directory
        let workspace = std::env::current_dir()?;
        let userprofile = dirs::home_dir().ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Could not determine home directory"))?;

        // Collect files to copy
        let mut files_to_copy: Vec<(PathBuf, PathBuf)> = Vec::new();

        // Add general templates
        for entry in &config.general
        {
            let source_path = self.config_dir.join(&entry.source);
            if source_path.exists() == false
            {
                continue;
            }

            let target_path = self.resolve_placeholder(&entry.target, &workspace, &userprofile);
            files_to_copy.push((source_path, target_path));
        }

        // Add language-specific templates
        if let Some(lang_config) = config.languages.get(lang)
        {
            for file_entry in &lang_config.files
            {
                let source_path = self.config_dir.join(&file_entry.source);
                if source_path.exists() == false
                {
                    continue;
                }

                let target_path = self.resolve_placeholder(&file_entry.target, &workspace, &userprofile);
                files_to_copy.push((source_path, target_path));
            }
        }

        // Add agent-specific templates
        if let Some(agent_config) = config.agents.get(agent)
        {
            // Add instruction file if present
            if let Some(instruction) = &agent_config.instruction
            {
                let source_path = self.config_dir.join(&instruction.source);
                if source_path.exists()
                {
                    let target_path = self.resolve_placeholder(&instruction.target, &workspace, &userprofile);
                    files_to_copy.push((source_path, target_path));
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

        for (source, target) in &files_to_copy
        {
            if target.exists() && self.has_local_modifications(target, source)?
            {
                has_modifications = true;
                modified_files.push(target.clone());
            }
        }

        if has_modifications && force == false
        {
            println!("{} Local modifications detected:", "!".yellow());
            for file in &modified_files
            {
                println!("  - {}", file.display().to_string().yellow());
            }
            println!("{} Use --force to overwrite", "→".blue());
            return Err("Local modifications detected. Aborting.".into());
        }

        // Create backup of existing local files
        self.create_backup(&workspace)?;

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

        // Find and remove agent directories (.claude, .copilot, .cursor, .codex)
        let agent_dirs = vec![".claude", ".copilot", ".cursor", ".codex"];
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
                        name != "AGENTS.md" &&
                        name != "README.md" &&
                        name != "LICENSE.md" &&
                        name != "CHANGELOG.md" &&
                        name != "CONTRIBUTING.md"
                    {
                        // Check if it matches known template patterns
                        // Currently supported languages: c++, swift, rust
                        let lowercase = name.to_lowercase();
                        if lowercase == "c++.md" || lowercase == "swift.md" || lowercase == "rust.md"
                        {
                            println!("{} Removing {}", "→".blue(), path.display().to_string().yellow());
                            fs::remove_file(&path)?;
                            cleared_count += 1;
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
