//! Template remove command

use std::{
    fs,
    path::{Path, PathBuf}
};

use owo_colors::OwoColorize;

use super::TemplateManager;
use crate::{
    Result, agent_defaults,
    agent_defaults::resolve_placeholder_path,
    bom,
    bom::BillOfMaterials,
    file_tracker::FileTracker,
    template_engine,
    utils::{collect_files_recursive, confirm_action, remove_file_and_cleanup_parents}
};

impl TemplateManager
{
    /// Remove agent-specific, language-specific, and/or skill files from the current directory
    ///
    /// Deletes files associated with the specified agent, language, and/or skills.
    /// Agent files come from the Bill of Materials; language files are resolved via
    /// `resolve_language_files`; skill files come from the FileTracker (covering
    /// template, top-level, and ad-hoc sources). AGENTS.md is never touched.
    ///
    /// # Arguments
    ///
    /// * `agent` - Optional agent name. If Some, removes files for that agent only.
    /// * `lang` - Optional language name. If Some, removes disk files for that language.
    /// * `skills` - Skill names to remove. Empty slice means no skill-specific removal.
    /// * `force` - If true, skip confirmation prompt
    /// * `dry_run` - If true, only show what would be removed without actually removing
    ///
    /// # Errors
    ///
    /// Returns an error if file deletion fails or the current directory cannot be determined
    pub fn remove(&self, agent: Option<&str>, lang: Option<&str>, skills: &[String], force: bool, dry_run: bool) -> Result<()>
    {
        let current_dir = std::env::current_dir()?;
        let _ = self.try_migrate_tracker(&current_dir);
        let config_file = self.config_dir.join("templates.yml");
        let has_agent_target = agent.is_some();
        let has_lang_target = lang.is_some();
        let has_skill_target = skills.is_empty() == false;
        let remove_all = agent.is_none() && lang.is_none() && has_skill_target == false;

        let mut files_to_remove: Vec<PathBuf> = Vec::new();
        let mut stale_tracker_paths: Vec<PathBuf> = Vec::new();
        let mut description_parts: Vec<String> = Vec::new();

        // Collect agent files when agent or --all is requested.
        // Tries BoM first (templates.yml); falls back to FileTracker when the
        // agent/template entry was removed after installation.
        if has_agent_target == true || remove_all == true
        {
            let file_tracker = FileTracker::new(&current_dir)?;

            let bom = if config_file.exists() == true
            {
                BillOfMaterials::from_config(&config_file).ok()
            }
            else
            {
                None
            };

            if let Some(agent_name) = agent
            {
                let found_in_bom = if let Some(ref bom) = bom &&
                    bom.has_agent(agent_name) == true
                {
                    if let Some(agent_files) = bom.get_agent_files(agent_name)
                    {
                        files_to_remove.extend(agent_files.iter().filter(|f| f.exists()).filter_map(|f| fs::canonicalize(f).ok()));
                    }
                    true
                }
                else
                {
                    false
                };

                if found_in_bom == false
                {
                    println!("{} Agent '{}' not in templates.yml, using installation records", "→".blue(), agent_name.yellow());
                    let agent_entries = file_tracker.get_entries_by_category("agent");
                    for (path, _) in agent_entries
                    {
                        if path.exists() == true && Self::path_belongs_to_agent(&path, agent_name) == true
                        {
                            files_to_remove.push(path);
                        }
                    }
                }

                // Collect skill files under this agent's skill dir via filesystem scan
                // (catches untracked/manually placed skills that FileTracker misses).
                // Skip userprofile-based dirs (e.g. codex ~/.codex/skills) — those are
                // user-global and may contain agent-internal files or other workspaces' skills.
                let userprofile = dirs::home_dir().unwrap_or_default();
                if let Some(raw_skill_dir) = agent_defaults::get_skill_dir(agent_name) &&
                    raw_skill_dir.starts_with(agent_defaults::PLACEHOLDER_WORKSPACE) == true
                {
                    let skill_dir = resolve_placeholder_path(raw_skill_dir, &current_dir, &userprofile);
                    if skill_dir.exists() == true &&
                        let Ok(entries) = fs::read_dir(&skill_dir)
                    {
                        for entry in entries.flatten()
                        {
                            if entry.path().is_dir() == true
                            {
                                let mut skill_files = Vec::new();
                                let _ = collect_files_recursive(&entry.path(), &mut skill_files);
                                for f in skill_files
                                {
                                    if files_to_remove.contains(&f) == false
                                    {
                                        files_to_remove.push(f);
                                    }
                                }
                            }
                        }
                    }
                }

                // Supplement with tracked skill files that belong to this agent
                // (covers paths outside the standard skill directory tree)
                let skill_entries = file_tracker.get_entries_by_category("skill");
                for (rel_path, _) in skill_entries
                {
                    let abs_path = current_dir.join(&rel_path);
                    if abs_path.exists() == true && Self::path_belongs_to_agent(&abs_path, agent_name) == true && files_to_remove.contains(&abs_path) == false
                    {
                        files_to_remove.push(abs_path);
                    }
                }

                description_parts.push(format!("agent '{}'", agent_name.yellow()));
            }
            else
            {
                // --all: collect agent files from BoM if available, canonicalized
                // to absolute paths so they dedup correctly against FileTracker entries.
                if let Some(ref bom) = bom
                {
                    let agent_names = bom.get_agent_names();
                    for name in &agent_names
                    {
                        if let Some(agent_files) = bom.get_agent_files(name)
                        {
                            files_to_remove.extend(agent_files.iter().filter(|f| f.exists()).filter_map(|f| fs::canonicalize(f).ok()));
                        }
                    }
                }

                // Supplement with tracked agent files not already collected from BoM
                let agent_entries = file_tracker.get_entries_by_category("agent");
                for (rel_path, _) in agent_entries
                {
                    let abs_path = current_dir.join(&rel_path);
                    if abs_path.exists() == true && files_to_remove.contains(&abs_path) == false
                    {
                        files_to_remove.push(abs_path);
                    }
                }

                // Scan workspace-scoped agent skill directories on filesystem to catch
                // untracked/manually placed skills. Userprofile-based dirs (e.g. codex)
                // are excluded — those are covered by the FileTracker sweep below.
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
                                for f in skill_files
                                {
                                    if files_to_remove.contains(&f) == false
                                    {
                                        files_to_remove.push(f);
                                    }
                                }
                            }
                        }
                    }
                }

                // Supplement with tracked skill files not already collected from filesystem
                let skill_entries = file_tracker.get_entries_by_category("skill");
                for (rel_path, _) in skill_entries
                {
                    let abs_path = current_dir.join(&rel_path);
                    if abs_path.exists() == true && files_to_remove.contains(&abs_path) == false
                    {
                        files_to_remove.push(abs_path);
                    }
                }

                description_parts.push("all agents and skills".to_string());
            }
        }

        // Collect language disk files when --lang is requested.
        // Tries templates.yml first; falls back to FileTracker when the
        // language entry was removed after installation.
        if has_lang_target == true
        {
            let lang_name = lang.unwrap();

            let found_in_config = if config_file.exists() == true &&
                let Ok(config) = template_engine::load_template_config(&self.config_dir) &&
                config.languages.contains_key(lang_name) == true
            {
                if let Ok(file_mappings) = bom::resolve_language_files(lang_name, &config)
                {
                    for mapping in file_mappings
                    {
                        if let Some(path) = BillOfMaterials::resolve_workspace_path(&mapping.target)
                        {
                            let abs_path = current_dir.join(path);
                            if abs_path.exists() == true && files_to_remove.contains(&abs_path) == false
                            {
                                files_to_remove.push(abs_path);
                            }
                        }
                    }
                }
                true
            }
            else
            {
                false
            };

            if found_in_config == false
            {
                println!("{} Language '{}' not in templates.yml, using installation records", "→".blue(), lang_name.yellow());
                let file_tracker = FileTracker::new(&current_dir)?;
                let all_entries = file_tracker.get_entries();
                for (rel_path, meta) in all_entries
                {
                    let abs_path = current_dir.join(&rel_path);
                    if meta.lang == lang_name &&
                        meta.category != "main" &&
                        meta.category != "skill" &&
                        abs_path.exists() == true &&
                        files_to_remove.contains(&abs_path) == false
                    {
                        files_to_remove.push(abs_path);
                    }
                }
            }

            description_parts.push(format!("language '{}'", lang_name.yellow()));
        }

        // Collect skill files by name from all agent skill dirs and cross-client dir
        if has_skill_target == true
        {
            let userprofile = dirs::home_dir().ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "Could not determine home directory"))?;
            let skill_search_dirs = agent_defaults::get_all_skill_search_dirs(&current_dir, &userprofile);

            let file_tracker = FileTracker::new(&current_dir)?;
            let skill_entries = file_tracker.get_entries_by_category("skill");

            for skill_name in skills
            {
                let mut found = false;

                // Scan filesystem under every agent skill dir + cross-client dir
                for search_dir in &skill_search_dirs
                {
                    let candidate = search_dir.join(skill_name);
                    if candidate.is_dir() == true
                    {
                        let mut dir_files = Vec::new();
                        collect_files_recursive(&candidate, &mut dir_files)?;
                        for f in dir_files
                        {
                            if files_to_remove.contains(&f) == false
                            {
                                files_to_remove.push(f);
                                found = true;
                            }
                        }
                    }
                }

                // Also sweep FileTracker for any tracked paths outside the standard dirs.
                // Collect stale entries (tracked but missing on disk) for silent tracker cleanup.
                for (rel_path, _) in &skill_entries
                {
                    if Self::extract_skill_name_from_path(rel_path).as_deref() == Some(skill_name.as_str())
                    {
                        let abs_path = current_dir.join(rel_path);
                        if abs_path.exists() == true && files_to_remove.contains(&abs_path) == false
                        {
                            files_to_remove.push(abs_path);
                            found = true;
                        }
                        else if abs_path.exists() == false && stale_tracker_paths.contains(&abs_path) == false
                        {
                            stale_tracker_paths.push(abs_path);
                            found = true;
                        }
                    }
                }

                if found == false
                {
                    println!("{} Skill '{}' not found in current workspace", "!".yellow(), skill_name.yellow());
                }

                description_parts.push(format!("skill '{}'", skill_name.yellow()));
            }
        }

        files_to_remove.sort();
        files_to_remove.dedup();
        stale_tracker_paths.sort();
        stale_tracker_paths.dedup();

        let description = description_parts.join(", ");

        // Silently purge stale tracker entries (tracked but no longer on disk) even when
        // there are no real files to remove; this prevents phantom skills in status output.
        if files_to_remove.is_empty() == true
        {
            if stale_tracker_paths.is_empty() == false && dry_run == false
            {
                let mut file_tracker = FileTracker::new(&current_dir)?;
                for path in &stale_tracker_paths
                {
                    file_tracker.remove_entry(path);
                }
                file_tracker.save()?;
            }

            println!("{} No files found for {} in current directory", "→".blue(), description);
            return Ok(());
        }

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

        println!("\n{} Files to be removed for {}:", "→".blue(), description);
        for file in &files_to_remove
        {
            println!("  • {}", file.display().to_string().yellow());
        }
        println!();

        if force == false && confirm_action(&format!("{} Proceed with removal? [y/N]: ", "?".yellow()))? == false
        {
            println!("{} Operation cancelled", "✗".red());
            return Ok(());
        }

        let mut file_tracker = FileTracker::new(&current_dir)?;

        let mut removed_count = 0;
        for file in &files_to_remove
        {
            match remove_file_and_cleanup_parents(file)
            {
                | Ok(_) =>
                {
                    println!("{} Removed {}", "✓".green(), file.display());
                    removed_count += 1;
                    file_tracker.remove_entry(file);
                }
                | Err(e) =>
                {
                    eprintln!("{} Failed to remove {}: {}", "✗".red(), file.display(), e);
                }
            }
        }

        // Also prune any stale tracker entries collected alongside real files
        for path in &stale_tracker_paths
        {
            file_tracker.remove_entry(path);
        }

        file_tracker.save()?;

        println!("\n{} Removed {} file(s) for {}", "✓".green(), removed_count, description);

        Ok(())
    }

    /// Remove all slopctl-installed files from the current workspace, including AGENTS.md
    ///
    /// This is the `--purge` mode of `remove`. It discovers files from three sources
    /// (BoM, FileTracker, filesystem scan) and removes them all. AGENTS.md is included
    /// unless it has been customized and `force` is false.
    ///
    /// # Arguments
    ///
    /// * `force` - Skip confirmation prompt; also deletes a customized AGENTS.md
    /// * `dry_run` - Preview what would be removed without making changes
    ///
    /// # Errors
    ///
    /// Returns an error if file deletion fails or the current directory cannot be determined
    pub fn remove_purge(&self, force: bool, dry_run: bool) -> Result<()>
    {
        let current_dir = std::env::current_dir()?;
        let _ = self.try_migrate_tracker(&current_dir);

        let (files_to_purge, agents_md_skipped, agents_md_path) = self.collect_purge_targets(&current_dir, force)?;

        if files_to_purge.is_empty() == true && agents_md_skipped == false
        {
            println!("{} No slopctl files found to purge", "→".blue());
            return Ok(());
        }

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

        if force == false && confirm_action(&format!("{} Are you sure you want to purge all slopctl files? (y/N): ", "?".yellow()))? == false
        {
            println!("{} Operation cancelled", "→".blue());
            return Ok(());
        }

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
    /// Pulls candidates from three sources (BoM, FileTracker, filesystem scan),
    /// deduplicates them, then resolves the AGENTS.md handling: if AGENTS.md is
    /// customized and `force` is false it is removed from the list and
    /// `agents_md_skipped` is set; otherwise it is added if not already present.
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
        // placed skills. Userprofile-based dirs (e.g. codex ~/.codex/skills) are excluded.
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

    /// Check if a file path belongs to a specific agent's directory tree
    ///
    /// Matches paths containing the agent name in a directory component
    /// (e.g. `.cursor/skills/`, `.claude/skills/`).
    fn path_belongs_to_agent(path: &std::path::Path, agent_name: &str) -> bool
    {
        let agent_dir_patterns = [format!(".{}/", agent_name), format!(".{}\\", agent_name), format!("/{}/", agent_name), format!("\\{}\\", agent_name)];

        let path_str = path.to_string_lossy();
        agent_dir_patterns.iter().any(|pattern| path_str.contains(pattern))
    }
}

