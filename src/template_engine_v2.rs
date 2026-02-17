//! Template engine v2 - Template generation logic for version 2 templates
//!
//! This module contains the template generation and merging logic for
//! templates.yml version 2 format (agents.md standard).
//!
//! V2 Philosophy:
//! - One AGENTS.md file that works across all agents
//! - Agent-specific instruction files (e.g. CLAUDE.md) reference AGENTS.md
//! - Follows https://agents.md community standard
//! - Compatible with Claude, Cursor, Copilot, Aider, Jules, Factory, and more

use std::path::{Path, PathBuf};

use owo_colors::OwoColorize;

use crate::{
    Result,
    file_tracker::FileTracker,
    template_engine::{self, CopyFilesResult, TemplateContext, TemplateEngine, UpdateOptions}
};

/// Template engine for version 2 templates (agents.md standard)
///
/// Handles template generation, fragment merging, and placeholder resolution
/// for the version 2 template format.
pub struct TemplateEngineV2<'a>
{
    config_dir: &'a Path
}

impl<'a> TemplateEngine for TemplateEngineV2<'a>
{
    fn config_dir(&self) -> &Path
    {
        self.config_dir
    }
}

impl<'a> TemplateEngineV2<'a>
{
    /// Creates a new TemplateEngineV2 instance
    ///
    /// # Arguments
    ///
    /// * `config_dir` - Path to the global template storage directory
    pub fn new(config_dir: &'a Path) -> Self
    {
        Self { config_dir }
    }

    /// Updates local templates from global storage (V2 - agent parameter optional)
    ///
    /// This method:
    /// 1. Verifies global templates exist
    /// 2. Detects local modifications to AGENTS.md
    /// 3. Copies templates to current directory
    ///
    /// V2 Philosophy: Single AGENTS.md works for all agents. Agent-specific
    /// instruction files (e.g. CLAUDE.md) and prompts are copied if agent is specified.
    ///
    /// # Arguments
    ///
    /// * `lang` - Programming language or framework identifier (ignored when no_lang)
    /// * `agent` - Optional agent identifier for copying agent-specific files
    /// * `no_lang` - If true, skip language-specific fragments
    /// * `mission` - Optional custom mission statement to override template default
    /// * `force` - If true, overwrite local modifications without warning
    /// * `dry_run` - If true, only show what would happen without making changes
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Global templates don't exist
    /// - Local modifications detected and force is false
    /// - Copy operations fail
    pub fn update(&self, lang: &str, agent: Option<&str>, no_lang: bool, mission: Option<&str>, force: bool, dry_run: bool) -> Result<()>
    {
        let templates_yml_path = self.config_dir.join("templates.yml");

        // Check if global templates exist
        if self.config_dir.exists() == false || templates_yml_path.exists() == false
        {
            return Err("Global templates not found. Please run 'vibe-check update' first to download templates.".into());
        }

        // Load template configuration
        let config = template_engine::load_template_config(self.config_dir)?;

        // Get current working directory and user home directory
        let workspace = std::env::current_dir()?;
        let userprofile = dirs::home_dir().ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "Could not determine home directory"))?;

        // Initialize file tracker
        let mut file_tracker = FileTracker::new(self.config_dir)?;

        // Resolve main template (required)
        let main_config = config.main.as_ref().ok_or("Missing 'main' section in templates.yml")?;
        let main_source = self.config_dir.join(&main_config.source);
        if main_source.exists() == false
        {
            return Err(format!("Main template not found: {}", main_source.display()).into());
        }
        let main_target = self.resolve_placeholder(&main_config.target, &workspace, &userprofile);

        // Collect files to copy and fragments to merge
        let mut files_to_copy: Vec<(PathBuf, PathBuf)> = Vec::new();
        let mut fragments: Vec<(PathBuf, String)> = Vec::new();

        // Helper closure to process file entries
        let mut process_entry = |source: &str, target: &str, category: &str| {
            let source_path = self.config_dir.join(source);
            if source_path.exists() == false
            {
                return;
            }

            if target.starts_with("$instructions")
            {
                fragments.push((source_path, category.to_string()));
            }
            else
            {
                let target_path = self.resolve_placeholder(target, &workspace, &userprofile);
                files_to_copy.push((source_path, target_path));
            }
        };

