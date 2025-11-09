//! Template management functionality for vibe-check

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

    /// Verifies checksum file exists, creates it if missing
    ///
    /// Checksums are stored alongside templates with .sha extension.
    /// For example, template.md -> template.sha
    ///
    /// # Arguments
    ///
    /// * `template_path` - Path to the template file
    ///
    /// # Errors
    ///
    /// Returns an error if checksum calculation or file write fails
    fn verify_or_create_checksum(&self, template_path: &Path) -> Result<()>
    {
        let checksum_path = template_path.with_extension("sha");

        if !checksum_path.exists()
        {
            println!("{} Creating missing checksum for {}", "→".blue(), template_path.display().to_string().yellow());

            let checksum = self.calculate_checksum(template_path)?;
            fs::write(&checksum_path, checksum)?;
        }

        Ok(())
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
        if !local_path.exists()
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
        if !source_dir.exists()
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

    /// Updates local templates from global storage
    ///
    /// This method:
    /// 1. Verifies global template existence and integrity
    /// 2. Creates missing checksums for global templates
    /// 3. Detects local modifications
    /// 4. Creates backup of existing local files
    /// 5. Copies templates to current directory
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
    /// - Template files don't exist
    /// - Local modifications detected and force is false
    /// - Backup or copy operations fail
    pub fn update(&self, lang: &str, agent: &str, force: bool) -> Result<()>
    {
        println!("{} Updating templates for {} with {}", "→".blue(), lang.green(), agent.green());

        // Build paths
        let lang_template = self.config_dir.join(format!("{}.md", lang));
        let agent_template = self.config_dir.join(agent).join("instructions.md");

        // Verify global template existence
        if !lang_template.exists()
        {
            return Err(format!("Language template not found: {}", lang).into());
        }
        if !agent_template.exists()
        {
            return Err(format!("Agent template not found: {}", agent).into());
        }

        // Verify or create checksums
        self.verify_or_create_checksum(&lang_template)?;
        self.verify_or_create_checksum(&agent_template)?;

        // Check for local modifications
        let current_dir = std::env::current_dir()?;
        let local_lang = current_dir.join(format!("{}.md", lang));
        let local_agent_dir = current_dir.join(format!(".{}", agent));
        let local_agent = local_agent_dir.join("instructions.md");

        let has_lang_mods = self.has_local_modifications(&local_lang, &lang_template)?;
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
        fs::copy(&lang_template, &local_lang)?;
        fs::create_dir_all(&local_agent_dir)?;
        fs::copy(&agent_template, &local_agent)?;

        println!("{} Templates updated successfully", "✓".green());

        Ok(())
    }

    /// Clears all templates from global storage
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
        if !force
        {
            print!("{} Are you sure you want to clear all templates? (y/N): ", "?".yellow());
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;

            if !input.trim().eq_ignore_ascii_case("y")
            {
                println!("{} Operation cancelled", "→".blue());
                return Ok(());
            }
        }

        // Create backup before clearing
        self.create_backup(&self.config_dir)?;

        if self.config_dir.exists()
        {
            println!("{} Clearing templates from {}", "→".blue(), self.config_dir.display().to_string().yellow());
            fs::remove_dir_all(&self.config_dir)?;
        }

        println!("{} Templates cleared successfully", "✓".green());

        Ok(())
    }
}