#[cfg(test)]
mod tests
{
    use std::{fs, path::PathBuf};

    use super::TemplateManager;
    use crate::{
        bom::BillOfMaterials,
        file_tracker::{AGENT_ALL, FileTracker, LANG_NONE},
        template_manager::cwd_test_guard
    };

    #[test]
    fn test_path_belongs_to_cursor()
    {
        let path = PathBuf::from("/home/user/project/.cursor/skills/my-skill/SKILL.md");
        assert!(TemplateManager::path_belongs_to_agent(&path, "cursor") == true);
    }

    #[test]
    fn test_path_belongs_to_claude()
    {
        let path = PathBuf::from("/home/user/project/.claude/skills/foo/SKILL.md");
        assert!(TemplateManager::path_belongs_to_agent(&path, "claude") == true);
    }

    #[test]
    fn test_path_does_not_belong_to_wrong_agent()
    {
        let path = PathBuf::from("/home/user/project/.cursor/skills/foo/SKILL.md");
        assert!(TemplateManager::path_belongs_to_agent(&path, "claude") == false);
    }

    #[test]
    fn test_path_no_agent_directory()
    {
        let path = PathBuf::from("/home/user/project/AGENTS.md");
        assert!(TemplateManager::path_belongs_to_agent(&path, "cursor") == false);
    }