        // Add principles templates (fragments) if present
        if let Some(principles_entries) = &config.principles
        {
            for entry in principles_entries
            {
                process_entry(&entry.source, &entry.target, "principles");
            }
        }

        // Add mission templates (fragments) if present, unless custom mission is provided
        if mission.is_none() == true &&
            let Some(mission_entries) = &config.mission
        {
            for entry in mission_entries
            {
                process_entry(&entry.source, &entry.target, "mission");
            }
        }

        // Add language-specific templates (fragments) unless --no-lang
        if no_lang == false
        {
            if let Some(lang_config) = config.languages.get(lang)
            {
                for file_entry in &lang_config.files
                {
                    process_entry(&file_entry.source, &file_entry.target, "languages");
                }
            }
            else
            {
                return Err(format!("Language '{}' not found in templates.yml", lang).into());
            }
        }

        // Add integration templates (fragments)
        if let Some(integration_map) = &config.integration
        {
            for integration_config in integration_map.values()
            {
                for file_entry in &integration_config.files
                {
                    process_entry(&file_entry.source, &file_entry.target, "integration");
                }
            }
        }

        // Process agent-specific instruction and prompt files if agent is specified
        if let Some(agent_name) = agent &&
            let Some(agents) = config.agents.as_ref()
        {
            if let Some(agent_config) = agents.get(agent_name)
            {
                // Add agent instruction files (e.g. CLAUDE.md referencing AGENTS.md)
                if let Some(instructions) = &agent_config.instructions
                {
                    for instruction in instructions
                    {
                        let source_path = self.config_dir.join(&instruction.source);
                        if source_path.exists()
                        {
                            let target_path = self.resolve_placeholder(&instruction.target, &workspace, &userprofile);
                            files_to_copy.push((source_path, target_path));
                        }
                    }
                }

                // Add agent prompts
                if let Some(prompts) = &agent_config.prompts
                {
                    for prompt in prompts
                    {
                        let source_path = self.config_dir.join(&prompt.source);
                        if source_path.exists()
                        {
                            let target_path = self.resolve_placeholder(&prompt.target, &workspace, &userprofile);
                            files_to_copy.push((source_path, target_path));
                        }
                    }
                }
            }
            else
            {
                println!("{} Agent '{}' not found in templates.yml", "!".yellow(), agent_name.yellow());
            }
        }

        // Build template context and update options
        let ctx = TemplateContext { source: main_source, target: main_target, fragments, template_version: config.version };
        let options = UpdateOptions { lang, agent, no_lang, mission, force, dry_run };

        // Check if main AGENTS.md has been customized (marker removed)
        let skip_agents_md = ctx.target.exists() && template_engine::is_file_customized(&ctx.target)?;

        if skip_agents_md && options.force == false
        {
            println!("{} Local AGENTS.md has been customized and will be skipped", "!".yellow());
            if options.dry_run == false
            {
                println!("{} Other files will still be updated", "→".blue());
            }
            println!("{} Use --force to overwrite AGENTS.md", "→".blue());
        }

        // Dry run mode: just show what would happen
        if options.dry_run == true
        {
            self.show_dry_run_files(&ctx, skip_agents_md, &options, &files_to_copy);
            return Ok(());
        }

        // Handle main AGENTS.md with fragment merging
        self.handle_main_template(&ctx, &options, skip_agents_md, &mut file_tracker)?;

        // Copy templates with file modification checking
        let copy_result = self.copy_files_with_tracking(&files_to_copy, &mut file_tracker, &ctx, &options)?;

        match copy_result
        {
            | CopyFilesResult::Done { skipped } =>
            {
                self.show_skipped_files_summary(&skipped);
            }
            | CopyFilesResult::Cancelled =>
            {
                return Ok(());
            }
        }

        // Save file tracker metadata
        file_tracker.save()?;

        println!("{} Templates updated successfully", "✓".green());
        if agent.is_some()
        {
            println!("{} V2 templates: Single AGENTS.md + agent-specific files", "→".blue());
        }
        else
        {
            println!("{} V2 templates: Single AGENTS.md works with all agents", "→".blue());
        }

        Ok(())
    }
}
