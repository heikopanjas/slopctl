//! Model defaults catalog command

use std::{fs, path::Path};

use owo_colors::OwoColorize;

use super::TemplateManager;
use crate::{
    Result,
    download_manager::DownloadManager,
    github,
    model_defaults::{self, MODEL_DEFAULTS_FILE}
};

impl TemplateManager
{
    /// Returns true if the global model defaults catalog exists
    pub fn has_model_defaults(&self) -> bool
    {
        self.config_dir.join(MODEL_DEFAULTS_FILE).exists()
    }

    /// Downloads or copies model defaults from a source
    ///
    /// Supports the same source forms as templates: GitHub URL or local path.
    ///
    /// # Errors
    ///
    /// Returns an error if the source cannot be read, copied, downloaded, or parsed.
    pub fn download_or_copy_model_defaults(&self, source: &str) -> Result<()>
    {
        if source.starts_with("http://") == true || source.starts_with("https://") == true
        {
            println!("{} Downloading model defaults from URL...", "→".blue());
            let download_manager = DownloadManager::new(self.config_dir.clone());
            download_manager.download_model_defaults_from_url(source)?;
        }
        else
        {
            let source_path = Path::new(source);
            let catalog_source = if source_path.is_file() == true
            {
                source_path.to_path_buf()
            }
            else
            {
                source_path.join(MODEL_DEFAULTS_FILE)
            };

            if catalog_source.exists() == false
            {
                return Err(anyhow::anyhow!("Model defaults source does not exist: {}", catalog_source.display()));
            }

            model_defaults::load_model_catalog_file(&catalog_source)?;
            println!("{} Copying model defaults from local path...", "→".blue());
            fs::create_dir_all(&self.config_dir)?;
            fs::copy(&catalog_source, self.config_dir.join(MODEL_DEFAULTS_FILE))?;
        }

        Ok(())
    }

    /// List known model defaults from the catalog
    ///
    /// # Errors
    ///
    /// Returns an error if the effective catalog cannot be loaded.
    pub fn list_models_catalog(&self) -> Result<()>
    {
        let catalog = model_defaults::load_model_catalog_from_dir(&self.config_dir)?;
        let source = if self.has_model_defaults() == true
        {
            self.config_dir.join(MODEL_DEFAULTS_FILE).display().to_string()
        }
        else
        {
            "embedded fallback".to_string()
        };

        println!("{}", "Model Defaults:".bold());
        println!("  {} Source: {}", "→".blue(), source.yellow());
        println!("  {} Version: {}", "→".blue(), catalog.version.to_string().green());
        println!();

        for provider in &catalog.providers
        {
            println!("{}", provider.name.bold());
            println!("  {} default model: {}", "→".blue(), provider.default_model.yellow());
            println!("  {} endpoint: {}", "→".blue(), provider.endpoint.yellow());
            println!("  {} models endpoint: {}", "→".blue(), provider.models_endpoint.yellow());
            if let Some(env_var) = &provider.api_key_env
            {
                println!("  {} api key env: {}", "→".blue(), env_var.yellow());
            }
        }

        Ok(())
    }

    /// Verify local model defaults for YAML validity and source freshness
    ///
    /// # Arguments
    ///
    /// * `source` - URL or local path used as the model defaults source
    ///
    /// # Errors
    ///
    /// Returns an error summarising the number of issues found.
    pub fn verify_models(&self, source: &str) -> Result<()>
    {
        let mut issues: Vec<String> = Vec::new();

        println!("{} Model defaults YAML", "→".blue());
        let local_catalog = match model_defaults::load_cached_model_catalog_from_dir(&self.config_dir)
        {
            | Ok(catalog) =>
            {
                println!("  {} version: {}", "✓".green(), catalog.version);
                println!("  {} providers: {}", "✓".green(), catalog.providers.len());
                Some(catalog)
            }
            | Err(e) =>
            {
                println!("  {} {}", "✗".red(), e);
                issues.push(format!("model defaults parse error: {}", e));
                None
            }
        };

        println!("{} Source freshness", "→".blue());
        if local_catalog.is_some() == true
        {
            self.verify_model_source_freshness(source, &mut issues);
        }

        println!();
        if issues.is_empty() == true
        {
            println!("{} All checks passed", "✓".green());
            Ok(())
        }
        else
        {
            Err(anyhow::anyhow!("Verification found {} issue(s)", issues.len()))
        }
    }

    fn verify_model_source_freshness(&self, source: &str, issues: &mut Vec<String>)
    {
        let local_path = self.config_dir.join(MODEL_DEFAULTS_FILE);
        let local_content = match fs::read_to_string(&local_path)
        {
            | Ok(content) => content,
            | Err(e) =>
            {
                let msg = format!("cannot read local {}: {}", MODEL_DEFAULTS_FILE, e);
                println!("  {} {}", "✗".red(), msg);
                issues.push(msg);
                return;
            }
        };

        let remote_content = match fetch_model_defaults_yml(source)
        {
            | Ok(content) => content,
            | Err(e) =>
            {
                let msg = format!("cannot fetch source {}: {}", MODEL_DEFAULTS_FILE, e);
                println!("  {} {}", "!".yellow(), msg);
                issues.push(msg);
                return;
            }
        };

        if let Err(e) = model_defaults::parse_model_catalog(&remote_content)
        {
            let msg = format!("source {} is invalid: {}", MODEL_DEFAULTS_FILE, e);
            println!("  {} {}", "✗".red(), msg);
            issues.push(msg);
            return;
        }

        if local_content == remote_content
        {
            println!("  {} local catalog matches source", "✓".green());
        }
        else
        {
            let msg = format!("local {} differs from source; run slopctl models --update", MODEL_DEFAULTS_FILE);
            println!("  {} {}", "!".yellow(), msg);
            issues.push(msg);
        }
    }
}

/// Fetch model defaults from a local path or GitHub source
///
/// # Errors
///
/// Returns an error if the source cannot be read or downloaded.
pub fn fetch_model_defaults_yml(source: &str) -> Result<String>
{
    if source.starts_with("http://") == true || source.starts_with("https://") == true
    {
        return fetch_remote_model_defaults_yml(source);
    }

    let source_path = Path::new(source);
    let catalog_path = if source_path.is_file() == true
    {
        source_path.to_path_buf()
    }
    else
    {
        source_path.join(MODEL_DEFAULTS_FILE)
    };
    Ok(fs::read_to_string(catalog_path)?)
}

fn fetch_remote_model_defaults_yml(source: &str) -> Result<String>
{
    let parsed = github::parse_github_url(source).ok_or_else(|| anyhow::anyhow!("Invalid GitHub URL format"))?;
    let base_url = format!("https://raw.githubusercontent.com/{}/{}/{}", parsed.owner, parsed.repo, parsed.branch);
    let url_path = if parsed.path.is_empty() == false
    {
        format!("/{}", parsed.path)
    }
    else
    {
        String::new()
    };
    let url = format!("{}{}/{}", base_url, url_path, MODEL_DEFAULTS_FILE);
    let temp_dir = tempfile::TempDir::new()?;
    let dest_path = temp_dir.path().join(MODEL_DEFAULTS_FILE);
    github::download_file(&url, &dest_path)?;
    Ok(fs::read_to_string(dest_path)?)
}
