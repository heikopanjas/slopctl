//! Agent defaults catalog command

use std::{fs, path::Path};

use owo_colors::OwoColorize;

use super::TemplateManager;
use crate::{
    Result,
    agent_defaults::{self, AGENT_DEFAULTS_FILE},
    download_manager::DownloadManager,
    github
};

impl TemplateManager
{
    /// Returns true if the global agent defaults catalog exists
    pub fn has_agent_defaults(&self) -> bool
    {
        self.config_dir.join(AGENT_DEFAULTS_FILE).exists()
    }

    /// Downloads or copies agent defaults from a source
    ///
    /// Supports the same source forms as templates: GitHub URL or local path.
    ///
    /// # Errors
    ///
    /// Returns an error if the source cannot be read, copied, downloaded, or parsed.
    pub fn download_or_copy_agent_defaults(&self, source: &str) -> Result<()>
    {
        if source.starts_with("http://") == true || source.starts_with("https://") == true
        {
            println!("{} Downloading agent defaults from URL...", "→".blue());
            let download_manager = DownloadManager::new(self.config_dir.clone());
            download_manager.download_agent_defaults_from_url(source)?;
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
                source_path.join(AGENT_DEFAULTS_FILE)
            };

            if catalog_source.exists() == false
            {
                return Err(anyhow::anyhow!("Agent defaults source does not exist: {}", catalog_source.display()));
            }

            agent_defaults::load_agent_catalog_file(&catalog_source)?;
            println!("{} Copying agent defaults from local path...", "→".blue());
            fs::create_dir_all(&self.config_dir)?;
            fs::copy(&catalog_source, self.config_dir.join(AGENT_DEFAULTS_FILE))?;
        }