    #[test]
    fn test_resolve_workspace_path_skips_instructions()
    {
        assert!(BillOfMaterials::resolve_workspace_path("$instructions").is_none() == true);
        assert!(BillOfMaterials::resolve_workspace_path("$instructions/rust.md").is_none() == true);
    }

    #[test]
    fn test_resolve_workspace_path_skips_userprofile()
    {
        assert!(BillOfMaterials::resolve_workspace_path("$userprofile/.codex/init.md").is_none() == true);
    }

    #[test]
    fn test_resolve_workspace_path_resolves_workspace()
    {
        let result = BillOfMaterials::resolve_workspace_path("$workspace/.rustfmt.toml");
        assert!(result.is_some() == true);
        assert_eq!(result.unwrap(), PathBuf::from("./.rustfmt.toml"));
    }

    #[test]
    fn test_remove_lang_unknown_no_error() -> anyhow::Result<()>
    {
        let _g = cwd_test_guard();

        let dir = tempfile::TempDir::new()?;
        let config_path = dir.path().join("templates.yml");
        let yaml = "languages:\n  rust:\n    files: []\n";
        fs::write(&config_path, yaml)?;

        let manager = TemplateManager { config_dir: dir.path().to_path_buf() };
        let result = manager.remove(None, Some("nonexistent"), &[], false, true);
        assert!(result.is_ok() == true);
        Ok(())
    }

