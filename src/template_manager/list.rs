//! Template list command

use std::collections::BTreeSet;

use owo_colors::OwoColorize;

use super::TemplateManager;
use crate::{Result, bom::BillOfMaterials, file_tracker::FileTracker, template_engine};

impl TemplateManager
{
    /// List available agents, languages, and skills
    ///
    /// Displays all available agents and languages from the global templates,
    /// along with their installation status. Shows template-defined skills
    /// and any ad-hoc installed skills from the FileTracker.
    ///
    /// # Errors
    ///
    /// Returns an error if templates.yml cannot be loaded
    pub fn list(&self) -> Result<()>
    {
        println!("{}", "regulator list".bold());
        println!();

        if self.has_global_templates() == false
        {
            println!("{} Global templates not installed", "✗".red());
            println!("{} Run 'regulator update' to download templates", "→".blue());
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
            let includes_annotation = lang_config.map(|lc| &lc.includes).filter(|inc| inc.is_empty() == false).map(|inc| format!(" (includes: {})", inc.join(", ")));

            if let Some(annotation) = includes_annotation
            {
                println!("  • {}{}", lang_name, annotation.dimmed());
            }
            else
            {
                println!("  • {}", lang_name);
            }
        }

        // Collect template-defined skill names for deduplication
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
        println!("{} Use 'regulator install --lang <lang> --agent <agent>' to install", "→".blue());

        Ok(())
    }
}
