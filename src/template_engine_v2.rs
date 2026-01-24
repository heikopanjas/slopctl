//! Template engine v2 - Template generation logic for version 2 templates
//!
//! This module contains the template generation and merging logic for
//! templates.yml version 2 format (agents.md standard).
//!
//! V2 Philosophy:
//! - One AGENTS.md file that works across all agents
//! - No agent-specific instruction files (CLAUDE.md, copilot-instructions.md, etc.)
//! - Follows https://agents.md community standard
//! - Compatible with Claude, Cursor, Copilot, Aider, Jules, Factory, and more

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf}
};

use owo_colors::OwoColorize;

use crate::{
    Result,
    bom::TemplateConfig,
    file_tracker::{FileStatus, FileTracker},
    utils::{FileActionResponse, copy_file_with_mkdir, prompt_file_modification}
};

/// Template engine for version 2 templates (agents.md standard)
///
/// Handles template generation, fragment merging, and placeholder resolution
/// for the version 2 template format. V2 templates have no agent-specific files.
pub struct TemplateEngineV2<'a>
{
    config_dir: &'a Path
}

impl<'a> TemplateEngineV2<'a>
{
    /// Creates a new TemplateEngineV2 instance
    ///
    /// # Arguments
    ///
    /// * `config_dir` - Path to the global template storage directory
    pub fn new(config_dir: &'a Path) -> Self
    {
        Self { config_dir }
    }

    /// Loads template configuration from templates.yml
    ///
    /// Loads and parses templates.yml from the global config directory.
    ///
    /// # Errors
    ///
    /// Returns an error if templates.yml cannot be loaded or parsed
    pub fn load_template_config(&self) -> Result<TemplateConfig>
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
    pub fn is_file_customized(&self, local_path: &Path) -> Result<bool>
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

    /// Updates local templates from global storage (V2 - agent parameter optional)
    ///
    /// This method:
    /// 1. Verifies global templates exist
    /// 2. Detects local modifications to AGENTS.md
    /// 3. Copies templates to current directory
    ///
    /// V2 Philosophy: Single AGENTS.md works for all agents, but agent-specific
    /// prompts/commands can still be copied if agent is specified.
    ///
    /// # Arguments
    ///
    /// * `lang` - Programming language or framework identifier
    /// * `agent` - Optional agent identifier for copying agent-specific prompts
    /// * `mission` - Optional custom mission statement to override template default
    /// * `force` - If true, overwrite local modifications without warning
    /// * `dry_run` - If true, only show what would happen without making changes
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Global templates don't exist
    /// - Local modifications detected and force is false
    /// - Copy operations fail
    pub fn update(&self, lang: &str, agent: Option<&str>, mission: Option<&str>, force: bool, dry_run: bool) -> Result<()>
    {
        let templates_yml_path = self.config_dir.join("templates.yml");

        // Check if global templates exist
        if self.config_dir.exists() == false || templates_yml_path.exists() == false
        {
            return Err("Global templates not found. Please run 'vibe-check update' first to download templates.".into());
        }

        // Load template configuration
        let config = self.load_template_config()?;

        // Get current working directory and user home directory
        let workspace = std::env::current_dir()?;
        let userprofile = dirs::home_dir().ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "Could not determine home directory"))?;

        // Initialize file tracker
        let mut file_tracker = FileTracker::new(self.config_dir)?;

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

        // Add mission templates (fragments) if present, unless custom mission is provided
        if mission.is_none() == true &&
            let Some(mission_entries) = &config.mission
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
        else
        {
            return Err(format!("Language '{}' not found in templates.yml", lang).into());
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

        // V2: Process agent-specific prompts if agent is specified
        // Note: V2 has no agent-specific instruction files (single AGENTS.md for all)
        // but agents can still have operational prompts/commands
        if let Some(agent_name) = agent &&
            let Some(agents) = config.agents.as_ref()
        {
            if let Some(agent_config) = agents.get(agent_name)
            {
                // Add agent prompts
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
                println!("{} Agent '{}' not found in templates.yml", "!".yellow(), agent_name.yellow());
            }
        }

        if files_to_copy.is_empty() && main_template.is_none()
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
            if dry_run == false
            {
                println!("{} Other files will still be updated", "→".blue());
            }
            println!("{} Use --force to overwrite AGENTS.md", "→".blue());
        }