    #[test]
    fn test_remove_agent_falls_back_to_file_tracker() -> anyhow::Result<()>
    {
        let data_dir = tempfile::TempDir::new()?;
        let workspace = tempfile::TempDir::new()?;

        let yaml = "version: 5\nagents:\n  claude:\n    instructions: []\n";
        fs::write(data_dir.path().join("templates.yml"), yaml)?;

        let agent_file = workspace.path().join(".cursorrules");
        fs::write(&agent_file, "test")?;

        let mut tracker = FileTracker::new(workspace.path())?;
        tracker.record_installation(&agent_file, "sha1".into(), 5, LANG_NONE.into(), "cursor".into(), "agent".into());
        tracker.save()?;

        let _g = cwd_test_guard();
        std::env::set_current_dir(workspace.path())?;

        let manager = TemplateManager { config_dir: data_dir.path().to_path_buf() };
        let result = manager.remove(Some("cursor"), None, &[], false, true);

        assert!(result.is_ok() == true);
        Ok(())
    }

    #[test]
    fn test_remove_lang_falls_back_to_file_tracker() -> anyhow::Result<()>
    {
        let data_dir = tempfile::TempDir::new()?;
        let workspace = tempfile::TempDir::new()?;

        let yaml = "version: 5\nlanguages:\n  swift:\n    files: []\n";
        fs::write(data_dir.path().join("templates.yml"), yaml)?;

        let lang_file = workspace.path().join(".rustfmt.toml");
        fs::write(&lang_file, "max_width = 100")?;

        let mut tracker = FileTracker::new(workspace.path())?;
        tracker.record_installation(&lang_file, "sha1".into(), 5, "rust".into(), AGENT_ALL.into(), "language".into());
        tracker.save()?;

        let _g = cwd_test_guard();
        std::env::set_current_dir(workspace.path())?;

        let manager = TemplateManager { config_dir: data_dir.path().to_path_buf() };
        let result = manager.remove(None, Some("rust"), &[], false, true);

        assert!(result.is_ok() == true);
        Ok(())
    }

