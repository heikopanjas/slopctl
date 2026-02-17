//! Template purge command

use std::path::PathBuf;

use owo_colors::OwoColorize;

use super::TemplateManager;
use crate::{
    Result,
    bom::BillOfMaterials,
    file_tracker::FileTracker,
    template_engine,
    utils::{confirm_action, remove_file_and_cleanup_parents}
};

impl TemplateManager
{
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
            let agents_md_customized = template_engine::is_file_customized(&agents_md_path)?;

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
}
