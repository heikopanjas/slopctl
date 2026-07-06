//! Partial update command: refresh individual template files or skills

use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    path::{Path, PathBuf}
};

use owo_colors::OwoColorize;

use super::TemplateManager;
use crate::{
    Result, agent_defaults,
    file_tracker::{AGENT_ALL, FileStatus, FileTracker},
    template_engine::{self, ResolvedFile, ResolvedFiles, TemplateEngine, UpdateOptions, normalize_path}
};

impl TemplateManager
{
    /// Refreshes individual template files or skills from the global catalog
    ///
    /// Unlike `init`, which installs a language's complete file set, this refreshes
    /// only the selected targets. It reuses `TemplateEngine::resolve_all_files()` so
    /// routing (native vs cross-client skill dirs, includes, shared groups) matches
    /// what `init` produced. Selected targets are overwritten directly; a locally
    /// customized or untracked target is skipped unless `force` is set. When a skill
    /// is refreshed, slopctl-managed files removed upstream are pruned from disk and
    /// the tracker; user-added untracked files inside the skill directory are preserved.
    ///
    /// # Arguments
    ///
    /// * `files` - Workspace file paths to refresh
    /// * `skills` - Skill names to refresh
    /// * `lang` - Language scope override (defaults to the installed language)
    /// * `agent` - Agent scope override (defaults to detected agents)
    /// * `force` - Overwrite customized or untracked files
    /// * `dry_run` - Preview changes without applying them
    ///
    /// # Errors
    ///
    /// Returns an error if global templates are missing, a scope override is unknown,
    /// a selector matches nothing, a customized file is targeted without `force`, or
    /// file I/O fails
    pub fn update_partial(&self, files: &[String], skills: &[String], lang: Option<&str>, agent: Option<&str>, force: bool, dry_run: bool) -> Result<()>
    {
        require!(
            self.has_global_templates() == true,
            Err(anyhow::anyhow!("Global templates not found. Please run 'slopctl templates --update' first to download templates."))
        );

        let workspace = std::env::current_dir()?;
        let _ = self.try_migrate_tracker(&workspace);

        let config = template_engine::load_template_config(&self.config_dir)?;
        let agent_catalog = agent_defaults::load_agent_catalog_from_dir(&self.config_dir)?;

        // Resolve language scope: explicit override must exist; an auto-detected stale
        // language is quietly dropped so resolution still covers agent/top-level targets.
        let tracker = FileTracker::new(&workspace)?;
        let effective_lang = match lang
        {
            | Some(l) =>
            {
                require!(
                    config.languages.contains_key(l) == true,
                    Err(anyhow::anyhow!("Language '{}' not found in templates.yml.\nAvailable languages: {}", l, sorted_keys(config.languages.keys())))
                );
                Some(l.to_string())
            }
            | None => tracker.get_installed_language().filter(|l| config.languages.contains_key(l))
        };

        // Resolve agent scope: explicit override must exist; otherwise use detected
        // agents that are present in the catalog. Falls back to a single agent-less pass.
        let effective_agents: Vec<Option<String>> = match agent
        {
            | Some(a) =>
            {
                require!(
                    config.agents.contains_key(a) == true,
                    Err(anyhow::anyhow!("Agent '{}' not found in templates.yml.\nAvailable agents: {}", a, sorted_keys(config.agents.keys())))
                );
                vec![Some(a.to_string())]
            }
            | None =>
            {
                let detected: Vec<Option<String>> = agent_defaults::detect_all_installed_agents_from_catalog(&agent_catalog, &workspace)
                    .into_iter()
                    .filter(|name| config.agents.contains_key(name))
                    .map(Some)
                    .collect();
                if detected.is_empty() == true
                {
                    vec![None]
                }
                else
                {
                    detected
                }
            }
        };

        // Build the candidate universe by resolving each effective agent scope and
        // unioning the results. The owned `ResolvedFiles` values are retained so their
        // temp directories (GitHub-downloaded sources) survive until the copy phase.
        let engine = TemplateEngine::new(&self.config_dir);
        let mut resolved_sets: Vec<ResolvedFiles> = Vec::with_capacity(effective_agents.len());
        for agent_opt in &effective_agents
        {
            let options = UpdateOptions { lang: effective_lang.as_deref(), agent: agent_opt.as_deref(), mission: None, force, dry_run };
            resolved_sets.push(engine.resolve_all_files(&options)?);
        }

        let main_target = resolved_sets.first().map(|set| normalize_path(&set.context.target));

        let mut candidates: BTreeMap<PathBuf, &ResolvedFile> = BTreeMap::new();
        for set in &resolved_sets
        {
            for entry in &set.files
            {
                candidates.insert(normalize_path(&entry.target), entry);
            }
        }

        let mut selected: BTreeMap<PathBuf, &ResolvedFile> = BTreeMap::new();
        let mut unmatched: Vec<String> = Vec::new();

        for requested in files
        {
            let raw = Path::new(requested);
            let resolved = if raw.is_absolute() == true
            {
                normalize_path(raw)
            }
            else
            {
                normalize_path(&workspace.join(raw))
            };

            if main_target.as_ref() == Some(&resolved)
            {
                return Err(anyhow::anyhow!(
                    "'{}' is the merged AGENTS.md and cannot be refreshed as a single file. Use 'slopctl merge' or 'slopctl init' instead.", requested
                ));
            }

            if let Some(entry) = candidates.get(&resolved)
            {
                selected.insert(resolved, entry);
            }
            else
            {
                unmatched.push(format!("file '{}'", requested));
            }
        }

        for requested in skills
        {
            let mut matched = false;
            for (target, entry) in &candidates
            {
                if skill_name_of(target).as_deref() == Some(requested.as_str())
                {
                    selected.insert(target.clone(), entry);
                    matched = true;
                }
            }
            if matched == false
            {
                unmatched.push(format!("skill '{}'", requested));
            }
        }

        if unmatched.is_empty() == false
        {
            return Err(anyhow::anyhow!(
                "No template match for: {}.\n\nAvailable files:\n{}\n\nAvailable skills:\n{}",
                unmatched.join(", "),
                available_list(available_files(&candidates, &workspace)),
                available_list(available_skills(&candidates))
            ));
        }

        let stale = collect_stale_skill_files(&tracker, &workspace, skills, &selected);

        // Guard customized or untracked targets unless forced.
        let mut blocked: Vec<String> = Vec::new();
        for target in selected.keys().chain(stale.iter())
        {
            if target.exists() == true && force == false
            {
                match tracker.check_modification(target)?
                {
                    | FileStatus::Modified | FileStatus::NotTracked => blocked.push(display_path(target, &workspace)),
                    | FileStatus::Unmodified | FileStatus::Deleted =>
                    {}
                }
            }
        }

        if blocked.is_empty() == false
        {
            let details = blocked.iter().map(|path| format!("- {}", path)).collect::<Vec<_>>().join("\n");
            return Err(anyhow::anyhow!("The following targets are customized or untracked:\n{}\n\nUse --force to overwrite them.", details));
        }

        if dry_run == true
        {
            println!("{} Files that would be refreshed:", "→".blue());
            for target in selected.keys()
            {
                let display = display_path(target, &workspace);
                if target.exists() == true
                {
                    println!("  {} {} (would be overwritten)", "●".yellow(), display.yellow());
                }
                else
                {
                    println!("  {} {} (would be created)", "●".green(), display.green());
                }
            }
            if stale.is_empty() == false
            {
                println!("\n{} Files that would be removed (stale):", "→".blue());
                for target in &stale
                {
                    println!("  {} {} (would be removed)", "●".red(), display_path(target, &workspace).red());
                }
            }
            println!("\n{} Dry run complete. No files were modified.", "✓".green());
            return Ok(());
        }

        let mut file_tracker = FileTracker::new(&workspace)?;
        println!("{} Refreshing selected templates", "→".blue());
        for (target, entry) in &selected
        {
            crate::utils::copy_file_with_mkdir(&entry.source, target)?;
            let sha = FileTracker::calculate_sha256(target)?;
            let category = categorize_target(target, &entry.agent);
            file_tracker.record_installation(target, sha, config.version, entry.lang.clone(), entry.agent.clone(), category);
            println!("  {} {}", "✓".green(), display_path(target, &workspace).yellow());
        }
        for target in &stale
        {
            if target.exists() == true
            {
                crate::utils::remove_file_and_cleanup_parents(target)?;
            }
            file_tracker.remove_entry(target);
            println!("  {} {} (removed stale)", "✓".green(), display_path(target, &workspace).red());
        }
        file_tracker.save()?;

        println!("{} Refresh complete.", "✓".green());
        Ok(())
    }
}