        // Dry run mode: just show what would happen
        if dry_run == true
        {
            println!("\n{} Files that would be created/modified:", "→".blue());

            // Show main AGENTS.md status
            if let Some((_, main_target)) = &main_template
            {
                if skip_agents_md && force == false
                {
                    println!("  {} {} (skipped - customized)", "○".yellow(), main_target.display());
                }
                else if main_target.exists()
                {
                    println!("  {} {} (would be overwritten)", "●".yellow(), main_target.display());
                }
                else
                {
                    println!("  {} {} (would be created)", "●".green(), main_target.display());
                }
            }

            // Show other files
            for (_, target) in &files_to_copy
            {
                if target.exists()
                {
                    println!("  {} {} (would be overwritten)", "●".yellow(), target.display());
                }
                else
                {
                    println!("  {} {} (would be created)", "●".green(), target.display());
                }
            }

            println!("\n{} Dry run complete. No files were modified.", "✓".green());
            return Ok(());
        }

        // Handle main AGENTS.md with fragment merging if fragments exist
        if let Some((main_source, main_target)) = main_template
        {
            // Skip AGENTS.md if customized and force is false
            if skip_agents_md && force == false
            {
                println!("{} Skipping AGENTS.md (customized)", "→".blue());
            }
            else if fragments.is_empty() == false || mission.is_some() == true
            {
                println!("{} Merging fragments into AGENTS.md", "→".blue());
                self.merge_fragments(&main_source, &main_target, &fragments, mission)?;
                println!("  {} {}", "✓".green(), main_target.display().to_string().yellow());

                // Record installation in file tracker
                let sha = FileTracker::calculate_sha256(&main_target)?;
                file_tracker.record_installation(&main_target, sha, config.version, Some(lang.to_string()), "main".to_string());
            }
            else
            {
                // No fragments, just copy main file as-is
                if let Some(parent) = main_target.parent()
                {
                    fs::create_dir_all(parent)?;
                }
                fs::copy(&main_source, &main_target)?;
                println!("  {} {}", "✓".green(), main_target.display().to_string().yellow());

                // Record installation in file tracker
                let sha = FileTracker::calculate_sha256(&main_target)?;
                file_tracker.record_installation(&main_target, sha, config.version, Some(lang.to_string()), "main".to_string());
            }
        }

        // Copy templates with file modification checking
        println!("{} Copying templates to target directories", "→".blue());

        let mut skipped_files = Vec::new();

