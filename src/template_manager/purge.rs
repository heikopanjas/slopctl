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
    /// Purges all regulator files from the current directory
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

        let mut files_to_purge: Vec<PathBuf> = Vec::new();
        let mut agents_md_skipped = false;

        // Collect agent files from BoM (template-defined)
        let config_file = self.config_dir.join("templates.yml");
        if config_file.exists() == true &&
            let Ok(bom) = BillOfMaterials::from_config(&config_file)
        {
            for agent in &bom.get_agent_names()
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

        // Merge all FileTracker entries for the workspace (catches ad-hoc and top-level skills)
        let file_tracker = FileTracker::new(&self.config_dir)?;
        for (path, _) in file_tracker.get_workspace_entries(&current_dir)
        {
            if path.exists() == true
            {
                files_to_purge.push(path);
            }
        }

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
            else if files_to_purge.iter().any(|f| f.ends_with("AGENTS.md")) == false
            {
                files_to_purge.push(agents_md_path.clone());
            }
        }

        if files_to_purge.is_empty() == true && agents_md_skipped == false
        {
            println!("{} No regulator files found to purge", "→".blue());
            return Ok(());
        }

        // Dry run mode
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
        if force == false && confirm_action(&format!("{} Are you sure you want to purge all regulator files? (y/N): ", "?".yellow()))? == false
        {
            println!("{} Operation cancelled", "→".blue());
            return Ok(());
        }

        // Re-load as mutable for cleanup
        let mut file_tracker = FileTracker::new(&self.config_dir)?;

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
                file_tracker.remove_entry(file);
            }
        }

        file_tracker.save()?;

        if agents_md_skipped == true
        {
            println!("{} AGENTS.md has been customized and was not deleted", "→".yellow());
            println!("{} Use --force to delete it anyway", "→".yellow());
        }

        if purged_count == 0
        {
            println!("{} No regulator files found to purge", "→".blue());
        }
        else
        {
            println!("{} Purged {} file(s) successfully", "✓".green(), purged_count);
        }

        Ok(())
    }
}
