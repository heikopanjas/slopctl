//! Template verification command

use std::{
    fs,
    path::{Path, PathBuf}
};

use owo_colors::OwoColorize;

use super::TemplateManager;
use crate::{
    Result,
    bom::TemplateConfig,
    github::{download_file, is_github_url, is_url, parse_github_url},
    template_engine::load_template_config
};

impl TemplateManager
{
    /// Verify local templates for YAML validity, file integrity, and source freshness
    ///
    /// Runs three sequential checks and prints results:
    /// - **C – YAML structure**: parses `templates.yml`, validates version and required fields
    /// - **A – Local file integrity**: every source file/directory declared in `templates.yml` must exist in the local template cache
    /// - **B – Source freshness**: downloads (or reads) `templates.yml` from the configured source and compares its content with the local copy
    ///
    /// Returns `Ok(())` when all checks pass, or `Err` (non-zero exit) if any issue is found.
    ///
    /// # Arguments
    ///
    /// * `source` - URL or local path used as the template source for the freshness check
    ///
    /// # Errors
    ///
    /// Returns an error summarising the number of issues found, or an I/O error if a
    /// critical operation fails before verification can complete.
    pub fn verify(&self, source: &str) -> Result<()>
    {
        let mut issues: Vec<String> = Vec::new();

        if self.has_global_templates() == false
        {
            eprintln!("{} Global templates not installed", "✗".red());
            eprintln!("{} Run: slopctl templates --update", "→".blue());
            return Err(anyhow::anyhow!("Verification found 1 issue(s)"));
        }

        // Section C: YAML structure
        let config = self.verify_yaml_structure(&mut issues);

        // Section A: Local file integrity (only possible when YAML parsed successfully)
        if let Some(ref cfg) = config
        {
            self.verify_local_integrity(cfg, &mut issues);
        }

        // Section B: Source freshness
        self.verify_source_freshness(source, &mut issues);

        println!();
        if issues.is_empty() == true
        {
            println!("{} All checks passed", "✓".green());
            Ok(())
        }
        else
        {
            eprintln!("{} Verification found {} issue(s)", "✗".red(), issues.len());
            Err(anyhow::anyhow!("Verification found {} issue(s)", issues.len()))
        }
    }

    /// Section C: parse `templates.yml` and validate its structure
    ///
    /// Returns `Some(config)` on success so later sections can reuse the parsed config,
    /// or `None` when parsing fails (further checks are skipped).
    fn verify_yaml_structure(&self, issues: &mut Vec<String>) -> Option<TemplateConfig>
    {
        println!("{} YAML structure", "→".blue());

        let config = match load_template_config(&self.config_dir)
        {
            | Ok(c) => c,
            | Err(e) =>
            {
                println!("  {} templates.yml: {}", "✗".red(), e);
                issues.push(format!("templates.yml parse error: {}", e));
                return None;
            }
        };

        // Version range
        if (2..=5).contains(&config.version) == true
        {
            println!("  {} version: {}", "✓".green(), config.version);
        }
        else
        {
            let msg = format!("unsupported version: {}", config.version);
            println!("  {} {}", "✗".red(), msg);
            issues.push(msg);
        }

        // Main source present
        match &config.main
        {
            | Some(main) if main.source.is_empty() == false =>
            {
                println!("  {} main source present", "✓".green());
            }
            | _ =>
            {
                let msg = "main.source is missing or empty".to_string();
                println!("  {} {}", "✗".red(), msg);
                issues.push(msg);
            }
        }

        // No duplicate targets: collect all (source, target) pairs and check
        let duplicate_issues = collect_duplicate_target_issues(&config, &self.config_dir);
        if duplicate_issues.is_empty() == true
        {
            println!("  {} no duplicate targets", "✓".green());
        }
        else
        {
            for msg in &duplicate_issues
            {
                println!("  {} {}", "✗".red(), msg);
                issues.push(msg.clone());
            }
        }

        Some(config)
    }

