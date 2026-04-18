//! Template update command

use owo_colors::OwoColorize;

use super::TemplateManager;
use crate::{Result, template_engine};

impl TemplateManager
{
    /// Installs ad-hoc skills without requiring global templates
    ///
    /// Delegates to `TemplateEngine::install_skills_only` which installs skills
    /// to the cross-client `.agents/skills/` directory.
    ///
    /// # Arguments
    ///
    /// * `options` - Aggregated CLI parameters (only `skills`, `force`, `dry_run` are used)
    ///
    /// # Errors
    ///
    /// Returns an error if skill installation fails
    pub fn install_skills(&self, options: &template_engine::UpdateOptions) -> Result<()>
    {
        let engine = crate::template_engine::TemplateEngine::new(&self.config_dir);
        engine.install_skills_only(options)
    }

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
        require!(
            self.has_global_templates() == true,
            Err(anyhow::anyhow!("Global templates not found. Please run 'slopctl templates --update' first to download templates."))
        );

        let config = template_engine::load_template_config(&self.config_dir)?;
        let version = config.version;

        match version
        {
            | 1 => Err(anyhow::anyhow!(
                "V1 templates are no longer supported. Migrate to V5: slopctl config source.url https://github.com/heikopanjas/slopctl/tree/develop/templates/v5"
            )),
            | 2..=5 =>
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
            | _ => Err(anyhow::anyhow!("Unsupported template version: {}. Please update slopctl to the latest version.", version))
        }
    }
}
