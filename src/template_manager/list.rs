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

        // List agents (V2 templates don't have agents section - agents.md standard)
        if let Some(agents_map) = &config.agents
        {
            println!("{}", "Available Agents:".bold());
            let mut agents: Vec<&String> = agents_map.keys().collect();
            agents.sort();

            for agent_name in agents
            {
                // Check if agent is installed (has files in current directory)
                let is_installed = if let Some(files) = bom.get_agent_files(agent_name)
                {
                    files.iter().any(|f| f.exists())
                }
                else
                {
                    false
                };

                if is_installed == true
                {
                    println!("  {} {} (installed)", "✓".green(), agent_name.green());
                }
                else
                {
                    println!("  {} {}", "○".blue(), agent_name);
                }
            }

            println!();
        }
        else
        {
            println!("{}", "Available Agents:".bold());
            println!("  {} V2 templates (agents.md standard) - no agent-specific files", "→".blue());
            println!("  {} Single AGENTS.md works with all agents", "→".blue());
            println!();
        }

        // List languages (no installation status - language content is merged into AGENTS.md)
        println!("{}", "Available Languages:".bold());
        let mut languages: Vec<&String> = config.languages.keys().collect();
        languages.sort();

        for lang_name in languages
        {
            println!("  • {}", lang_name);
        }

        println!();
        println!("{} Use 'vibe-check init --lang <lang> --agent <agent>' to install", "→".blue());

        Ok(())
    }
}