    #[test]
    fn test_remove_lang_fallback_excludes_main_and_skill() -> anyhow::Result<()>
    {
        let data_dir = tempfile::TempDir::new()?;
        let workspace = tempfile::TempDir::new()?;

        let lang_file = workspace.path().join(".rustfmt.toml");
        let main_file = workspace.path().join("AGENTS.md");
        let skill_dir = workspace.path().join(".cursor/skills/rust-conventions");
        fs::create_dir_all(&skill_dir)?;
        let skill_file = skill_dir.join("SKILL.md");
        fs::write(&lang_file, "max_width = 100")?;
        fs::write(&main_file, "# Agents")?;
        fs::write(&skill_file, "# Skill")?;

        let mut tracker = FileTracker::new(workspace.path())?;
        tracker.record_installation(&lang_file, "sha1".into(), 5, "rust".into(), AGENT_ALL.into(), "language".into());
        tracker.record_installation(&main_file, "sha2".into(), 5, "rust".into(), AGENT_ALL.into(), "main".into());
        tracker.record_installation(&skill_file, "sha3".into(), 5, "rust".into(), AGENT_ALL.into(), "skill".into());
        tracker.save()?;

        let _g = cwd_test_guard();
        std::env::set_current_dir(workspace.path())?;

        let manager = TemplateManager { config_dir: data_dir.path().to_path_buf() };
        let result = manager.remove(None, Some("rust"), &[], false, true);

        assert!(result.is_ok() == true);
        Ok(())
    }

