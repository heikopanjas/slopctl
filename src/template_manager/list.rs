//! Template list command

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf}
};

use owo_colors::OwoColorize;

use super::TemplateManager;
use crate::{
    Result, agent_defaults,
    bom::{self, BillOfMaterials},
    file_tracker::FileTracker,
    template_engine
};

impl TemplateManager
{
    fn installed_agent_names(&self, workspace: &std::path::Path) -> Result<Vec<String>>
    {
        let catalog = agent_defaults::load_agent_catalog_from_dir(&self.config_dir)?;
        Ok(agent_defaults::detect_all_installed_agents_from_catalog(&catalog, workspace))
    }

    /// Returns installed skill names from existing FileTracker entries.
    fn tracked_skill_names(file_tracker: &FileTracker, workspace: &Path) -> BTreeSet<String>
    {
        file_tracker
            .get_entries_by_category("skill")
            .into_iter()
            .filter_map(|(path, _)| {
                let absolute = workspace.join(&path);
                if absolute.exists() == true
                {
                    Self::extract_skill_name_from_path(&path)
                }
                else
                {
                    None
                }
            })
            .collect()
    }

    /// Collects the deduplicated set of slopctl-managed workspace files
    ///
    /// Merges candidates from the Bill of Materials, the FileTracker, and the main
    /// AGENTS.md path. Every candidate is canonicalized before deduplication so the
    /// same file collected from different sources (BoM `./x`, tracker `x`, absolute
    /// AGENTS.md) collapses to a single entry rather than surviving as distinct
    /// spellings. Only files that exist on disk are included; the result is sorted.
    fn collect_managed_files(current_dir: &Path, config_file: &Path, file_tracker: &FileTracker, agents_md_path: &Path) -> Vec<PathBuf>
    {
        let mut managed_files: Vec<PathBuf> = Vec::new();

        if config_file.exists() == true &&
            let Ok(bom) = BillOfMaterials::from_config(config_file)
        {
            for agent_name in bom.get_agent_names()
            {
                if let Some(files) = bom.get_agent_files(&agent_name)
                {
                    managed_files.extend(files.iter().filter(|f| f.exists()).cloned());
                }
            }
        }

        for (path, _) in file_tracker.get_entries()
        {
            if path.exists() == true
            {
                managed_files.push(path);
            }
        }

        if agents_md_path.exists() == true
        {
            managed_files.push(agents_md_path.to_path_buf());
        }

        let mut normalized: Vec<PathBuf> = managed_files.iter().map(|file| template_engine::normalize_path(&current_dir.join(file))).collect();
        normalized.sort();
        normalized.dedup();
        normalized
    }

    /// Show workspace status
    ///
    /// Displays the current state of slopctl in the project:
    /// - Global template status (downloaded, location)
    /// - AGENTS.md status (exists, customized)
    /// - Installed agents (detected by checking for their files)
    /// - Installed skills (from FileTracker ownership records)
    /// - All slopctl managed files in current directory (verbose only)
    ///
    /// # Arguments
    ///
    /// * `verbose` - When true, prints the full list of managed files
    ///
    /// # Errors
    ///
    /// Returns an error if the current directory cannot be determined or templates.yml cannot be loaded
    pub fn status(&self, verbose: bool) -> Result<()>
    {
        self.list_workspace(verbose)
    }

