//! Template management functionality for vibe-check

use std::{
    fs, io,
    path::{Path, PathBuf}
};

use owo_colors::OwoColorize;

use crate::{
    Result,
    bom::{BillOfMaterials, TemplateConfig},
    download_manager::DownloadManager,
    file_tracker::FileTracker,
    utils::{confirm_action, copy_dir_all, remove_file_and_cleanup_parents}
};

/// Manages template files for coding agent instructions
///
/// The `TemplateManager` handles all operations related to template storage,
/// verification, and synchronization. Templates are stored in the
/// local data directory (e.g., `$HOME/.local/share/vibe-check/templates` on Linux,
/// `$HOME/Library/Application Support/vibe-check/templates` on macOS).
pub struct TemplateManager
{
    config_dir: PathBuf
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

        let config_dir = data_dir.join("vibe-check/templates");

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

    /// Gets the template version from templates.yml
    ///
    /// Reads templates.yml and extracts the version field.
    /// If the version field is missing, returns 1 as the default.
    ///
    /// # Errors
    ///
    /// Returns an error if templates.yml cannot be read or parsed
    fn get_template_version(&self) -> Result<u32>
    {
        let config_path = self.config_dir.join("templates.yml");

        if config_path.exists() == false
        {
            return Err("templates.yml not found in global template directory".into());
        }

        let content = fs::read_to_string(&config_path)?;
        let config: TemplateConfig = serde_yaml::from_str(&content)?;

        Ok(config.version)
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
                return Err(format!("Source path does not exist: {}", source).into());
            }

