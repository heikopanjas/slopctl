//! Template remove command

use std::path::PathBuf;

use owo_colors::OwoColorize;

use super::TemplateManager;
use crate::{
    Result, agent_defaults, bom,
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
    /// Returns an error if templates.yml cannot be loaded, agent/language is not found,
    /// or file deletion fails
    pub fn remove(&self, agent: Option<&str>, lang: Option<&str>, skills: &[String], force: bool, dry_run: bool) -> Result<()>
    {
        let current_dir = std::env::current_dir()?;
        let config_file = self.config_dir.join("templates.yml");
        let has_agent_target = agent.is_some();
        let has_lang_target = lang.is_some();
        let has_skill_target = skills.is_empty() == false;
        let remove_all = agent.is_none() && lang.is_none() && has_skill_target == false;

        let mut files_to_remove: Vec<PathBuf> = Vec::new();
        let mut stale_tracker_paths: Vec<PathBuf> = Vec::new();
        let mut description_parts: Vec<String> = Vec::new();

        // Collect agent files from BoM when agent or --all is requested
        if has_agent_target == true || remove_all == true
        {
            require!(config_file.exists() == true, Err(anyhow::anyhow!("Global templates not found. Run 'vibe-cop install' first to set up templates.")));

            let bom = BillOfMaterials::from_config(&config_file)?;

            if let Some(agent_name) = agent
            {
                if bom.has_agent(agent_name) == false
                {
                    let available_agents = bom.get_agent_names();
                    return Err(anyhow::anyhow!("Agent '{}' not found in Bill of Materials.\nAvailable agents: {}", agent_name, available_agents.join(", ")));
                }

                if let Some(agent_files) = bom.get_agent_files(agent_name)
                {
                    files_to_remove.extend(agent_files.iter().filter(|f| f.exists()).cloned());
                }

                // Also collect ad-hoc/top-level skill files under this agent's skill dir
                let file_tracker = FileTracker::new(&self.config_dir)?;
                let skill_entries = file_tracker.get_workspace_entries_by_category(&current_dir, "skill");
                for (path, _) in skill_entries
                {
                    if path.exists() == true && Self::path_belongs_to_agent(&path, agent_name) == true
                    {
                        files_to_remove.push(path);
                    }
                }

                description_parts.push(format!("agent '{}'", agent_name.yellow()));
            }
            else
            {
                // --all: collect files for every agent
                let agent_names = bom.get_agent_names();
                for name in &agent_names
                {
                    if let Some(agent_files) = bom.get_agent_files(name)
                    {
                        files_to_remove.extend(agent_files.iter().filter(|f| f.exists()).cloned());
                    }
                }

                // Also collect ALL skill files from FileTracker
                let file_tracker = FileTracker::new(&self.config_dir)?;
                let skill_entries = file_tracker.get_workspace_entries_by_category(&current_dir, "skill");
                for (path, _) in skill_entries
                {
                    if path.exists() == true
                    {
                        files_to_remove.push(path);
                    }
                }

                description_parts.push("all agents and skills".to_string());
            }
        }

        // Collect language disk files when --lang is requested
        if has_lang_target == true
        {
            require!(config_file.exists() == true, Err(anyhow::anyhow!("Global templates not found. Run 'vibe-cop install' first to set up templates.")));

            let lang_name = lang.unwrap();
            let config = template_engine::load_template_config(&self.config_dir)?;

            if config.languages.contains_key(lang_name) == false
            {
                let mut available: Vec<&String> = config.languages.keys().collect();
                available.sort();
                return Err(anyhow::anyhow!(
                    "Language '{}' not found in templates.yml.\nAvailable languages: {}",
                    lang_name,
                    available.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
                ));
            }

            let file_mappings = bom::resolve_language_files(lang_name, &config)?;
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

            description_parts.push(format!("language '{}'", lang_name.yellow()));
        }

        // Collect skill files by name from all agent skill dirs and cross-client dir
        if has_skill_target == true
        {
            let userprofile = dirs::home_dir().ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "Could not determine home directory"))?;
            let skill_search_dirs = agent_defaults::get_all_skill_search_dirs(&current_dir, &userprofile);

            let file_tracker = FileTracker::new(&self.config_dir)?;
            let skill_entries = file_tracker.get_workspace_entries_by_category(&current_dir, "skill");

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
                for (path, _) in &skill_entries
                {
                    if Self::extract_skill_name_from_path(path).as_deref() == Some(skill_name.as_str())
                    {
                        if path.exists() == true && files_to_remove.contains(path) == false
                        {
                            files_to_remove.push(path.clone());
                            found = true;
                        }
                        else if path.exists() == false && stale_tracker_paths.contains(path) == false
                        {
                            stale_tracker_paths.push(path.clone());
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
                let mut file_tracker = FileTracker::new(&self.config_dir)?;
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

        let mut file_tracker = FileTracker::new(&self.config_dir)?;

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
    use crate::bom::BillOfMaterials;

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
    fn test_remove_lang_unknown_errors() -> anyhow::Result<()>
    {
        let dir = tempfile::TempDir::new()?;
        let config_path = dir.path().join("templates.yml");
        let yaml = "languages:\n  rust:\n    files: []\n";
        fs::write(&config_path, yaml)?;

        let manager = TemplateManager { config_dir: dir.path().to_path_buf() };
        let result = manager.remove(None, Some("nonexistent"), &[], false, true);
        assert!(result.is_err() == true);
        assert!(result.unwrap_err().to_string().contains("not found in templates.yml") == true);
        Ok(())
    }
}