    /// Section A: verify that every non-URL source file/dir exists in the local template cache
    fn verify_local_integrity(&self, config: &TemplateConfig, issues: &mut Vec<String>)
    {
        println!("{} Local file integrity", "→".blue());

        let mut checked = 0u32;
        let mut local_issues: Vec<String> = Vec::new();

        // main source
        if let Some(main) = &config.main
        {
            check_source_file(&self.config_dir, &main.source, &mut checked, &mut local_issues);
        }

        // agent instructions and prompts
        let mut agent_names: Vec<&str> = config.agents.keys().map(String::as_str).collect();
        agent_names.sort();
        for agent in agent_names
        {
            let agent_cfg = &config.agents[agent];
            for mapping in &agent_cfg.instructions
            {
                check_source_file(&self.config_dir, &mapping.source, &mut checked, &mut local_issues);
            }
            for mapping in &agent_cfg.prompts
            {
                check_source_file(&self.config_dir, &mapping.source, &mut checked, &mut local_issues);
            }
            for skill in &agent_cfg.skills
            {
                check_source_skill(&self.config_dir, &skill.source, &mut checked, &mut local_issues);
            }
        }

        // language files and skills
        let mut lang_names: Vec<&str> = config.languages.keys().map(String::as_str).collect();
        lang_names.sort();
        for lang in lang_names
        {
            let lang_cfg = &config.languages[lang];
            for mapping in &lang_cfg.files
            {
                check_source_file(&self.config_dir, &mapping.source, &mut checked, &mut local_issues);
            }
            for skill in &lang_cfg.skills
            {
                check_source_skill(&self.config_dir, &skill.source, &mut checked, &mut local_issues);
            }
        }

        // shared group files and skills
        let mut shared_names: Vec<&str> = config.shared.keys().map(String::as_str).collect();
        shared_names.sort();
        for group in shared_names
        {
            let shared_cfg = &config.shared[group];
            for mapping in &shared_cfg.files
            {
                check_source_file(&self.config_dir, &mapping.source, &mut checked, &mut local_issues);
            }
            for skill in &shared_cfg.skills
            {
                check_source_skill(&self.config_dir, &skill.source, &mut checked, &mut local_issues);
            }
        }

        // integration files
        let mut int_names: Vec<&str> = config.integration.keys().map(String::as_str).collect();
        int_names.sort();
        for int_key in int_names
        {
            for mapping in &config.integration[int_key].files
            {
                check_source_file(&self.config_dir, &mapping.source, &mut checked, &mut local_issues);
            }
        }

        // top-level skills
        for skill in &config.skills
        {
            check_source_skill(&self.config_dir, &skill.source, &mut checked, &mut local_issues);
        }

        // principles and mission fragments
        for mapping in config.principles.iter().chain(config.mission.iter())
        {
            check_source_file(&self.config_dir, &mapping.source, &mut checked, &mut local_issues);
        }

        if local_issues.is_empty() == true
        {
            println!("  {} {} file(s) present", "✓".green(), checked);
        }
        else
        {
            for msg in &local_issues
            {
                println!("  {} {}", "✗".red(), msg);
            }
            println!("  {} {} of {} file(s) OK", "!".yellow(), checked - local_issues.len() as u32, checked);
        }

        issues.extend(local_issues);
    }

