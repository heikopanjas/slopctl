//! Template list command

use std::{collections::BTreeSet, fs, path::PathBuf};

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
    /// Show workspace status
    ///
    /// Displays the current state of slopctl in the project:
    /// - Global template status (downloaded, location)
    /// - AGENTS.md status (exists, customized)
    /// - Installed agents (detected by checking for their files)
    /// - Installed skills (filesystem scan of agent skill dirs + FileTracker fallback)
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

        let file_tracker = FileTracker::new(&self.config_dir)?;

        // Detect installed agents via BoM
        let mut installed_agents: Vec<String> = Vec::new();

        let config_file = self.config_dir.join("templates.yml");
        if config_file.exists() == true &&
            let Ok(bom) = BillOfMaterials::from_config(&config_file)
        {
            for agent_name in bom.get_agent_names()
            {
                if let Some(files) = bom.get_agent_files(&agent_name) &&
                    files.iter().any(|f| f.exists()) == true
                {
                    installed_agents.push(agent_name.clone());
                }
            }
        }

        if installed_agents.is_empty() == false
        {
            println!("  {} Installed agents: {}", "✓".green(), installed_agents.join(", ").green());
        }
        else
        {
            println!("  {} No agents installed", "○".yellow());
        }

        // Installed language (from FileTracker metadata)
        if let Some(lang) = file_tracker.get_installed_language_for_workspace(&current_dir)
        {
            println!("  {} Installed language: {}", "✓".green(), lang.green());
        }
        else
        {
            println!("  {} No language installed", "○".yellow());
        }

        // Detect installed skills by scanning workspace-scoped agent skill directories on disk,
        // then merge in FileTracker entries for skills outside standard directories.
        // Userprofile-based dirs (e.g. codex) are excluded from scanning; FileTracker below
        // still picks up any userprofile skills that slopctl installed.
        let userprofile = dirs::home_dir().unwrap_or_default();
        let skill_search_dirs = agent_defaults::get_workspace_skill_search_dirs(&current_dir, &userprofile);

        let mut skill_names: BTreeSet<String> = BTreeSet::new();

        for dir in &skill_search_dirs
        {
            if dir.exists() == true &&
                let Ok(entries) = fs::read_dir(dir)
            {
                for entry in entries.flatten()
                {
                    if entry.path().is_dir() == true &&
                        let Some(name) = entry.file_name().to_str()
                    {
                        skill_names.insert(name.to_string());
                    }
                }
            }
        }

        let skill_entries = file_tracker.get_workspace_entries_by_category(&current_dir, "skill");
        for (path, _) in &skill_entries
        {
            if path.exists() == true &&
                let Some(name) = Self::extract_skill_name_from_path(path)
            {
                skill_names.insert(name);
            }
        }

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
            let mut managed_files: Vec<PathBuf> = Vec::new();

            if config_file.exists() == true &&
                let Ok(bom) = BillOfMaterials::from_config(&config_file)
            {
                for agent_name in bom.get_agent_names()
                {
                    if let Some(files) = bom.get_agent_files(&agent_name)
                    {
                        managed_files.extend(files.iter().filter(|f| f.exists()).cloned());
                    }
                }
            }

            let all_tracked = file_tracker.get_workspace_entries(&current_dir);
            for (path, _) in all_tracked
            {
                if path.exists() == true
                {
                    managed_files.push(path);
                }
            }

            if agents_md_path.exists() == true
            {
                managed_files.push(agents_md_path);
            }

            println!();

            managed_files.sort();
            managed_files.dedup();

            if managed_files.is_empty() == false
            {
                println!("{}", "Managed Files:".bold());
                for file in &managed_files
                {
                    let display_path = file.strip_prefix(&current_dir).unwrap_or(file);
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

        let config_path = self.config_dir.join("templates.yml");
        let bom = BillOfMaterials::from_config(&config_path)?;

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

            for agent_name in agents
            {
                let is_installed = bom.get_agent_files(agent_name).is_some_and(|files| files.iter().any(|f| f.exists()));

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
                println!("    {} {} {}", "•".blue(), skill.name, source_info.dimmed());
            }
        }

        // Collect template-defined skill names for deduplication against ad-hoc
        let mut template_skill_names: BTreeSet<String> = BTreeSet::new();

        if config.skills.is_empty() == false
        {
            println!();
            println!("{}", "Available Skills:".bold());
            for skill in &config.skills
            {
                template_skill_names.insert(skill.name.clone());
                let source_info = if crate::github::is_url(&skill.source) == true
                {
                    "(GitHub)"
                }
                else
                {
                    "(local)"
                };
                println!("  • {} {}", skill.name, source_info.dimmed());
            }
        }

        // Show ad-hoc installed skills not in template config
        let current_dir = std::env::current_dir().ok();
        if let Some(ref cwd) = current_dir
        {
            let file_tracker = FileTracker::new(&self.config_dir)?;
            let skill_entries = file_tracker.get_workspace_entries_by_category(cwd, "skill");

            let mut adhoc_names: BTreeSet<String> = BTreeSet::new();
            for (path, _) in &skill_entries
            {
                if let Some(name) = Self::extract_skill_name_from_path(path) &&
                    template_skill_names.contains(&name) == false
                {
                    adhoc_names.insert(name);
                }
            }

            if adhoc_names.is_empty() == false
            {
                if template_skill_names.is_empty() == true
                {
                    println!();
                    println!("{}", "Installed Skills:".bold());
                }
                for name in &adhoc_names
                {
                    println!("  • {} {}", name, "(ad-hoc)".dimmed());
                }
            }
        }

        println!();
        println!("{} Use 'slopctl init --lang <lang> --agent <agent>' to install", "→".blue());

        Ok(())
    }
}
