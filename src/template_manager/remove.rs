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
    file_tracker::{FileTracker, LANG_NONE},
    template_engine,
    utils::{collect_files_recursive, confirm_action, remove_file_and_cleanup_parents}
};

impl TemplateManager
{
    /// Remove agent-specific and/or language-specific files from the current directory
    ///
    /// Deletes files associated with the specified agent and/or language.
    /// Agent files come from the Bill of Materials; language files are resolved via
    /// `resolve_language_files`; language-associated skills are removed with the
    /// language. AGENTS.md is never touched.
    ///
    /// # Arguments
    ///
    /// * `agent` - Optional agent name. If Some, removes files for that agent only.
    /// * `lang` - Optional language name. If Some, removes disk files for that language.
    /// * `force` - If true, skip confirmation prompt
    /// * `dry_run` - If true, only show what would be removed without actually removing
    ///
    /// # Errors
    ///
    /// Returns an error if file deletion fails or the current directory cannot be determined
    pub fn remove(&self, agent: Option<&str>, lang: Option<&str>, force: bool, dry_run: bool) -> Result<()>
    {
        let current_dir = std::env::current_dir()?;
        let _ = self.try_migrate_tracker(&current_dir);
        let config_file = self.config_dir.join("templates.yml");
        let has_agent_target = agent.is_some();
        let has_lang_target = lang.is_some();
        let remove_all = agent.is_none() && lang.is_none();

        let mut files_to_remove: Vec<PathBuf> = Vec::new();
        let mut description_parts: Vec<String> = Vec::new();
        let mut dirs_to_cleanup: Vec<PathBuf> = Vec::new();

        // Collect agent files when agent or --all is requested.
        // Tries BoM first (templates.yml); falls back to FileTracker when the
        // agent/template entry was removed after installation.
        if has_agent_target == true || remove_all == true
        {
            let file_tracker = FileTracker::new(&current_dir)?;
            let agent_catalog = agent_defaults::load_agent_catalog_from_dir(&self.config_dir)?;

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
                // Skip userprofile-based dirs because those are user-global and may
                // contain agent-internal files or other workspaces' skills.
                let userprofile = dirs::home_dir().unwrap_or_default();
                if let Some(raw_skill_dir) = agent_defaults::get_skill_dir_from_catalog(&agent_catalog, agent_name) &&
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

                // For cross-client agents, decide what happens to .agents/skills/.
                // If this is the last cross-client agent in the workspace, collect its
                // contents for deletion so orphaned skills don't accumulate.
                // If another cross-client agent is still installed, preserve the directory
                // and print an informational note so the user knows why it was skipped.
                if agent_defaults::reads_cross_client_skills_from_catalog(&agent_catalog, agent_name) == true
                {
                    let other_cross_client: Vec<String> = agent_defaults::detect_all_installed_agents_from_catalog(&agent_catalog, &current_dir)
                        .into_iter()
                        .filter(|other| *other != agent_name && agent_defaults::reads_cross_client_skills_from_catalog(&agent_catalog, other) == true)
                        .collect();

                    let cross_client_dir = resolve_placeholder_path(agent_defaults::CROSS_CLIENT_SKILL_DIR, &current_dir, &userprofile);

                    if other_cross_client.is_empty() == true
                    {
                        if cross_client_dir.exists() == true &&
                            let Ok(entries) = fs::read_dir(&cross_client_dir)
                        {
                            for entry in entries.flatten()
                            {
                                if entry.path().is_dir() == true
                                {
                                    let mut skill_files = Vec::new();
                                    let _ = collect_files_recursive(&entry.path(), &mut skill_files);
                                    for f in skill_files
                                    {
                                        // Skip skills that belong to a language installation — those
                                        // must survive agent removal. Only agent-specific and
                                        // top-level skills (lang == LANG_NONE) are orphaned when
                                        // the last cross-client agent leaves.
                                        // Untracked files (no metadata) are treated as agent-owned.
                                        let is_lang_skill = file_tracker.get_metadata(&f).map(|meta| meta.lang != LANG_NONE).unwrap_or(false);

                                        if is_lang_skill == false && files_to_remove.contains(&f) == false
                                        {
                                            files_to_remove.push(f);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    else
                    {
                        let agents_list = other_cross_client.join(", ");
                        println!("{} Keeping {} (still in use by: {})", "→".blue(), agent_defaults::CROSS_CLIENT_SKILL_DIR, agents_list.yellow());
                    }
                }

                description_parts.push(format!("agent '{}'", agent_name.yellow()));
                dirs_to_cleanup.extend(agent_defaults::get_workspace_marker_dirs_from_catalog(&agent_catalog, agent_name, &current_dir));
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
                // untracked/manually placed skills. Userprofile-based dirs are excluded;
                // those are covered by the FileTracker sweep below.
                let userprofile = dirs::home_dir().unwrap_or_default();
                let skill_search_dirs = agent_defaults::get_workspace_skill_search_dirs_from_catalog(&agent_catalog, &current_dir, &userprofile);
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
                for name in agent_defaults::list_agent_names_from_catalog(&agent_catalog)
                {
                    dirs_to_cleanup.extend(agent_defaults::get_workspace_marker_dirs_from_catalog(&agent_catalog, name, &current_dir));
                }
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
                if let Ok(lang_skills) = bom::resolve_language_skills(lang_name, &config)
                {
                    let userprofile = dirs::home_dir().unwrap_or_default();
                    let agent_catalog = agent_defaults::load_agent_catalog_from_dir(&self.config_dir)?;
                    let skill_search_dirs = agent_defaults::get_workspace_skill_search_dirs_from_catalog(&agent_catalog, &current_dir, &userprofile);
                    for skill in lang_skills
                    {
                        let skill_name = skill.derive_name();
                        for search_dir in &skill_search_dirs
                        {
                            let candidate = search_dir.join(skill_name);
                            if candidate.is_dir() == true
                            {
                                let mut skill_files = Vec::new();
                                collect_files_recursive(&candidate, &mut skill_files)?;
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

            let file_tracker = FileTracker::new(&current_dir)?;
            let all_entries = file_tracker.get_entries();
            for (rel_path, meta) in all_entries
            {
                let abs_path = current_dir.join(&rel_path);
                if meta.lang == lang_name && meta.category == "skill" && abs_path.exists() == true && files_to_remove.contains(&abs_path) == false
                {
                    files_to_remove.push(abs_path);
                }
            }

            description_parts.push(format!("language '{}'", lang_name.yellow()));
        }

        files_to_remove.sort();
        files_to_remove.dedup();

        let description = description_parts.join(", ");

        if files_to_remove.is_empty() == true
        {
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

            for dir in &dirs_to_cleanup
            {
                if dir.exists() == true
                {
                    println!("  {} {} (removed if empty)", "○".yellow(), dir.display());
                }
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
            file_tracker.remove_entry(file);
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

        // When a language is removed, AGENTS.md stays on disk but its tracker entry
        // still carries `lang: "<lang>"`. Reset it to LANG_NONE so that
        // `get_installed_language()` and `status` no longer report the language.
        if has_lang_target == true
        {
            let lang_name = lang.unwrap();
            file_tracker.clear_lang_for_category(lang_name, "main");
        }

        file_tracker.save()?;

        for dir in &dirs_to_cleanup
        {
            if dir.exists() == true && fs::remove_dir(dir).is_ok() == true
            {
                println!("{} Removed empty directory {}", "✓".green(), dir.display());
            }
        }

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

        let agent_catalog = agent_defaults::load_agent_catalog_from_dir(&self.config_dir)?;
        for name in agent_defaults::list_agent_names_from_catalog(&agent_catalog)
        {
            for dir in agent_defaults::get_workspace_marker_dirs_from_catalog(&agent_catalog, name, &current_dir)
            {
                if dir.exists() == true && fs::remove_dir(&dir).is_ok() == true
                {
                    println!("{} Removed empty directory {}", "✓".green(), dir.display());
                }
            }
        }

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

        // Merge all FileTracker entries for the workspace (catches top-level skills
        // and files from older slopctl versions).
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
        // placed skills. Userprofile-based dirs are excluded.
        let userprofile = dirs::home_dir().unwrap_or_default();
        let agent_catalog = agent_defaults::load_agent_catalog_from_dir(&self.config_dir)?;
        let skill_search_dirs = agent_defaults::get_workspace_skill_search_dirs_from_catalog(&agent_catalog, current_dir, &userprofile);
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
    /// Matches paths containing the agent name in a directory component.
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
        agent_defaults::AGENT_DEFAULTS_FILE,
        bom::BillOfMaterials,
        file_tracker::{AGENT_ALL, FileTracker, LANG_NONE},
        template_manager::cwd_test_guard
    };

    fn write_synthetic_agent_defaults(config_dir: &std::path::Path, agents: &[(&str, bool, Option<&str>, Option<&str>)]) -> anyhow::Result<()>
    {
        let entries = agents
            .iter()
            .map(|(name, reads_cross_client_skills, userprofile_skill_dir, skill_dir_override)| {
                let userprofile = userprofile_skill_dir.map(|dir| format!("    userprofile_skill_dir: '{dir}'\n")).unwrap_or_default();
                let skill = skill_dir_override.map(|d| format!("'{d}'")).unwrap_or_else(|| format!("'$workspace/.{name}/skills'"));
                format!(
                    "  - name: {name}\n    markers:\n      - .{name}\n    prompt_dir: '$workspace/.{name}/prompts'\n    skill_dir: {skill}\n{userprofile}    \
                     reads_cross_client_skills: {reads_cross_client_skills}\n"
                )
            })
            .collect::<Vec<_>>()
            .join("");
        fs::write(config_dir.join(AGENT_DEFAULTS_FILE), format!("version: 1\nagents:\n{entries}"))?;
        Ok(())
    }

    #[test]
    fn test_path_belongs_to_bogus()
    {
        let path = PathBuf::from("/home/user/project/.bogus/skills/my-skill/SKILL.md");
        assert!(TemplateManager::path_belongs_to_agent(&path, "bogus") == true);
    }

    #[test]
    fn test_path_belongs_to_fake()
    {
        let path = PathBuf::from("/home/user/project/.fake/skills/foo/SKILL.md");
        assert!(TemplateManager::path_belongs_to_agent(&path, "fake") == true);
    }

    #[test]
    fn test_path_does_not_belong_to_wrong_agent()
    {
        let path = PathBuf::from("/home/user/project/.bogus/skills/foo/SKILL.md");
        assert!(TemplateManager::path_belongs_to_agent(&path, "fake") == false);
    }

    #[test]
    fn test_path_no_agent_directory()
    {
        let path = PathBuf::from("/home/user/project/AGENTS.md");
        assert!(TemplateManager::path_belongs_to_agent(&path, "bogus") == false);
    }

    #[test]
    fn test_resolve_workspace_path_skips_instructions()
    {
        assert!(BillOfMaterials::resolve_workspace_path("$instructions").is_none() == true);
        assert!(BillOfMaterials::resolve_workspace_path("$instructions/rpp.md").is_none() == true);
    }

    #[test]
    fn test_resolve_workspace_path_skips_userprofile()
    {
        assert!(BillOfMaterials::resolve_workspace_path("$userprofile/.bogus/init.md").is_none() == true);
    }

    #[test]
    fn test_resolve_workspace_path_resolves_workspace()
    {
        let result = BillOfMaterials::resolve_workspace_path("$workspace/.rpp.toml");
        assert!(result.is_some() == true);
        assert_eq!(result.unwrap(), PathBuf::from("./.rpp.toml"));
    }

    #[test]
    fn test_remove_lang_unknown_no_error() -> anyhow::Result<()>
    {
        let _g = cwd_test_guard();

        let dir = tempfile::TempDir::new()?;
        let config_path = dir.path().join("templates.yml");
        let yaml = "languages:\n  Rust++:\n    files: []\n";
        fs::write(&config_path, yaml)?;

        let manager = TemplateManager { config_dir: dir.path().to_path_buf() };
        let result = manager.remove(None, Some("nonexistent"), false, true);
        assert!(result.is_ok() == true);
        Ok(())
    }

    #[test]
    fn test_remove_agent_falls_back_to_file_tracker() -> anyhow::Result<()>
    {
        let data_dir = tempfile::TempDir::new()?;
        let workspace = tempfile::TempDir::new()?;

        let yaml = "version: 5\nagents:\n  fake:\n    instructions: []\n";
        fs::write(data_dir.path().join("templates.yml"), yaml)?;
        write_synthetic_agent_defaults(data_dir.path(), &[("bogus", true, None, None), ("fake", true, None, None)])?;

        let agent_file = workspace.path().join(".bogus/instructions.md");
        fs::create_dir_all(agent_file.parent().ok_or_else(|| anyhow::anyhow!("missing parent"))?)?;
        fs::write(&agent_file, "test")?;

        let mut tracker = FileTracker::new(workspace.path())?;
        tracker.record_installation(&agent_file, "sha1".into(), 5, LANG_NONE.into(), "bogus".into(), "agent".into());
        tracker.save()?;

        let _g = cwd_test_guard();
        std::env::set_current_dir(workspace.path())?;

        let manager = TemplateManager { config_dir: data_dir.path().to_path_buf() };
        let result = manager.remove(Some("bogus"), None, false, true);

        assert!(result.is_ok() == true);
        Ok(())
    }

    #[test]
    fn test_remove_lang_falls_back_to_file_tracker() -> anyhow::Result<()>
    {
        let data_dir = tempfile::TempDir::new()?;
        let workspace = tempfile::TempDir::new()?;

        let yaml = "version: 5\nlanguages:\n  CppScript:\n    files: []\n";
        fs::write(data_dir.path().join("templates.yml"), yaml)?;

        let lang_file = workspace.path().join(".rpp.toml");
        fs::write(&lang_file, "max_width = 100")?;

        let mut tracker = FileTracker::new(workspace.path())?;
        tracker.record_installation(&lang_file, "sha1".into(), 5, "Rust++".into(), AGENT_ALL.into(), "language".into());
        tracker.save()?;

        let _g = cwd_test_guard();
        std::env::set_current_dir(workspace.path())?;

        let manager = TemplateManager { config_dir: data_dir.path().to_path_buf() };
        let result = manager.remove(None, Some("Rust++"), false, true);

        assert!(result.is_ok() == true);
        // Tracker must not report Rust++ after removal (dry-run, so tracker is unchanged — no invariant check here)
        Ok(())
    }

    #[test]
    fn test_remove_lang_fallback_removes_language_skills_but_keeps_main() -> anyhow::Result<()>
    {
        let data_dir = tempfile::TempDir::new()?;
        let workspace = tempfile::TempDir::new()?;

        let lang_file = workspace.path().join(".rpp.toml");
        let main_file = workspace.path().join("AGENTS.md");
        let skill_dir = workspace.path().join(".bogus/skills/rpp-conventions");
        fs::create_dir_all(&skill_dir)?;
        let skill_file = skill_dir.join("SKILL.md");
        fs::write(&lang_file, "max_width = 100")?;
        fs::write(&main_file, "# Agents")?;
        fs::write(&skill_file, "# Skill")?;

        let mut tracker = FileTracker::new(workspace.path())?;
        tracker.record_installation(&lang_file, "sha1".into(), 5, "Rust++".into(), AGENT_ALL.into(), "language".into());
        tracker.record_installation(&main_file, "sha2".into(), 5, "Rust++".into(), AGENT_ALL.into(), "main".into());
        tracker.record_installation(&skill_file, "sha3".into(), 5, "Rust++".into(), AGENT_ALL.into(), "skill".into());
        tracker.save()?;

        let _g = cwd_test_guard();
        std::env::set_current_dir(workspace.path())?;

        let manager = TemplateManager { config_dir: data_dir.path().to_path_buf() };
        let result = manager.remove(None, Some("Rust++"), true, false);

        assert!(result.is_ok() == true);
        assert!(lang_file.exists() == false);
        assert!(skill_file.exists() == false);
        assert!(main_file.exists() == true);
        // Tracker consistency invariant: language must no longer be reported as installed
        let tracker_after = FileTracker::new(&std::env::current_dir()?)?;
        assert!(tracker_after.get_installed_language().is_none() == true, "tracker must report no language after remove --lang");
        Ok(())
    }

    #[test]
    fn test_remove_lang_discovers_template_skill_files() -> anyhow::Result<()>
    {
        let data_dir = tempfile::TempDir::new()?;
        let workspace = tempfile::TempDir::new()?;

        let yaml = "version: 5\nlanguages:\n  Rust++:\n    skills:\n      - source: skills/rpp-skill\n";
        fs::write(data_dir.path().join("templates.yml"), yaml)?;

        let skill_dir = workspace.path().join(".agents/skills/rpp-skill");
        fs::create_dir_all(&skill_dir)?;
        let skill_file = skill_dir.join("SKILL.md");
        fs::write(&skill_file, "# Rust++ Coding Conventions")?;

        let _g = cwd_test_guard();
        std::env::set_current_dir(workspace.path())?;

        let manager = TemplateManager { config_dir: data_dir.path().to_path_buf() };
        let result = manager.remove(None, Some("Rust++"), true, false);

        assert!(result.is_ok() == true);
        assert!(skill_file.exists() == false);
        Ok(())
    }

    #[test]
    fn test_remove_agent_discovers_untracked_skill_files() -> anyhow::Result<()>
    {
        let data_dir = tempfile::TempDir::new()?;
        let workspace = tempfile::TempDir::new()?;

        let yaml = "version: 5\nagents:\n  bogus:\n    instructions:\n      - source: instructions.md\n        target: $workspace/.bogus/instructions.md\n";
        fs::write(data_dir.path().join("templates.yml"), yaml)?;
        write_synthetic_agent_defaults(data_dir.path(), &[("bogus", true, None, None)])?;

        let agent_file = workspace.path().join(".bogus/instructions.md");
        fs::create_dir_all(agent_file.parent().ok_or_else(|| anyhow::anyhow!("missing parent"))?)?;
        fs::write(&agent_file, "test")?;

        // Place a skill directory on disk without any FileTracker entry
        let skill_dir = workspace.path().join(".bogus/skills/my-skill");
        fs::create_dir_all(&skill_dir)?;
        let skill_file = skill_dir.join("SKILL.md");
        fs::write(&skill_file, "# My Skill")?;

        let _g = cwd_test_guard();
        std::env::set_current_dir(workspace.path())?;

        let manager = TemplateManager { config_dir: data_dir.path().to_path_buf() };
        let result = manager.remove(Some("bogus"), None, true, false);

        assert!(result.is_ok() == true);
        // Positive: untracked skill under the agent's skill dir must be deleted
        assert!(skill_file.exists() == false);
        // Negative: the agent file itself is also removed (it was in the skill dir's parent scope)
        // and the workspace root must not have been disturbed
        assert!(workspace.path().exists() == true, "workspace root must not be deleted");
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

        // An unrelated file outside the skill dir must not be touched
        let other_file = workspace.path().join("README.md");
        fs::write(&other_file, "# Project")?;

        let _g = cwd_test_guard();
        std::env::set_current_dir(workspace.path())?;

        let manager = TemplateManager { config_dir: data_dir.path().to_path_buf() };
        let result = manager.remove(None, None, true, false);

        assert!(result.is_ok() == true);
        // Positive: untracked skill must be deleted
        assert!(skill_file.exists() == false);
        // Negative: unrelated file must be untouched
        assert!(other_file.exists() == true, "files outside slopctl dirs must not be deleted");
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

        let yaml = "version: 5\nagents:\n  bogus:\n    instructions:\n      - source: instructions.md\n        target: $workspace/.bogus/instructions.md\n";
        fs::write(data_dir.path().join("templates.yml"), yaml)?;
        write_synthetic_agent_defaults(data_dir.path(), &[("bogus", true, None, None)])?;

        let agent_file = workspace.path().join(".bogus/instructions.md");
        fs::create_dir_all(agent_file.parent().ok_or_else(|| anyhow::anyhow!("missing parent"))?)?;
        fs::write(&agent_file, "test")?;

        let mut tracker = FileTracker::new(workspace.path())?;
        tracker.record_installation(&agent_file, "sha1".into(), 5, LANG_NONE.into(), "bogus".into(), "agent".into());
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

        // Track an agent file; purge should delete it without scanning userprofile skills.
        let agent_file = workspace.path().join(".bogus/instructions.md");
        fs::create_dir_all(agent_file.parent().ok_or_else(|| anyhow::anyhow!("missing parent"))?)?;
        fs::write(&agent_file, "Read AGENTS.md")?;

        let mut tracker = FileTracker::new(workspace.path())?;
        tracker.record_installation(&agent_file, "sha1".into(), 5, LANG_NONE.into(), "bogus".into(), "agent".into());
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
    fn test_remove_agent_skips_userprofile_skill_scan() -> anyhow::Result<()>
    {
        let data_dir = tempfile::TempDir::new()?;
        let workspace = tempfile::TempDir::new()?;

        write_synthetic_agent_defaults(data_dir.path(), &[("bogus", true, Some("$userprofile/.bogus/skills"), None)])?;

        // Track an agent file so remove --agent has something to find.
        let agent_file = workspace.path().join(".bogus/instructions.md");
        fs::create_dir_all(agent_file.parent().ok_or_else(|| anyhow::anyhow!("missing parent"))?)?;
        fs::write(&agent_file, "Read AGENTS.md")?;

        // Track the instruction file so remove has something to find.
        let mut tracker = FileTracker::new(workspace.path())?;
        tracker.record_installation(&agent_file, "sha1".into(), 5, LANG_NONE.into(), "bogus".into(), "agent".into());
        tracker.save()?;

        let _g = cwd_test_guard();
        std::env::set_current_dir(workspace.path())?;

        // Use dry-run to inspect what would be removed without side effects.
        // The key assertion is that this succeeds without attempting to scan
        // the userprofile-based skill directory.
        let manager = TemplateManager { config_dir: data_dir.path().to_path_buf() };
        let result = manager.remove(Some("bogus"), None, false, true);

        assert!(result.is_ok() == true);
        Ok(())
    }

    #[test]
    fn test_remove_cross_client_agent_preserves_cross_client_skills_when_other_agent_installed() -> anyhow::Result<()>
    {
        let data_dir = tempfile::TempDir::new()?;
        let workspace = tempfile::TempDir::new()?;

        write_synthetic_agent_defaults(data_dir.path(), &[("fake", true, None, None), ("foobar", true, None, Some("$workspace/.agents/skills"))])?;
        fs::create_dir_all(workspace.path().join(".fake"))?;
        fs::create_dir_all(workspace.path().join(".foobar"))?;

        // Place a cross-client skill
        let skill_dir = workspace.path().join(".agents/skills/git-workflow");
        fs::create_dir_all(&skill_dir)?;
        let skill_file = skill_dir.join("SKILL.md");
        fs::write(&skill_file, "# Git Workflow")?;

        let _g = cwd_test_guard();
        std::env::set_current_dir(workspace.path())?;

        let manager = TemplateManager { config_dir: data_dir.path().to_path_buf() };
        let result = manager.remove(Some("fake"), None, false, true);

        assert!(result.is_ok() == true);
        // The cross-client skill must be preserved because another agent still needs it.
        assert!(skill_file.exists() == true);
        Ok(())
    }

    #[test]
    fn test_remove_cross_client_agent_cleans_cross_client_skills_when_last_agent() -> anyhow::Result<()>
    {
        let data_dir = tempfile::TempDir::new()?;
        let workspace = tempfile::TempDir::new()?;

        write_synthetic_agent_defaults(data_dir.path(), &[("fake", true, None, None), ("foobar", true, None, Some("$workspace/.agents/skills"))])?;
        fs::create_dir_all(workspace.path().join(".fake"))?;

        // Agent/top-level skill (lang: none → must be deleted when last agent removed)
        let skill_dir = workspace.path().join(".agents/skills/git-workflow");
        fs::create_dir_all(&skill_dir)?;
        let skill_file = skill_dir.join("SKILL.md");
        fs::write(&skill_file, "# Git Workflow")?;

        // Language skill (lang: CppScript → must survive even when last agent is removed)
        let lang_skill_dir = workspace.path().join(".agents/skills/cpp-conventions");
        fs::create_dir_all(&lang_skill_dir)?;
        let lang_skill_file = lang_skill_dir.join("SKILL.md");
        fs::write(&lang_skill_file, "# CppScript Conventions")?;

        let mut tracker = FileTracker::new(workspace.path())?;
        tracker.record_installation(&skill_file, "sha1".into(), 5, LANG_NONE.into(), AGENT_ALL.into(), "skill".into());
        tracker.record_installation(&lang_skill_file, "sha2".into(), 5, "CppScript".into(), AGENT_ALL.into(), "skill".into());
        tracker.save()?;

        let _g = cwd_test_guard();
        std::env::set_current_dir(workspace.path())?;

        let manager = TemplateManager { config_dir: data_dir.path().to_path_buf() };
        let result = manager.remove(Some("fake"), None, true, false);

        assert!(result.is_ok() == true);
        // Agent/top-level skill must be deleted — it has no language owner
        assert!(skill_file.exists() == false, "agent-owned skill must be deleted when last cross-client agent is removed");
        // Language skill must survive — it belongs to CppScript, not to the agent
        assert!(lang_skill_file.exists() == true, "language skill must not be deleted when removing an agent");
        Ok(())
    }

    #[test]
    fn test_remove_agent_cleans_up_empty_marker_dir() -> anyhow::Result<()>
    {
        let data_dir = tempfile::TempDir::new()?;
        let workspace = tempfile::TempDir::new()?;

        write_synthetic_agent_defaults(data_dir.path(), &[("bogus", false, None, None)])?;

        let agent_file = workspace.path().join(".bogus/instructions.md");
        fs::create_dir_all(agent_file.parent().ok_or_else(|| anyhow::anyhow!("missing parent"))?)?;
        fs::write(&agent_file, "test")?;

        let mut tracker = FileTracker::new(workspace.path())?;
        tracker.record_installation(&agent_file, "sha1".into(), 5, LANG_NONE.into(), "bogus".into(), "agent".into());
        tracker.save()?;

        let _g = cwd_test_guard();
        std::env::set_current_dir(workspace.path())?;

        let manager = TemplateManager { config_dir: data_dir.path().to_path_buf() };
        let result = manager.remove(Some("bogus"), None, true, false);

        assert!(result.is_ok() == true);
        assert!(agent_file.exists() == false);
        assert!(workspace.path().join(".bogus").exists() == false, ".bogus/ should be removed when empty");
        Ok(())
    }

    #[test]
    fn test_remove_agent_keeps_nonempty_marker_dir() -> anyhow::Result<()>
    {
        let data_dir = tempfile::TempDir::new()?;
        let workspace = tempfile::TempDir::new()?;

        write_synthetic_agent_defaults(data_dir.path(), &[("bogus", false, None, None)])?;

        let agent_file = workspace.path().join(".bogus/instructions.md");
        let user_file = workspace.path().join(".bogus/user-notes.md");
        fs::create_dir_all(agent_file.parent().ok_or_else(|| anyhow::anyhow!("missing parent"))?)?;
        fs::write(&agent_file, "test")?;
        fs::write(&user_file, "my notes")?;

        let mut tracker = FileTracker::new(workspace.path())?;
        tracker.record_installation(&agent_file, "sha1".into(), 5, LANG_NONE.into(), "bogus".into(), "agent".into());
        tracker.save()?;

        let _g = cwd_test_guard();
        std::env::set_current_dir(workspace.path())?;

        let manager = TemplateManager { config_dir: data_dir.path().to_path_buf() };
        let result = manager.remove(Some("bogus"), None, true, false);

        assert!(result.is_ok() == true);
        assert!(agent_file.exists() == false);
        assert!(workspace.path().join(".bogus").exists() == true, ".bogus/ should be kept when non-empty");
        assert!(user_file.exists() == true, "user file in .bogus/ must not be deleted");
        Ok(())
    }

    // ── Lifecycle fixture ────────────────────────────────────────────────────

    /// Builds a workspace that mirrors what `slopctl init --agent bogus --lang CppScript` produces.
    ///
    /// Returns `(data_dir, workspace, agent_file, lang_file, agent_skill_file, lang_skill_file, agents_md)`.
    /// All returned TempDir values must be kept alive for the duration of the test.
    fn setup_workspace_with_agent_and_lang()
    -> anyhow::Result<(tempfile::TempDir, tempfile::TempDir, std::path::PathBuf, std::path::PathBuf, std::path::PathBuf, std::path::PathBuf, std::path::PathBuf)>
    {
        let data_dir = tempfile::TempDir::new()?;
        let workspace = tempfile::TempDir::new()?;

        write_synthetic_agent_defaults(data_dir.path(), &[("bogus", true, None, None)])?;

        let agents_md = workspace.path().join("AGENTS.md");
        let agent_file = workspace.path().join(".bogus/instructions.md");
        let lang_file = workspace.path().join(".editorconfig");
        let agent_skill_file = workspace.path().join(".agents/skills/git-workflow/SKILL.md");
        let lang_skill_file = workspace.path().join(".agents/skills/cpp-conventions/SKILL.md");

        fs::create_dir_all(agent_file.parent().ok_or_else(|| anyhow::anyhow!("no parent"))?)?;
        fs::create_dir_all(agent_skill_file.parent().ok_or_else(|| anyhow::anyhow!("no parent"))?)?;
        fs::create_dir_all(lang_skill_file.parent().ok_or_else(|| anyhow::anyhow!("no parent"))?)?;

        fs::write(&agents_md, "# Agents\n<!-- SLOPCTL-TEMPLATE -->\n")?;
        fs::write(&agent_file, "Read AGENTS.md")?;
        fs::write(&lang_file, "root = true")?;
        fs::write(&agent_skill_file, "# Git Workflow")?;
        fs::write(&lang_skill_file, "# CppScript Conventions")?;

        let mut tracker = FileTracker::new(workspace.path())?;
        tracker.record_installation(&agents_md, "sha0".into(), 5, "CppScript".into(), AGENT_ALL.into(), "main".into());
        tracker.record_installation(&agent_file, "sha1".into(), 5, LANG_NONE.into(), "bogus".into(), "agent".into());
        tracker.record_installation(&lang_file, "sha2".into(), 5, "CppScript".into(), AGENT_ALL.into(), "language".into());
        tracker.record_installation(&agent_skill_file, "sha3".into(), 5, LANG_NONE.into(), AGENT_ALL.into(), "skill".into());
        tracker.record_installation(&lang_skill_file, "sha4".into(), 5, "CppScript".into(), AGENT_ALL.into(), "skill".into());
        tracker.save()?;

        Ok((data_dir, workspace, agent_file, lang_file, agent_skill_file, lang_skill_file, agents_md))
    }

    // ── Regression: bug 1 ────────────────────────────────────────────────────

    #[test]
    fn test_remove_agent_preserves_language_skills_in_cross_client_dir() -> anyhow::Result<()>
    {
        let data_dir = tempfile::TempDir::new()?;
        let workspace = tempfile::TempDir::new()?;

        // fake is the only cross-client agent installed
        write_synthetic_agent_defaults(data_dir.path(), &[("fake", true, None, None)])?;
        fs::create_dir_all(workspace.path().join(".fake"))?;

        // Agent-specific top-level skill (lang: none → should be deleted)
        let agent_skill_dir = workspace.path().join(".agents/skills/git-workflow");
        fs::create_dir_all(&agent_skill_dir)?;
        let agent_skill_file = agent_skill_dir.join("SKILL.md");
        fs::write(&agent_skill_file, "# Git Workflow")?;

        // Language skill (lang: CppScript → must survive agent removal)
        let lang_skill_dir = workspace.path().join(".agents/skills/cpp-conventions");
        fs::create_dir_all(&lang_skill_dir)?;
        let lang_skill_file = lang_skill_dir.join("SKILL.md");
        fs::write(&lang_skill_file, "# CppScript Conventions")?;

        // Record both skills in tracker with the correct lang field
        let mut tracker = FileTracker::new(workspace.path())?;
        tracker.record_installation(&agent_skill_file, "sha1".into(), 5, LANG_NONE.into(), AGENT_ALL.into(), "skill".into());
        tracker.record_installation(&lang_skill_file, "sha2".into(), 5, "CppScript".into(), AGENT_ALL.into(), "skill".into());
        tracker.save()?;

        let _g = cwd_test_guard();
        std::env::set_current_dir(workspace.path())?;

        let manager = TemplateManager { config_dir: data_dir.path().to_path_buf() };
        let result = manager.remove(Some("fake"), None, true, false);

        assert!(result.is_ok() == true);
        // Agent/top-level skill must be gone — it belongs to no language
        assert!(agent_skill_file.exists() == false, "agent-owned skill must be deleted");
        // Language skill must survive — it belongs to CppScript, not to the agent
        assert!(lang_skill_file.exists() == true, "language skill must not be deleted by remove --agent");
        Ok(())
    }

    // ── Regression: bug 2 ────────────────────────────────────────────────────

    #[test]
    fn test_remove_lang_clears_installed_language_from_tracker() -> anyhow::Result<()>
    {
        let data_dir = tempfile::TempDir::new()?;
        let workspace = tempfile::TempDir::new()?;

        // AGENTS.md stays on disk but carries lang: "CppScript" in the tracker
        let agents_md = workspace.path().join("AGENTS.md");
        let lang_file = workspace.path().join(".editorconfig");
        fs::write(&agents_md, "# Agents\n<!-- SLOPCTL-TEMPLATE -->\n")?;
        fs::write(&lang_file, "root = true\n")?;

        let mut tracker = FileTracker::new(workspace.path())?;
        tracker.record_installation(&agents_md, "sha1".into(), 5, "CppScript".into(), AGENT_ALL.into(), "main".into());
        tracker.record_installation(&lang_file, "sha2".into(), 5, "CppScript".into(), AGENT_ALL.into(), "language".into());
        tracker.save()?;

        let _g = cwd_test_guard();
        std::env::set_current_dir(workspace.path())?;

        let manager = TemplateManager { config_dir: data_dir.path().to_path_buf() };
        // remove --lang CppScript: lang_file must be deleted, AGENTS.md must remain
        let result = manager.remove(None, Some("CppScript"), true, false);

        assert!(result.is_ok() == true);
        assert!(lang_file.exists() == false, "language file must be deleted");
        assert!(agents_md.exists() == true, "AGENTS.md must not be deleted by remove --lang");

        // The critical invariant: status must no longer report CppScript
        let tracker_after = FileTracker::new(&std::env::current_dir()?)?;
        assert!(tracker_after.get_installed_language().is_none() == true, "status must report no language after remove --lang");
        Ok(())
    }

    // ── Regression: adopted native-agent skill copies inherit lang ───────────

    #[test]
    fn test_remove_lang_removes_adopted_native_agent_skill_copies() -> anyhow::Result<()>
    {
        // Scenario: init --agent codex --lang swift → init --agent claude → remove --lang swift
        //
        // After `init --agent claude` the cross-client skills in .agents/skills/ are
        // adopted to .claude/skills/ (native-only agent). The adoption previously stamped
        // every copy with lang: LANG_NONE, so `remove --lang swift` could not find them.
        // After the fix, adopted copies carry the original lang and are properly removed.

        let data_dir = tempfile::TempDir::new()?;
        let workspace = tempfile::TempDir::new()?;

        write_synthetic_agent_defaults(data_dir.path(), &[("fake", false, None, None)])?;

        // Skill in cross-client dir (tracked with lang: CppScript — installed by a cross-client agent)
        let cc_skill_dir = workspace.path().join(".agents/skills/cppscript-conventions");
        fs::create_dir_all(&cc_skill_dir)?;
        let cc_skill_file = cc_skill_dir.join("SKILL.md");
        fs::write(&cc_skill_file, "# CppScript Conventions")?;

        // Same skill adopted to native agent dir (tracked with lang: CppScript after fix)
        let native_skill_dir = workspace.path().join(".fake/skills/cppscript-conventions");
        fs::create_dir_all(&native_skill_dir)?;
        let native_skill_file = native_skill_dir.join("SKILL.md");
        fs::write(&native_skill_file, "# CppScript Conventions")?;

        let mut tracker = FileTracker::new(workspace.path())?;
        tracker.record_installation(&cc_skill_file, "sha1".into(), 5, "CppScript".into(), AGENT_ALL.into(), "skill".into());
        tracker.record_installation(&native_skill_file, "sha2".into(), 5, "CppScript".into(), AGENT_ALL.into(), "skill".into());
        tracker.save()?;

        let _g = cwd_test_guard();
        std::env::set_current_dir(workspace.path())?;

        let manager = TemplateManager { config_dir: data_dir.path().to_path_buf() };
        let result = manager.remove(None, Some("CppScript"), true, false);

        assert!(result.is_ok() == true);
        assert!(cc_skill_file.exists() == false, "cross-client lang skill must be removed");
        assert!(native_skill_file.exists() == false, "adopted native-agent copy must also be removed");

        let tracker_after = FileTracker::new(&std::env::current_dir()?)?;
        assert!(tracker_after.get_installed_language().is_none() == true, "tracker must report no language after remove --lang");
        Ok(())
    }

    // ── Lifecycle tests ──────────────────────────────────────────────────────

    #[test]
    fn test_remove_agent_does_not_disturb_lang_artifacts() -> anyhow::Result<()>
    {
        let (data_dir, workspace, agent_file, lang_file, agent_skill_file, lang_skill_file, agents_md) = setup_workspace_with_agent_and_lang()?;

        let _g = cwd_test_guard();
        std::env::set_current_dir(workspace.path())?;

        let manager = TemplateManager { config_dir: data_dir.path().to_path_buf() };
        let result = manager.remove(Some("bogus"), None, true, false);

        assert!(result.is_ok() == true);

        // Agent artifacts must be gone
        assert!(agent_file.exists() == false, "agent instruction file must be deleted");
        assert!(agent_skill_file.exists() == false, "agent/top-level skill must be deleted");

        // Language artifacts must survive
        assert!(lang_file.exists() == true, "language file must not be deleted by remove --agent");
        assert!(lang_skill_file.exists() == true, "language skill must not be deleted by remove --agent");
        assert!(agents_md.exists() == true, "AGENTS.md must not be deleted by remove --agent");

        // Tracker must still report CppScript as installed
        let tracker_after = FileTracker::new(&std::env::current_dir()?)?;
        assert_eq!(tracker_after.get_installed_language(), Some("CppScript".to_string()), "tracker must still report language after agent removal");
        Ok(())
    }

    #[test]
    fn test_remove_lang_then_status_reports_no_lang() -> anyhow::Result<()>
    {
        let (data_dir, workspace, agent_file, lang_file, _agent_skill_file, lang_skill_file, agents_md) = setup_workspace_with_agent_and_lang()?;

        let _g = cwd_test_guard();
        std::env::set_current_dir(workspace.path())?;

        let manager = TemplateManager { config_dir: data_dir.path().to_path_buf() };
        let result = manager.remove(None, Some("CppScript"), true, false);

        assert!(result.is_ok() == true);

        // Language artifacts must be gone
        assert!(lang_file.exists() == false, "language file must be deleted");
        assert!(lang_skill_file.exists() == false, "language skill must be deleted");

        // Agent and shared artifacts must survive
        assert!(agent_file.exists() == true, "agent file must not be deleted by remove --lang");
        assert!(agents_md.exists() == true, "AGENTS.md must not be deleted by remove --lang");

        // Tracker must no longer report any language
        let tracker_after = FileTracker::new(&std::env::current_dir()?)?;
        assert!(tracker_after.get_installed_language().is_none() == true, "status must report no language after remove --lang");
        Ok(())
    }
}
