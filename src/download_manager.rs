//! Download management functionality for slopctl
//!
//! Handles downloading templates from GitHub repositories.

use std::{
    collections::HashSet,
    fs,
    io::{self, Write},
    path::PathBuf
};

use owo_colors::OwoColorize;

use crate::{Result, bom::TemplateConfig, github};

/// Manages downloading templates from remote sources
///
/// The `DownloadManager` handles all operations related to downloading
/// templates from GitHub repositories.
pub struct DownloadManager
{
    config_dir: PathBuf
}

impl DownloadManager
{
    /// Creates a new DownloadManager instance
    ///
    /// # Arguments
    ///
    /// * `config_dir` - Path to the global template storage directory
    pub fn new(config_dir: PathBuf) -> Self
    {
        Self { config_dir }
    }

    /// Downloads templates from a GitHub URL
    ///
    /// Downloads template files from a GitHub repository based on templates.yml configuration.
    ///
    /// # Arguments
    ///
    /// * `url` - GitHub URL to download from
    ///
    /// # Errors
    ///
    /// Returns an error if URL parsing or download fails
    pub fn download_templates_from_url(&self, url: &str) -> Result<()>
    {
        let parsed =
            github::parse_github_url(url).ok_or_else(|| anyhow::anyhow!("Invalid GitHub URL format. Expected: https://github.com/owner/repo/tree/branch/path"))?;

        println!("{} Repository: {}/{} (branch: {})", "→".blue(), parsed.owner.green(), parsed.repo.green(), parsed.branch.yellow());

        // Build base raw URL
        let base_url = format!("https://raw.githubusercontent.com/{}/{}/{}", parsed.owner, parsed.repo, parsed.branch);
        let url_path = if parsed.path.is_empty() == false
        {
            format!("/{}", parsed.path)
        }
        else
        {
            String::new()
        };

        fs::create_dir_all(&self.config_dir)?;

        // Load template configuration
        let config = self.load_template_config(&base_url, &url_path)?;

        // Helper closure to download a file entry
        let download_entry = |source: &str| -> Result<()> {
            let file_url = format!("{}{}/{}", base_url, url_path, source);
            let dest_path = self.config_dir.join(source);

            print!("{} Downloading {}... ", "→".blue(), source.yellow());
            io::stdout().flush()?;

            match github::download_file(&file_url, &dest_path)
            {
                | Ok(_) => println!("{}", "✓".green()),
                | Err(_) => println!("{} (skipped)", "✗".red())
            }
            Ok(())
        };

        // Download main AGENTS.md template if present
        if let Some(main) = &config.main
        {
            download_entry(&main.source)?;
        }

        for entry in &config.principles
        {
            download_entry(&entry.source)?;
        }

        for entry in &config.mission
        {
            download_entry(&entry.source)?;
        }

        // Download shared file groups (used by language includes)
        for shared_config in config.shared.values()
        {
            for file_entry in &shared_config.files
            {
                download_entry(&file_entry.source)?;
            }
        }

        // Download language templates
        for lang_config in config.languages.values()
        {
            for file_entry in &lang_config.files
            {
                download_entry(&file_entry.source)?;
            }
        }

        for integration_config in config.integration.values()
        {
            for file_entry in &integration_config.files
            {
                download_entry(&file_entry.source)?;
            }
        }

        for agent_config in config.agents.values()
        {
            for entry in agent_config.instructions.iter().chain(&agent_config.prompts)
            {
                download_entry(&entry.source)?;
            }
        }

        // Download skill directories (local-path only; URL skills are fetched at install time)
        let skill_sources = Self::collect_local_skill_sources(&config);
        for source in &skill_sources
        {
            self.download_skill_directory(&parsed, source)?;
        }

        println!("{} Templates downloaded successfully", "✓".green());

        Ok(())
    }

    /// Collects deduplicated local-path skill sources from all config sections
    ///
    /// Gathers skill sources from top-level skills, agent skills, language skills,
    /// and shared group skills. Skips URL-based sources (handled at install time).
    fn collect_local_skill_sources(config: &TemplateConfig) -> Vec<String>
    {
        let mut seen = HashSet::new();
        let mut sources = Vec::new();

        let all_skills = config
            .skills
            .iter()
            .chain(config.agents.values().flat_map(|a| &a.skills))
            .chain(config.languages.values().flat_map(|l| &l.skills))
            .chain(config.shared.values().flat_map(|s| &s.skills));

        for skill in all_skills
        {
            if github::is_url(&skill.source) == false && seen.insert(skill.source.clone()) == true
            {
                sources.push(skill.source.clone());
            }
        }

        sources
    }

    /// Downloads a skill directory from the GitHub repository into the global template cache
    ///
    /// Uses the GitHub Contents API to list directory contents, then downloads each file
    /// preserving the directory structure under `config_dir`.
    ///
    /// # Arguments
    ///
    /// * `parsed` - Parsed GitHub URL of the template repository
    /// * `source` - Relative path to the skill directory within the repo (e.g. `skills/rust-coding-conventions`)
    fn download_skill_directory(&self, parsed: &github::GitHubUrl, source: &str) -> Result<()>
    {
        let skill_url = if parsed.path.is_empty() == true
        {
            github::GitHubUrl { owner: parsed.owner.clone(), repo: parsed.repo.clone(), branch: parsed.branch.clone(), path: source.to_string() }
        }
        else
        {
            github::GitHubUrl {
                owner:  parsed.owner.clone(),
                repo:   parsed.repo.clone(),
                branch: parsed.branch.clone(),
                path:   format!("{}/{}", parsed.path, source)
            }
        };

        print!("{} Downloading {}... ", "→".blue(), source.yellow());
        io::stdout().flush()?;

        let entries = match github::list_directory_contents(&skill_url)
        {
            | Ok(entries) => entries,
            | Err(_) =>
            {
                println!("{} (skipped)", "✗".red());
                return Ok(());
            }
        };

        println!("{}", "✓".green());
        self.download_skill_entries(&entries, &skill_url, source)
    }