    /// Show workspace state (default mode)
    fn list_workspace(&self, verbose: bool) -> Result<()>
    {
        let current_dir = std::env::current_dir()?;
        let _ = self.try_migrate_tracker(&current_dir);

        println!("{}", "slopctl status".bold());
        println!();

        // Global templates status
        println!("{}", "Global Templates:".bold());
        if self.has_global_templates() == true
        {
            println!("  {} Installed at: {}", "✓".green(), self.config_dir.display().to_string().yellow());

            if let Ok(config) = template_engine::load_template_config(&self.config_dir)
            {
                println!("  {} Template version: {}", "→".blue(), config.version.to_string().green());

                if config.agents.is_empty() == false
                {
                    let mut agents: Vec<&String> = config.agents.keys().collect();
                    agents.sort();
                    println!("  {} Available agents: {}", "→".blue(), agents.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ").green());
                }

                let mut languages: Vec<&String> = config.languages.keys().collect();
                languages.sort();
                if languages.is_empty() == false
                {
                    println!("  {} Available languages: {}", "→".blue(), languages.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ").green());
                }
            }
        }
        else
        {
            println!("  {} Not installed", "✗".red());
            println!("  {} Run 'slopctl templates --update' to download templates", "→".blue());
        }

        println!();

        // AGENTS.md status
        println!("{}", "Project Status:".bold());
        let agents_md_path = current_dir.join("AGENTS.md");
        if agents_md_path.exists() == true
        {
            let customized = template_engine::is_file_customized(&agents_md_path).unwrap_or(false);
            if customized == true
            {
                println!("  {} AGENTS.md: {} (customized)", "✓".green(), "exists".green());
            }
            else
            {
                println!("  {} AGENTS.md: {} (from template)", "✓".green(), "exists".yellow());
            }
        }
        else
        {
            println!("  {} AGENTS.md: {}", "○".yellow(), "not found".yellow());
        }

        let file_tracker = FileTracker::new(&current_dir)?;

        // Detect installed agents via AgentDefaults markers. Some agents install
        // only marker directories and skills, so BoM file checks are insufficient.
        let installed_agents = self.installed_agent_names(&current_dir)?;
        let config_file = self.config_dir.join("templates.yml");

        if installed_agents.is_empty() == false
        {
            println!("  {} Installed agents: {}", "✓".green(), installed_agents.join(", ").green());
        }
        else
        {
            println!("  {} No agents installed", "○".yellow());
        }

        // Installed languages (from FileTracker ownership metadata)
        let installed_languages = file_tracker.get_installed_languages();
        if installed_languages.is_empty() == false
        {
            println!("  {} Installed languages: {}", "✓".green(), installed_languages.join(", ").green());
        }
        else
        {
            println!("  {} No languages installed", "○".yellow());
        }

        let skill_names = Self::tracked_skill_names(&file_tracker, &current_dir);

        if skill_names.is_empty() == false
        {
            println!("  {} Installed skills: {}", "✓".green(), skill_names.len().to_string().green());
            for name in &skill_names
            {
                println!("    {} {}", "•".blue(), name.yellow());
            }
        }
        else
        {
            println!("  {} No skills installed", "○".yellow());
        }

        if verbose == true
        {
            let managed_files = Self::collect_managed_files(&current_dir, &config_file, &file_tracker, &agents_md_path);

            println!();

            let canonical_dir = fs::canonicalize(&current_dir).unwrap_or_else(|_| current_dir.clone());

            if managed_files.is_empty() == false
            {
                println!("{}", "Managed Files:".bold());
                for file in &managed_files
                {
                    let display_path = file.strip_prefix(&canonical_dir).unwrap_or(file);
                    println!("  • {}", display_path.display().to_string().yellow());
                }
            }
            else
            {
                println!("{}", "Managed Files:".bold());
                println!("  {} No slopctl files found in current directory", "○".yellow());
                println!("  {} Run 'slopctl init --lang <lang> --agent <agent>' to set up", "→".blue());
            }
        }

        Ok(())
    }

    /// Show available templates catalog
    ///
    /// Shows the available template catalog:
    /// - Available agents with install status and skill counts
    /// - Available languages with includes, resolved skill names
    /// - Top-level skills from templates.yml
    /// - Ad-hoc installed skills from FileTracker
    ///
    /// # Errors
    ///
    /// Returns an error if global templates are not installed or templates.yml cannot be loaded
    pub fn list_global(&self) -> Result<()>
    {
        println!("{}", "slopctl templates --list".bold());
        println!();

        if self.has_global_templates() == false
        {
            println!("{} Global templates not installed", "✗".red());
            println!("{} Run 'slopctl templates --update' to download templates", "→".blue());
            return Ok(());
        }

        let config = template_engine::load_template_config(&self.config_dir)?;

        println!("{}", "Available Agents:".bold());
        if config.agents.is_empty() == true
        {
            println!("  {} agents.md standard - no agent-specific files", "→".blue());
            println!("  {} Single AGENTS.md works with all agents", "→".blue());
        }
        else
        {
            let mut agents: Vec<&String> = config.agents.keys().collect();
            agents.sort();
            let installed_agents: BTreeSet<String> = self.installed_agent_names(&std::env::current_dir()?)?.into_iter().collect();

            for agent_name in agents
            {
                let is_installed = installed_agents.contains(agent_name.as_str());

                let skill_count = config.agents.get(agent_name).map_or(0, |c| c.skills.len());

                let skill_info = if skill_count > 0
                {
                    format!(", {} skill(s)", skill_count)
                }
                else
                {
                    String::new()
                };

                if is_installed == true
                {
                    println!("  {} {} (installed{})", "✓".green(), agent_name.green(), skill_info);
                }
                else if skill_count > 0
                {
                    println!("  {} {} ({} skill(s))", "○".blue(), agent_name, skill_count);
                }
                else
                {
                    println!("  {} {}", "○".blue(), agent_name);
                }
            }
        }
        println!();

        println!("{}", "Available Languages:".bold());
        let mut languages: Vec<&String> = config.languages.keys().collect();
        languages.sort();

        for lang_name in languages
        {
            let lang_config = config.languages.get(lang_name.as_str());
            let includes_annotation = lang_config.map(|lc| &lc.includes).filter(|inc| inc.is_empty() == false).map(|inc| format!("includes: {}", inc.join(", ")));

            let resolved_skills = bom::resolve_language_skills(lang_name, &config).unwrap_or_default();
            let skill_annotation = if resolved_skills.is_empty() == false
            {
                Some(format!("{} skill(s)", resolved_skills.len()))
            }
            else
            {
                None
            };

            let annotations: Vec<String> = [includes_annotation, skill_annotation].into_iter().flatten().collect();

            if annotations.is_empty() == true
            {
                println!("  • {}", lang_name);
            }
            else
            {
                println!("  • {} ({})", lang_name, annotations.join(", ").dimmed());
            }

            for skill in &resolved_skills
            {
                let source_info = if crate::github::is_url(&skill.source) == true
                {
                    "(GitHub)"
                }
                else
                {
                    "(local)"
                };
                println!("    {} {} {}", "•".blue(), skill.derive_name(), source_info.dimmed());
            }
        }

        // Collect template-defined skill names for deduplication against installed
        // skills that came from older templates or previous slopctl versions.
        let mut template_skill_names: BTreeSet<String> = BTreeSet::new();

        if config.skills.is_empty() == false
        {
            println!();
            println!("{}", "Available Skills:".bold());
            for skill in &config.skills
            {
                template_skill_names.insert(skill.derive_name().to_string());
                let source_info = if crate::github::is_url(&skill.source) == true
                {
                    "(GitHub)"
                }
                else
                {
                    "(local)"
                };
                println!("  • {} {}", skill.derive_name(), source_info.dimmed());
            }
        }

        // Show installed skills not in the current template config.
        let current_dir = std::env::current_dir().ok();
        if let Some(ref cwd) = current_dir
        {
            let file_tracker = FileTracker::new(cwd)?;
            let skill_entries = file_tracker.get_entries_by_category("skill");

            let mut external_names: BTreeSet<String> = BTreeSet::new();
            for (path, _) in &skill_entries
            {
                if let Some(name) = Self::extract_skill_name_from_path(path) &&
                    template_skill_names.contains(&name) == false
                {
                    external_names.insert(name);
                }
            }

            if external_names.is_empty() == false
            {
                if template_skill_names.is_empty() == true
                {
                    println!();
                    println!("{}", "Installed Skills:".bold());
                }
                for name in &external_names
                {
                    println!("  • {} {}", name, "(installed)".dimmed());
                }
            }
        }

        println!();
        println!("{} Use 'slopctl init --lang <lang> --agent <agent>' to install", "→".blue());

        Ok(())
    }
}

#[cfg(test)]
mod tests
{
    use std::path::PathBuf;

    use super::TemplateManager;
    use crate::file_tracker::{AGENT_ALL, FileTracker, LANG_NONE};

    #[test]
    fn test_collect_managed_files_dedups_across_sources() -> anyhow::Result<()>
    {
        let workspace = tempfile::TempDir::new()?;
        let config_dir = tempfile::TempDir::new()?;

        // templates.yml whose agent BoM yields a './'-prefixed workspace path.
        std::fs::write(
            config_dir.path().join("templates.yml"),
            "version: 5\nmain:\n  source: AGENTS.md\n  target: '$workspace/AGENTS.md'\nagents:\n  bogus:\n    prompts:\n      - source: bogus/init.md\n        \
             target: '$workspace/.bogus/commands/init.md'\nlanguages: {}\n"
        )?;

        // AGENTS.md and the agent file exist on disk and are tracked (relative keys).
        let agents_md = workspace.path().join("AGENTS.md");
        std::fs::write(&agents_md, "# Project\n")?;
        let agent_file = workspace.path().join(".bogus/commands/init.md");
        std::fs::create_dir_all(agent_file.parent().ok_or_else(|| anyhow::anyhow!("missing parent"))?)?;
        std::fs::write(&agent_file, "# init\n")?;

        let mut tracker = FileTracker::new(workspace.path())?;
        tracker.record_installation(&agents_md, "sha1".into(), 5, LANG_NONE.into(), AGENT_ALL.into(), "main".into());
        tracker.record_installation(&agent_file, "sha2".into(), 5, LANG_NONE.into(), "bogus".into(), "agent".into());

        // agents_md_path is passed as an absolute path, matching list_workspace.
        let collected = TemplateManager::collect_managed_files(workspace.path(), &config_dir.path().join("templates.yml"), &tracker, &agents_md);

        let canonical_dir = std::fs::canonicalize(workspace.path())?;
        let relative: Vec<PathBuf> = collected.iter().map(|p| p.strip_prefix(&canonical_dir).unwrap_or(p).to_path_buf()).collect();

        let agents_md_count = relative.iter().filter(|p| p.as_os_str() == "AGENTS.md").count();
        assert_eq!(agents_md_count, 1, "AGENTS.md must appear exactly once, got {:?}", relative);

        let mut deduped = relative.clone();
        deduped.sort();
        deduped.dedup();
        assert_eq!(deduped.len(), relative.len(), "managed files must contain no duplicate spellings: {:?}", relative);

        Ok(())
    }

    #[test]
    fn test_tracked_skill_names_ignores_untracked_empty_directories() -> anyhow::Result<()>
    {
        let workspace = tempfile::TempDir::new()?;
        let empty_skill = workspace.path().join(".agents/skills/fake-stale-skill");
        let tracked_skill = workspace.path().join(".agents/skills/fake-active-skill/SKILL.md");
        std::fs::create_dir_all(&empty_skill)?;
        std::fs::create_dir_all(tracked_skill.parent().ok_or_else(|| anyhow::anyhow!("missing skill parent"))?)?;
        std::fs::write(&tracked_skill, "# Fake Active Skill")?;

        let mut tracker = FileTracker::new(workspace.path())?;
        tracker.record_installation(&tracked_skill, "sha1".into(), 5, "Rust++".into(), AGENT_ALL.into(), "skill".into());

        let names = TemplateManager::tracked_skill_names(&tracker, workspace.path());

        assert_eq!(names, ["fake-active-skill".to_string()].into_iter().collect());
        Ok(())
    }

    #[test]
    fn test_installed_agent_names_detects_marker_only_agent() -> anyhow::Result<()>
    {
        let workspace = tempfile::TempDir::new()?;
        let config_dir = tempfile::TempDir::new()?;
        std::fs::create_dir(workspace.path().join(".bogus"))?;
        std::fs::write(
            config_dir.path().join(crate::agent_defaults::AGENT_DEFAULTS_FILE),
            "version: 1\nagents:\n  - name: bogus\n    markers:\n      - .bogus\n    prompt_dir: '$workspace/.bogus/prompts'\n    skill_dir: \
             '$workspace/.bogus/skills'\n    reads_cross_client_skills: false\n"
        )?;

        let manager = TemplateManager { config_dir: config_dir.path().to_path_buf() };
        let agents = manager.installed_agent_names(workspace.path())?;

        assert_eq!(agents, vec!["bogus".to_string()]);
        Ok(())
    }

    #[test]
    fn test_installed_agent_names_detects_bogus_from_defaults() -> anyhow::Result<()>
    {
        let workspace = tempfile::TempDir::new()?;
        let config_dir = tempfile::TempDir::new()?;
        std::fs::create_dir(workspace.path().join(".bogus"))?;
        std::fs::write(
            config_dir.path().join(crate::agent_defaults::AGENT_DEFAULTS_FILE),
            "version: 1\nagents:\n  - name: bogus\n    markers:\n      - .bogus\n    prompt_dir: '$workspace/.bogus/prompts'\n    skill_dir: \
             '$workspace/.bogus/skills'\n    reads_cross_client_skills: false\n"
        )?;

        let manager = TemplateManager { config_dir: config_dir.path().to_path_buf() };
        let agents = manager.installed_agent_names(workspace.path())?;

        assert_eq!(agents, vec!["bogus".to_string()]);
        Ok(())
    }
}
