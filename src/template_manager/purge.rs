//! Template purge command

use std::{fs, path::PathBuf};

use owo_colors::OwoColorize;

use super::TemplateManager;
use crate::{
    Result, agent_defaults,
    bom::BillOfMaterials,
    file_tracker::FileTracker,
    template_engine,
    utils::{collect_files_recursive, confirm_action, remove_file_and_cleanup_parents}
};

impl TemplateManager
{
    /// Purges all slopctl files from the current directory
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

        // Collect agent files from BoM (template-defined), canonicalized to
        // absolute paths so they dedup correctly against FileTracker entries.
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
                        if file.exists() == true &&
                            let Ok(canonical) = fs::canonicalize(file)
                        {
                            files_to_purge.push(canonical);
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

        // Scan workspace-scoped agent skill directories on disk to catch untracked/manually
        // placed skills. Userprofile-based dirs (e.g. codex ~/.codex/skills) are excluded —
        // those are user-global and may contain agent-internal files. FileTracker entries
        // above already cover userprofile skills that slopctl installed.
        let userprofile = dirs::home_dir().unwrap_or_default();
        let skill_search_dirs = agent_defaults::get_workspace_skill_search_dirs(&current_dir, &userprofile);
        for dir in &skill_search_dirs
        {
            if dir.exists() == true &&
                let Ok(entries) = fs::read_dir(dir)
            {
                for entry in entries.flatten()
                {
                    if entry.path().is_dir() == true
                    {
                        let mut skill_files = Vec::new();
                        let _ = collect_files_recursive(&entry.path(), &mut skill_files);
                        files_to_purge.extend(skill_files);
                    }
                }
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
            println!("{} No slopctl files found to purge", "→".blue());
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
        if force == false && confirm_action(&format!("{} Are you sure you want to purge all slopctl files? (y/N): ", "?".yellow()))? == false
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
            println!("{} No slopctl files found to purge", "→".blue());
        }
        else
        {
            println!("{} Purged {} file(s) successfully", "✓".green(), purged_count);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests
{
    use std::fs;

    use super::TemplateManager;
    use crate::{file_tracker::FileTracker, template_manager::CWD_LOCK};

    #[test]
    fn test_purge_dry_run_no_files() -> anyhow::Result<()>
    {
        let _lock = CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        let data_dir = tempfile::TempDir::new()?;
        let workspace = tempfile::TempDir::new()?;

        let original_dir = std::env::current_dir()?;
        std::env::set_current_dir(workspace.path())?;

        let manager = TemplateManager { config_dir: data_dir.path().to_path_buf() };
        let result = manager.purge(false, true);

        std::env::set_current_dir(original_dir)?;

        assert!(result.is_ok() == true);
        Ok(())
    }

    #[test]
    fn test_purge_deduplicates_bom_and_tracker_paths() -> anyhow::Result<()>
    {
        let _lock = CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        let data_dir = tempfile::TempDir::new()?;
        let workspace = tempfile::TempDir::new()?;

        // Write a minimal templates.yml that declares an agent with a workspace file
        let yaml = "version: 5\nagents:\n  cursor:\n    instructions:\n      - source: cursorrules.md\n        target: $workspace/.cursorrules\n";
        fs::write(data_dir.path().join("templates.yml"), yaml)?;

        // Create the agent file on disk so BoM can find it
        let agent_file = workspace.path().join(".cursorrules");
        fs::write(&agent_file, "test")?;

        // Record the same file in FileTracker (stores absolute path)
        let mut tracker = FileTracker::new(data_dir.path())?;
        tracker.record_installation(&agent_file, "sha1".into(), 5, None, "agent".into(), workspace.path());
        tracker.save()?;

        let original_dir = std::env::current_dir()?;
        std::env::set_current_dir(workspace.path())?;

        let manager = TemplateManager { config_dir: data_dir.path().to_path_buf() };
        let result = manager.purge(true, false);

        std::env::set_current_dir(original_dir)?;

        assert!(result.is_ok() == true);
        // The file should have been removed exactly once (no double-removal error)
        assert!(agent_file.exists() == false);
        Ok(())
    }

    #[test]
    fn test_purge_discovers_untracked_skill_files() -> anyhow::Result<()>
    {
        let _lock = CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        let data_dir = tempfile::TempDir::new()?;
        let workspace = tempfile::TempDir::new()?;

        // Place a skill directory on disk without any FileTracker entry
        let skill_dir = workspace.path().join(".agents/skills/my-skill");
        fs::create_dir_all(&skill_dir)?;
        let skill_file = skill_dir.join("SKILL.md");
        fs::write(&skill_file, "# My Skill")?;

        let original_dir = std::env::current_dir()?;
        std::env::set_current_dir(workspace.path())?;

        let manager = TemplateManager { config_dir: data_dir.path().to_path_buf() };
        let result = manager.purge(true, false);

        std::env::set_current_dir(original_dir)?;

        assert!(result.is_ok() == true);
        // The untracked skill file should have been discovered and removed
        assert!(skill_file.exists() == false);
        Ok(())
    }

    #[test]
    fn test_purge_skips_userprofile_skill_dir_scan() -> anyhow::Result<()>
    {
        let _lock = CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        let data_dir = tempfile::TempDir::new()?;
        let workspace = tempfile::TempDir::new()?;

        // Create CODEX.md so codex is detected as installed
        fs::write(workspace.path().join("CODEX.md"), "Read AGENTS.md")?;

        // Track the codex instruction file
        let codex_file = workspace.path().join("CODEX.md");
        let mut tracker = FileTracker::new(data_dir.path())?;
        tracker.record_installation(&codex_file, "sha1".into(), 5, None, "agent".into(), workspace.path());
        tracker.save()?;

        let original_dir = std::env::current_dir()?;
        std::env::set_current_dir(workspace.path())?;

        // Purge should succeed without scanning ~/.codex/skills (userprofile dir).
        // Only workspace-scoped dirs and FileTracker entries are used.
        let manager = TemplateManager { config_dir: data_dir.path().to_path_buf() };
        let result = manager.purge(true, false);

        std::env::set_current_dir(original_dir)?;

        assert!(result.is_ok() == true);
        // The tracked codex file should be removed
        assert!(codex_file.exists() == false);
        Ok(())
    }
}
