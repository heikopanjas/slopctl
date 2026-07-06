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

use crate::{
    Result,
    agent_defaults::{self, AGENT_DEFAULTS_FILE},
    bom::TemplateConfig,
    github,
    model_defaults::{self, MODEL_DEFAULTS_FILE}
};

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

        let mut tarball_cache = github::RepoTarballCache::new();

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
                | Err(error) =>
                {
                    println!("{} ({})", "✗".red(), error);
                    return Err(error);
                }
            }
            Ok(())
        };

        // Download main AGENTS.md template if present
        if let Some(main) = &config.main
        {
            download_entry(&main.source)?;
        }

        for entry in &config.preamble
        {
            download_entry(&entry.source)?;
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

        // Copy bundled skill directories from a single template-repo tarball
        let skill_sources = Self::collect_local_skill_sources(&config);
        for source in &skill_sources
        {
            self.copy_template_skill_from_tarball(&mut tarball_cache, &parsed, source)?;
        }

        // Cache URL-based skills from one tarball fetch per external repository
        let url_skill_sources = Self::collect_url_skill_sources(&config);
        for source in &url_skill_sources
        {
            self.download_url_skill_to_cache(&mut tarball_cache, source)?;
        }

        println!("{} Templates downloaded successfully", "✓".green());

        Ok(())
    }

    /// Downloads the agent defaults catalog from a GitHub URL
    ///
    /// Downloads only `agent-defaults.yml` into the global template cache.
    ///
    /// # Arguments
    ///
    /// * `url` - GitHub URL to download from
    ///
    /// # Errors
    ///
    /// Returns an error if URL parsing or download fails
    pub fn download_agent_defaults_from_url(&self, url: &str) -> Result<()>
    {
        let parsed =
            github::parse_github_url(url).ok_or_else(|| anyhow::anyhow!("Invalid GitHub URL format. Expected: https://github.com/owner/repo/tree/branch/path"))?;

        println!("{} Repository: {}/{} (branch: {})", "→".blue(), parsed.owner.green(), parsed.repo.green(), parsed.branch.yellow());

        let base_url = format!("https://raw.githubusercontent.com/{}/{}/{}", parsed.owner, parsed.repo, parsed.branch);
        let url_path = if parsed.path.is_empty() == false
        {
            format!("/{}", parsed.path)
        }
        else
        {
            String::new()
        };

        let catalog_url = format!("{}{}/{}", base_url, url_path, AGENT_DEFAULTS_FILE);
        let temp_dir = tempfile::TempDir::new()?;
        let temp_path = temp_dir.path().join(AGENT_DEFAULTS_FILE);

        print!("{} Downloading {}... ", "→".blue(), AGENT_DEFAULTS_FILE.yellow());
        io::stdout().flush()?;

        match github::download_file(&catalog_url, &temp_path)
        {
            | Ok(_) =>
            {
                agent_defaults::load_agent_catalog_file(&temp_path)?;
                fs::create_dir_all(&self.config_dir)?;
                fs::copy(&temp_path, self.config_dir.join(AGENT_DEFAULTS_FILE))?;
                println!("{}", "✓".green());
                Ok(())
            }
            | Err(e) =>
            {
                println!("{}", "✗".red());
                Err(anyhow::anyhow!("Failed to download {}: {}", AGENT_DEFAULTS_FILE, e))
            }
        }
    }

    /// Downloads the model defaults catalog from a GitHub URL
    ///
    /// Downloads only `model-defaults.yml` into the global template cache.
    ///
    /// # Arguments
    ///
    /// * `url` - GitHub URL to download from
    ///
    /// # Errors
    ///
    /// Returns an error if URL parsing or download fails
    pub fn download_model_defaults_from_url(&self, url: &str) -> Result<()>
    {
        let parsed =
            github::parse_github_url(url).ok_or_else(|| anyhow::anyhow!("Invalid GitHub URL format. Expected: https://github.com/owner/repo/tree/branch/path"))?;

        println!("{} Repository: {}/{} (branch: {})", "→".blue(), parsed.owner.green(), parsed.repo.green(), parsed.branch.yellow());

        let base_url = format!("https://raw.githubusercontent.com/{}/{}/{}", parsed.owner, parsed.repo, parsed.branch);
        let url_path = if parsed.path.is_empty() == false
        {
            format!("/{}", parsed.path)
        }
        else
        {
            String::new()
        };

        let catalog_url = format!("{}{}/{}", base_url, url_path, MODEL_DEFAULTS_FILE);
        let temp_dir = tempfile::TempDir::new()?;
        let temp_path = temp_dir.path().join(MODEL_DEFAULTS_FILE);

        print!("{} Downloading {}... ", "→".blue(), MODEL_DEFAULTS_FILE.yellow());
        io::stdout().flush()?;

        match github::download_file(&catalog_url, &temp_path)
        {
            | Ok(_) =>
            {
                model_defaults::load_model_catalog_file(&temp_path)?;
                fs::create_dir_all(&self.config_dir)?;
                fs::copy(&temp_path, self.config_dir.join(MODEL_DEFAULTS_FILE))?;
                println!("{}", "✓".green());
                Ok(())
            }
            | Err(e) =>
            {
                println!("{}", "✗".red());
                Err(anyhow::anyhow!("Failed to download {}: {}", MODEL_DEFAULTS_FILE, e))
            }
        }
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

    /// Collects deduplicated URL-based skill sources from all config sections
    fn collect_url_skill_sources(config: &TemplateConfig) -> Vec<String>
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
            if github::is_url(&skill.source) == true && seen.insert(skill.source.clone()) == true
            {
                sources.push(skill.source.clone());
            }
        }

        sources
    }

    /// Copies a bundled skill directory from the template repository tarball
    fn copy_template_skill_from_tarball(&self, cache: &mut github::RepoTarballCache, parsed: &github::GitHubUrl, source: &str) -> Result<()>
    {
        let key = github::RepoTarballKey::from_github_url(parsed);
        let repo_root = cache.repo_root(&key)?;
        let skill_src = repo_root.join(github::repo_relative_template_path(parsed, source));
        let skill_dest = self.config_dir.join(source);

        print!("{} Caching {}... ", "→".blue(), source.yellow());
        io::stdout().flush()?;

        if skill_src.is_dir() == false
        {
            return Err(anyhow::anyhow!("Skill source not found in repository tarball: {}", skill_src.display()));
        }

        github::copy_skill_tree(&skill_src, &skill_dest)?;
        println!("{}", "✓".green());
        Ok(())
    }

    /// Downloads a URL-based skill repository into `config_dir/skills/<name>/`
    fn download_url_skill_to_cache(&self, cache: &mut github::RepoTarballCache, source: &str) -> Result<()>
    {
        let parsed = github::parse_github_url(source).ok_or_else(|| anyhow::anyhow!("Invalid GitHub URL for skill cache: {}", source))?;

        println!("{} Caching skills from {}...", "→".blue(), source.yellow());

        let key = github::RepoTarballKey::from_github_url(&parsed);
        let repo_root = cache.repo_root(&key)?;
        let search_root = if parsed.path.is_empty() == true
        {
            repo_root.to_path_buf()
        }
        else
        {
            repo_root.join(&parsed.path)
        };

        let discovered = github::discover_skills_in_dir(&search_root);
        if discovered.is_empty() == true
        {
            println!("{} No skills found (no SKILL.md) at {}", "!".yellow(), source.yellow());
            return Ok(());
        }

        for (skill_name, skill_path) in discovered
        {
            let dest = self.config_dir.join("skills").join(&skill_name);
            print!("{} Caching skill '{}'... ", "→".blue(), skill_name.green());
            io::stdout().flush()?;
            github::copy_skill_tree(&skill_path, &dest)?;
            println!("{}", "✓".green());
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

    fn make_skill(_name: &str, source: &str) -> SkillDefinition
    {
        SkillDefinition { source: source.to_string(), target: None }
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
            .insert("bogus".to_string(), crate::bom::AgentConfig { skills: vec![make_skill("git-workflow-agent", "skills/git-workflow")], ..Default::default() });

        let sources = DownloadManager::collect_local_skill_sources(&config);
        assert_eq!(sources, vec!["skills/git-workflow"]);
    }

    #[test]
    fn test_collect_local_skill_sources_all_sections()
    {
        let mut config = empty_config();
        config.skills = vec![make_skill("top", "skills/top-skill")];
        config.agents.insert("bogus".to_string(), crate::bom::AgentConfig { skills: vec![make_skill("agent", "skills/agent-skill")], ..Default::default() });
        config.languages.insert("Rust++".to_string(), crate::bom::LanguageConfig { skills: vec![make_skill("lang", "skills/lang-skill")], ..Default::default() });
        config.shared.insert("CppScript".to_string(), crate::bom::SharedConfig { skills: vec![make_skill("shared", "skills/shared-skill")], ..Default::default() });

        let sources = DownloadManager::collect_local_skill_sources(&config);
        assert_eq!(sources.len(), 4);
        assert!(sources.contains(&"skills/top-skill".to_string()) == true);
        assert!(sources.contains(&"skills/agent-skill".to_string()) == true);
        assert!(sources.contains(&"skills/lang-skill".to_string()) == true);
        assert!(sources.contains(&"skills/shared-skill".to_string()) == true);
    }

    #[test]
    fn test_download_agent_defaults_from_url_via_hook() -> anyhow::Result<()>
    {
        let config_dir = tempfile::TempDir::new()?;
        let agent_yaml = b"version: 1\nagents:\n  - name: bogus\n    markers:\n      - .bogus\n    prompt_dir: '$workspace/.bogus/prompts'\n    skill_dir: \
                           '$workspace/.bogus/skills'\n    reads_cross_client_skills: false\n";

        let _hook = github::set_test_hooks(Box::new(|_url| Ok(vec![])), Box::new(move |_url| Ok(agent_yaml.to_vec())));

        let dm = DownloadManager::new(config_dir.path().to_path_buf());
        dm.download_agent_defaults_from_url("https://github.com/test/repo/tree/main/templates/v5")?;

        assert!(config_dir.path().join(AGENT_DEFAULTS_FILE).exists() == true, "agent-defaults.yml must be written to config dir");
        Ok(())
    }

    #[test]
    fn test_download_model_defaults_from_url_via_hook() -> anyhow::Result<()>
    {
        let config_dir = tempfile::TempDir::new()?;
        let model_yaml =
            b"version: 1\nproviders:\n  - name: ollama\n    endpoint: http://localhost:11434/api/chat\n    models_endpoint: http://localhost:11434/api/tags\n    \
              default_model: llama3.2\n";

        let _hook = github::set_test_hooks(Box::new(|_url| Ok(vec![])), Box::new(move |_url| Ok(model_yaml.to_vec())));

        let dm = DownloadManager::new(config_dir.path().to_path_buf());
        dm.download_model_defaults_from_url("https://github.com/test/repo/tree/main/templates/v5")?;

        assert!(config_dir.path().join(MODEL_DEFAULTS_FILE).exists() == true, "model-defaults.yml must be written to config dir");
        Ok(())
    }

    #[test]
    fn test_collect_url_skill_sources_collects_urls()
    {
        let mut config = empty_config();
        config.skills = vec![make_skill("local", "skills/local-skill"), make_skill("remote", "https://github.com/user/repo")];

        let sources = DownloadManager::collect_url_skill_sources(&config);
        assert_eq!(sources, vec!["https://github.com/user/repo"]);
    }

    #[test]
    fn test_download_url_skill_to_cache_uses_no_list_hook() -> anyhow::Result<()>
    {
        let config_dir = tempfile::TempDir::new()?;
        let tarball = build_test_github_tarball("fake-skill", b"# Fake skill\n");

        let _tarball_hook = github::set_tarball_test_hook(Box::new(move |_owner, _repo, _branch| Ok(tarball.clone())));
        let _hooks =
            github::set_test_hooks(Box::new(|_url| panic!("list_directory_contents must not be called during tarball skill cache")), Box::new(|_url| Ok(vec![])));

        let dm = DownloadManager::new(config_dir.path().to_path_buf());
        let mut cache = github::RepoTarballCache::new();
        dm.download_url_skill_to_cache(&mut cache, "https://github.com/user/repo/tree/main")?;

        assert!(config_dir.path().join("skills/fake-skill/SKILL.md").exists() == true);
        Ok(())
    }

    #[test]
    fn test_copy_template_skill_from_tarball_via_hook() -> anyhow::Result<()>
    {
        let config_dir = tempfile::TempDir::new()?;
        let tarball = build_test_github_tarball_at("templates/v5/skills/fake-skill/SKILL.md", b"# Fake skill\n");

        let _tarball_hook = github::set_tarball_test_hook(Box::new(move |_owner, _repo, _branch| Ok(tarball.clone())));

        let parsed = github::GitHubUrl { owner: "user".into(), repo: "repo".into(), branch: "main".into(), path: "templates/v5".into() };

        let dm = DownloadManager::new(config_dir.path().to_path_buf());
        let mut cache = github::RepoTarballCache::new();
        dm.copy_template_skill_from_tarball(&mut cache, &parsed, "skills/fake-skill")?;

        assert!(config_dir.path().join("skills/fake-skill/SKILL.md").exists() == true);
        Ok(())
    }

    /// Builds a GitHub-style tarball containing a file at `repo_relative_path`
    fn build_test_github_tarball_at(repo_relative_path: &str, content: &[u8]) -> Vec<u8>
    {
        use flate2::{Compression, write::GzEncoder};

        let gz = GzEncoder::new(Vec::new(), Compression::default());
        let mut builder = tar::Builder::new(gz);
        let path = format!("owner-repo-deadbeef/{}", repo_relative_path);
        let mut header = tar::Header::new_gnu();
        header.set_size(content.len() as u64);
        header.set_mode(0o644);
        header.set_entry_type(tar::EntryType::Regular);
        header.set_cksum();
        builder.append_data(&mut header, &path, content).expect("append file to tarball");
        builder.finish().expect("finish tarball");
        builder.into_inner().expect("unwrap gzip encoder").finish().expect("finish gzip")
    }

    /// Builds a GitHub-style tarball with a skill directory at the repository root
    fn build_test_github_tarball(skill_name: &str, skill_content: &[u8]) -> Vec<u8>
    {
        build_test_github_tarball_at(&format!("{}/SKILL.md", skill_name), skill_content)
    }
}