        for (source, target) in &files_to_copy
        {
            // Calculate new template SHA
            let new_template_sha = FileTracker::calculate_sha256(source)?;

            // Check if file needs to be processed
            let should_copy = if target.exists() == false
            {
                // File doesn't exist, safe to copy
                true
            }
            else if force == true
            {
                // Force flag set, always overwrite
                true
            }
            else
            {
                // Check modification status
                match file_tracker.check_modification(target)?
                {
                    | FileStatus::NotTracked =>
                    {
                        // Not tracked, could be user file - prompt for safety
                        let response = prompt_file_modification(target, "<not tracked>", "<current file>", source)?;
                        match response
                        {
                            | FileActionResponse::Overwrite => true,
                            | FileActionResponse::Skip =>
                            {
                                skipped_files.push(target.clone());
                                false
                            }
                            | FileActionResponse::Quit =>
                            {
                                println!("\n{} Operation cancelled by user", "!".yellow());
                                return Ok(());
                            }
                        }
                    }
                    | FileStatus::Unmodified =>
                    {
                        // User didn't modify, safe to update
                        true
                    }
                    | FileStatus::Modified =>
                    {
                        // User modified, prompt
                        if let Some(metadata) = file_tracker.get_metadata(target)
                        {
                            let current_sha = FileTracker::calculate_sha256(target)?;
                            let response = prompt_file_modification(target, &metadata.original_sha, &current_sha, source)?;
                            match response
                            {
                                | FileActionResponse::Overwrite => true,
                                | FileActionResponse::Skip =>
                                {
                                    skipped_files.push(target.clone());
                                    false
                                }
                                | FileActionResponse::Quit =>
                                {
                                    println!("\n{} Operation cancelled by user", "!".yellow());
                                    return Ok(());
                                }
                            }
                        }
                        else
                        {
                            // Shouldn't happen, but treat as unmodified
                            true
                        }
                    }
                    | FileStatus::Deleted =>
                    {
                        // Was tracked but deleted, safe to recreate
                        true
                    }
                }
            };

            if should_copy == true
            {
                copy_file_with_mkdir(source, target)?;
                println!("  {} {}", "✓".green(), target.display().to_string().yellow());

                // Record installation in file tracker
                // Determine category based on target path
                let category = if target.to_string_lossy().contains(".git")
                {
                    "integration"
                }
                else if let Some(agent_name) = agent
                {
                    if target.to_string_lossy().contains(&format!(".{}", agent_name)) || target.to_string_lossy().contains(agent_name)
                    {
                        "agent"
                    }
                    else
                    {
                        "language"
                    }
                }
                else
                {
                    "language"
                };

                file_tracker.record_installation(target, new_template_sha, config.version, Some(lang.to_string()), category.to_string());
            }
        }

        // Show summary of skipped files
        if skipped_files.is_empty() == false
        {
            println!("\n{} Skipped {} modified file(s):", "!".yellow(), skipped_files.len());
            for file in &skipped_files
            {
                println!("  {} {}", "○".yellow(), file.display());
            }
            println!("{} Use --force to overwrite modified files", "→".blue());
        }

        // Save file tracker metadata
        file_tracker.save()?;

        println!("{} Templates updated successfully", "✓".green());
        if agent.is_some()
        {
            println!("{} V2 templates: Single AGENTS.md + agent-specific prompts", "→".blue());
        }
        else
        {
            println!("{} V2 templates: Single AGENTS.md works with all agents", "→".blue());
        }

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
    /// * `custom_mission` - Optional custom mission statement to override template default
    ///
    /// # Errors
    ///
    /// Returns an error if file reading or writing fails
    fn merge_fragments(&self, main_source: &Path, main_target: &Path, fragments: &[(PathBuf, String)], custom_mission: Option<&str>) -> Result<()>
    {
        // Read main AGENTS.md template
        let mut main_content = fs::read_to_string(main_source)?;

        // Remove the template marker to indicate this is a merged/customized file
        let marker = "<!-- VIBE-CHECK-TEMPLATE: This marker indicates an unmerged template. Do not remove manually. -->\n";
        main_content = main_content.replace(marker, "");

        // Group fragments by category to handle multiple fragments per insertion point
        let mut fragments_by_category: HashMap<String, Vec<String>> = HashMap::new();

        for (fragment_path, category) in fragments
        {
            let fragment_content = fs::read_to_string(fragment_path)?;
            fragments_by_category.entry(category.clone()).or_default().push(fragment_content);
        }

        // If custom mission is provided, add it to the fragments
        if let Some(mission_content) = custom_mission
        {
            // Format mission content with header
            let formatted_mission = format!("## Mission Statement\n\n{}", mission_content.trim());
            fragments_by_category.entry("mission".to_string()).or_default().push(formatted_mission);
            println!("{} Using custom mission statement", "→".blue());
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
}