        Ok(())
    }

    /// List known agent defaults
    ///
    /// # Errors
    ///
    /// Returns an error if the effective catalog cannot be loaded.
    pub fn list_agents(&self) -> Result<()>
    {
        let catalog = agent_defaults::load_agent_catalog_from_dir(&self.config_dir)?;
        let source = if self.has_agent_defaults() == true
        {
            self.config_dir.join(AGENT_DEFAULTS_FILE).display().to_string()
        }
        else
        {
            "embedded fallback".to_string()
        };

        println!("{}", "Agent Defaults:".bold());
        println!("  {} Source: {}", "→".blue(), source.yellow());
        println!("  {} Version: {}", "→".blue(), catalog.version.to_string().green());
        println!();

        for agent in &catalog.agents
        {
            println!("{}", agent.name.bold());
            println!("  {} prompts: {}", "→".blue(), agent.prompt_dir.yellow());
            println!("  {} skills: {}", "→".blue(), agent.skill_dir.yellow());
            if let Some(userprofile_skill_dir) = &agent.userprofile_skill_dir
            {
                println!("  {} userprofile skills: {}", "→".blue(), userprofile_skill_dir.yellow());
            }
            println!("  {} cross-client skills: {}", "→".blue(), agent.reads_cross_client_skills.to_string().green());
            let markers = agent.markers.join(", ");
            println!("  {} markers: {}", "→".blue(), markers.yellow());
        }

        Ok(())
    }

    /// Verify local agent defaults for YAML validity and source freshness
    ///
    /// # Arguments
    ///
    /// * `source` - URL or local path used as the agent defaults source
    ///
    /// # Errors
    ///
    /// Returns an error summarising the number of issues found.
    pub fn verify_agents(&self, source: &str) -> Result<()>
    {
        let mut issues: Vec<String> = Vec::new();

        println!("{} Agent defaults YAML", "→".blue());
        let local_catalog = match agent_defaults::load_cached_agent_catalog_from_dir(&self.config_dir)
        {
            | Ok(catalog) =>
            {
                println!("  {} version: {}", "✓".green(), catalog.version);
                println!("  {} agents: {}", "✓".green(), catalog.agents.len());
                Some(catalog)
            }
            | Err(e) =>
            {
                println!("  {} {}", "✗".red(), e);
                issues.push(format!("agent defaults parse error: {}", e));
                None
            }
        };

        println!("{} Source freshness", "→".blue());
        if local_catalog.is_some() == true
        {
            self.verify_agent_source_freshness(source, &mut issues);
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

    fn verify_agent_source_freshness(&self, source: &str, issues: &mut Vec<String>)
    {
        let local_path = self.config_dir.join(AGENT_DEFAULTS_FILE);
        let local_content = match fs::read_to_string(&local_path)
        {
            | Ok(content) => content,
            | Err(e) =>
            {
                let msg = format!("cannot read local {}: {}", AGENT_DEFAULTS_FILE, e);
                println!("  {} {}", "✗".red(), msg);
                issues.push(msg);
                return;
            }
        };

        let remote_content = match fetch_agent_defaults_yml(source)
        {
            | Ok(content) => content,
            | Err(e) =>
            {
                let msg = format!("cannot fetch source {}: {}", AGENT_DEFAULTS_FILE, e);
                println!("  {} {}", "!".yellow(), msg);
                issues.push(msg);
                return;
            }
        };

        if let Err(e) = agent_defaults::parse_agent_catalog(&remote_content)
        {
            let msg = format!("source {} is invalid: {}", AGENT_DEFAULTS_FILE, e);
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
            let msg = format!("local {} differs from source; run slopctl agents --update", AGENT_DEFAULTS_FILE);
            println!("  {} {}", "!".yellow(), msg);
            issues.push(msg);
        }
    }
}

/// Fetch agent defaults from a local path or GitHub source
///
/// # Errors
///
/// Returns an error if the source cannot be read or downloaded.
pub fn fetch_agent_defaults_yml(source: &str) -> Result<String>
{
    if source.starts_with("http://") == true || source.starts_with("https://") == true
    {
        return fetch_remote_agent_defaults_yml(source);
    }

    let source_path = Path::new(source);
    let catalog_path = if source_path.is_file() == true
    {
        source_path.to_path_buf()
    }
    else
    {
        source_path.join(AGENT_DEFAULTS_FILE)
    };
    Ok(fs::read_to_string(catalog_path)?)
}

fn fetch_remote_agent_defaults_yml(source: &str) -> Result<String>
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
    let url = format!("{}{}/{}", base_url, url_path, AGENT_DEFAULTS_FILE);
    let temp_dir = tempfile::TempDir::new()?;
    let dest_path = temp_dir.path().join(AGENT_DEFAULTS_FILE);
    github::download_file(&url, &dest_path)?;
    Ok(fs::read_to_string(dest_path)?)
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn test_has_agent_defaults_true_when_file_exists() -> anyhow::Result<()>
    {
        let config_dir = tempfile::TempDir::new()?;
        std::fs::write(config_dir.path().join(AGENT_DEFAULTS_FILE), "version: 1\nagents: []\n")?;
        let manager = TemplateManager { config_dir: config_dir.path().to_path_buf() };
        assert!(manager.has_agent_defaults() == true);
        Ok(())
    }

    #[test]
    fn test_has_agent_defaults_false_when_missing()
    {
        let config_dir = tempfile::TempDir::new().unwrap();
        let manager = TemplateManager { config_dir: config_dir.path().to_path_buf() };
        assert!(manager.has_agent_defaults() == false);
    }

    #[test]
    fn test_list_agents_succeeds_with_valid_catalog() -> anyhow::Result<()>
    {
        let config_dir = tempfile::TempDir::new()?;
        std::fs::write(
            config_dir.path().join(AGENT_DEFAULTS_FILE),
            "version: 1\nagents:\n  - name: bogus\n    markers:\n      - .bogus\n    prompt_dir: '$workspace/.bogus/prompts'\n    skill_dir: \
             '$workspace/.bogus/skills'\n    reads_cross_client_skills: false\n"
        )?;
        let manager = TemplateManager { config_dir: config_dir.path().to_path_buf() };
        manager.list_agents()?;
        Ok(())
    }

    #[test]
    fn test_verify_agents_passes_with_matching_source() -> anyhow::Result<()>
    {
        let config_dir = tempfile::TempDir::new()?;
        let yaml = "version: 1\nagents:\n  - name: bogus\n    markers:\n      - .bogus\n    prompt_dir: '$workspace/.bogus/prompts'\n    skill_dir: \
                    '$workspace/.bogus/skills'\n    reads_cross_client_skills: false\n";
        std::fs::write(config_dir.path().join(AGENT_DEFAULTS_FILE), yaml)?;

        // Use the config dir itself as source (local path); freshness check compares identical bytes
        let source = config_dir.path().to_string_lossy().to_string();
        let manager = TemplateManager { config_dir: config_dir.path().to_path_buf() };
        manager.verify_agents(&source)?;
        Ok(())
    }

    #[test]
    fn test_verify_agents_detects_stale_local() -> anyhow::Result<()>
    {
        let config_dir = tempfile::TempDir::new()?;
        let source_dir = tempfile::TempDir::new()?;

        let local_yaml = "version: 1\nagents:\n  - name: bogus\n    markers:\n      - .bogus\n    prompt_dir: '$workspace/.bogus/prompts'\n    skill_dir: \
                          '$workspace/.bogus/skills'\n    reads_cross_client_skills: false\n";
        let remote_yaml = "version: 1\nagents:\n  - name: bogus\n    markers:\n      - .bogus\n    prompt_dir: '$workspace/.bogus/prompts'\n    skill_dir: \
                           '$workspace/.bogus/skills'\n    reads_cross_client_skills: true\n";

        std::fs::write(config_dir.path().join(AGENT_DEFAULTS_FILE), local_yaml)?;
        std::fs::write(source_dir.path().join(AGENT_DEFAULTS_FILE), remote_yaml)?;

        let source = source_dir.path().to_string_lossy().to_string();
        let manager = TemplateManager { config_dir: config_dir.path().to_path_buf() };
        let result = manager.verify_agents(&source);
        assert!(result.is_err() == true, "verify must fail when local differs from source");
        Ok(())
    }

    #[test]
    fn test_download_or_copy_agent_defaults_from_local_path() -> anyhow::Result<()>
    {
        let config_dir = tempfile::TempDir::new()?;
        let source_dir = tempfile::TempDir::new()?;

        let yaml = "version: 1\nagents:\n  - name: bogus\n    markers:\n      - .bogus\n    prompt_dir: '$workspace/.bogus/prompts'\n    skill_dir: \
                    '$workspace/.bogus/skills'\n    reads_cross_client_skills: false\n";
        std::fs::write(source_dir.path().join(AGENT_DEFAULTS_FILE), yaml)?;

        let source = source_dir.path().to_string_lossy().to_string();
        let manager = TemplateManager { config_dir: config_dir.path().to_path_buf() };
        manager.download_or_copy_agent_defaults(&source)?;

        assert!(config_dir.path().join(AGENT_DEFAULTS_FILE).exists() == true, "agent defaults must be copied to config dir");
        Ok(())
    }

    #[test]
    fn test_fetch_agent_defaults_yml_local_path() -> anyhow::Result<()>
    {
        let source_dir = tempfile::TempDir::new()?;
        let yaml = "version: 1\nagents: []\n";
        std::fs::write(source_dir.path().join(AGENT_DEFAULTS_FILE), yaml)?;

        let content = fetch_agent_defaults_yml(&source_dir.path().to_string_lossy())?;
        assert!(content.contains("version: 1") == true);
        Ok(())
    }
}
