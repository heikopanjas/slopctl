//! Template status command

use std::{collections::BTreeSet, path::PathBuf};

use owo_colors::OwoColorize;

use super::TemplateManager;
use crate::{Result, bom::BillOfMaterials, file_tracker::FileTracker, template_engine};

impl TemplateManager
{
    /// Show current project status
    ///
    /// Displays information about:
    /// - Global template status (downloaded, location)
    /// - AGENTS.md status (exists, customized)
    /// - Installed agents (detected by checking for their files)
    /// - Installed skills (from FileTracker, covers all sources)
    /// - All regulator managed files in current directory
    ///
    /// # Errors
    ///
    /// Returns an error if the current directory cannot be determined
    pub fn status(&self) -> Result<()>
    {
        let current_dir = std::env::current_dir()?;

        println!("{}", "regulator status".bold());
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

                if config.agents.is_empty() == false
                {
                    let agents: Vec<&String> = config.agents.keys().collect();
                    println!("  {} Available agents: {}", "→".blue(), agents.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ").green());
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
            println!("  {} Run 'regulator update' to download templates", "→".blue());
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

        // Detect installed agents via BoM
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

        // Detect installed skills via FileTracker (covers template, top-level, and ad-hoc)
        let file_tracker = FileTracker::new(&self.config_dir)?;
        let skill_entries = file_tracker.get_workspace_entries_by_category(&current_dir, "skill");

        if skill_entries.is_empty() == false
        {
            let mut skill_names: BTreeSet<String> = BTreeSet::new();
            for (path, _) in &skill_entries
            {
                if let Some(name) = Self::extract_skill_name_from_path(path)
                {
                    skill_names.insert(name);
                }
            }

            let count = skill_names.len();
            println!("  {} Installed skills: {}", "✓".green(), count.to_string().green());
            for name in &skill_names
            {
                println!("    {} {}", "•".blue(), name.yellow());
            }
        }

        // Merge FileTracker entries into managed files for the complete list
        let all_tracked = file_tracker.get_workspace_entries(&current_dir);
        for (path, _) in all_tracked
        {
            managed_files.push(path);
        }

        if agents_md_path.exists() == true
        {
            managed_files.push(agents_md_path);
        }

        println!();

        // List all managed files
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
            println!("  {} No regulator files found in current directory", "○".yellow());
            println!("  {} Run 'regulator install --lang <lang> --agent <agent>' to set up", "→".blue());
        }

        Ok(())
    }
}