/// Returns the skill name for a target path that lives under a `skills/<name>/` segment
fn skill_name_of(target: &Path) -> Option<String>
{
    TemplateManager::extract_skill_name_from_path(target)
}

/// Collects tracked skill files for the requested skills that are absent from the new source set
fn collect_stale_skill_files(tracker: &FileTracker, workspace: &Path, requested_skills: &[String], selected: &BTreeMap<PathBuf, &ResolvedFile>) -> Vec<PathBuf>
{
    if requested_skills.is_empty() == true
    {
        return Vec::new();
    }

    let requested: HashSet<&str> = requested_skills.iter().map(|s| s.as_str()).collect();
    let new_targets: BTreeSet<PathBuf> =
        selected.keys().filter(|target| skill_name_of(target).is_some_and(|name| requested.contains(name.as_str()))).cloned().collect();

    let mut stale: Vec<PathBuf> = tracker
        .get_entries_by_category("skill")
        .into_iter()
        .filter_map(|(relative_path, _meta)| {
            let absolute = normalize_path(&workspace.join(relative_path));
            if skill_name_of(&absolute).is_some_and(|name| requested.contains(name.as_str())) == true && new_targets.contains(&absolute) == false
            {
                Some(absolute)
            }
            else
            {
                None
            }
        })
        .collect();
    stale.sort();
    stale.dedup();
    stale
}