    /// Section B: compare local `templates.yml` content with the remote/configured source
    fn verify_source_freshness(&self, source: &str, issues: &mut Vec<String>)
    {
        println!("{} Source freshness", "→".blue());

        let local_path = self.config_dir.join("templates.yml");
        let local_content = match fs::read_to_string(&local_path)
        {
            | Ok(c) => c,
            | Err(e) =>
            {
                let msg = format!("cannot read local templates.yml: {}", e);
                println!("  {} {}", "✗".red(), msg);
                issues.push(msg);
                return;
            }
        };

        let remote_content = match fetch_remote_templates_yml(source)
        {
            | Ok(c) => c,
            | Err(e) =>
            {
                println!("  {} could not fetch source: {}", "!".yellow(), e);
                // Treat as a warning, not a hard issue (network may be unavailable)
                return;
            }
        };

        if local_content == remote_content
        {
            println!("  {} local templates match source", "✓".green());
        }
        else
        {
            let msg = "local templates differ from source".to_string();
            println!("  {} {} — run: slopctl templates --update", "!".yellow(), msg);
            issues.push(msg);
        }
    }
}

/// Collect duplicate-target violations across all sections of a `TemplateConfig`
///
/// Returns one error string per duplicate found.
fn collect_duplicate_target_issues(config: &TemplateConfig, config_dir: &Path) -> Vec<String>
{
    use std::collections::HashMap;

    let mut seen: HashMap<String, String> = HashMap::new();
    let mut violations: Vec<String> = Vec::new();

    let mut record = |source: &str, target: &str| {
        let resolved = resolve_target(target, config_dir);
        let key = resolved.to_string_lossy().to_string();
        if let Some(prev_source) = seen.insert(key.clone(), source.to_string())
        {
            violations.push(format!("duplicate target '{}': '{}' and '{}'", key, prev_source, source));
        }
    };

    if let Some(main) = &config.main
    {
        record(&main.source, &main.target);
    }

    for agent_cfg in config.agents.values()
    {
        for m in agent_cfg.instructions.iter().chain(agent_cfg.prompts.iter())
        {
            record(&m.source, &m.target);
        }
    }

    for lang_cfg in config.languages.values()
    {
        for m in &lang_cfg.files
        {
            record(&m.source, &m.target);
        }
    }

    for shared_cfg in config.shared.values()
    {
        for m in &shared_cfg.files
        {
            record(&m.source, &m.target);
        }
    }

    for int_cfg in config.integration.values()
    {
        for m in &int_cfg.files
        {
            record(&m.source, &m.target);
        }
    }

    violations
}

/// Resolve a target string to a display path, replacing `$workspace` with `<workspace>`
/// and stripping `$instructions` markers, for use in duplicate-target reporting only.
fn resolve_target(target: &str, _config_dir: &Path) -> PathBuf
{
    let normalized = target
        .replace("$workspace/", "")
        .replace("$workspace\\", "")
        .replace("$instructions", "<instructions>")
        .replace("$userprofile/", "")
        .replace("$userprofile\\", "");
    PathBuf::from(normalized)
}

/// Check that a source file path (relative to `config_dir`) exists on disk
///
/// Skips `$instructions` and `$userprofile` targets, and URL-based sources.
/// Increments `checked` for each file inspected.
fn check_source_file(config_dir: &Path, source: &str, checked: &mut u32, issues: &mut Vec<String>)
{
    if is_url(source) == true
    {
        return;
    }

    let path = config_dir.join(source);
    *checked += 1;

    if path.exists() == false
    {
        issues.push(format!("{} (missing)", source));
    }
}

/// Check that a skill source directory exists in the local template cache
///
/// Only checks local-path skills (skips GitHub URL sources).
/// Increments `checked` for each directory inspected.
fn check_source_skill(config_dir: &Path, source: &str, checked: &mut u32, issues: &mut Vec<String>)
{
    if is_url(source) == true || is_github_url(source) == true
    {
        return;
    }

    let path = config_dir.join(source);
    *checked += 1;

    if path.exists() == false
    {
        issues.push(format!("{} (missing)", source));
    }
}