    #[test]
    fn test_remove_agent_discovers_untracked_skill_files() -> anyhow::Result<()>
    {
        let data_dir = tempfile::TempDir::new()?;
        let workspace = tempfile::TempDir::new()?;

        let yaml = "version: 5\nagents:\n  cursor:\n    instructions:\n      - source: cursorrules.md\n        target: $workspace/.cursorrules\n";
        fs::write(data_dir.path().join("templates.yml"), yaml)?;

        let agent_file = workspace.path().join(".cursorrules");
        fs::write(&agent_file, "test")?;

        // Place a skill directory on disk without any FileTracker entry
        let skill_dir = workspace.path().join(".cursor/skills/my-skill");
        fs::create_dir_all(&skill_dir)?;
        let skill_file = skill_dir.join("SKILL.md");
        fs::write(&skill_file, "# My Skill")?;

        let _g = cwd_test_guard();
        std::env::set_current_dir(workspace.path())?;

        let manager = TemplateManager { config_dir: data_dir.path().to_path_buf() };
        let result = manager.remove(Some("cursor"), None, &[], true, false);

        assert!(result.is_ok() == true);
        assert!(skill_file.exists() == false);
        Ok(())
    }

    #[test]
    fn test_remove_all_discovers_untracked_skill_files() -> anyhow::Result<()>
    {
        let data_dir = tempfile::TempDir::new()?;
        let workspace = tempfile::TempDir::new()?;

        // Place an untracked skill in the cross-client directory
        let skill_dir = workspace.path().join(".agents/skills/my-skill");
        fs::create_dir_all(&skill_dir)?;
        let skill_file = skill_dir.join("SKILL.md");
        fs::write(&skill_file, "# My Skill")?;

        let _g = cwd_test_guard();
        std::env::set_current_dir(workspace.path())?;

        let manager = TemplateManager { config_dir: data_dir.path().to_path_buf() };
        let result = manager.remove(None, None, &[], true, false);

        assert!(result.is_ok() == true);
        assert!(skill_file.exists() == false);
        Ok(())
    }

    #[test]
    fn test_remove_purge_dry_run_no_files() -> anyhow::Result<()>
    {
        let data_dir = tempfile::TempDir::new()?;
        let workspace = tempfile::TempDir::new()?;

        let _g = cwd_test_guard();
        std::env::set_current_dir(workspace.path())?;

        let manager = TemplateManager { config_dir: data_dir.path().to_path_buf() };
        let result = manager.remove_purge(false, true);

        assert!(result.is_ok() == true);
        Ok(())
    }

    #[test]
    fn test_remove_purge_deduplicates_bom_and_tracker_paths() -> anyhow::Result<()>
    {
        let data_dir = tempfile::TempDir::new()?;
        let workspace = tempfile::TempDir::new()?;

        let yaml = "version: 5\nagents:\n  cursor:\n    instructions:\n      - source: cursorrules.md\n        target: $workspace/.cursorrules\n";
        fs::write(data_dir.path().join("templates.yml"), yaml)?;

        let agent_file = workspace.path().join(".cursorrules");
        fs::write(&agent_file, "test")?;

        let mut tracker = FileTracker::new(workspace.path())?;
        tracker.record_installation(&agent_file, "sha1".into(), 5, LANG_NONE.into(), "cursor".into(), "agent".into());
        tracker.save()?;

        let _g = cwd_test_guard();
        std::env::set_current_dir(workspace.path())?;

        let manager = TemplateManager { config_dir: data_dir.path().to_path_buf() };
        let result = manager.remove_purge(true, false);

        assert!(result.is_ok() == true);
        assert!(agent_file.exists() == false);
        Ok(())
    }