/// Determines the tracking category for a refreshed target
fn categorize_target(target: &Path, agent: &str) -> String
{
    if skill_name_of(target).is_some() == true
    {
        "skill".to_string()
    }
    else if target.to_string_lossy().contains(".git") == true
    {
        "integration".to_string()
    }
    else if agent != AGENT_ALL
    {
        "agent".to_string()
    }
    else
    {
        "language".to_string()
    }
}

/// Renders a target path relative to the workspace for display
fn display_path(target: &Path, workspace: &Path) -> String
{
    target.strip_prefix(workspace).unwrap_or(target).display().to_string()
}

/// Collects sorted, workspace-relative display paths for non-skill candidates
fn available_files(candidates: &BTreeMap<PathBuf, &ResolvedFile>, workspace: &Path) -> Vec<String>
{
    let mut names: Vec<String> = candidates.keys().filter(|target| skill_name_of(target).is_none()).map(|target| display_path(target, workspace)).collect();
    names.sort();
    names.dedup();
    names
}

/// Collects sorted, unique skill names from the candidate set
fn available_skills(candidates: &BTreeMap<PathBuf, &ResolvedFile>) -> Vec<String>
{
    let mut names: Vec<String> = candidates.keys().filter_map(|target| skill_name_of(target)).collect();
    names.sort();
    names.dedup();
    names
}

/// Formats a list for error output, using a placeholder when empty
fn available_list(items: Vec<String>) -> String
{
    if items.is_empty() == true
    {
        "  (none)".to_string()
    }
    else
    {
        items.iter().map(|item| format!("  {}", item)).collect::<Vec<_>>().join("\n")
    }
}

/// Joins sorted map keys into a comma-separated list for error messages
fn sorted_keys<'a, I>(keys: I) -> String
where I: Iterator<Item = &'a String>
{
    let mut names: Vec<&str> = keys.map(|k| k.as_str()).collect();
    names.sort();
    names.join(", ")
}

#[cfg(test)]
mod tests
{
    use std::fs;

    use super::*;
    use crate::template_manager::cwd_test_guard;

