//! Template list command

use owo_colors::OwoColorize;

use super::TemplateManager;
use crate::{Result, bom::BillOfMaterials, template_engine};

impl TemplateManager
{
    /// List available agents and languages
    ///
    /// Displays all available agents and languages from the global templates,
    /// along with their installation status in the current project.
    ///
    /// # Errors
    ///
    /// Returns an error if templates.yml cannot be loaded
    pub fn list(&self) -> Result<()>
    {
        println!("{}", "vibe-check list".bold());
        println!();

        // Check if global templates exist
        if self.has_global_templates() == false
        {
            println!("{} Global templates not installed", "✗".red());
            println!("{} Run 'vibe-check update' to download templates", "→".blue());
            return Ok(());
        }

        // Load template configuration
        let config = template_engine::load_template_config(&self.config_dir)?;

        // Build BoM for checking installed status
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
                else
                {
                    if skill_count > 0
                    {
                        println!("  {} {} ({} skill(s))", "○".blue(), agent_name, skill_count);
                    }
                    else
                    {
                        println!("  {} {}", "○".blue(), agent_name);
                    }
                }
            }
        }
        println!();

        // List languages (no installation status - language content is merged into AGENTS.md)
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

        // List top-level skills (agent-agnostic)
        if config.skills.is_empty() == false
        {
            println!();
            println!("{}", "Available Skills:".bold());
            for skill in &config.skills
            {
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

        println!();
        println!("{} Use 'vibe-check install --lang <lang> --agent <agent>' to install", "→".blue());

        Ok(())
    }
}
