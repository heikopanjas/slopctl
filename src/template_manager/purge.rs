//! Template purge command

use std::{
    fs,
    path::{Path, PathBuf}
};

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
        let _ = self.try_migrate_tracker(&current_dir);

        let (files_to_purge, agents_md_skipped, agents_md_path) = self.collect_purge_targets(&current_dir, force)?;

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
        let mut file_tracker = FileTracker::new(&current_dir)?;

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

    /// Collects every workspace file slopctl is responsible for, plus the AGENTS.md
    /// preservation decision.
    ///
    /// Pulls candidates from three sources, deduplicates them, then resolves the
    /// AGENTS.md handling: if AGENTS.md is customized and `force` is false it is
    /// removed from the list and `agents_md_skipped` is set; otherwise it is added
    /// (if not already present via the BoM/tracker sweep).
    ///
    /// Returns `(files_to_purge, agents_md_skipped, agents_md_path)`.
    ///
    /// # Errors
    ///
    /// Returns an error if reading AGENTS.md fails.
    fn collect_purge_targets(&self, current_dir: &Path, force: bool) -> Result<(Vec<PathBuf>, bool, PathBuf)>
    {
        let mut files_to_purge: Vec<PathBuf> = Vec::new();

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
        let file_tracker = FileTracker::new(current_dir)?;
        for (rel_path, _) in file_tracker.get_entries()
        {
            let abs_path = current_dir.join(&rel_path);
            if abs_path.exists() == true
            {
                files_to_purge.push(abs_path);
            }
        }

        // Scan workspace-scoped agent skill directories on disk to catch untracked/manually
        // placed skills. Userprofile-based dirs (e.g. codex ~/.codex/skills) are excluded —
        // those are user-global and may contain agent-internal files. FileTracker entries
        // above already cover userprofile skills that slopctl installed.
        let userprofile = dirs::home_dir().unwrap_or_default();
        let skill_search_dirs = agent_defaults::get_workspace_skill_search_dirs(current_dir, &userprofile);
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

        // Resolve AGENTS.md handling
        let agents_md_path = current_dir.join("AGENTS.md");
        let mut agents_md_skipped = false;
        if agents_md_path.exists() == true
        {
            let agents_md_customized = template_engine::is_file_customized(&agents_md_path)?;
            let agents_md_canonical = fs::canonicalize(&agents_md_path).unwrap_or_else(|_| agents_md_path.clone());

            if agents_md_customized == true && force == false
            {
                agents_md_skipped = true;
                // AGENTS.md may already be in files_to_purge via the FileTracker or
                // BoM sweep above. Drop those entries so we honour the "skipped"
                // promise instead of silently deleting the customized file.
                files_to_purge.retain(|f| {
                    let canonical = fs::canonicalize(f).unwrap_or_else(|_| f.clone());
                    canonical != agents_md_canonical
                });
            }
            else if files_to_purge.iter().any(|f| {
                let canonical = fs::canonicalize(f).unwrap_or_else(|_| f.clone());
                canonical == agents_md_canonical
            }) == false
            {
                files_to_purge.push(agents_md_path.clone());
            }
        }

        Ok((files_to_purge, agents_md_skipped, agents_md_path))
    }
}

#[cfg(test)]
mod tests
{
    use std::fs;

    use super::TemplateManager;
    use crate::{
        file_tracker::{FileTracker, LANG_NONE},
        template_manager::cwd_test_guard
    };

    #[test]
    fn test_purge_dry_run_no_files() -> anyhow::Result<()>
    {
        let data_dir = tempfile::TempDir::new()?;
        let workspace = tempfile::TempDir::new()?;

        let _g = cwd_test_guard();
        std::env::set_current_dir(workspace.path())?;

        let manager = TemplateManager { config_dir: data_dir.path().to_path_buf() };
        let result = manager.purge(false, true);

        assert!(result.is_ok() == true);
        Ok(())
    }

    #[test]
    fn test_purge_deduplicates_bom_and_tracker_paths() -> anyhow::Result<()>
    {
        let data_dir = tempfile::TempDir::new()?;
        let workspace = tempfile::TempDir::new()?;

        // Write a minimal templates.yml that declares an agent with a workspace file
        let yaml = "version: 5\nagents:\n  cursor:\n    instructions:\n      - source: cursorrules.md\n        target: $workspace/.cursorrules\n";
        fs::write(data_dir.path().join("templates.yml"), yaml)?;

        // Create the agent file on disk so BoM can find it
        let agent_file = workspace.path().join(".cursorrules");
        fs::write(&agent_file, "test")?;

        // Record the same file in FileTracker (workspace-local)
        let mut tracker = FileTracker::new(workspace.path())?;
        tracker.record_installation(&agent_file, "sha1".into(), 5, LANG_NONE.into(), "cursor".into(), "agent".into());
        tracker.save()?;

        let _g = cwd_test_guard();
        std::env::set_current_dir(workspace.path())?;

        let manager = TemplateManager { config_dir: data_dir.path().to_path_buf() };
        let result = manager.purge(true, false);

        assert!(result.is_ok() == true);
        // The file should have been removed exactly once (no double-removal error)
        assert!(agent_file.exists() == false);
        Ok(())
    }