/// Fetch `templates.yml` from a remote URL or local path and return its content
///
/// For GitHub URLs the raw download URL for `templates.yml` is constructed via
/// `GitHubUrl::raw_file_url`. For local paths the file is read directly.
fn fetch_remote_templates_yml(source: &str) -> Result<String>
{
    if is_github_url(source) == true
    {
        let gh = parse_github_url(source).ok_or_else(|| anyhow::anyhow!("Cannot parse GitHub URL: {}", source))?;
        let raw_url = gh.raw_file_url("templates.yml");
        let tmp = tempfile::NamedTempFile::new()?;
        download_file(&raw_url, tmp.path())?;
        Ok(fs::read_to_string(tmp.path())?)
    }
    else if is_url(source) == true
    {
        let url = format!("{}/templates.yml", source.trim_end_matches('/'));
        let tmp = tempfile::NamedTempFile::new()?;
        download_file(&url, tmp.path())?;
        Ok(fs::read_to_string(tmp.path())?)
    }
    else
    {
        let path = Path::new(source).join("templates.yml");
        Ok(fs::read_to_string(&path).map_err(|e| anyhow::anyhow!("Cannot read {}: {}", path.display(), e))?)
    }
}

#[cfg(test)]
mod tests
{
    use std::fs;

    use super::*;
    use crate::template_manager::cwd_test_guard;

    #[test]
    fn test_verify_no_global_templates_returns_error() -> anyhow::Result<()>
    {
        let data_dir = tempfile::TempDir::new()?;
        let workspace = tempfile::TempDir::new()?;

        let _g = cwd_test_guard();
        std::env::set_current_dir(workspace.path())?;

        let manager = TemplateManager { config_dir: data_dir.path().to_path_buf() };
        let result = manager.verify("https://github.com/heikopanjas/slopctl/tree/main/templates/v5");

        assert!(result.is_err() == true);
        Ok(())
    }

    #[test]
    fn test_verify_yaml_structure_valid_config() -> anyhow::Result<()>
    {
        let data_dir = tempfile::TempDir::new()?;

        let yaml = "version: 5\nmain:\n  source: AGENTS.md\n  target: '$workspace/AGENTS.md'\nagents: {}\nlanguages: {}\n";
        fs::write(data_dir.path().join("templates.yml"), yaml)?;

        let manager = TemplateManager { config_dir: data_dir.path().to_path_buf() };
        let mut issues: Vec<String> = Vec::new();

        let config = manager.verify_yaml_structure(&mut issues);

        assert!(config.is_some() == true);
        assert!(issues.is_empty() == true, "unexpected issues: {:?}", issues);
        Ok(())
    }

    #[test]
    fn test_verify_yaml_structure_unsupported_version() -> anyhow::Result<()>
    {
        let data_dir = tempfile::TempDir::new()?;

        let yaml = "version: 99\nmain:\n  source: AGENTS.md\n  target: '$workspace/AGENTS.md'\nagents: {}\nlanguages: {}\n";
        fs::write(data_dir.path().join("templates.yml"), yaml)?;

        let manager = TemplateManager { config_dir: data_dir.path().to_path_buf() };
        let mut issues: Vec<String> = Vec::new();

        manager.verify_yaml_structure(&mut issues);

        assert!(issues.iter().any(|i| i.contains("unsupported version")) == true);
        Ok(())
    }

    #[test]
    fn test_verify_yaml_structure_missing_main_source() -> anyhow::Result<()>
    {
        let data_dir = tempfile::TempDir::new()?;

        let yaml = "version: 5\nagents: {}\nlanguages: {}\n";
        fs::write(data_dir.path().join("templates.yml"), yaml)?;

        let manager = TemplateManager { config_dir: data_dir.path().to_path_buf() };
        let mut issues: Vec<String> = Vec::new();

        manager.verify_yaml_structure(&mut issues);

        assert!(issues.iter().any(|i| i.contains("main.source")) == true);
        Ok(())
    }