    /// Writes a minimal global template catalog with one language file, one language
    /// skill, and one top-level skill, plus synthetic agent defaults for `bogus`.
    fn setup_config(config_dir: &Path) -> anyhow::Result<()>
    {
        setup_config_with_git_hook(config_dir, true)
    }

    /// Like `setup_config`, but optionally includes `scripts/hook.sh` in git-workflow.
    fn setup_config_with_git_hook(config_dir: &Path, include_hook: bool) -> anyhow::Result<()>
    {
        let yaml = "version: 5\nmain:\n  source: AGENTS.md\n  target: '$workspace/AGENTS.md'\nagents:\n  bogus: {}\nlanguages:\n  Rust++:\n    files:\n      - \
                    source: rpp-format.toml\n        target: '$workspace/.rpp.toml'\n    skills:\n      - source: 'skills/rpp-conventions'\n        target: \
                    '$workspace'\nskills:\n  - source: 'skills/git-workflow'\n    target: '$workspace'\nintegration: {}\n";
        fs::write(config_dir.join("templates.yml"), yaml)?;
        fs::write(config_dir.join("AGENTS.md"), "<!-- SLOPCTL-TEMPLATE -->\n# Project\n")?;
        fs::write(config_dir.join("rpp-format.toml"), "max_width = 120\n")?;

        let git_skill = config_dir.join("skills/git-workflow");
        fs::create_dir_all(git_skill.join("scripts"))?;
        fs::write(git_skill.join("SKILL.md"), "# Git Workflow\n")?;
        if include_hook == true
        {
            fs::write(git_skill.join("scripts/hook.sh"), "#!/bin/sh\necho hook\n")?;
        }

        let rpp_skill = config_dir.join("skills/rpp-conventions");
        fs::create_dir_all(&rpp_skill)?;
        fs::write(rpp_skill.join("SKILL.md"), "# Rust++ Conventions\n")?;

        let defaults = "version: 1\nagents:\n  - name: bogus\n    markers:\n      - .bogus\n    prompt_dir: '$workspace/.bogus/prompts'\n    skill_dir: \
                        '$workspace/.bogus/skills'\n    reads_cross_client_skills: false\n";
        fs::write(config_dir.join(agent_defaults::AGENT_DEFAULTS_FILE), defaults)?;
        Ok(())
    }

    fn install_git_workflow_skill(manager: &TemplateManager) -> anyhow::Result<()>
    {
        manager.update_partial(&[], &["git-workflow".to_string()], None, None, false, false)
    }

    #[test]
    fn test_update_partial_refreshes_language_file() -> anyhow::Result<()>
    {
        let _guard = cwd_test_guard();
        let config_dir = tempfile::TempDir::new()?;
        let workspace = tempfile::TempDir::new()?;
        setup_config(config_dir.path())?;
        std::env::set_current_dir(workspace.path())?;

        let manager = TemplateManager { config_dir: config_dir.path().to_path_buf() };
        let result = manager.update_partial(&[".rpp.toml".to_string()], &[], Some("Rust++"), None, false, false);
        let _ = std::env::set_current_dir(std::env::temp_dir());

        result?;
        let target = workspace.path().join(".rpp.toml");
        assert!(target.exists() == true);
        assert_eq!(fs::read_to_string(&target)?, "max_width = 120\n");

        let tracker = FileTracker::new(workspace.path())?;
        assert!(tracker.get_metadata(&target).is_some() == true);
        Ok(())
    }

    #[test]
    fn test_update_partial_refreshes_skill() -> anyhow::Result<()>
    {
        let _guard = cwd_test_guard();
        let config_dir = tempfile::TempDir::new()?;
        let workspace = tempfile::TempDir::new()?;
        setup_config(config_dir.path())?;
        std::env::set_current_dir(workspace.path())?;

        let manager = TemplateManager { config_dir: config_dir.path().to_path_buf() };
        let result = manager.update_partial(&[], &["git-workflow".to_string()], None, None, false, false);
        let _ = std::env::set_current_dir(std::env::temp_dir());

        result?;
        let skill_md = workspace.path().join(".agents/skills/git-workflow/SKILL.md");
        let hook = workspace.path().join(".agents/skills/git-workflow/scripts/hook.sh");
        assert!(skill_md.exists() == true);
        assert!(hook.exists() == true);
        Ok(())
    }

