//! Template management functionality for vibe-check

#![allow(clippy::bool_comparison)]

use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    time::SystemTime
};

use chrono::{DateTime, Utc};
use owo_colors::OwoColorize;
use sha2::{Digest, Sha256};

use crate::{Result, utils::copy_dir_all};

/// Manages template files for coding agent instructions
///
/// The `TemplateManager` handles all operations related to template storage,
/// verification, backup, and synchronization. Templates are stored in
/// `$HOME/.config/vibe-check/templates` and backed up to
/// `$HOME/.cache/vibe-check/backups` before modifications.
pub struct TemplateManager
{
    config_dir: PathBuf,
    cache_dir:  PathBuf
}

impl TemplateManager
{
    /// Creates a new TemplateManager instance
    ///
    /// Initializes paths to configuration and cache directories based on
    /// the user's HOME environment variable.
    ///
    /// # Errors
    ///
    /// Returns an error if the HOME environment variable is not set
    pub fn new() -> Result<Self>
    {
        let home = std::env::var("HOME")?;
        let config_dir = PathBuf::from(&home).join(".config/vibe-check/templates");
        let cache_dir = PathBuf::from(&home).join(".cache/vibe-check/backups");

        Ok(Self { config_dir, cache_dir })
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
    /// Backups are stored in `$HOME/.cache/vibe-check/backups/YYYY-MM-DD_HH_MM_SS/`
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
    /// Downloads known template files from a GitHub repository.
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

        // Download known template files
        let files_to_download = vec!["AGENTS.md", "C++.md", "CMake.md", "General.md", "Git.md", "Rust.md", "Swift.md", "Python.md", "TypeScript.md", "JavaScript.md"];

        for file in &files_to_download
        {
            let file_url = format!("{}{}/{}", base_url, url_path, file);
            let dest_path = self.config_dir.join(file);

            print!("{} Downloading {}... ", "→".blue(), file.yellow());
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

        // Download agent-specific templates
        let agents = vec!["claude", "copilot", "cursor", "codex"];

        for agent in &agents
        {
            let file_url = format!("{}{}/{}/instructions.md", base_url, url_path, agent);
            let dest_dir = self.config_dir.join(agent);
            let dest_path = dest_dir.join("instructions.md");

            print!("{} Downloading {}/instructions.md... ", "→".blue(), agent.yellow());
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

        // Build paths - try both lowercase and capitalized versions
        let lang_lower = lang.to_lowercase();
        let lang_capitalized = lang.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_default() + &lang[1..];

        let lang_template_lower = self.config_dir.join(format!("{}.md", lang_lower));
        let lang_template_cap = self.config_dir.join(format!("{}.md", lang_capitalized));
        let agent_template = self.config_dir.join(agent).join("instructions.md");

        // Check if global templates exist, if not download/copy them
        if self.config_dir.exists() == false || agent_template.exists() == false
        {
            let source = from.unwrap_or("https://github.com/heikopanjas/vibe-check/tree/feature/template-management/templates");
            println!("{} Global templates not found, downloading from {}", "→".blue(), source.yellow());
            self.download_or_copy_templates(source)?;
        }

        // Determine which language template exists (if any)
        let lang_template = if lang_template_lower.exists()
        {
            Some(lang_template_lower.clone())
        }
        else if lang_template_cap.exists()
        {
            Some(lang_template_cap.clone())
        }
        else
        {
            None
        };

        // Verify agent template existence (required)
        if agent_template.exists() == false
        {
            return Err(format!("Agent template not found: {}", agent).into());
        }

        // Checksums are already created during download/copy, no need to verify or create here

        // Check for local modifications
        let current_dir = std::env::current_dir()?;
        let local_lang = current_dir.join(format!("{}.md", lang_lower));
        let local_agent_dir = current_dir.join(format!(".{}", agent));
        let local_agent = local_agent_dir.join("instructions.md");

        let has_lang_mods = if let Some(ref lt) = lang_template
        {
            self.has_local_modifications(&local_lang, lt)?
        }
        else
        {
            false
        };
        let has_agent_mods = self.has_local_modifications(&local_agent, &agent_template)?;

        if (has_lang_mods || has_agent_mods) && !force
        {
            println!("{} Local modifications detected:", "!".yellow());
            if has_lang_mods
            {
                println!("  - {}", local_lang.display().to_string().yellow());
            }
            if has_agent_mods
            {
                println!("  - {}", local_agent.display().to_string().yellow());
            }
            println!("{} Use --force to overwrite", "→".blue());
            return Err("Local modifications detected. Aborting.".into());
        }

        // Create backup of existing local files
        self.create_backup(&current_dir)?;

        // Copy templates
        println!("{} Copying templates to current directory", "→".blue());

        // Copy language template if it exists
        if let Some(ref lt) = lang_template
        {
            fs::copy(lt, &local_lang)?;
            println!("  - Copied language template: {}", local_lang.display().to_string().yellow());
        }
        else
        {
            println!("  - {} No language-specific template found (skipped)", "!".yellow());
        }

        // Copy agent template (required)
        fs::create_dir_all(&local_agent_dir)?;
        fs::copy(&agent_template, &local_agent)?;
        println!("  - Copied agent template: {}", local_agent.display().to_string().yellow());

        println!("{} Templates updated successfully", "✓".green());

        Ok(())
    }

    /// Clears local templates from current directory
    ///
    /// Removes agent instruction directories and language template files from
    /// the current directory. Global templates in $HOME/.config/vibe-check/templates
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
