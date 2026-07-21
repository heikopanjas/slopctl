//! Partial update command: refresh individual template files or skills

use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    path::{Path, PathBuf}
};

use owo_colors::OwoColorize;

use super::TemplateManager;
use crate::{
    Result, agent_defaults,
    agent_defaults::AgentCatalog,
    bom::TemplateConfig,
    file_tracker::{FileStatus, FileTracker},
    template_engine::{self, PartialSelectors, ResolvedFile, ResolvedFiles, TemplateEngine, UpdateOptions, normalize_path}
};

impl TemplateManager
{
    /// Refreshes individual template files or skills from the global catalog
    ///
    /// Unlike `init`, which installs a language's complete file set, this refreshes
    /// only the selected targets from the local global template cache (no remote fetches).
    /// It uses scoped `TemplateEngine::resolve_all_files()` so only matching files/skills
    /// are resolved, while routing (native vs cross-client skill dirs, includes, shared
    /// groups) matches what `init` produced. Selected targets are overwritten directly; a locally
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

        let effective_agents = effective_agent_scope(agent, &config, &agent_catalog, &workspace)?;

        // Build partial selectors and resolve only matching targets from the local cache.
        let file_selectors: HashSet<String> = files.iter().cloned().collect();
        let skill_selectors: HashSet<String> = skills.iter().cloned().collect();
        let partial = PartialSelectors { files: &file_selectors, skills: &skill_selectors };