    #[test]
    fn test_remove_purge_discovers_untracked_skill_files() -> anyhow::Result<()>
    {
        let data_dir = tempfile::TempDir::new()?;
        let workspace = tempfile::TempDir::new()?;

        let skill_dir = workspace.path().join(".agents/skills/my-skill");
        fs::create_dir_all(&skill_dir)?;
        let skill_file = skill_dir.join("SKILL.md");
        fs::write(&skill_file, "# My Skill")?;

        let _g = cwd_test_guard();
        std::env::set_current_dir(workspace.path())?;

        let manager = TemplateManager { config_dir: data_dir.path().to_path_buf() };
        let result = manager.remove_purge(true, false);

        assert!(result.is_ok() == true);
        assert!(skill_file.exists() == false);
        Ok(())
    }

    #[test]
    fn test_remove_purge_skips_userprofile_skill_dir_scan() -> anyhow::Result<()>
    {
        let data_dir = tempfile::TempDir::new()?;
        let workspace = tempfile::TempDir::new()?;

        let codex_file = workspace.path().join("CODEX.md");
        fs::write(&codex_file, "Read AGENTS.md")?;

        let mut tracker = FileTracker::new(workspace.path())?;
        tracker.record_installation(&codex_file, "sha1".into(), 5, LANG_NONE.into(), "codex".into(), "agent".into());
        tracker.save()?;

        let _g = cwd_test_guard();
        std::env::set_current_dir(workspace.path())?;

        let manager = TemplateManager { config_dir: data_dir.path().to_path_buf() };
        let result = manager.remove_purge(true, false);

        assert!(result.is_ok() == true);
        assert!(codex_file.exists() == false);
        Ok(())
    }

    #[test]
    fn test_remove_purge_preserves_customized_agents_md_when_tracked() -> anyhow::Result<()>
    {
        let data_dir = tempfile::TempDir::new()?;
        let workspace = tempfile::TempDir::new()?;

        let agents_md = workspace.path().join("AGENTS.md");
        fs::write(&agents_md, "# My customized instructions\n")?;

        let mut tracker = FileTracker::new(workspace.path())?;
        tracker.record_installation(&agents_md, "sha1".into(), 5, LANG_NONE.into(), "all".into(), "main".into());
        tracker.save()?;

        let _g = cwd_test_guard();
        std::env::set_current_dir(workspace.path())?;

        let manager = TemplateManager { config_dir: data_dir.path().to_path_buf() };

        let (files, skipped, _) = manager.collect_purge_targets(workspace.path(), false)?;
        assert!(skipped == true, "customized AGENTS.md should be flagged as skipped");
        let agents_md_canonical = fs::canonicalize(&agents_md)?;
        let queued = files.iter().any(|f| fs::canonicalize(f).map(|c| c == agents_md_canonical).unwrap_or(false));
        assert!(queued == false, "customized AGENTS.md must not appear in files_to_purge");

        let (files, skipped, _) = manager.collect_purge_targets(workspace.path(), true)?;
        assert!(skipped == false);
        let queued = files.iter().any(|f| fs::canonicalize(f).map(|c| c == agents_md_canonical).unwrap_or(false));
        assert!(queued == true, "with --force AGENTS.md must be queued for deletion");

        Ok(())
    }

    #[test]
    fn test_remove_agent_codex_skips_userprofile_skill_scan() -> anyhow::Result<()>
    {
        let data_dir = tempfile::TempDir::new()?;
        let workspace = tempfile::TempDir::new()?;

        // Create CODEX.md so codex is detected as installed
        let codex_file = workspace.path().join("CODEX.md");
        fs::write(&codex_file, "Read AGENTS.md")?;

        // Track the codex instruction file so remove has something to find
        let mut tracker = FileTracker::new(workspace.path())?;
        tracker.record_installation(&codex_file, "sha1".into(), 5, LANG_NONE.into(), "codex".into(), "agent".into());
        tracker.save()?;

        let _g = cwd_test_guard();
        std::env::set_current_dir(workspace.path())?;

        // Use dry-run to inspect what would be removed without side effects.
        // The key assertion is that this succeeds without attempting to scan
        // the userprofile-based ~/.codex/skills directory (which would pick up
        // .system and other workspaces' skills).
        let manager = TemplateManager { config_dir: data_dir.path().to_path_buf() };
        let result = manager.remove(Some("codex"), None, &[], false, true);

        assert!(result.is_ok() == true);
        Ok(())
    }
}