    /// Recursively downloads skill directory entries into the global template cache
    fn download_skill_entries(&self, entries: &[github::GitHubContentEntry], parent_url: &github::GitHubUrl, rel_path: &str) -> Result<()>
    {
        for entry in entries
        {
            let entry_path = format!("{}/{}", rel_path, entry.name);

            if entry.entry_type == "file" &&
                let Some(ref dl_url) = entry.download_url
            {
                let dest_path = self.config_dir.join(&entry_path);

                print!("  {} Downloading {}... ", "→".blue(), entry_path.yellow());
                io::stdout().flush()?;

                match github::download_file(dl_url, &dest_path)
                {
                    | Ok(_) => println!("{}", "✓".green()),
                    | Err(_) => println!("{} (skipped)", "✗".red())
                }
            }
            else if entry.entry_type == "dir"
            {
                let child_url = parent_url.child(&entry.name);
                match github::list_directory_contents(&child_url)
                {
                    | Ok(sub_entries) => self.download_skill_entries(&sub_entries, &child_url, &entry_path)?,
                    | Err(e) => println!("  {} Skipping {}: {}", "!".yellow(), entry.name.yellow(), e)
                }
            }
        }

        Ok(())
    }

    /// Loads template configuration from templates.yml
    ///
    /// Downloads templates.yml from the remote URL.
    ///
    /// # Arguments
    ///
    /// * `base_url` - Base URL for downloading templates.yml from GitHub
    /// * `url_path` - Path within the repository
    ///
    /// # Errors
    ///
    /// Returns an error if templates.yml cannot be loaded or parsed
    fn load_template_config(&self, base_url: &str, url_path: &str) -> Result<TemplateConfig>
    {
        let config_path = self.config_dir.join("templates.yml");
        let config_url = format!("{}{}/templates.yml", base_url, url_path);

        print!("{} Downloading templates.yml... ", "→".blue());
        io::stdout().flush()?;

        match github::download_file(&config_url, &config_path)
        {
            | Ok(_) => println!("{}", "✓".green()),
            | Err(e) =>
            {
                println!("{}", "✗".red());
                return Err(anyhow::anyhow!("Failed to download templates.yml: {}", e));
            }
        }

        let content = fs::read_to_string(&config_path)?;
        let config: TemplateConfig = serde_yaml::from_str(&content)?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests
{
    use super::*;
    use crate::bom::SkillDefinition;

    fn make_skill(name: &str, source: &str) -> SkillDefinition
    {
        SkillDefinition { name: name.to_string(), source: source.to_string() }
    }

    fn empty_config() -> TemplateConfig
    {
        serde_yaml::from_str("version: 5\nlanguages: {}").unwrap()
    }

    #[test]
    fn test_collect_local_skill_sources_empty()
    {
        let config = empty_config();
        let sources = DownloadManager::collect_local_skill_sources(&config);
        assert!(sources.is_empty() == true);
    }

    #[test]
    fn test_collect_local_skill_sources_top_level()
    {
        let mut config = empty_config();
        config.skills = vec![make_skill("git-workflow", "skills/git-workflow"), make_skill("semver", "skills/semantic-versioning")];

        let sources = DownloadManager::collect_local_skill_sources(&config);
        assert_eq!(sources, vec!["skills/git-workflow", "skills/semantic-versioning"]);
    }

    #[test]
    fn test_collect_local_skill_sources_skips_urls()
    {
        let mut config = empty_config();
        config.skills = vec![make_skill("local", "skills/local-skill"), make_skill("remote", "https://github.com/user/repo")];

        let sources = DownloadManager::collect_local_skill_sources(&config);
        assert_eq!(sources, vec!["skills/local-skill"]);
    }

    #[test]
    fn test_collect_local_skill_sources_deduplicates()
    {
        let mut config = empty_config();
        config.skills = vec![make_skill("git-workflow", "skills/git-workflow")];
        config
            .agents
            .insert("cursor".to_string(), crate::bom::AgentConfig { skills: vec![make_skill("git-workflow-agent", "skills/git-workflow")], ..Default::default() });

        let sources = DownloadManager::collect_local_skill_sources(&config);
        assert_eq!(sources, vec!["skills/git-workflow"]);
    }

    #[test]
    fn test_collect_local_skill_sources_all_sections()
    {
        let mut config = empty_config();
        config.skills = vec![make_skill("top", "skills/top-skill")];
        config.agents.insert("cursor".to_string(), crate::bom::AgentConfig { skills: vec![make_skill("agent", "skills/agent-skill")], ..Default::default() });
        config.languages.insert("rust".to_string(), crate::bom::LanguageConfig { skills: vec![make_skill("lang", "skills/lang-skill")], ..Default::default() });
        config.shared.insert("cmake".to_string(), crate::bom::SharedConfig { skills: vec![make_skill("shared", "skills/shared-skill")], ..Default::default() });

        let sources = DownloadManager::collect_local_skill_sources(&config);
        assert_eq!(sources.len(), 4);
        assert!(sources.contains(&"skills/top-skill".to_string()) == true);
        assert!(sources.contains(&"skills/agent-skill".to_string()) == true);
        assert!(sources.contains(&"skills/lang-skill".to_string()) == true);
        assert!(sources.contains(&"skills/shared-skill".to_string()) == true);
    }
}