            println!("{} Copying templates from local path...", "→".blue());
            fs::create_dir_all(&self.config_dir)?;
            copy_dir_all(source_path, &self.config_dir)?;
        }

        Ok(())
    }

    /// Updates local templates from global storage
    ///
    /// This method detects the template version and dispatches to the
    /// appropriate template engine for processing.
    ///
    /// # Arguments
    ///
    /// * `lang` - Programming language or framework identifier
    /// * `agent` - AI coding agent identifier
    /// * `mission` - Optional custom mission statement to override template default
    /// * `force` - If true, overwrite local modifications without warning
    /// * `dry_run` - If true, only show what would happen without making changes
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Global templates don't exist
    /// - Template version is unsupported
    /// - Template generation fails
    pub fn update(&self, lang: &str, agent: Option<&str>, mission: Option<&str>, force: bool, dry_run: bool) -> Result<()>
    {
        // Check if global templates exist
        if self.has_global_templates() == false
        {
            return Err("Global templates not found. Please run 'vibe-check update' first to download templates.".into());
        }

        // Get template version and dispatch to appropriate engine
        let version = self.get_template_version()?;

        match version
        {
            | 1 =>
            {
                // Deprecation warning for v1 templates
                println!("{} V1 templates are deprecated and will be removed in v7.0.0", "!".yellow());
                println!("{} Consider migrating to V2 templates (agents.md standard)", "!".yellow());
                println!("{} Run: vibe-check config source.url https://github.com/heikopanjas/vibe-check/tree/develop/templates/v2", "→".blue());
                println!();

                // V1 requires agent parameter
                let agent_str = agent.ok_or("--agent is required for v1 templates. Use: vibe-check init --lang <lang> --agent <agent>")?;
                let engine = crate::template_engine_v1::TemplateEngineV1::new(&self.config_dir);
                engine.update(lang, agent_str, mission, force, dry_run)
            }
            | 2 =>
            {
                // V2: Single AGENTS.md for all agents, but agent-specific prompts can be copied
                if agent.is_some()
                {
                    println!("{} V2 templates: Using single AGENTS.md + copying agent-specific prompts", "→".blue());
                }
                else
                {
                    println!("{} V2 templates: Using single AGENTS.md (no agent-specific prompts)", "→".blue());
                }
                let engine = crate::template_engine_v2::TemplateEngineV2::new(&self.config_dir);
                engine.update(lang, agent, mission, force, dry_run)
            }
            | _ => Err(format!("Unsupported template version: {}. Please update vibe-check to the latest version.", version).into())
        }
    }

    /// Purges all vibe-check files from the current directory
    ///
    /// Removes all agent-specific files and AGENTS.md from the current directory.
    /// Global templates in the local data directory are never affected.
    ///
    /// # Arguments
    ///
    /// * `force` - If true, purge without confirmation prompt and delete customized AGENTS.md
    /// * `dry_run` - If true, only show what would happen without making changes
    ///
    /// # Errors
    ///
    /// Returns an error if file deletion fails or templates.yml cannot be loaded
    pub fn purge(&self, force: bool, dry_run: bool) -> Result<()>
    {
        let current_dir = std::env::current_dir()?;

        // Collect all files to be purged
        let mut files_to_purge: Vec<PathBuf> = Vec::new();
        let mut agents_md_skipped = false;

        // Load templates.yml and build Bill of Materials to get agent files
        let config_file = self.config_dir.join("templates.yml");
        if config_file.exists() == true &&
            let Ok(bom) = BillOfMaterials::from_config(&config_file)
        {
            let agent_names = bom.get_agent_names();

            for agent in &agent_names
            {
                if let Some(files) = bom.get_agent_files(agent)
                {
                    for file in files
                    {
                        if file.exists() == true
                        {
                            files_to_purge.push(file.clone());
                        }
                    }
                }
            }
        }

        // Remove duplicates
        files_to_purge.sort();
        files_to_purge.dedup();

        // Check AGENTS.md
        let agents_md_path = current_dir.join("AGENTS.md");
        if agents_md_path.exists() == true
        {
            let agents_md_customized = self.is_file_customized(&agents_md_path)?;

            if agents_md_customized == true && force == false
            {
                agents_md_skipped = true;
            }
            else
            {
                files_to_purge.push(agents_md_path.clone());
            }
        }

        if files_to_purge.is_empty() && agents_md_skipped == false
        {
            println!("{} No vibe-check files found to purge", "→".blue());
            return Ok(());
        }

        // Dry run mode: just show what would happen
        if dry_run == true
        {
            println!("\n{} Files that would be deleted:", "→".blue());

            for file in &files_to_purge
            {
                println!("  {} {}", "●".red(), file.display());
            }

            if agents_md_skipped == true
            {
                println!("  {} {} (skipped - customized, use --force)", "○".yellow(), agents_md_path.display());
            }

            println!("\n{} Dry run complete. No files were modified.", "✓".green());
            return Ok(());
        }

        // Ask for confirmation unless force is true
        if force == false && confirm_action(&format!("{} Are you sure you want to purge all vibe-check files? (y/N): ", "?".yellow()))? == false
        {
            println!("{} Operation cancelled", "→".blue());
            return Ok(());
        }

        // Initialize file tracker for cleanup
        let mut file_tracker = FileTracker::new(&self.config_dir)?;

        // Remove files
        let mut purged_count = 0;
        for file in &files_to_purge
        {
            println!("{} Removing {}", "→".blue(), file.display().to_string().yellow());
            if let Err(e) = remove_file_and_cleanup_parents(file)
            {
                eprintln!("{} Failed to remove {}: {}", "✗".red(), file.display(), e);
            }
            else
            {
                purged_count += 1;
                // Remove from file tracker
                file_tracker.remove_entry(file);
            }
        }

        // Save file tracker metadata
        file_tracker.save()?;

        if agents_md_skipped == true
        {
            println!("{} AGENTS.md has been customized and was not deleted", "→".yellow());
            println!("{} Use --force to delete it anyway", "→".yellow());
        }

        if purged_count == 0
        {
            println!("{} No vibe-check files found to purge", "→".blue());
        }
        else
        {
            println!("{} Purged {} file(s) successfully", "✓".green(), purged_count);
        }

        Ok(())
    }

    /// Remove agent-specific files from the current directory
    ///
    /// Deletes files associated with the specified agent (or all agents if None)
    /// based on the Bill of Materials built from templates.yml in global storage.
    /// AGENTS.md is never touched by this operation.
    ///
    /// # Arguments
    ///
    /// * `agent` - Optional agent name. If Some, removes files for that agent only. If None, removes files for all agents.
    /// * `force` - If true, skip confirmation prompt
    /// * `dry_run` - If true, only show what would be removed without actually removing
    ///
    /// # Returns
    ///
    /// Ok(()) if files were successfully removed or if no files were found
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - templates.yml cannot be loaded
    /// - Agent name is not found in the BoM (when agent is Some)
    /// - File deletion fails
    pub fn remove(&self, agent: Option<&str>, force: bool, dry_run: bool) -> Result<()>
    {
        // Load templates.yml and build Bill of Materials
        let config_file = self.config_dir.join("templates.yml");
        if config_file.exists() == false
        {
            return Err("Global templates not found. Run 'vibe-check init' first to set up templates.".to_string().into());
        }

        println!("{} Building Bill of Materials from templates.yml", "→".blue());
        let bom = BillOfMaterials::from_config(&config_file)?;

        // Collect files based on agent parameter
        let (files_to_remove, description): (Vec<PathBuf>, String) = if let Some(agent_name) = agent
        {
            // Single agent mode
            if bom.has_agent(agent_name) == false
            {
                let available_agents = bom.get_agent_names();
                return Err(format!("Agent '{}' not found in Bill of Materials.\nAvailable agents: {}", agent_name, available_agents.join(", ")).into());
            }

            let agent_files = bom.get_agent_files(agent_name).unwrap();
            let existing: Vec<PathBuf> = agent_files.iter().filter(|f| f.exists()).cloned().collect();
            (existing, format!("agent '{}'", agent_name.yellow()))
        }
        else
        {
            // All agents mode
            let agent_names = bom.get_agent_names();
            if agent_names.is_empty() == true
            {
                println!("{} No agents found in Bill of Materials", "→".blue());
                return Ok(());
            }

            let mut all_files: Vec<PathBuf> = Vec::new();
            for name in &agent_names
            {
                if let Some(agent_files) = bom.get_agent_files(name)
                {
                    for file in agent_files
                    {
                        if file.exists() == true
                        {
                            all_files.push(file.clone());
                        }
                    }
                }
            }
            all_files.sort();
            all_files.dedup();
            (all_files, "all agents".to_string())
        };

        if files_to_remove.is_empty() == true
        {
            println!("{} No files found for {} in current directory", "→".blue(), description);
            return Ok(());
        }

        // Dry run mode: just show what would happen
        if dry_run == true
        {
            println!("\n{} Files that would be deleted for {}:", "→".blue(), description);

            for file in &files_to_remove
            {
                println!("  {} {}", "●".red(), file.display());
            }

            println!("\n{} Dry run complete. No files were modified.", "✓".green());
            return Ok(());
        }

        // Show files to be removed
        println!("\n{} Files to be removed for {}:", "→".blue(), description);
        for file in &files_to_remove
        {
            println!("  • {}", file.display().to_string().yellow());
        }
        println!();

        // Ask for confirmation unless force is true
        if force == false && confirm_action(&format!("{} Proceed with removal? [y/N]: ", "?".yellow()))? == false
        {
            println!("{} Operation cancelled", "✗".red());
            return Ok(());
        }

        // Initialize file tracker for cleanup
        let mut file_tracker = FileTracker::new(&self.config_dir)?;

        // Remove files
        let mut removed_count = 0;
        for file in &files_to_remove
        {
            match remove_file_and_cleanup_parents(file)
            {
                | Ok(_) =>
                {
                    println!("{} Removed {}", "✓".green(), file.display());
                    removed_count += 1;
                    // Remove from file tracker
                    file_tracker.remove_entry(file);
                }
                | Err(e) =>
                {
                    eprintln!("{} Failed to remove {}: {}", "✗".red(), file.display(), e);
                }
            }
        }

        // Save file tracker metadata
        file_tracker.save()?;

        println!("\n{} Removed {} file(s) for {}", "✓".green(), removed_count, description);

        Ok(())
    }

    /// Show current project status
    ///
    /// Displays information about:
    /// - Global template status (downloaded, location)
    /// - AGENTS.md status (exists, customized)
    /// - Installed agents (detected by checking for their files)
    /// - All vibe-check managed files in current directory
    ///
    /// # Errors
    ///
    /// Returns an error if the current directory cannot be determined
    pub fn status(&self) -> Result<()>
    {
        let current_dir = std::env::current_dir()?;

        println!("{}", "vibe-check status".bold());
        println!();

        // Global templates status
        println!("{}", "Global Templates:".bold());
        if self.has_global_templates() == true
        {
            println!("  {} Installed at: {}", "✓".green(), self.config_dir.display().to_string().yellow());

            // Show template version
            if let Ok(version) = self.get_template_version()
            {
                println!("  {} Template version: {}", "→".blue(), version.to_string().green());
            }

            // Show available agents and languages from templates.yml
            let config_path = self.config_dir.join("templates.yml");
            if let Ok(content) = fs::read_to_string(&config_path) &&
                let Ok(config) = serde_yaml::from_str::<TemplateConfig>(&content)
            {
                // V2 templates don't have agents section (agents.md standard)
                if let Some(agents_map) = &config.agents
                {
                    let agents: Vec<&String> = agents_map.keys().collect();
                    if agents.is_empty() == false
                    {
                        println!("  {} Available agents: {}", "→".blue(), agents.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ").green());
                    }
                }

                let languages: Vec<&String> = config.languages.keys().collect();
                if languages.is_empty() == false
                {
                    println!("  {} Available languages: {}", "→".blue(), languages.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ").green());
                }
            }
        }
        else
        {
            println!("  {} Not installed", "✗".red());
            println!("  {} Run 'vibe-check update' to download templates", "→".blue());
        }

        println!();

        // AGENTS.md status
        println!("{}", "Project Status:".bold());
        let agents_md_path = current_dir.join("AGENTS.md");
        if agents_md_path.exists() == true
        {
            let customized = self.is_file_customized(&agents_md_path).unwrap_or(false);
            if customized == true
            {
                println!("  {} AGENTS.md: {} (customized)", "✓".green(), "exists".green());
            }
            else
            {
                println!("  {} AGENTS.md: {} (from template)", "✓".green(), "exists".yellow());
            }
        }
        else
        {
            println!("  {} AGENTS.md: {}", "○".yellow(), "not found".yellow());
        }

        // Detect installed agents by checking for their files
        let mut installed_agents: Vec<String> = Vec::new();
        let mut managed_files: Vec<PathBuf> = Vec::new();

        let config_file = self.config_dir.join("templates.yml");
        if config_file.exists() == true &&
            let Ok(bom) = BillOfMaterials::from_config(&config_file)
        {
            for agent_name in bom.get_agent_names()
            {
                if let Some(files) = bom.get_agent_files(&agent_name)
                {
                    let existing_files: Vec<PathBuf> = files.iter().filter(|f| f.exists()).cloned().collect();
                    if existing_files.is_empty() == false
                    {
                        installed_agents.push(agent_name.clone());
                        managed_files.extend(existing_files);
                    }
                }
            }
        }

        if installed_agents.is_empty() == false
        {
            println!("  {} Installed agents: {}", "✓".green(), installed_agents.join(", ").green());
        }
        else
        {
            println!("  {} No agents installed", "○".yellow());
        }

        // Add AGENTS.md to managed files if it exists
        if agents_md_path.exists() == true
        {
            managed_files.push(agents_md_path);
        }

        println!();

        // List all managed files
        if managed_files.is_empty() == false
        {
            managed_files.sort();
            managed_files.dedup();

            println!("{}", "Managed Files:".bold());
            for file in &managed_files
            {
                // Show relative path if possible
                let display_path = file.strip_prefix(&current_dir).unwrap_or(file);
                println!("  • {}", display_path.display().to_string().yellow());
            }
        }
        else
        {
            println!("{}", "Managed Files:".bold());
            println!("  {} No vibe-check files found in current directory", "○".yellow());
            println!("  {} Run 'vibe-check init --lang <lang> --agent <agent>' to set up", "→".blue());
        }

        Ok(())
    }

    /// List available agents and languages
    ///
    /// Displays all available agents and languages from the global templates,
    /// along with their installation status in the current project.
    ///
    /// # Errors
    ///
    /// Returns an error if templates.yml cannot be loaded
    pub fn list(&self) -> Result<()>
    {
        println!("{}", "vibe-check list".bold());
        println!();

        // Check if global templates exist
        if self.has_global_templates() == false
        {
            println!("{} Global templates not installed", "✗".red());
            println!("{} Run 'vibe-check update' to download templates", "→".blue());
            return Ok(());
        }

        // Load template configuration
        let config_path = self.config_dir.join("templates.yml");
        let content = fs::read_to_string(&config_path)?;
        let config: TemplateConfig = serde_yaml::from_str(&content)?;

        // Build BoM for checking installed status
        let bom = BillOfMaterials::from_config(&config_path)?;

        // List agents (V2 templates don't have agents section - agents.md standard)
        if let Some(agents_map) = &config.agents
        {
            println!("{}", "Available Agents:".bold());
            let mut agents: Vec<&String> = agents_map.keys().collect();
            agents.sort();

            for agent_name in agents
            {
                // Check if agent is installed (has files in current directory)
                let is_installed = if let Some(files) = bom.get_agent_files(agent_name)
                {
                    files.iter().any(|f| f.exists())
                }
                else
                {
                    false
                };

                if is_installed == true
                {
                    println!("  {} {} (installed)", "✓".green(), agent_name.green());
                }
                else
                {
                    println!("  {} {}", "○".blue(), agent_name);
                }
            }

            println!();
        }
        else
        {
            println!("{}", "Available Agents:".bold());
            println!("  {} V2 templates (agents.md standard) - no agent-specific files", "→".blue());
            println!("  {} Single AGENTS.md works with all agents", "→".blue());
            println!();
        }

        // List languages (no installation status - language content is merged into AGENTS.md)
        println!("{}", "Available Languages:".bold());
        let mut languages: Vec<&String> = config.languages.keys().collect();
        languages.sort();

        for lang_name in languages
        {
            println!("  • {}", lang_name);
        }

        println!();
        println!("{} Use 'vibe-check init --lang <lang> --agent <agent>' to install", "→".blue());

        Ok(())
    }
}
