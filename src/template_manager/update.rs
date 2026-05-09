//! Template update command

use std::path::Path;

use owo_colors::OwoColorize;

use super::TemplateManager;
use crate::{Result, file_tracker::FileTracker, template_engine};

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

    /// Ensure `init --lang` does not silently add a second language
    ///
    /// Adding another language can create real workspace-file conflicts (for
    /// example `.editorconfig` or `.gitignore`). The `merge --lang` path exists
    /// specifically to resolve those conflicts with the LLM.
    fn ensure_language_init_allowed(workspace: &Path, requested_lang: Option<&str>) -> Result<()>
    {
        if let Some(lang) = requested_lang
        {
            let file_tracker = FileTracker::new(workspace)?;
            if let Some(installed_lang) = file_tracker.get_installed_language() &&
                installed_lang != lang
            {
                return Err(anyhow::anyhow!(
                    "Language '{}' is already installed. Use 'slopctl merge --lang {}' to add '{}' with AI-assisted conflict resolution, or run 'slopctl remove \
                     --lang {}' first to replace it.",
                    installed_lang,
                    lang,
                    lang,
                    installed_lang
                ));
            }
        }

        Ok(())
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

        let workspace = std::env::current_dir()?;
        let _ = self.try_migrate_tracker(&workspace);
        Self::ensure_language_init_allowed(&workspace, options.lang)?;

        let config = template_engine::load_template_config(&self.config_dir)?;
        let version = config.version;

        match version
        {
            | 1 => Err(anyhow::anyhow!(
                "V1 templates are no longer supported. Migrate to V5: slopctl config --set templates.uri https://github.com/heikopanjas/slopctl/tree/develop/templates/v5"
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

#[cfg(test)]
mod tests
{
    use std::fs;

    use super::TemplateManager;
    use crate::file_tracker::{AGENT_ALL, FileTracker, LANG_NONE};

    #[test]
    fn test_ensure_language_init_allowed_blocks_different_language() -> anyhow::Result<()>
    {
        let workspace = tempfile::TempDir::new()?;
        let tracked_file = workspace.path().join("AGENTS.md");
        fs::write(&tracked_file, "# instructions")?;

        let mut tracker = FileTracker::new(workspace.path())?;
        tracker.record_installation(&tracked_file, "sha1".into(), 5, "rust".into(), AGENT_ALL.into(), "main".into());
        tracker.save()?;

        let result = TemplateManager::ensure_language_init_allowed(workspace.path(), Some("swift"));

        assert!(result.is_err() == true);
        let message = result.unwrap_err().to_string();
        assert!(message.contains("rust") == true);
        assert!(message.contains("slopctl merge --lang swift") == true);
        assert!(message.contains("slopctl remove --lang rust") == true);
        Ok(())
    }

    #[test]
    fn test_ensure_language_init_allowed_allows_same_language() -> anyhow::Result<()>
    {
        let workspace = tempfile::TempDir::new()?;
        let tracked_file = workspace.path().join("AGENTS.md");
        fs::write(&tracked_file, "# instructions")?;

        let mut tracker = FileTracker::new(workspace.path())?;
        tracker.record_installation(&tracked_file, "sha1".into(), 5, "rust".into(), AGENT_ALL.into(), "main".into());
        tracker.save()?;

        let result = TemplateManager::ensure_language_init_allowed(workspace.path(), Some("rust"));

        assert!(result.is_ok() == true);
        Ok(())
    }

    #[test]
    fn test_ensure_language_init_allowed_ignores_language_none() -> anyhow::Result<()>
    {
        let workspace = tempfile::TempDir::new()?;
        let tracked_file = workspace.path().join("CLAUDE.md");
        fs::write(&tracked_file, "Read AGENTS.md")?;

        let mut tracker = FileTracker::new(workspace.path())?;
        tracker.record_installation(&tracked_file, "sha1".into(), 5, LANG_NONE.into(), "claude".into(), "agent".into());
        tracker.save()?;

        let result = TemplateManager::ensure_language_init_allowed(workspace.path(), Some("rust"));

        assert!(result.is_ok() == true);
        Ok(())
    }
}