        // Build the candidate universe by resolving each effective agent scope and
        // unioning the results. The owned `ResolvedFiles` values are retained so their
        // temp directories (GitHub-downloaded sources) survive until the copy phase.
        let engine = TemplateEngine::new(&self.config_dir);
        let mut resolved_sets: Vec<ResolvedFiles> = Vec::with_capacity(effective_agents.len());
        for agent_opt in &effective_agents
        {
            let options = UpdateOptions {
                lang: effective_lang.as_deref(),
                agent: agent_opt.as_deref(),
                mission: None,
                force,
                dry_run,
                partial: Some(&partial),
                local_cache_only: true
            };
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
            file_tracker.record_installation_with_owners(target, sha, config.version, &entry.lang, &entry.agent, entry.category.clone());
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

    /// Refreshes the whole workspace from the global catalog
    ///
    /// Invoked by `slopctl update` without `--file`/`--skill` selectors. Resolves the
    /// full template set for every installed language and detected agent (overridable
    /// via `lang`/`agent`), then brings the workspace up to the cached template state:
    /// missing and deleted files are restored, unmodified files are overwritten, and
    /// locally modified or untracked files are skipped with a report unless `force`
    /// is set. Tracked skill files removed upstream are pruned; user-added files are
    /// preserved. AGENTS.md is never refreshed here; `slopctl merge` is its update path.
    ///
    /// # Arguments
    ///
    /// * `lang` - Language scope override (defaults to all installed languages)
    /// * `agent` - Agent scope override (defaults to detected agents)
    /// * `force` - Also overwrite customized or untracked files
    /// * `dry_run` - Preview changes without applying them
    ///
    /// # Errors
    ///
    /// Returns an error if global templates are missing, a scope override is unknown,
    /// or file I/O fails
    pub fn update_full(&self, lang: Option<&str>, agent: Option<&str>, force: bool, dry_run: bool) -> Result<()>
    {
        require!(
            self.has_global_templates() == true,
            Err(anyhow::anyhow!("Global templates not found. Please run 'slopctl templates --update' first to download templates."))
        );

        let workspace = std::env::current_dir()?;
        let _ = self.try_migrate_tracker(&workspace);

        let config = template_engine::load_template_config(&self.config_dir)?;
        let agent_catalog = agent_defaults::load_agent_catalog_from_dir(&self.config_dir)?;
        let tracker = FileTracker::new(&workspace)?;

        // Language scope: explicit override must exist; otherwise refresh every
        // installed language that is still present in the catalog.
        let effective_langs: Vec<Option<String>> = match lang
        {
            | Some(l) =>
            {
                require!(
                    config.languages.contains_key(l) == true,
                    Err(anyhow::anyhow!("Language '{}' not found in templates.yml.\nAvailable languages: {}", l, sorted_keys(config.languages.keys())))
                );
                vec![Some(l.to_string())]
            }
            | None =>
            {
                let installed: Vec<Option<String>> = tracker.get_installed_languages().into_iter().filter(|l| config.languages.contains_key(l)).map(Some).collect();
                if installed.is_empty() == true
                {
                    vec![None]
                }
                else
                {
                    installed
                }
            }
        };

        let effective_agents = effective_agent_scope(agent, &config, &agent_catalog, &workspace)?;

        // Resolve the full template set per (language, agent) combination and union
        // the candidates. AGENTS.md never appears here; it is carried in the resolved
        // context and updated only through 'slopctl merge'.
        let engine = TemplateEngine::new(&self.config_dir);
        let mut resolved_sets: Vec<ResolvedFiles> = Vec::with_capacity(effective_langs.len() * effective_agents.len());
        for lang_opt in &effective_langs
        {
            for agent_opt in &effective_agents
            {
                let options =
                    UpdateOptions { lang: lang_opt.as_deref(), agent: agent_opt.as_deref(), mission: None, force, dry_run, partial: None, local_cache_only: true };
                resolved_sets.push(engine.resolve_all_files(&options)?);
            }
        }

        let mut candidates: BTreeMap<PathBuf, &ResolvedFile> = BTreeMap::new();
        for set in &resolved_sets
        {
            for entry in &set.files
            {
                candidates.insert(normalize_path(&entry.target), entry);
            }
        }

        // Prune tracked files of still-resolved skills that were removed upstream.
        // Skills that left the catalog entirely are untouched; 'remove' owns that case.
        let resolved_skill_names: Vec<String> = candidates.keys().filter_map(|target| skill_name_of(target)).collect::<BTreeSet<String>>().into_iter().collect();
        let stale = collect_stale_skill_files(&tracker, &workspace, &resolved_skill_names, &candidates);

        // Classify: refresh unmodified/missing/deleted targets, skip modified or
        // untracked ones (kept with a report; --force overwrites), drop up-to-date copies.
        let mut to_refresh: Vec<(&PathBuf, &ResolvedFile)> = Vec::new();
        let mut skipped: Vec<String> = Vec::new();
        let mut up_to_date = 0usize;
        for (target, entry) in &candidates
        {
            if target.exists() == false
            {
                // Agent files are created only for slopctl-installed agents: marker
                // detection alone (the agent app created its own marker dir) must not
                // conjure prompt or instruction files the user never installed.
                if entry.category != "agent" || tracker.get_metadata(target).is_some() == true
                {
                    to_refresh.push((target, entry));
                }
            }
            else
            {
                match tracker.check_modification(target)?
                {
                    | FileStatus::Unmodified | FileStatus::Deleted =>
                    {
                        if FileTracker::calculate_sha256(&entry.source)? == FileTracker::calculate_sha256(target)?
                        {
                            up_to_date += 1;
                        }
                        else
                        {
                            to_refresh.push((target, entry));
                        }
                    }
                    | FileStatus::Modified | FileStatus::NotTracked =>
                    {
                        if force == true
                        {
                            to_refresh.push((target, entry));
                        }
                        else
                        {
                            skipped.push(display_path(target, &workspace));
                        }
                    }
                }
            }
        }

        // Modified stale files are kept unless forced; user edits are not slopctl's to delete.
        let mut stale_to_remove: Vec<&PathBuf> = Vec::new();
        for target in &stale
        {
            if force == false && tracker.check_modification(target)? == FileStatus::Modified
            {
                skipped.push(display_path(target, &workspace));
            }
            else
            {
                stale_to_remove.push(target);
            }
        }

        if to_refresh.is_empty() == true && stale_to_remove.is_empty() == true
        {
            println!("{} Workspace is up to date ({} file(s) checked)", "✓".green(), up_to_date + skipped.len());
            report_skipped(&skipped);
            println!("{} AGENTS.md is not refreshed by update; use 'slopctl merge' to update it", "→".blue());
            return Ok(());
        }

        if dry_run == true
        {
            println!("{} Files that would be refreshed:", "→".blue());
            for (target, _) in &to_refresh
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
            for path in &skipped
            {
                println!("  {} {} (skipped - local modifications preserved)", "○".yellow(), path);
            }
            if stale_to_remove.is_empty() == false
            {
                println!("\n{} Files that would be removed (stale):", "→".blue());
                for target in &stale_to_remove
                {
                    println!("  {} {} (would be removed)", "●".red(), display_path(target, &workspace).red());
                }
            }
            println!("\n{} Dry run complete. No files were modified.", "✓".green());
            return Ok(());
        }

        let mut file_tracker = FileTracker::new(&workspace)?;
        println!("{} Refreshing workspace templates", "→".blue());
        for (target, entry) in &to_refresh
        {
            crate::utils::copy_file_with_mkdir(&entry.source, target)?;
            let sha = FileTracker::calculate_sha256(target)?;
            file_tracker.record_installation_with_owners(target, sha, config.version, &entry.lang, &entry.agent, entry.category.clone());
            println!("  {} {}", "✓".green(), display_path(target, &workspace).yellow());
        }
        for target in &stale_to_remove
        {
            if target.exists() == true
            {
                crate::utils::remove_file_and_cleanup_parents(target)?;
            }
            file_tracker.remove_entry(target);
            println!("  {} {} (removed stale)", "✓".green(), display_path(target, &workspace).red());
        }
        file_tracker.save()?;

        report_skipped(&skipped);
        println!("{} Refreshed {} file(s); {} already up to date", "✓".green(), to_refresh.len(), up_to_date);
        println!("{} AGENTS.md is not refreshed by update; use 'slopctl merge' to update it", "→".blue());
        Ok(())
    }
}

/// Resolves the effective agent scope for update commands
///
/// An explicit override must exist in the catalog; otherwise detected agents present
/// in the catalog are used, falling back to a single agent-less pass.
pub(super) fn effective_agent_scope(agent: Option<&str>, config: &TemplateConfig, agent_catalog: &AgentCatalog, workspace: &Path) -> Result<Vec<Option<String>>>
{
    match agent
    {
        | Some(a) =>
        {
            require!(
                config.agents.contains_key(a) == true,
                Err(anyhow::anyhow!("Agent '{}' not found in templates.yml.\nAvailable agents: {}", a, sorted_keys(config.agents.keys())))
            );
            Ok(vec![Some(a.to_string())])
        }
        | None =>
        {
            let detected: Vec<Option<String>> = agent_defaults::detect_all_installed_agents_from_catalog(agent_catalog, workspace)
                .into_iter()
                .filter(|name| config.agents.contains_key(name))
                .map(Some)
                .collect();
            if detected.is_empty() == true
            {
                Ok(vec![None])
            }
            else
            {
                Ok(detected)
            }
        }
    }
}

/// Prints the skipped-files report shared by the up-to-date and refresh paths
fn report_skipped(skipped: &[String])
{
    for path in skipped
    {
        println!("  {} {} (skipped - local modifications preserved; use 'slopctl merge' or --force)", "○".yellow(), path.yellow());
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

    #[test]
    fn test_update_partial_skill_only_leaves_language_file_untouched() -> anyhow::Result<()>
    {
        let _guard = cwd_test_guard();
        let config_dir = tempfile::TempDir::new()?;
        let workspace = tempfile::TempDir::new()?;
        setup_config(config_dir.path())?;
        std::env::set_current_dir(workspace.path())?;

        let lang_file = workspace.path().join(".rpp.toml");
        fs::write(&lang_file, "custom = true\n")?;

        let manager = TemplateManager { config_dir: config_dir.path().to_path_buf() };
        manager.update_partial(&[], &["git-workflow".to_string()], None, None, false, false)?;
        let _ = std::env::set_current_dir(std::env::temp_dir());

        assert_eq!(fs::read_to_string(&lang_file)?, "custom = true\n");
        Ok(())
    }

    #[test]
    fn test_update_partial_skill_only_does_not_install_other_skills() -> anyhow::Result<()>
    {
        let _guard = cwd_test_guard();
        let config_dir = tempfile::TempDir::new()?;
        let workspace = tempfile::TempDir::new()?;
        setup_config(config_dir.path())?;
        std::env::set_current_dir(workspace.path())?;

        let manager = TemplateManager { config_dir: config_dir.path().to_path_buf() };
        manager.update_partial(&[], &["git-workflow".to_string()], None, None, false, false)?;
        let _ = std::env::set_current_dir(std::env::temp_dir());

        assert!(workspace.path().join(".agents/skills/git-workflow/SKILL.md").exists() == true);
        assert!(workspace.path().join(".agents/skills/rpp-conventions/SKILL.md").exists() == false);
        Ok(())
    }

    #[test]
    fn test_update_partial_missing_cached_skill_errors() -> anyhow::Result<()>
    {
        let _guard = cwd_test_guard();
        let config_dir = tempfile::TempDir::new()?;
        let workspace = tempfile::TempDir::new()?;
        setup_config(config_dir.path())?;
        fs::remove_dir_all(config_dir.path().join("skills/git-workflow"))?;
        std::env::set_current_dir(workspace.path())?;

        let manager = TemplateManager { config_dir: config_dir.path().to_path_buf() };
        let result = manager.update_partial(&[], &["git-workflow".to_string()], None, None, false, false);
        let _ = std::env::set_current_dir(std::env::temp_dir());

        assert!(result.is_err() == true);
        let message = result.unwrap_err().to_string();
        assert!(message.contains("templates --update") == true);
        Ok(())
    }

    #[test]
    fn test_update_partial_local_cache_only_no_github_hook() -> anyhow::Result<()>
    {
        let _guard = cwd_test_guard();
        let config_dir = tempfile::TempDir::new()?;
        let workspace = tempfile::TempDir::new()?;
        setup_config(config_dir.path())?;
        std::env::set_current_dir(workspace.path())?;

        let _hook = crate::github::set_test_hooks(
            Box::new(|_| panic!("update must not call GitHub list_directory_contents")),
            Box::new(|_| panic!("update must not call GitHub download_file"))
        );

        let manager = TemplateManager { config_dir: config_dir.path().to_path_buf() };
        manager.update_partial(&[], &["git-workflow".to_string()], None, None, false, false)?;
        let _ = std::env::set_current_dir(std::env::temp_dir());
        Ok(())
    }

    // -- update_full --

    #[test]
    fn test_update_full_refreshes_unmodified_files() -> anyhow::Result<()>
    {
        let _guard = cwd_test_guard();
        let config_dir = tempfile::TempDir::new()?;
        let workspace = tempfile::TempDir::new()?;
        setup_config(config_dir.path())?;
        std::env::set_current_dir(workspace.path())?;

        let manager = TemplateManager { config_dir: config_dir.path().to_path_buf() };
        manager.update_full(Some("Rust++"), None, false, false)?;
        let target = workspace.path().join(".rpp.toml");
        assert_eq!(fs::read_to_string(&target)?, "max_width = 120\n");

        // Upstream template change: an unmodified installed file is refreshed.
        fs::write(config_dir.path().join("rpp-format.toml"), "max_width = 140\n")?;
        let result = manager.update_full(Some("Rust++"), None, false, false);
        let _ = std::env::set_current_dir(std::env::temp_dir());

        result?;
        assert_eq!(fs::read_to_string(&target)?, "max_width = 140\n");
        Ok(())
    }

    #[test]
    fn test_update_full_skips_modified_file_without_error() -> anyhow::Result<()>
    {
        let _guard = cwd_test_guard();
        let config_dir = tempfile::TempDir::new()?;
        let workspace = tempfile::TempDir::new()?;
        setup_config(config_dir.path())?;
        std::env::set_current_dir(workspace.path())?;

        let manager = TemplateManager { config_dir: config_dir.path().to_path_buf() };
        manager.update_full(Some("Rust++"), None, false, false)?;

        let target = workspace.path().join(".rpp.toml");
        fs::write(&target, "max_width = 99\n")?;
        fs::write(config_dir.path().join("rpp-format.toml"), "max_width = 140\n")?;

        let result = manager.update_full(Some("Rust++"), None, false, false);
        let _ = std::env::set_current_dir(std::env::temp_dir());

        result?;
        assert_eq!(fs::read_to_string(&target)?, "max_width = 99\n", "modified file must be kept without an error");
        Ok(())
    }

    #[test]
    fn test_update_full_force_overwrites_modified_file() -> anyhow::Result<()>
    {
        let _guard = cwd_test_guard();
        let config_dir = tempfile::TempDir::new()?;
        let workspace = tempfile::TempDir::new()?;
        setup_config(config_dir.path())?;
        std::env::set_current_dir(workspace.path())?;

        let manager = TemplateManager { config_dir: config_dir.path().to_path_buf() };
        manager.update_full(Some("Rust++"), None, false, false)?;

        let target = workspace.path().join(".rpp.toml");
        fs::write(&target, "max_width = 99\n")?;

        let result = manager.update_full(Some("Rust++"), None, true, false);
        let _ = std::env::set_current_dir(std::env::temp_dir());

        result?;
        assert_eq!(fs::read_to_string(&target)?, "max_width = 120\n", "with --force the template must overwrite the modified file");
        Ok(())
    }

    #[test]
    fn test_update_full_restores_deleted_file() -> anyhow::Result<()>
    {
        let _guard = cwd_test_guard();
        let config_dir = tempfile::TempDir::new()?;
        let workspace = tempfile::TempDir::new()?;
        setup_config(config_dir.path())?;
        std::env::set_current_dir(workspace.path())?;

        let manager = TemplateManager { config_dir: config_dir.path().to_path_buf() };
        manager.update_full(Some("Rust++"), None, false, false)?;

        let target = workspace.path().join(".rpp.toml");
        fs::remove_file(&target)?;

        let result = manager.update_full(Some("Rust++"), None, false, false);
        let _ = std::env::set_current_dir(std::env::temp_dir());

        result?;
        assert!(target.exists() == true, "deleted tracked file must be restored");
        assert_eq!(fs::read_to_string(&target)?, "max_width = 120\n");
        Ok(())
    }

    #[test]
    fn test_update_full_prunes_stale_skill_file() -> anyhow::Result<()>
    {
        let _guard = cwd_test_guard();
        let config_dir = tempfile::TempDir::new()?;
        let workspace = tempfile::TempDir::new()?;
        setup_config(config_dir.path())?;
        std::env::set_current_dir(workspace.path())?;

        let manager = TemplateManager { config_dir: config_dir.path().to_path_buf() };
        manager.update_full(None, None, false, false)?;

        let hook = workspace.path().join(".agents/skills/git-workflow/scripts/hook.sh");
        assert!(hook.exists() == true, "hook must be installed initially");

        // Upstream removed the hook from the skill; full update prunes the tracked copy.
        fs::remove_file(config_dir.path().join("skills/git-workflow/scripts/hook.sh"))?;
        let result = manager.update_full(None, None, false, false);
        let _ = std::env::set_current_dir(std::env::temp_dir());

        result?;
        assert!(hook.exists() == false, "upstream-removed tracked skill file must be pruned");
        Ok(())
    }

    #[test]
    fn test_update_full_dry_run_writes_nothing() -> anyhow::Result<()>
    {
        let _guard = cwd_test_guard();
        let config_dir = tempfile::TempDir::new()?;
        let workspace = tempfile::TempDir::new()?;
        setup_config(config_dir.path())?;
        std::env::set_current_dir(workspace.path())?;

        let manager = TemplateManager { config_dir: config_dir.path().to_path_buf() };
        let result = manager.update_full(Some("Rust++"), None, false, true);
        let _ = std::env::set_current_dir(std::env::temp_dir());

        result?;
        assert!(workspace.path().join(".rpp.toml").exists() == false, "dry run must not create files");
        assert!(workspace.path().join(".agents").exists() == false, "dry run must not create skill dirs");
        Ok(())
    }

    #[test]
    fn test_update_full_local_cache_only_no_github_hook() -> anyhow::Result<()>
    {
        let _guard = cwd_test_guard();
        let config_dir = tempfile::TempDir::new()?;
        let workspace = tempfile::TempDir::new()?;
        setup_config(config_dir.path())?;
        std::env::set_current_dir(workspace.path())?;

        let _hook = crate::github::set_test_hooks(
            Box::new(|_| panic!("update must not call GitHub list_directory_contents")),
            Box::new(|_| panic!("update must not call GitHub download_file"))
        );

        let manager = TemplateManager { config_dir: config_dir.path().to_path_buf() };
        let result = manager.update_full(Some("Rust++"), None, false, false);
        let _ = std::env::set_current_dir(std::env::temp_dir());
        result?;
        Ok(())
    }
}