    #[test]
    fn test_verify_local_integrity_present_files() -> anyhow::Result<()>
    {
        let data_dir = tempfile::TempDir::new()?;

        fs::write(data_dir.path().join("AGENTS.md"), "# template")?;

        let yaml = "version: 5\nmain:\n  source: AGENTS.md\n  target: '$workspace/AGENTS.md'\nagents: {}\nlanguages: {}\n";
        fs::write(data_dir.path().join("templates.yml"), yaml)?;

        let manager = TemplateManager { config_dir: data_dir.path().to_path_buf() };
        let config = crate::template_engine::load_template_config(&data_dir.path().to_path_buf())?;
        let mut issues: Vec<String> = Vec::new();

        manager.verify_local_integrity(&config, &mut issues);

        assert!(issues.is_empty() == true, "unexpected issues: {:?}", issues);
        Ok(())
    }

    #[test]
    fn test_verify_local_integrity_missing_file() -> anyhow::Result<()>
    {
        let data_dir = tempfile::TempDir::new()?;

        // templates.yml references AGENTS.md but we do NOT create it
        let yaml = "version: 5\nmain:\n  source: AGENTS.md\n  target: '$workspace/AGENTS.md'\nagents: {}\nlanguages: {}\n";
        fs::write(data_dir.path().join("templates.yml"), yaml)?;

        let manager = TemplateManager { config_dir: data_dir.path().to_path_buf() };
        let config = crate::template_engine::load_template_config(&data_dir.path().to_path_buf())?;
        let mut issues: Vec<String> = Vec::new();

        manager.verify_local_integrity(&config, &mut issues);

        assert!(issues.iter().any(|i| i.contains("AGENTS.md") && i.contains("missing")) == true);
        Ok(())
    }

    #[test]
    fn test_verify_source_freshness_local_match() -> anyhow::Result<()>
    {
        let data_dir = tempfile::TempDir::new()?;
        let source_dir = tempfile::TempDir::new()?;

        let content = "version: 5\nagents: {}\nlanguages: {}\n";
        fs::write(data_dir.path().join("templates.yml"), content)?;
        fs::write(source_dir.path().join("templates.yml"), content)?;

        let manager = TemplateManager { config_dir: data_dir.path().to_path_buf() };
        let mut issues: Vec<String> = Vec::new();

        manager.verify_source_freshness(source_dir.path().to_str().unwrap(), &mut issues);

        assert!(issues.is_empty() == true, "unexpected freshness issues: {:?}", issues);
        Ok(())
    }

    #[test]
    fn test_verify_source_freshness_local_mismatch() -> anyhow::Result<()>
    {
        let data_dir = tempfile::TempDir::new()?;
        let source_dir = tempfile::TempDir::new()?;

        fs::write(data_dir.path().join("templates.yml"), "version: 4\nagents: {}\nlanguages: {}\n")?;
        fs::write(source_dir.path().join("templates.yml"), "version: 5\nagents: {}\nlanguages: {}\n")?;

        let manager = TemplateManager { config_dir: data_dir.path().to_path_buf() };
        let mut issues: Vec<String> = Vec::new();

        manager.verify_source_freshness(source_dir.path().to_str().unwrap(), &mut issues);

        assert!(issues.iter().any(|i| i.contains("differ from source")) == true);
        Ok(())
    }

    #[test]
    fn test_collect_duplicate_target_issues_no_dups() -> anyhow::Result<()>
    {
        let data_dir = tempfile::TempDir::new()?;

        let yaml = "version: 5\nmain:\n  source: AGENTS.md\n  target: '$workspace/AGENTS.md'\nagents:\n  cursor:\n    instructions:\n      - source: \
                    cursorrules.md\n        target: '$workspace/.cursorrules'\nlanguages: {}\n";
        fs::write(data_dir.path().join("templates.yml"), yaml)?;
        let config = crate::template_engine::load_template_config(&data_dir.path().to_path_buf())?;

        let issues = collect_duplicate_target_issues(&config, data_dir.path());
        assert!(issues.is_empty() == true);
        Ok(())
    }
}