    #[test]
    fn test_purge_discovers_untracked_skill_files() -> anyhow::Result<()>
    {
        let data_dir = tempfile::TempDir::new()?;
        let workspace = tempfile::TempDir::new()?;

        // Place a skill directory on disk without any FileTracker entry
        let skill_dir = workspace.path().join(".agents/skills/my-skill");
        fs::create_dir_all(&skill_dir)?;
        let skill_file = skill_dir.join("SKILL.md");
        fs::write(&skill_file, "# My Skill")?;

        let _g = cwd_test_guard();
        std::env::set_current_dir(workspace.path())?;

        let manager = TemplateManager { config_dir: data_dir.path().to_path_buf() };
        let result = manager.purge(true, false);

        assert!(result.is_ok() == true);
        // The untracked skill file should have been discovered and removed
        assert!(skill_file.exists() == false);
        Ok(())
    }

    #[test]
    fn test_purge_skips_userprofile_skill_dir_scan() -> anyhow::Result<()>
    {
        let data_dir = tempfile::TempDir::new()?;
        let workspace = tempfile::TempDir::new()?;

        // Create CODEX.md so codex is detected as installed
        fs::write(workspace.path().join("CODEX.md"), "Read AGENTS.md")?;

        // Track the codex instruction file
        let codex_file = workspace.path().join("CODEX.md");
        let mut tracker = FileTracker::new(workspace.path())?;
        tracker.record_installation(&codex_file, "sha1".into(), 5, LANG_NONE.into(), "codex".into(), "agent".into());
        tracker.save()?;

        let _g = cwd_test_guard();
        std::env::set_current_dir(workspace.path())?;

        // Purge should succeed without scanning ~/.codex/skills (userprofile dir).
        // Only workspace-scoped dirs and FileTracker entries are used.
        let manager = TemplateManager { config_dir: data_dir.path().to_path_buf() };
        let result = manager.purge(true, false);

        assert!(result.is_ok() == true);
        // The tracked codex file should be removed
        assert!(codex_file.exists() == false);
        Ok(())
    }

    #[test]
    fn test_purge_preserves_customized_agents_md_when_tracked() -> anyhow::Result<()>
    {
        let data_dir = tempfile::TempDir::new()?;
        let workspace = tempfile::TempDir::new()?;

        // Customized AGENTS.md = no TEMPLATE_MARKER present
        let agents_md = workspace.path().join("AGENTS.md");
        fs::write(&agents_md, "# My customized instructions\n")?;

        // Track it (this is the bug trigger: tracker entry would queue it for deletion)
        let mut tracker = FileTracker::new(workspace.path())?;
        tracker.record_installation(&agents_md, "sha1".into(), 5, LANG_NONE.into(), "all".into(), "main".into());
        tracker.save()?;

        let _g = cwd_test_guard();
        std::env::set_current_dir(workspace.path())?;

        let manager = TemplateManager { config_dir: data_dir.path().to_path_buf() };

        // force = false: customized AGENTS.md must be skipped AND removed from
        // files_to_purge. Otherwise the deletion loop would silently delete it
        // while the user is told it was preserved.
        let (files, skipped, _) = manager.collect_purge_targets(workspace.path(), false)?;
        assert!(skipped == true, "customized AGENTS.md should be flagged as skipped");
        let agents_md_canonical = fs::canonicalize(&agents_md)?;
        let queued = files.iter().any(|f| fs::canonicalize(f).map(|c| c == agents_md_canonical).unwrap_or(false));
        assert!(queued == false, "customized AGENTS.md must not appear in files_to_purge");

        // With force = true the same file is allowed through
        let (files, skipped, _) = manager.collect_purge_targets(workspace.path(), true)?;
        assert!(skipped == false);
        let queued = files.iter().any(|f| fs::canonicalize(f).map(|c| c == agents_md_canonical).unwrap_or(false));
        assert!(queued == true, "with --force AGENTS.md must be queued for deletion");

        Ok(())
    }
}