    #[test]
    fn test_update_partial_customized_without_force_errors() -> anyhow::Result<()>
    {
        let _guard = cwd_test_guard();
        let config_dir = tempfile::TempDir::new()?;
        let workspace = tempfile::TempDir::new()?;
        setup_config(config_dir.path())?;
        std::env::set_current_dir(workspace.path())?;

        let target = workspace.path().join(".rpp.toml");
        fs::write(&target, "custom = true\n")?;

        let manager = TemplateManager { config_dir: config_dir.path().to_path_buf() };
        let blocked = manager.update_partial(&[".rpp.toml".to_string()], &[], Some("Rust++"), None, false, false);
        let forced = if blocked.is_err() == true
        {
            manager.update_partial(&[".rpp.toml".to_string()], &[], Some("Rust++"), None, true, false)
        }
        else
        {
            Ok(())
        };
        let _ = std::env::set_current_dir(std::env::temp_dir());

        assert!(blocked.is_err() == true);
        let message = blocked.unwrap_err().to_string();
        assert!(message.contains("--force") == true);
        forced?;
        assert_eq!(fs::read_to_string(&target)?, "max_width = 120\n");
        Ok(())
    }

    #[test]
    fn test_update_partial_unknown_selector_errors() -> anyhow::Result<()>
    {
        let _guard = cwd_test_guard();
        let config_dir = tempfile::TempDir::new()?;
        let workspace = tempfile::TempDir::new()?;
        setup_config(config_dir.path())?;
        std::env::set_current_dir(workspace.path())?;

        let manager = TemplateManager { config_dir: config_dir.path().to_path_buf() };
        let result = manager.update_partial(&["nope.toml".to_string()], &["ghost".to_string()], Some("Rust++"), None, false, false);
        let _ = std::env::set_current_dir(std::env::temp_dir());

        assert!(result.is_err() == true);
        let message = result.unwrap_err().to_string();
        assert!(message.contains("No template match") == true);
        assert!(message.contains("nope.toml") == true);
        assert!(message.contains("ghost") == true);
        Ok(())
    }

    #[test]
    fn test_update_partial_dry_run_writes_nothing() -> anyhow::Result<()>
    {
        let _guard = cwd_test_guard();
        let config_dir = tempfile::TempDir::new()?;
        let workspace = tempfile::TempDir::new()?;
        setup_config(config_dir.path())?;
        std::env::set_current_dir(workspace.path())?;

        let manager = TemplateManager { config_dir: config_dir.path().to_path_buf() };
        let result = manager.update_partial(&[".rpp.toml".to_string()], &[], Some("Rust++"), None, false, true);
        let _ = std::env::set_current_dir(std::env::temp_dir());

        result?;
        assert!(workspace.path().join(".rpp.toml").exists() == false);
        Ok(())
    }

    #[test]
    fn test_update_partial_rejects_agents_md() -> anyhow::Result<()>
    {
        let _guard = cwd_test_guard();
        let config_dir = tempfile::TempDir::new()?;
        let workspace = tempfile::TempDir::new()?;
        setup_config(config_dir.path())?;
        std::env::set_current_dir(workspace.path())?;

        let manager = TemplateManager { config_dir: config_dir.path().to_path_buf() };
        let result = manager.update_partial(&["AGENTS.md".to_string()], &[], None, None, false, false);
        let _ = std::env::set_current_dir(std::env::temp_dir());

        assert!(result.is_err() == true);
        let message = result.unwrap_err().to_string();
        assert!(message.contains("AGENTS.md") == true);
        assert!(message.contains("merge") == true);
        Ok(())
    }

