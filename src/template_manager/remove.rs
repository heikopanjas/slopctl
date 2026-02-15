//! Template remove command

use std::path::PathBuf;

use owo_colors::OwoColorize;

use super::TemplateManager;
use crate::{
    Result,
    bom::BillOfMaterials,
    file_tracker::FileTracker,
    utils::{confirm_action, remove_file_and_cleanup_parents}
};

impl TemplateManager
{
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
}
