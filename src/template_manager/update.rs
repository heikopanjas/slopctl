//! Template update command

use owo_colors::OwoColorize;

use super::TemplateManager;
use crate::{Result, template_engine};

impl TemplateManager
{
    /// Updates local templates from global storage
    ///
    /// This method detects the template version and dispatches to the
    /// appropriate template engine for processing.
    ///
    /// # Arguments
    ///
    /// * `options` - Aggregated CLI parameters for the update operation
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Global templates don't exist
    /// - Template version is unsupported
    /// - Template generation fails
    pub fn update(&self, options: &template_engine::UpdateOptions) -> Result<()>
    {
        require!(self.has_global_templates() == true, Err(anyhow::anyhow!("Global templates not found. Please run 'vibe-check update' first to download templates.")));

        let config = template_engine::load_template_config(&self.config_dir)?;
        let version = config.version;

        match version
        {
            | 1 =>
            {
                Err(anyhow::anyhow!("V1 templates are no longer supported. Migrate to V3: vibe-check config source.url https://github.com/heikopanjas/vibe-check/tree/develop/templates/v3"))
            }
            | 2 | 3 =>
            {
                if options.lang.is_some() && options.agent.is_some()
                {
                    println!("{} Installing language setup + agent-specific files", "→".blue());
                }
                else if options.lang.is_some()
                {
                    println!("{} Installing language setup", "→".blue());
                }
                else if options.agent.is_some()
                {
                    println!("{} Installing agent-specific files", "→".blue());
                }

                let engine = crate::template_engine::TemplateEngine::new(&self.config_dir);
                engine.update(options)
            }
            | _ => Err(anyhow::anyhow!("Unsupported template version: {}. Please update vibe-check to the latest version.", version))
        }
    }
}