    #[test]
    fn test_update_partial_prunes_upstream_removed_skill_file() -> anyhow::Result<()>
    {
        let _guard = cwd_test_guard();
        let config_dir = tempfile::TempDir::new()?;
        let workspace = tempfile::TempDir::new()?;
        setup_config(config_dir.path())?;
        std::env::set_current_dir(workspace.path())?;

        let manager = TemplateManager { config_dir: config_dir.path().to_path_buf() };
        install_git_workflow_skill(&manager)?;

        let hook = workspace.path().join(".agents/skills/git-workflow/scripts/hook.sh");
        assert!(hook.exists() == true);

        fs::remove_file(config_dir.path().join("skills/git-workflow/scripts/hook.sh"))?;
        manager.update_partial(&[], &["git-workflow".to_string()], None, None, false, false)?;
        let _ = std::env::set_current_dir(std::env::temp_dir());

        assert!(hook.exists() == false);
        let tracker = FileTracker::new(workspace.path())?;
        assert!(tracker.get_metadata(&hook).is_none() == true);
        assert!(workspace.path().join(".agents/skills/git-workflow/SKILL.md").exists() == true);
        Ok(())
    }

    #[test]
    fn test_update_partial_preserves_untracked_user_skill_file() -> anyhow::Result<()>
    {
        let _guard = cwd_test_guard();
        let config_dir = tempfile::TempDir::new()?;
        let workspace = tempfile::TempDir::new()?;
        setup_config(config_dir.path())?;
        std::env::set_current_dir(workspace.path())?;

        let manager = TemplateManager { config_dir: config_dir.path().to_path_buf() };
        install_git_workflow_skill(&manager)?;

        let notes = workspace.path().join(".agents/skills/git-workflow/notes.md");
        fs::write(&notes, "# My notes\n")?;

        fs::remove_file(config_dir.path().join("skills/git-workflow/scripts/hook.sh"))?;
        manager.update_partial(&[], &["git-workflow".to_string()], None, None, false, false)?;
        let _ = std::env::set_current_dir(std::env::temp_dir());

        assert!(notes.exists() == true);
        assert_eq!(fs::read_to_string(&notes)?, "# My notes\n");
        Ok(())
    }

    #[test]
    fn test_update_partial_modified_stale_skill_file_requires_force() -> anyhow::Result<()>
    {
        let _guard = cwd_test_guard();
        let config_dir = tempfile::TempDir::new()?;
        let workspace = tempfile::TempDir::new()?;
        setup_config(config_dir.path())?;
        std::env::set_current_dir(workspace.path())?;

        let manager = TemplateManager { config_dir: config_dir.path().to_path_buf() };
        install_git_workflow_skill(&manager)?;

        let hook = workspace.path().join(".agents/skills/git-workflow/scripts/hook.sh");
        fs::write(&hook, "#!/bin/sh\necho customized\n")?;

        fs::remove_file(config_dir.path().join("skills/git-workflow/scripts/hook.sh"))?;
        let blocked = manager.update_partial(&[], &["git-workflow".to_string()], None, None, false, false);
        assert!(blocked.is_err() == true);
        assert!(blocked.unwrap_err().to_string().contains("--force") == true);
        assert!(hook.exists() == true);

        manager.update_partial(&[], &["git-workflow".to_string()], None, None, true, false)?;
        let _ = std::env::set_current_dir(std::env::temp_dir());

        assert!(hook.exists() == false);
        Ok(())
    }

    #[test]
    fn test_update_partial_dry_run_reports_stale_without_deleting() -> anyhow::Result<()>
    {
        let _guard = cwd_test_guard();
        let config_dir = tempfile::TempDir::new()?;
        let workspace = tempfile::TempDir::new()?;
        setup_config(config_dir.path())?;
        std::env::set_current_dir(workspace.path())?;

        let manager = TemplateManager { config_dir: config_dir.path().to_path_buf() };
        install_git_workflow_skill(&manager)?;

        let hook = workspace.path().join(".agents/skills/git-workflow/scripts/hook.sh");
        fs::remove_file(config_dir.path().join("skills/git-workflow/scripts/hook.sh"))?;
        manager.update_partial(&[], &["git-workflow".to_string()], None, None, false, true)?;
        let _ = std::env::set_current_dir(std::env::temp_dir());

        assert!(hook.exists() == true);
        let tracker = FileTracker::new(workspace.path())?;
        assert!(tracker.get_metadata(&hook).is_some() == true);
        Ok(())
    }
}
