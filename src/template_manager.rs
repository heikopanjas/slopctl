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
    utils::{confirm_action, copy_dir_all, copy_file_with_mkdir, remove_file_and_cleanup_parents}
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

    /// Loads template configuration from templates.yml
    ///
    /// Loads and parses templates.yml from the global config directory.
    ///
    /// # Errors
    ///
    /// Returns an error if templates.yml cannot be loaded or parsed
    fn load_template_config(&self) -> Result<TemplateConfig>
    {
        let config_path = self.config_dir.join("templates.yml");

        // Try to load and parse templates.yml
        if config_path.exists() == false
        {
            return Err("templates.yml not found in global template directory".into());
        }

        let content = fs::read_to_string(&config_path)?;
        let config: TemplateConfig = serde_yaml::from_str(&content)?;
        Ok(config)
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
        let config = self.load_template_config()?;

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

        // Helper closure to process file entries
        let mut process_entry = |source: &str, target: &str, category: &str| {
            let source_path = self.config_dir.join(source);
            if source_path.exists() == false
            {
                return;
            }

            if target.starts_with("$instructions")
            {
                fragments.push((source_path, category.to_string()));
            }
            else
            {
                let target_path = self.resolve_placeholder(target, &workspace, &userprofile);
                files_to_copy.push((source_path, target_path));
            }
        };

        // Add principles templates (fragments) if present
        if let Some(principles_entries) = &config.principles
        {
            for entry in principles_entries
            {
                process_entry(&entry.source, &entry.target, "principles");
            }
        }

        // Add mission templates (fragments) if present
        if let Some(mission_entries) = &config.mission
        {
            for entry in mission_entries
            {
                process_entry(&entry.source, &entry.target, "mission");
            }
        }

        // Add language-specific templates (fragments)
        if let Some(lang_config) = config.languages.get(lang)
        {
            for file_entry in &lang_config.files
            {
                process_entry(&file_entry.source, &file_entry.target, "languages");
            }
        }

        // Add integration templates (fragments)
        if let Some(integration_map) = &config.integration
        {
            for integration_config in integration_map.values()
            {
                for file_entry in &integration_config.files
                {
                    process_entry(&file_entry.source, &file_entry.target, "integration");
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

        // Check if main AGENTS.md has been customized (marker removed)
        let skip_agents_md = if let Some((_, main_target)) = &main_template
        {
            main_target.exists() && self.is_file_customized(main_target)?
        }
        else
        {
            false
        };

        if skip_agents_md && force == false
        {
            println!("{} Local AGENTS.md has been customized and will be skipped", "!".yellow());
            println!("{} Other files will still be updated", "→".blue());
            println!("{} Use --force to overwrite AGENTS.md", "→".blue());
        }

        // Handle main AGENTS.md with fragment merging if fragments exist
        if let Some((main_source, main_target)) = main_template
        {
            // Skip AGENTS.md if customized and force is false
            if skip_agents_md && force == false
            {
                println!("{} Skipping AGENTS.md (customized)", "→".blue());
            }
            else if fragments.is_empty() == false
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
            copy_file_with_mkdir(source, target)?;
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

    /// Purges all vibe-check files from the current directory
    ///
    /// Removes all agent-specific files and AGENTS.md from the current directory.
    /// Global templates in the local data directory are never affected.
    ///
    /// # Arguments
    ///
    /// * `force` - If true, purge without confirmation prompt and delete customized AGENTS.md
    ///
    /// # Errors
    ///
    /// Returns an error if file deletion fails or templates.yml cannot be loaded
    pub fn purge(&self, force: bool) -> Result<()>
    {
        let current_dir = std::env::current_dir()?;

        if force == false && confirm_action(&format!("{} Are you sure you want to purge all vibe-check files? (y/N): ", "?".yellow()))? == false
        {
            println!("{} Operation cancelled", "→".blue());
            return Ok(());
        }

        let mut purged_count = 0;

        // Load templates.yml and build Bill of Materials to get agent files
        let config_file = self.config_dir.join("templates.yml");
        if config_file.exists() == true &&
            let Ok(bom) = BillOfMaterials::from_config(&config_file)
        {
            let agent_names = bom.get_agent_names();

            // Collect all agent-specific files
            let mut agent_files: Vec<PathBuf> = Vec::new();
            for agent in &agent_names
            {
                if let Some(files) = bom.get_agent_files(agent)
                {
                    for file in files
                    {
                        if file.exists() == true
                        {
                            agent_files.push(file.clone());
                        }
                    }
                }
            }

            // Remove duplicates
            agent_files.sort();
            agent_files.dedup();

            // Remove agent files
            for file in agent_files
            {
                println!("{} Removing {}", "→".blue(), file.display().to_string().yellow());
                if let Err(e) = remove_file_and_cleanup_parents(&file)
                {
                    eprintln!("{} Failed to remove {}: {}", "✗".red(), file.display(), e);
                }
                else
                {
                    purged_count += 1;
                }
            }
        }

        // Remove AGENTS.md
        let agents_md_path = current_dir.join("AGENTS.md");
        if agents_md_path.exists() == true
        {
            let agents_md_customized = self.is_file_customized(&agents_md_path)?;

            if agents_md_customized == true && force == false
            {
                println!("{} AGENTS.md has been customized and will not be deleted", "→".yellow());
                println!("{} Use --force to delete it anyway", "→".yellow());
            }
            else
            {
                println!("{} Removing {}", "→".blue(), agents_md_path.display().to_string().yellow());
                fs::remove_file(&agents_md_path)?;
                purged_count += 1;
            }
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
    pub fn remove(&self, agent: Option<&str>, force: bool) -> Result<()>
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
                }
                | Err(e) =>
                {
                    eprintln!("{} Failed to remove {}: {}", "✗".red(), file.display(), e);
                }
            }
        }

        println!("\n{} Removed {} file(s) for {}", "✓".green(), removed_count, description);

        Ok(())
    }
}
