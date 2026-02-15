//! Template update command

use owo_colors::OwoColorize;

use super::TemplateManager;
use crate::{Result, file_tracker::FileTracker, template_engine};

impl TemplateManager
{
    /// Updates local templates from global storage
    ///
    /// This method detects the template version and dispatches to the
    /// appropriate template engine for processing.
    ///
    /// # Arguments
    ///
    /// * `lang` - Programming language or framework identifier. If None (and no_lang false), uses existing installation or first available.
    /// * `agent` - AI coding agent identifier. Required for v1 templates, optional for v2.
    /// * `no_lang` - If true, skip language-specific setup (AGENTS.md + agent prompts only)
    /// * `mission` - Optional custom mission statement to override template default
    /// * `force` - If true, overwrite local modifications without warning
    /// * `dry_run` - If true, only show what would happen without making changes
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Global templates don't exist
    /// - Template version is unsupported
    /// - Lang is None, no_lang is false, and no languages are defined in templates
    /// - Template generation fails
    pub fn update(&self, lang: Option<&str>, agent: Option<&str>, no_lang: bool, mission: Option<&str>, force: bool, dry_run: bool) -> Result<()>
    {
        // Check if global templates exist
        if self.has_global_templates() == false
        {
            return Err("Global templates not found. Please run 'vibe-check update' first to download templates.".into());
        }

        // Load config for version and optional lang resolution
        let config = template_engine::load_template_config(&self.config_dir)?;
        let version = config.version;

        // Resolve lang (only when not no_lang): use provided value, or existing installation, or first available
        let lang_resolved: Option<String> = if no_lang == true
        {
            None
        }
        else
        {
            match lang
            {
                | Some(l) => Some(l.to_string()),
                | None =>
                {
                    // Prefer language from existing installation (e.g. switching agent, keep lang)
                    let workspace = std::env::current_dir().ok();
                    let from_tracker = workspace.and_then(|w| FileTracker::new(&self.config_dir).ok().and_then(|t| t.get_installed_language_for_workspace(&w)));

                    match from_tracker
                    {
                        | Some(l) =>
                        {
                            println!("{} Using existing language: {}", "→".blue(), l.green());
                            Some(l)
                        }
                        | None =>
                        {
                            // Fresh init with only --agent: use first language from templates
                            let first = config.languages.keys().next().cloned();
                            match first
                            {
                                | Some(l) =>
                                {
                                    println!("{} No existing installation, using language: {}", "→".blue(), l.green());
                                    Some(l)
                                }
                                | None => return Err("No languages defined in templates.yml".into())
                            }
                        }
                    }
                }
            }
        };

        match version
        {
            | 1 =>
            {
                // Deprecation warning for v1 templates
                println!("{} V1 templates are deprecated and will be removed in v7.0.0", "!".yellow());
                println!("{} Consider migrating to V2 templates (agents.md standard)", "!".yellow());
                println!("{} Run: vibe-check config source.url https://github.com/heikopanjas/vibe-check/tree/develop/templates/v2", "→".blue());
                println!();

                // V1 requires agent parameter
                let agent_str = agent.ok_or("--agent is required for v1 templates. Specify: vibe-check init --lang <lang> --agent <agent>")?;
                let engine = crate::template_engine_v1::TemplateEngineV1::new(&self.config_dir);
                let lang_for_engine = lang_resolved.as_deref().unwrap_or("");
                engine.update(lang_for_engine, agent_str, no_lang, mission, force, dry_run)
            }
            | 2 =>
            {
                // V2: Single AGENTS.md for all agents, but agent-specific prompts can be copied
                if no_lang == true
                {
                    println!("{} V2 templates: Language-independent setup (no coding-conventions)", "→".blue());
                }
                else if agent.is_some()
                {
                    println!("{} V2 templates: Using single AGENTS.md + copying agent-specific prompts", "→".blue());
                }
                else
                {
                    println!("{} V2 templates: Using single AGENTS.md (no agent-specific prompts)", "→".blue());
                }
                let engine = crate::template_engine_v2::TemplateEngineV2::new(&self.config_dir);
                let lang_for_engine = lang_resolved.as_deref().unwrap_or("");
                engine.update(lang_for_engine, agent, no_lang, mission, force, dry_run)
            }
            | _ => Err(format!("Unsupported template version: {}. Please update vibe-check to the latest version.", version).into())
        }
    }
}
