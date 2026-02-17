//! Template status command

use std::path::PathBuf;

use owo_colors::OwoColorize;

use super::TemplateManager;
use crate::{Result, bom::BillOfMaterials, template_engine};

impl TemplateManager
{
    /// Show current project status
    ///
    /// Displays information about:
    /// - Global template status (downloaded, location)
    /// - AGENTS.md status (exists, customized)
    /// - Installed agents (detected by checking for their files)
    /// - All vibe-check managed files in current directory
    ///
    /// # Errors
    ///
    /// Returns an error if the current directory cannot be determined
    pub fn status(&self) -> Result<()>
    {
        let current_dir = std::env::current_dir()?;

        println!("{}", "vibe-check status".bold());
        println!();

        // Global templates status
        println!("{}", "Global Templates:".bold());
        if self.has_global_templates() == true
        {
            println!("  {} Installed at: {}", "✓".green(), self.config_dir.display().to_string().yellow());

            // Show template version, available agents and languages from templates.yml
            if let Ok(config) = template_engine::load_template_config(&self.config_dir)
            {
                println!("  {} Template version: {}", "→".blue(), config.version.to_string().green());

                // List agent-specific files (if agents section exists)
                if let Some(agents_map) = &config.agents
                {
                    let agents: Vec<&String> = agents_map.keys().collect();
                    if agents.is_empty() == false
                    {
                        println!("  {} Available agents: {}", "→".blue(), agents.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ").green());
                    }
                }

                let languages: Vec<&String> = config.languages.keys().collect();
                if languages.is_empty() == false
                {
                    println!("  {} Available languages: {}", "→".blue(), languages.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ").green());
                }
            }
        }
        else
        {
            println!("  {} Not installed", "✗".red());
            println!("  {} Run 'vibe-check update' to download templates", "→".blue());
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

        // Detect installed agents by checking for their files
        let mut installed_agents: Vec<String> = Vec::new();
        let mut managed_files: Vec<PathBuf> = Vec::new();

        let config_file = self.config_dir.join("templates.yml");
        if config_file.exists() == true &&
            let Ok(bom) = BillOfMaterials::from_config(&config_file)
        {
            for agent_name in bom.get_agent_names()
            {
                if let Some(files) = bom.get_agent_files(&agent_name)
                {
                    let existing_files: Vec<PathBuf> = files.iter().filter(|f| f.exists()).cloned().collect();
                    if existing_files.is_empty() == false
                    {
                        installed_agents.push(agent_name.clone());
                        managed_files.extend(existing_files);
                    }
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

        // Add AGENTS.md to managed files if it exists
        if agents_md_path.exists() == true
        {
            managed_files.push(agents_md_path);
        }

        println!();

        // List all managed files
        if managed_files.is_empty() == false
        {
            managed_files.sort();
            managed_files.dedup();

            println!("{}", "Managed Files:".bold());
            for file in &managed_files
            {
                // Show relative path if possible
                let display_path = file.strip_prefix(&current_dir).unwrap_or(file);
                println!("  • {}", display_path.display().to_string().yellow());
            }
        }
        else
        {
            println!("{}", "Managed Files:".bold());
            println!("  {} No vibe-check files found in current directory", "○".yellow());
            println!("  {} Run 'vibe-check init --lang <lang> --agent <agent>' to set up", "→".blue());
        }

        Ok(())
    }
}
