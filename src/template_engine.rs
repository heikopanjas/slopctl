//! Template engine for slopctl
//!
//! This module provides the `TemplateEngine` struct and supporting types for
//! template generation, fragment merging, and placeholder resolution.
//! Follows the agents.md standard: one AGENTS.md file that works across all agents.

use std::{
    collections::{HashMap, HashSet},
    fs,
    io::{self, Write},
    path::{Path, PathBuf}
};

use owo_colors::OwoColorize;

use crate::{
    Result, agent_defaults,
    bom::{self, TemplateConfig},
    file_tracker::{AGENT_ALL, FileStatus, FileTracker, LANG_NONE},
    github,
    utils::copy_file_with_mkdir
};

/// Template marker comment used to detect unmerged template files
pub const TEMPLATE_MARKER: &str = "<!-- SLOPCTL-TEMPLATE: This marker indicates an unmerged template. Do not remove manually. -->";

/// Changelog marker comment separating template-managed content from the user-owned log tail
pub const CHANGELOG_MARKER: &str = "<!-- {changelog} -->";

/// Selectors for partial workspace refresh (`slopctl update --file` / `--skill`)
pub struct PartialSelectors<'a>
{
    /// Workspace-relative or absolute file paths to refresh
    pub files:  &'a HashSet<String>,
    /// Skill directory names to refresh
    pub skills: &'a HashSet<String>
}

/// Options for the template update operation
///
/// Aggregates CLI parameters that are passed through the update call chain.
#[derive(Clone, Copy)]
pub struct UpdateOptions<'a>
{
    /// Programming language or framework identifier (None = no language setup)
    pub lang:             Option<&'a str>,
    /// AI coding agent identifier (None = no agent-specific files)
    pub agent:            Option<&'a str>,
    /// Custom mission statement to override template default
    pub mission:          Option<&'a str>,
    /// Force overwrite of local modifications without warning
    pub force:            bool,
    /// Preview changes without applying them
    pub dry_run:          bool,
    /// When set, resolve only matching files and/or skills (partial update command)
    pub partial:          Option<&'a PartialSelectors<'a>>,
    /// When true, read only from the global template cache (no remote fetches)
    pub local_cache_only: bool
}

/// Context for the main AGENTS.md template and its fragments
///
/// Groups the source/target paths and fragment list that flow together
/// through `show_dry_run_files`, `handle_main_template`, and `merge_fragments`.
pub struct TemplateContext
{
    /// Path to the source AGENTS.md template in global storage
    pub source:           PathBuf,
    /// Path to the target AGENTS.md location in the workspace
    pub target:           PathBuf,
    /// Fragment files to merge into AGENTS.md: (source_path, category) pairs
    pub fragments:        Vec<(PathBuf, String)>,
    /// Template version from templates.yml for file tracking
    pub template_version: u32
}

/// A single resolved file with its provenance metadata
pub struct ResolvedFile
{
    pub source:   PathBuf,
    pub target:   PathBuf,
    /// Language owners for this file
    pub lang:     Vec<String>,
    /// Agent owners for this file
    pub agent:    Vec<String>,
    /// FileTracker category derived from the templates.yml section this file resolves from
    pub category: String
}

/// All files, fragments, and directories resolved from templates.yml for a given set of options
///
/// Produced by `TemplateEngine::resolve_all_files()` and consumed by both `update()` (init)
/// and the merge command.
///
/// Holds an owned `TempDir` for any GitHub-downloaded sources so the temp files
/// remain on disk until the consumer has finished copying or reading them.
pub struct ResolvedFiles
{
    /// Main AGENTS.md template context (source, target, fragments, version)
    pub context:     TemplateContext,
    /// Resolved file entries with provenance metadata
    pub files:       Vec<ResolvedFile>,
    /// Directories to create (agent-declared workspace directories)
    pub directories: Vec<PathBuf>,
    /// RAII guard keeping any GitHub-downloaded source files alive
    _temp_dir:       tempfile::TempDir
}

/// A resolved file's content with provenance metadata for the merge command
pub struct ResolvedContent
{
    pub content:  String,
    pub lang:     Vec<String>,
    pub agent:    Vec<String>,
    /// FileTracker category carried from the resolved file (`"main"` for AGENTS.md)
    pub category: String
}

struct PreflightPlan
{
    files: Vec<PlannedFileAction>
}

struct PlannedFileAction
{
    index:      usize,
    source_sha: String,
    category:   String,
    kind:       PlannedFileActionKind
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PlannedFileActionKind
{
    Copy,
    RefreshTracker,
    SkipModified,
    SkipChangelog
}

/// Loads template configuration from templates.yml
///
/// # Arguments
///
/// * `config_dir` - Path to the global template storage directory
///
/// # Errors
///
/// Returns an error if templates.yml cannot be loaded or parsed
pub fn load_template_config(config_dir: &Path) -> Result<TemplateConfig>
{
    let config_path = config_dir.join("templates.yml");

    require!(config_path.exists() == true, Err(anyhow::anyhow!("templates.yml not found in global template directory")));

    let content = fs::read_to_string(&config_path)?;
    let config: TemplateConfig = serde_yaml::from_str(&content)?;
    Ok(config)
}

/// Checks if a local file has been customized by checking for the template marker
///
/// If the template marker is missing from the local file, it means the file
/// has been merged or customized and should not be overwritten without confirmation.
///
/// # Arguments
///
/// * `local_path` - Path to local file to check
///
/// # Returns
///
/// Returns `true` if file exists and marker is missing (file is customized)
pub fn is_file_customized(local_path: &Path) -> Result<bool>
{
    require!(local_path.exists() == true, Ok(false));

    let content = fs::read_to_string(local_path)?;
    Ok(content.contains(TEMPLATE_MARKER) == false)
}

/// Checks whether a file contains the changelog marker comment as its own line
///
/// Files carrying the marker keep a user-owned, append-only log below it, so
/// install and cleanup paths must preserve their local modifications. Matching
/// requires a standalone (trimmed) line rather than a raw substring, so
/// documentation that merely mentions the marker as example text (e.g. the
/// `recent-updates` skill explaining the syntax) is not mistaken for a real
/// changelog-marker file. Returns `false` when the file is missing or unreadable.
pub fn file_contains_changelog_marker(path: &Path) -> bool
{
    fs::read_to_string(path).map(|content| content.lines().any(|line| line.trim() == CHANGELOG_MARKER)).unwrap_or(false)
}

/// Returns true when an existing target carries a user-owned changelog log.
///
/// Such files are never written by install or refresh paths; `merge` is the
/// only command that may update the template half above the marker.
pub fn is_changelog_protected(target: &Path) -> bool
{
    target.exists() == true && file_contains_changelog_marker(target) == true
}

/// Validates that no two file entries target the same destination path
///
/// Prevents silent overwrites when multiple template sections (language, integration,
/// agents, skills) produce files targeting the same workspace path.
///
/// # Arguments
///
/// * `files` - List of (source, target) file pairs to validate
///
/// # Errors
///
/// Returns an error if two entries share the same target path
#[cfg(test)]
pub fn validate_no_duplicate_targets(files: &[ResolvedFile]) -> Result<()>
{
    let mut seen_targets: HashMap<&Path, &Path> = HashMap::new();
    for entry in files
    {
        if let Some(previous_source) = seen_targets.insert(entry.target.as_path(), entry.source.as_path())
        {
            return Err(anyhow::anyhow!(
                "Duplicate target '{}': '{}' and '{}' both write to the same file",
                entry.target.display(),
                previous_source.display(),
                entry.source.display()
            ));
        }
    }
    Ok(())
}

/// Template engine for slopctl (agents.md standard)
///
/// Handles template generation, fragment merging, placeholder resolution,
/// and skill installation. Supports V2-V4 template formats.
pub struct TemplateEngine<'a>
{
    config_dir: &'a Path
}

impl<'a> TemplateEngine<'a>
{
    /// Creates a new TemplateEngine instance
    ///
    /// # Arguments
    ///
    /// * `config_dir` - Path to the global template storage directory
    pub fn new(config_dir: &'a Path) -> Self
    {
        Self { config_dir }
    }

    /// Returns the path to the global template storage directory
    pub fn config_dir(&self) -> &Path
    {
        self.config_dir
    }

    /// Maps a templates.yml section name to its FileTracker category.
    ///
    /// The section namespace uses `"languages"` (plural, matching the YAML key and the
    /// AGENTS.md fragment marker) while the tracker category is `"language"` (singular);
    /// all other sections map to themselves. Categories are authoritative from the
    /// resolution pipeline and must never be derived from file paths.
    fn tracker_category_for_section(section: &str) -> String
    {
        if section == "languages"
        {
            "language".to_string()
        }
        else
        {
            section.to_string()
        }
    }

    /// Convert a scalar owner into a tracker owner list.
    fn owner_list(owner: &str, sentinel: &str) -> Vec<String>
    {
        if owner == sentinel
        {
            Vec::new()
        }
        else
        {
            vec![owner.to_string()]
        }
    }

    /// Resolves a target path string containing placeholder variables
    ///
    /// Public wrapper around `resolve_placeholder` for use by the merge command
    /// and other modules that need to map templates.yml targets to workspace paths.
    ///
    /// # Arguments
    ///
    /// * `target` - Target path string (may contain `$workspace` or `$userprofile`)
    /// * `workspace` - Workspace directory path
    /// * `userprofile` - User profile directory path
    pub fn resolve_target(&self, target: &str, workspace: &Path, userprofile: &Path) -> PathBuf
    {
        self.resolve_placeholder(target, workspace, userprofile)
    }

    /// Resolves placeholder variables in target paths
    ///
    /// Replaces `$workspace` with the workspace directory path
    /// and `$userprofile` with the user's home directory path.
    /// Uses `Path::join` for cross-platform correctness (avoids mixed separators on Windows).
    ///
    /// # Arguments
    ///
    /// * `path` - Path string containing placeholders
    /// * `workspace` - Workspace directory path
    /// * `userprofile` - User profile directory path
    fn resolve_placeholder(&self, path: &str, workspace: &Path, userprofile: &Path) -> PathBuf
    {
        if path.starts_with("$workspace") == true
        {
            let suffix = path["$workspace".len()..].trim_start_matches('/').trim_start_matches('\\');
            return workspace.join(suffix);
        }
        if path.starts_with("$userprofile") == true
        {
            let suffix = path["$userprofile".len()..].trim_start_matches('/').trim_start_matches('\\');
            return userprofile.join(suffix);
        }
        PathBuf::from(path)
    }

    /// Resolves a skill `target` field to an absolute base directory
    ///
    /// Bare `"$workspace"` returns `default` — the contextually correct skill dir for
    /// this skill type and agent (cross-client dir for cross-client agents on non-agent
    /// skills; native dir for agent-specific skills). This keeps `target: "$workspace"`
    /// semantically consistent with the `None` case and avoids routing skills to a
    /// native agent dir when the agent also scans the cross-client dir.
    ///
    /// Bare `"$userprofile"` resolves to the agent's userprofile skill dir, or to
    /// `~/.agents/skills` as fallback. Use this when
    /// you explicitly want a user-global (not workspace-local) installation.
    ///
    /// Any full path (e.g. `"$workspace/.agents/skills"`) is resolved via
    /// `resolve_placeholder` as-is. `None` also falls back to `default`.
    fn resolve_skill_target(
        &self, target: Option<&str>, default: &Path, agent: Option<&str>, agent_catalog: &agent_defaults::AgentCatalog, workspace: &Path, userprofile: &Path
    ) -> PathBuf
    {
        let Some(t) = target
        else
        {
            return default.to_path_buf();
        };

        if t == agent_defaults::PLACEHOLDER_WORKSPACE
        {
            // Treat bare "$workspace" as "use the smart contextual default" so that
            // explicit `target: '$workspace'` in templates.yml has the same routing as
            // no target at all.  This avoids native-dir routing for cross-client agents.
            return default.to_path_buf();
        }

        if t == agent_defaults::PLACEHOLDER_USERPROFILE
        {
            let raw = agent
                .map(|agent_name| agent_defaults::get_effective_userprofile_skill_dir_from_catalog(agent_catalog, agent_name))
                .unwrap_or("$userprofile/.agents/skills".to_string());
            return self.resolve_placeholder(&raw, workspace, userprofile);
        }

        self.resolve_placeholder(t, workspace, userprofile)
    }

    /// Groups a skill list by their resolved install base directory
    ///
    /// Skills with an explicit `target` are routed to their resolved dir;
    /// skills without `target` use `default`. Returns a `Vec` of `(dir, skills)`
    /// pairs in insertion order, preserving the original skill ordering within each group.
    fn group_skills_by_target<'s>(
        &self, skills: &'s [bom::SkillDefinition], default: &Path, agent: Option<&str>, agent_catalog: &agent_defaults::AgentCatalog, workspace: &Path,
        userprofile: &Path
    ) -> Vec<(PathBuf, Vec<&'s bom::SkillDefinition>)>
    {
        let mut order: Vec<PathBuf> = Vec::new();
        let mut map: HashMap<PathBuf, Vec<&bom::SkillDefinition>> = HashMap::new();

        for skill in skills
        {
            let dir = self.resolve_skill_target(skill.target.as_deref(), default, agent, agent_catalog, workspace, userprofile);
            if map.contains_key(&dir) == false
            {
                order.push(dir.clone());
            }
            map.entry(dir).or_default().push(skill);
        }

        order
            .into_iter()
            .map(|dir| {
                let skills = map.remove(&dir).unwrap_or_default();
                (dir, skills)
            })
            .collect()
    }

    /// Returns true when a skill uses smart multi-target distribution (omitted target or bare `$workspace`).
    fn skill_uses_smart_distribution(target: Option<&str>) -> bool
    {
        target.is_none() == true || target == Some(agent_defaults::PLACEHOLDER_WORKSPACE)
    }

    /// Compute all workspace skill directories for non-agent-specific skills.
    ///
    /// Distributes language and top-level skills based on installed agents:
    /// - no agents: `.agents/skills/` only
    /// - cross-client agents: one shared copy in `.agents/skills/`
    /// - native-only agents: one copy per native skill dir
    /// - mixed: both shared and each native-only copy
    fn non_agent_skill_target_dirs(
        &self, options_agent: Option<&str>, agent_catalog: &agent_defaults::AgentCatalog, workspace: &Path, userprofile: &Path
    ) -> Vec<PathBuf>
    {
        let cross_client_dir = self.resolve_placeholder(agent_defaults::CROSS_CLIENT_SKILL_DIR, workspace, userprofile);
        let mut installed = agent_defaults::detect_all_installed_agents_from_catalog(agent_catalog, workspace);

        if let Some(agent_name) = options_agent &&
            installed.iter().any(|name| name == agent_name) == false
        {
            installed.push(agent_name.to_string());
        }

        let mut needs_cross_client = installed.is_empty() == true;
        let mut native_only_agents: Vec<String> = Vec::new();

        for agent_name in &installed
        {
            if agent_defaults::reads_cross_client_skills_from_catalog(agent_catalog, agent_name) == true
            {
                needs_cross_client = true;
            }
            else
            {
                native_only_agents.push(agent_name.clone());
            }
        }

        let mut dirs: Vec<PathBuf> = Vec::new();

        if needs_cross_client == true
        {
            dirs.push(cross_client_dir.clone());
        }

        for agent_name in native_only_agents
        {
            if let Some(raw_skill_dir) = agent_defaults::get_skill_dir_from_catalog(agent_catalog, &agent_name) &&
                raw_skill_dir.starts_with(agent_defaults::PLACEHOLDER_WORKSPACE) == true
            {
                let native_dir = self.resolve_placeholder(raw_skill_dir, workspace, userprofile);
                if dirs.contains(&native_dir) == false
                {
                    dirs.push(native_dir);
                }
            }
        }

        if dirs.is_empty() == true
        {
            dirs.push(cross_client_dir);
        }

        dirs
    }

    /// Install non-agent-specific skills using smart multi-target distribution or explicit targets.
    #[allow(clippy::too_many_arguments)]
    fn install_non_agent_skills(
        &self, skills: &[bom::SkillDefinition], target_dirs: &[PathBuf], options_agent: Option<&str>, agent_catalog: &agent_defaults::AgentCatalog, workspace: &Path,
        userprofile: &Path, temp_path: &Path, lang: &str, agent: &str, local_cache_only: bool, files_to_copy: &mut Vec<ResolvedFile>
    ) -> Result<()>
    {
        let default_dir = target_dirs.first().cloned().unwrap_or_else(|| self.resolve_placeholder(agent_defaults::CROSS_CLIENT_SKILL_DIR, workspace, userprofile));

        for (dir, group) in self.group_skills_by_target(skills, &default_dir, options_agent, agent_catalog, workspace, userprofile)
        {
            for skill in group
            {
                if Self::skill_uses_smart_distribution(skill.target.as_deref()) == true
                {
                    for target_dir in target_dirs
                    {
                        self.install_skills(
                            std::iter::once((skill.derive_name(), skill.source.as_str())),
                            target_dir,
                            temp_path,
                            lang,
                            agent,
                            local_cache_only,
                            files_to_copy
                        )?;
                    }
                }
                else
                {
                    self.install_skills(std::iter::once((skill.derive_name(), skill.source.as_str())), &dir, temp_path, lang, agent, local_cache_only, files_to_copy)?;
                }
            }
        }

        Ok(())
    }

    /// Install selected non-agent-specific skills using smart multi-target distribution.
    #[allow(clippy::too_many_arguments)]
    fn install_partial_non_agent_skills(
        &self, skills: &[bom::SkillDefinition], requested: &HashSet<String>, target_dirs: &[PathBuf], options_agent: Option<&str>,
        agent_catalog: &agent_defaults::AgentCatalog, workspace: &Path, userprofile: &Path, temp_path: &Path, lang: &str, agent: &str, local_cache_only: bool,
        files_to_copy: &mut Vec<ResolvedFile>
    ) -> Result<()>
    {
        let default_dir = target_dirs.first().cloned().unwrap_or_else(|| self.resolve_placeholder(agent_defaults::CROSS_CLIENT_SKILL_DIR, workspace, userprofile));

        for (dir, group) in self.group_skills_by_target(skills, &default_dir, options_agent, agent_catalog, workspace, userprofile)
        {
            for skill in group
            {
                let pairs: Vec<(String, String)> = self.skill_install_pairs_for_partial(skill, requested);
                if pairs.is_empty() == false
                {
                    if Self::skill_uses_smart_distribution(skill.target.as_deref()) == true
                    {
                        for target_dir in target_dirs
                        {
                            self.install_skills(
                                pairs.iter().map(|(n, s)| (n.as_str(), s.as_str())),
                                target_dir,
                                temp_path,
                                lang,
                                agent,
                                local_cache_only,
                                files_to_copy
                            )?;
                        }
                    }
                    else
                    {
                        self.install_skills(pairs.iter().map(|(n, s)| (n.as_str(), s.as_str())), &dir, temp_path, lang, agent, local_cache_only, files_to_copy)?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Hydrate language skills for already-installed languages into a native-only agent directory.
    #[allow(clippy::too_many_arguments)]
    fn hydrate_language_skills_for_native_agent(
        &self, agent_name: &str, native_skill_dir: &Path, config: &TemplateConfig, agent_catalog: &agent_defaults::AgentCatalog, workspace: &Path, userprofile: &Path,
        existing_tracker: &FileTracker, temp_path: &Path, files_to_copy: &mut Vec<ResolvedFile>
    ) -> Result<()>
    {
        let target_dirs = [native_skill_dir.to_path_buf()];

        for lang in existing_tracker.get_installed_languages()
        {
            let lang_skills = bom::resolve_language_skills(&lang, config)?;
            if lang_skills.is_empty() == false
            {
                self.install_non_agent_skills(
                    &lang_skills,
                    &target_dirs,
                    Some(agent_name),
                    agent_catalog,
                    workspace,
                    userprofile,
                    temp_path,
                    &lang,
                    AGENT_ALL,
                    false,
                    files_to_copy
                )?;
            }
        }

        Ok(())
    }

    /// Resolves a source string to a local file path
    ///
    /// If the source is a URL, downloads it to the temp directory and returns
    /// the temp path unless `local_cache_only` is set. Otherwise, joins it with
    /// `config_dir` for local lookup.
    fn resolve_source_to_path(&self, source: &str, temp_dir: &Path, local_cache_only: bool) -> Result<PathBuf>
    {
        if github::is_url(source) == true
        {
            require!(
                local_cache_only == false,
                Err(anyhow::anyhow!("Template source '{}' is not in the local template cache. Run 'slopctl templates --update' first.", source))
            );

            let parsed = github::parse_github_url(source).ok_or_else(|| anyhow::anyhow!("Invalid GitHub URL: {}", source))?;

            let filename = source.rsplit('/').next().unwrap_or("downloaded");
            let temp_path = temp_dir.join(filename);

            print!("{} Downloading {}... ", "→".blue(), filename.yellow());
            io::stdout().flush()?;

            match github::download_github_file(&parsed, &temp_path)
            {
                | Ok(_) =>
                {
                    println!("{}", "✓".green());
                }
                | Err(e) =>
                {
                    println!("{}", "✗".red());
                    return Err(e);
                }
            }

            Ok(temp_path)
        }
        else
        {
            Ok(self.config_dir.join(source))
        }
    }

    /// Returns the cached skill directory for a skill name under the global template cache
    fn cached_skill_dir(&self, skill_name: &str) -> PathBuf
    {
        self.config_dir.join("skills").join(skill_name)
    }

    /// Returns true when a skill definition matches a partial-update skill selector
    fn skill_definition_matches_partial(&self, skill: &bom::SkillDefinition, requested: &HashSet<String>) -> bool
    {
        requested.iter().any(|name| skill.derive_name() == name.as_str() || (github::is_url(&skill.source) == true && self.cached_skill_dir(name).is_dir() == true))
    }

    /// Builds `(skill_name, source)` install pairs for a skill definition under partial selectors
    fn skill_install_pairs_for_partial(&self, skill: &bom::SkillDefinition, requested: &HashSet<String>) -> Vec<(String, String)>
    {
        if github::is_url(&skill.source) == true
        {
            requested.iter().filter(|name| self.cached_skill_dir(name).is_dir() == true).map(|name| (name.clone(), skill.source.clone())).collect()
        }
        else if requested.contains(skill.derive_name()) == true
        {
            vec![(skill.derive_name().to_string(), skill.source.clone())]
        }
        else
        {
            Vec::new()
        }
    }

    /// Returns true when a resolved workspace target matches a partial file selector
    fn target_matches_partial_files(&self, target: &Path, partial_files: &HashSet<String>, workspace: &Path) -> bool
    {
        let normalized_target = normalize_path(target);
        partial_files.iter().any(|requested| {
            let raw = Path::new(requested.as_str());
            let resolved = if raw.is_absolute() == true
            {
                normalize_path(raw)
            }
            else
            {
                normalize_path(&workspace.join(raw))
            };
            resolved == normalized_target
        })
    }

    /// Resolves all files, fragments, and directories from templates.yml for the given options
    ///
    /// Walks every section of the template configuration (main, principles, mission,
    /// languages, integration, agents, skills) and produces the complete set of
    /// (source, target) file pairs, AGENTS.md fragments, and directories to create.
    ///
    /// This is the shared pipeline used by both `update()` (init) and the merge command.
    ///
    /// # Arguments
    ///
    /// * `options` - Aggregated CLI parameters controlling which sections are resolved
    ///
    /// # Errors
    ///
    /// Returns an error if global templates are missing, agent/language validation fails,
    /// or any source resolution fails
    pub fn resolve_all_files(&self, options: &UpdateOptions) -> Result<ResolvedFiles>
    {
        let templates_yml_path = self.config_dir.join("templates.yml");

        require!(
            self.config_dir.exists() == true && templates_yml_path.exists() == true,
            Err(anyhow::anyhow!("Global templates not found. Please run 'slopctl templates --update' first to download templates."))
        );

        let config = load_template_config(self.config_dir)?;
        let agent_catalog = agent_defaults::load_agent_catalog_from_dir(self.config_dir)?;

        if let Some(agent_name) = options.agent &&
            config.agents.contains_key(agent_name) == false
        {
            let mut available: Vec<&String> = config.agents.keys().collect();
            available.sort();
            return Err(anyhow::anyhow!(
                "Agent '{}' not found in templates.yml.\nAvailable agents: {}",
                agent_name,
                available.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
            ));
        }

        if let Some(lang) = options.lang &&
            config.languages.contains_key(lang) == false
        {
            let mut available: Vec<&String> = config.languages.keys().collect();
            available.sort();
            return Err(anyhow::anyhow!(
                "Language '{}' not found in templates.yml.\nAvailable languages: {}",
                lang,
                available.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
            ));
        }

        let workspace = std::env::current_dir()?;
        let userprofile = dirs::home_dir().ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "Could not determine home directory"))?;

        let temp_dir = tempfile::TempDir::new()?;

        let main_config = config.main.as_ref().ok_or_else(|| anyhow::anyhow!("Missing 'main' section in templates.yml"))?;
        let main_target = self.resolve_placeholder(&main_config.target, &workspace, &userprofile);
        let skill_only_partial = options.partial.is_some_and(|partial| partial.files.is_empty() == true && partial.skills.is_empty() == false);
        let main_source = if skill_only_partial == true || options.local_cache_only == true
        {
            self.config_dir.join(&main_config.source)
        }
        else
        {
            self.resolve_source_to_path(&main_config.source, temp_dir.path(), false)?
        };

        if options.partial.is_none() == true && main_source.exists() == false
        {
            return Err(anyhow::anyhow!("Main template not found: {}", main_source.display()));
        }

        let mut files_to_copy: Vec<ResolvedFile> = Vec::new();
        let mut fragments: Vec<(PathBuf, String)> = Vec::new();
        let mut directories_to_create: Vec<PathBuf> = Vec::new();
        let temp_path = temp_dir.path();
        let local_cache_only = options.local_cache_only;

        if let Some(partial) = options.partial
        {
            let mut process_errors: Vec<String> = Vec::new();
            let mut process_entry = |source: &str, target: &str, category: &str, lang: &str, agent: &str| {
                let target_path = self.resolve_placeholder(target, &workspace, &userprofile);
                if self.target_matches_partial_files(&target_path, partial.files, &workspace) == false
                {
                    return;
                }

                let source_path = if github::is_url(source) == true
                {
                    if local_cache_only == true
                    {
                        process_errors.push(format!("Template source '{}' is not in the local template cache. Run 'slopctl templates --update' first.", source));
                        return;
                    }
                    match self.resolve_source_to_path(source, temp_path, false)
                    {
                        | Ok(p) => p,
                        | Err(e) =>
                        {
                            process_errors.push(format!("Failed to download {}: {}", source, e));
                            return;
                        }
                    }
                }
                else
                {
                    self.config_dir.join(source)
                };

                if source_path.exists() == false
                {
                    if local_cache_only == true
                    {
                        process_errors.push(format!("Template file '{}' not found in local template cache. Run 'slopctl templates --update' first.", source));
                    }
                    return;
                }

                if target.starts_with("$instructions") == true
                {
                    fragments.push((source_path, category.to_string()));
                }
                else
                {
                    files_to_copy.push(ResolvedFile {
                        source:   source_path,
                        target:   target_path,
                        lang:     Self::owner_list(lang, LANG_NONE),
                        agent:    Self::owner_list(agent, AGENT_ALL),
                        category: Self::tracker_category_for_section(category)
                    });
                }
            };

            if partial.files.is_empty() == false
            {
                if let Some(lang) = options.lang
                {
                    let resolved_files = bom::resolve_language_files(lang, &config)?;
                    for file_entry in &resolved_files
                    {
                        process_entry(&file_entry.source, &file_entry.target, "languages", lang, AGENT_ALL);
                    }
                }

                for integration_config in config.integration.values()
                {
                    for file_entry in &integration_config.files
                    {
                        process_entry(&file_entry.source, &file_entry.target, "integration", LANG_NONE, AGENT_ALL);
                    }
                }

                if let Some(agent_name) = options.agent &&
                    let Some(agent_config) = config.agents.get(agent_name)
                {
                    for entry in agent_config.instructions.iter().chain(&agent_config.prompts)
                    {
                        let target_path = self.resolve_placeholder(&entry.target, &workspace, &userprofile);
                        if self.target_matches_partial_files(&target_path, partial.files, &workspace) == false
                        {
                            continue;
                        }

                        let source_path = if github::is_url(&entry.source) == true
                        {
                            if local_cache_only == true
                            {
                                process_errors
                                    .push(format!("Template source '{}' is not in the local template cache. Run 'slopctl templates --update' first.", entry.source));
                                continue;
                            }
                            match self.resolve_source_to_path(&entry.source, temp_path, false)
                            {
                                | Ok(p) => p,
                                | Err(e) =>
                                {
                                    process_errors.push(format!("Failed to download {}: {}", entry.source, e));
                                    continue;
                                }
                            }
                        }
                        else
                        {
                            self.config_dir.join(&entry.source)
                        };

                        if source_path.exists() == true
                        {
                            files_to_copy.push(ResolvedFile {
                                source:   source_path,
                                target:   target_path,
                                lang:     Vec::new(),
                                agent:    Self::owner_list(agent_name, AGENT_ALL),
                                category: "agent".to_string()
                            });
                        }
                        else if local_cache_only == true
                        {
                            process_errors
                                .push(format!("Template file '{}' not found in local template cache. Run 'slopctl templates --update' first.", entry.source));
                        }
                    }
                }
            }

            if partial.skills.is_empty() == false
            {
                let agent_skill_dir = options
                    .agent
                    .and_then(|agent| agent_defaults::get_skill_dir_from_catalog(&agent_catalog, agent))
                    .map(|dir| self.resolve_placeholder(dir, &workspace, &userprofile));
                let non_agent_skill_dirs = self.non_agent_skill_target_dirs(options.agent, &agent_catalog, &workspace, &userprofile);

                if let Some(agent_name) = options.agent &&
                    let Some(agent_config) = config.agents.get(agent_name) &&
                    agent_config.skills.is_empty() == false &&
                    let Some(ref default_dir) = agent_skill_dir
                {
                    let filtered: Vec<bom::SkillDefinition> =
                        agent_config.skills.iter().filter(|skill| self.skill_definition_matches_partial(skill, partial.skills)).cloned().collect();
                    for (dir, group) in self.group_skills_by_target(&filtered, default_dir, options.agent, &agent_catalog, &workspace, &userprofile)
                    {
                        let pairs: Vec<(String, String)> = group.iter().flat_map(|skill| self.skill_install_pairs_for_partial(skill, partial.skills)).collect();
                        self.install_skills(
                            pairs.iter().map(|(n, s)| (n.as_str(), s.as_str())),
                            &dir,
                            temp_path,
                            LANG_NONE,
                            agent_name,
                            local_cache_only,
                            &mut files_to_copy
                        )?;
                    }
                }

                if let Some(lang) = options.lang
                {
                    let lang_skills = bom::resolve_language_skills(lang, &config)?;
                    let filtered: Vec<bom::SkillDefinition> =
                        lang_skills.iter().filter(|skill| self.skill_definition_matches_partial(skill, partial.skills)).cloned().collect();
                    if filtered.is_empty() == false
                    {
                        self.install_partial_non_agent_skills(
                            &filtered, partial.skills, &non_agent_skill_dirs, options.agent, &agent_catalog, &workspace, &userprofile, temp_path, lang, AGENT_ALL,
                            local_cache_only, &mut files_to_copy
                        )?;
                    }
                }

                if config.skills.is_empty() == false
                {
                    let filtered: Vec<bom::SkillDefinition> =
                        config.skills.iter().filter(|skill| self.skill_definition_matches_partial(skill, partial.skills)).cloned().collect();
                    if filtered.is_empty() == false
                    {
                        self.install_partial_non_agent_skills(
                            &filtered, partial.skills, &non_agent_skill_dirs, options.agent, &agent_catalog, &workspace, &userprofile, temp_path, LANG_NONE,
                            AGENT_ALL, local_cache_only, &mut files_to_copy
                        )?;
                    }
                }
            }

            if process_errors.is_empty() == false
            {
                return Err(anyhow::anyhow!("{}", process_errors.join("\n")));
            }
        }
        else
        {
            let mut process_errors: Vec<String> = Vec::new();
            let mut process_entry = |source: &str, target: &str, category: &str, lang: &str, agent: &str| {
                let source_path = if github::is_url(source) == true
                {
                    match self.resolve_source_to_path(source, temp_path, local_cache_only)
                    {
                        | Ok(p) => p,
                        | Err(e) =>
                        {
                            process_errors.push(format!("Failed to download {}: {}", source, e));
                            return;
                        }
                    }
                }
                else
                {
                    self.config_dir.join(source)
                };

                if source_path.exists() == false
                {
                    return;
                }

                if target.starts_with("$instructions")
                {
                    fragments.push((source_path, category.to_string()));
                }
                else
                {
                    let target_path = self.resolve_placeholder(target, &workspace, &userprofile);
                    files_to_copy.push(ResolvedFile {
                        source:   source_path,
                        target:   target_path,
                        lang:     Self::owner_list(lang, LANG_NONE),
                        agent:    Self::owner_list(agent, AGENT_ALL),
                        category: Self::tracker_category_for_section(category)
                    });
                }
            };

            for entry in &config.preamble
            {
                process_entry(&entry.source, &entry.target, "preamble", LANG_NONE, AGENT_ALL);
            }

            for entry in &config.principles
            {
                process_entry(&entry.source, &entry.target, "principles", LANG_NONE, AGENT_ALL);
            }

            if options.mission.is_none() == true
            {
                for entry in &config.mission
                {
                    process_entry(&entry.source, &entry.target, "mission", LANG_NONE, AGENT_ALL);
                }
            }

            if let Some(lang) = options.lang
            {
                let resolved_files = bom::resolve_language_files(lang, &config)?;
                for file_entry in &resolved_files
                {
                    process_entry(&file_entry.source, &file_entry.target, "languages", lang, AGENT_ALL);
                }
            }

            for integration_config in config.integration.values()
            {
                for file_entry in &integration_config.files
                {
                    process_entry(&file_entry.source, &file_entry.target, "integration", LANG_NONE, AGENT_ALL);
                }
            }

            if let Some(agent_name) = options.agent
            {
                for marker_dir in agent_defaults::get_workspace_marker_dirs_from_catalog(&agent_catalog, agent_name, &workspace)
                {
                    if directories_to_create.contains(&marker_dir) == false
                    {
                        directories_to_create.push(marker_dir);
                    }
                }
            }

            if let Some(agent_name) = options.agent &&
                let Some(agent_config) = config.agents.get(agent_name)
            {
                for entry in agent_config.instructions.iter().chain(&agent_config.prompts)
                {
                    let source_path = match self.resolve_source_to_path(&entry.source, temp_path, local_cache_only)
                    {
                        | Ok(p) => p,
                        | Err(e) =>
                        {
                            println!("{} Failed to resolve {}: {}", "!".yellow(), entry.source, e);
                            continue;
                        }
                    };

                    if source_path.exists()
                    {
                        let target_path = self.resolve_placeholder(&entry.target, &workspace, &userprofile);
                        files_to_copy.push(ResolvedFile {
                            source:   source_path,
                            target:   target_path,
                            lang:     Vec::new(),
                            agent:    Self::owner_list(agent_name, AGENT_ALL),
                            category: "agent".to_string()
                        });
                    }
                }

                for dir_entry in &agent_config.directories
                {
                    let dir_path = self.resolve_placeholder(&dir_entry.target, &workspace, &userprofile);
                    if directories_to_create.contains(&dir_path) == false
                    {
                        directories_to_create.push(dir_path);
                    }
                }
            }

            for err in &process_errors
            {
                println!("{} {}", "!".yellow(), err.yellow());
            }

            let agent_skill_dir = options
                .agent
                .and_then(|agent| agent_defaults::get_skill_dir_from_catalog(&agent_catalog, agent))
                .map(|dir| self.resolve_placeholder(dir, &workspace, &userprofile));
            let non_agent_skill_dirs = self.non_agent_skill_target_dirs(options.agent, &agent_catalog, &workspace, &userprofile);
            let existing_tracker = FileTracker::new(&workspace).ok();
            let native_only_agent = options.agent.is_some_and(|a| agent_defaults::reads_cross_client_skills_from_catalog(&agent_catalog, a) == false);

            if let Some(agent_name) = options.agent &&
                let Some(agent_config) = config.agents.get(agent_name) &&
                agent_config.skills.is_empty() == false &&
                let Some(ref default_dir) = agent_skill_dir
            {
                for (dir, group) in self.group_skills_by_target(&agent_config.skills, default_dir, options.agent, &agent_catalog, &workspace, &userprofile)
                {
                    self.install_skills(
                        group.iter().map(|s| (s.derive_name(), s.source.as_str())),
                        &dir,
                        temp_path,
                        LANG_NONE,
                        agent_name,
                        local_cache_only,
                        &mut files_to_copy
                    )?;
                }
            }

            if let Some(lang) = options.lang
            {
                let lang_skills = bom::resolve_language_skills(lang, &config)?;
                if lang_skills.is_empty() == false
                {
                    self.install_non_agent_skills(
                        &lang_skills, &non_agent_skill_dirs, options.agent, &agent_catalog, &workspace, &userprofile, temp_path, lang, AGENT_ALL, local_cache_only,
                        &mut files_to_copy
                    )?;
                }
            }

            if config.skills.is_empty() == false
            {
                self.install_non_agent_skills(
                    &config.skills,
                    &non_agent_skill_dirs,
                    options.agent,
                    &agent_catalog,
                    &workspace,
                    &userprofile,
                    temp_path,
                    options.lang.unwrap_or(LANG_NONE),
                    options.agent.unwrap_or(AGENT_ALL),
                    local_cache_only,
                    &mut files_to_copy
                )?;
            }

            // When adding a native-only agent after language install, hydrate language skills from
            // templates into the agent's native skill dir (replaces broad .agents/skills/ adoption).
            if native_only_agent == true &&
                options.lang.is_none() == true &&
                let Some(agent_name) = options.agent &&
                let Some(ref native_dir) = agent_skill_dir &&
                let Some(ref tracker) = existing_tracker &&
                tracker.get_installed_languages().is_empty() == false
            {
                self.hydrate_language_skills_for_native_agent(
                    agent_name, native_dir, &config, &agent_catalog, &workspace, &userprofile, tracker, temp_path, &mut files_to_copy
                )?;
            }
        }

        let ctx = TemplateContext { source: main_source, target: main_target, fragments, template_version: config.version };

        Ok(ResolvedFiles { context: ctx, files: files_to_copy, directories: directories_to_create, _temp_dir: temp_dir })
    }

    /// Builds a map from resolved workspace target path to fresh template content
    ///
    /// Calls `resolve_all_files()` and reads each source file into a `HashMap`.
    /// For the main AGENTS.md, generates a fresh merged version with all fragments
    /// filled in. This is consumed by the merge command to compare against disk.
    ///
    /// # Arguments
    ///
    /// * `options` - Aggregated CLI parameters controlling which sections are resolved
    ///
    /// # Errors
    ///
    /// Returns an error if file resolution or reading fails
    pub fn build_target_content_map(&self, options: &UpdateOptions) -> Result<HashMap<PathBuf, ResolvedContent>>
    {
        let resolved = self.resolve_all_files(options)?;
        let mut map: HashMap<PathBuf, ResolvedContent> = HashMap::new();

        let fresh_main = Self::generate_fresh_main(&resolved.context, options)?;
        let main_target = normalize_path(&resolved.context.target);
        map.insert(main_target, ResolvedContent {
            content:  fresh_main,
            lang:     options.lang.map(|lang| vec![lang.to_string()]).unwrap_or_default(),
            agent:    options.agent.map(|agent| vec![agent.to_string()]).unwrap_or_default(),
            category: "main".to_string()
        });

        for entry in &resolved.files
        {
            if entry.source.exists() == true &&
                let Ok(content) = fs::read_to_string(&entry.source)
            {
                map.insert(normalize_path(&entry.target), ResolvedContent {
                    content,
                    lang: entry.lang.clone(),
                    agent: entry.agent.clone(),
                    category: entry.category.clone()
                });
            }
        }

        Ok(map)
    }

    /// Generates a fresh AGENTS.md by merging the base template with all fragment sections
    ///
    /// Reproduces what `init` would produce without actually installing anything.
    /// When a mission override is set in options, it replaces any template-defined mission fragments.
    fn generate_fresh_main(ctx: &TemplateContext, options: &UpdateOptions) -> Result<String>
    {
        let mut content = fs::read_to_string(&ctx.source)?;

        let marker_line = format!("{}\n", TEMPLATE_MARKER);
        content = content.replace(&marker_line, "");

        let mut fragments_by_category: HashMap<String, Vec<String>> = HashMap::new();

        if options.lang.is_none() == true
        {
            fragments_by_category.entry("languages".to_string()).or_default();
        }

        if let Some(mission_content) = options.mission
        {
            let formatted_mission = format!("## Mission Statement\n\n{}", mission_content.trim());
            fragments_by_category.entry("mission".to_string()).or_default().push(formatted_mission);
        }

        for (fragment_path, category) in &ctx.fragments
        {
            if options.mission.is_some() == true && category == "mission"
            {
                continue;
            }
            if let Ok(frag) = fs::read_to_string(fragment_path)
            {
                fragments_by_category.entry(category.clone()).or_default().push(frag);
            }
        }

        for (category, contents) in &fragments_by_category
        {
            let insertion_point = format!("<!-- {{{}}} -->", category);
            let combined = contents.iter().map(|c| c.trim()).collect::<Vec<_>>().join("\n\n");
            let replacement = format!("<!-- {{{}}} -->\n\n{}", category, combined);
            content = content.replace(&insertion_point, &replacement);
        }

        Ok(content)
    }

    fn push_parent_conflict(conflicts: &mut Vec<String>, path: &Path)
    {
        if let Some(parent) = path.parent() &&
            parent.exists() == true &&
            parent.is_dir() == false
        {
            conflicts.push(format!("parent path '{}' is not a directory", parent.display()));
        }
    }

    fn preflight_installation(
        &self, ctx: &TemplateContext, skip_agents_md: bool, options: &UpdateOptions, files_to_copy: &[ResolvedFile], directories: &[PathBuf],
        file_tracker: &FileTracker
    ) -> Result<PreflightPlan>
    {
        let mut conflicts = Vec::new();
        let mut planned_files = Vec::new();
        let mut seen_targets: HashMap<PathBuf, &Path> = HashMap::new();

        if skip_agents_md == false
        {
            if ctx.target.exists() == true && ctx.target.is_dir() == true
            {
                conflicts.push(format!("target '{}' is a directory", ctx.target.display()));
            }
            Self::push_parent_conflict(&mut conflicts, &ctx.target);
        }

        for dir in directories
        {
            if dir.exists() == true && dir.is_dir() == false
            {
                conflicts.push(format!("directory target '{}' exists as a file", dir.display()));
            }
            Self::push_parent_conflict(&mut conflicts, dir);
        }

        for (index, entry) in files_to_copy.iter().enumerate()
        {
            if let Some(previous_source) = seen_targets.insert(entry.target.clone(), entry.source.as_path())
            {
                conflicts.push(format!(
                    "duplicate target '{}': '{}' and '{}' both write to the same file",
                    entry.target.display(),
                    previous_source.display(),
                    entry.source.display()
                ));
                continue;
            }

            let source_sha = FileTracker::calculate_sha256(&entry.source)?;
            let category = entry.category.clone();
            Self::push_parent_conflict(&mut conflicts, &entry.target);

            if entry.target.exists() == true && entry.target.is_dir() == true
            {
                conflicts.push(format!("target '{}' exists as a directory", entry.target.display()));
            }
            else if entry.target.exists() == false
            {
                planned_files.push(PlannedFileAction { index, source_sha, category, kind: PlannedFileActionKind::Copy });
            }
            else if is_changelog_protected(&entry.target) == true
            {
                // Changelog-marker files (e.g. UPDATES.md) hold a user-owned, append-only
                // log below the marker. Keying protection on FileStatus::Modified alone
                // misses the common case where 'merge' already re-recorded the tracker SHA,
                // so key on the marker itself instead; 'merge' is the only refresh path.
                planned_files.push(PlannedFileAction { index, source_sha, category, kind: PlannedFileActionKind::SkipChangelog });
            }
            else
            {
                match file_tracker.check_modification(&entry.target)?
                {
                    | FileStatus::Unmodified =>
                    {
                        if let Some(metadata) = file_tracker.get_metadata(&entry.target)
                        {
                            let adds_new_owners = metadata.would_add_owner_lists(&entry.lang, &entry.agent);
                            if metadata.original_sha == source_sha
                            {
                                planned_files.push(PlannedFileAction { index, source_sha, category, kind: PlannedFileActionKind::RefreshTracker });
                            }
                            else if adds_new_owners == true
                            {
                                conflicts.push(format!(
                                    "target '{}' is already owned by another language or agent with different template content; use 'slopctl merge' to combine it",
                                    entry.target.display()
                                ));
                            }
                            else
                            {
                                planned_files.push(PlannedFileAction { index, source_sha, category, kind: PlannedFileActionKind::Copy });
                            }
                        }
                        else
                        {
                            conflicts.push(format!("target '{}' is tracked as unmodified but metadata is missing", entry.target.display()));
                        }
                    }
                    | FileStatus::Deleted =>
                    {
                        planned_files.push(PlannedFileAction { index, source_sha, category, kind: PlannedFileActionKind::Copy });
                    }
                    | FileStatus::NotTracked =>
                    {
                        conflicts.push(format!("target '{}' already exists but is not tracked", entry.target.display()));
                    }
                    | FileStatus::Modified =>
                    {
                        let adds_new_owners =
                            file_tracker.get_metadata(&entry.target).map(|metadata| metadata.would_add_owner_lists(&entry.lang, &entry.agent)).unwrap_or(true);
                        if adds_new_owners == true
                        {
                            conflicts.push(format!(
                                "target '{}' has local modifications and cannot be shared with a new language or agent; use 'slopctl merge' to combine it",
                                entry.target.display()
                            ));
                        }
                        else if options.force == true
                        {
                            planned_files.push(PlannedFileAction { index, source_sha, category, kind: PlannedFileActionKind::Copy });
                        }
                        else
                        {
                            // Already-owned modified files never block an install: keep the
                            // local version; 'slopctl merge' is the update path.
                            planned_files.push(PlannedFileAction { index, source_sha, category, kind: PlannedFileActionKind::SkipModified });
                        }
                    }
                }
            }
        }

        if conflicts.is_empty() == false
        {
            let details = conflicts.iter().map(|conflict| format!("- {}", conflict)).collect::<Vec<_>>().join("\n");
            return Err(anyhow::anyhow!("Installation preflight failed:\n{}", details));
        }

        Ok(PreflightPlan { files: planned_files })
    }

    /// Updates local templates from global storage
    ///
    /// This method:
    /// 1. Resolves all files from templates.yml
    /// 2. Detects local modifications to AGENTS.md
    /// 3. Copies templates to current directory
    /// 4. Installs skills from templates.yml and CLI args
    ///
    /// Single AGENTS.md works for all agents. Agent-specific instruction files
    /// and prompts are copied if agent is specified.
    ///
    /// # Arguments
    ///
    /// * `options` - Aggregated CLI parameters for the update operation
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Global templates don't exist
    /// - Local modifications detected and force is false
    /// - Copy operations fail
    pub fn update(&self, options: &UpdateOptions) -> Result<()>
    {
        let resolved = self.resolve_all_files(options)?;
        let ctx = &resolved.context;

        let workspace = std::env::current_dir()?;
        let mut file_tracker = FileTracker::new(&workspace)?;

        let skip_agents_md = ctx.target.exists() && is_file_customized(&ctx.target)?;
        let preflight = self.preflight_installation(ctx, skip_agents_md, options, &resolved.files, &resolved.directories, &file_tracker)?;

        if skip_agents_md && options.force == false
        {
            println!("{} Local AGENTS.md has been customized and will be skipped", "!".yellow());
            if options.dry_run == false
            {
                println!("{} Other files will still be updated", "→".blue());
            }
            println!("{} Use --force to overwrite AGENTS.md", "→".blue());
        }

        if options.dry_run == true
        {
            self.show_dry_run_files(ctx, skip_agents_md, options, &resolved.files, &resolved.directories, &preflight);
            return Ok(());
        }

        self.handle_main_template(ctx, options, skip_agents_md, &mut file_tracker)?;

        for dir_path in &resolved.directories
        {
            fs::create_dir_all(dir_path)?;
            println!("  {} {} (directory)", "✓".green(), dir_path.display().to_string().yellow());
        }

        self.copy_files_with_tracking(&resolved.files, &preflight, &mut file_tracker, ctx.template_version)?;

        file_tracker.save()?;

        println!("{} Templates updated successfully", "✓".green());
        if options.agent.is_some()
        {
            println!("{} Single AGENTS.md + agent-specific files", "→".blue());
        }
        else
        {
            println!("{} Single AGENTS.md works with all agents", "→".blue());
        }

        Ok(())
    }

    /// Merges fragment files into main AGENTS.md at insertion points and writes to disk
    ///
    /// Delegates content generation to `generate_fresh_main()`, then writes the
    /// result to the target path.
    ///
    /// # Arguments
    ///
    /// * `ctx` - Main template context containing source, target, and fragments
    /// * `options` - Update options containing lang and mission settings
    ///
    /// # Errors
    ///
    /// Returns an error if file reading or writing fails
    fn merge_fragments(&self, ctx: &TemplateContext, options: &UpdateOptions) -> Result<()>
    {
        if options.mission.is_some() == true
        {
            println!("{} Using custom mission statement", "→".blue());
        }

        let content = Self::generate_fresh_main(ctx, options)?;

        if let Some(parent) = ctx.target.parent()
        {
            fs::create_dir_all(parent)?;
        }
        fs::write(&ctx.target, content)?;

        Ok(())
    }

    /// Shows dry-run preview of files and directories
    ///
    /// # Arguments
    ///
    /// * `ctx` - Template context for main AGENTS.md
    /// * `skip_agents_md` - Whether AGENTS.md is customized and should be skipped
    /// * `options` - Update options containing force and dry_run settings
    /// * `files_to_copy` - List of (source, target) file tuples
    /// * `directories` - List of directory paths
    fn show_dry_run_files(
        &self, ctx: &TemplateContext, skip_agents_md: bool, options: &UpdateOptions, files_to_copy: &[ResolvedFile], directories: &[PathBuf],
        preflight: &PreflightPlan
    )
    {
        println!("\n{} Files that would be created/modified:", "→".blue());

        if skip_agents_md && options.force == false
        {
            println!("  {} {} (skipped - customized)", "○".yellow(), ctx.target.display());
        }
        else if ctx.target.exists()
        {
            println!("  {} {} (would be overwritten)", "●".yellow(), ctx.target.display());
        }
        else
        {
            println!("  {} {} (would be created)", "●".green(), ctx.target.display());
        }

        for planned in &preflight.files
        {
            let entry = &files_to_copy[planned.index];
            if planned.kind == PlannedFileActionKind::SkipModified
            {
                println!("  {} {} (skipped - local modifications preserved)", "○".yellow(), entry.target.display());
            }
            else if planned.kind == PlannedFileActionKind::SkipChangelog
            {
                println!("  {} {} (skipped - changelog log preserved)", "○".yellow(), entry.target.display());
            }
            else if planned.kind == PlannedFileActionKind::RefreshTracker
            {
                println!("  {} {} (already installed, tracker would be refreshed)", "○".yellow(), entry.target.display());
            }
            else if entry.target.exists()
            {
                println!("  {} {} (would be overwritten)", "●".yellow(), entry.target.display());
            }
            else
            {
                println!("  {} {} (would be created)", "●".green(), entry.target.display());
            }
        }

        if directories.is_empty() == false
        {
            println!("\n{} Directories that would be created:", "→".blue());
            for dir_path in directories
            {
                if dir_path.exists() == true
                {
                    println!("  {} {} (exists)", "○".yellow(), dir_path.display());
                }
                else
                {
                    println!("  {} {} (would be created)", "●".green(), dir_path.display());
                }
            }
        }

        println!("\n{} Dry run complete. No files were modified.", "✓".green());
    }

    /// Handles the main AGENTS.md template (merge fragments or copy as-is)
    ///
    /// Processes the main AGENTS.md template by either merging fragments into it
    /// or copying it directly. Records the installation in the file tracker.
    ///
    /// # Arguments
    ///
    /// * `ctx` - Main template context containing source, target, fragments, and template version
    /// * `options` - Update options containing mission, lang, and force settings
    /// * `skip_agents_md` - Whether AGENTS.md is customized and should be skipped
    /// * `file_tracker` - File tracker for recording installations
    ///
    /// # Errors
    ///
    /// Returns an error if file operations fail
    fn handle_main_template(&self, ctx: &TemplateContext, options: &UpdateOptions, skip_agents_md: bool, file_tracker: &mut FileTracker) -> Result<()>
    {
        if skip_agents_md && options.force == false
        {
            println!("{} Skipping AGENTS.md (customized)", "→".blue());
            return Ok(());
        }

        if ctx.fragments.is_empty() == false || options.mission.is_some() == true
        {
            println!("{} Merging fragments into AGENTS.md", "→".blue());
            self.merge_fragments(ctx, options)?;
        }
        else
        {
            if let Some(parent) = ctx.target.parent()
            {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&ctx.source, &ctx.target)?;
        }

        println!("  {} {}", "✓".green(), ctx.target.display().to_string().yellow());

        let sha = FileTracker::calculate_sha256(&ctx.target)?;
        let lang_owners = options.lang.map(|lang| vec![lang.to_string()]).unwrap_or_default();
        let agent_owners = options.agent.map(|agent| vec![agent.to_string()]).unwrap_or_default();
        file_tracker.record_installation_with_owners(&ctx.target, sha, ctx.template_version, &lang_owners, &agent_owners, "main".to_string());

        Ok(())
    }

    /// Copies preflighted template files to targets
    ///
    /// Preflight validates all targets before this method is called, so execution
    /// can copy files and refresh tracker metadata without interactive prompts.
    ///
    /// # Arguments
    ///
    /// * `files_to_copy` - Resolved file entries
    /// * `preflight` - Validated file actions to execute
    /// * `file_tracker` - File tracker for recording installations
    /// * `template_version` - Template version for file tracking (0 for standalone skill installs)
    /// # Errors
    ///
    /// Returns an error if file operations fail
    fn copy_files_with_tracking(&self, files_to_copy: &[ResolvedFile], preflight: &PreflightPlan, file_tracker: &mut FileTracker, template_version: u32)
    -> Result<()>
    {
        println!("{} Copying templates to target directories", "→".blue());

        for planned in &preflight.files
        {
            let entry = &files_to_copy[planned.index];
            let source = &entry.source;
            let target = &entry.target;

            if planned.kind == PlannedFileActionKind::SkipModified
            {
                // Keep the user's version and the original tracker SHA; 'slopctl merge' is the update path.
                println!(
                    "  {} {} (skipped - local modifications preserved; use 'slopctl merge' to apply template updates)",
                    "○".yellow(),
                    target.display().to_string().yellow()
                );
            }
            else if planned.kind == PlannedFileActionKind::SkipChangelog
            {
                // Never write a changelog-marker file here; 'slopctl merge' splices the
                // template half back in without touching the user-owned log below the marker.
                println!(
                    "  {} {} (skipped - changelog log preserved; use 'slopctl merge' to apply template updates)",
                    "○".yellow(),
                    target.display().to_string().yellow()
                );
            }
            else
            {
                if planned.kind == PlannedFileActionKind::Copy
                {
                    copy_file_with_mkdir(source, target)?;
                    println!("  {} {}", "✓".green(), target.display().to_string().yellow());
                }
                else
                {
                    println!("  {} {} (already installed)", "○".green(), target.display().to_string().yellow());
                }

                file_tracker.record_installation_with_owners(
                    target,
                    planned.source_sha.clone(),
                    template_version,
                    &entry.lang,
                    &entry.agent,
                    planned.category.clone()
                );
            }
        }

        Ok(())
    }

    /// Install skills into the given skill directory
    ///
    /// For each skill, resolves the source (local or GitHub) and adds file entries
    /// to the files_to_copy list. GitHub skills are discovered via SKILL.md scanning
    /// and downloaded recursively (including subdirectories). Local skills are copied
    /// recursively; absolute paths are used directly while relative paths are resolved
    /// against the global template cache.
    ///
    /// # Arguments
    ///
    /// * `skills` - Iterator of (name, source) pairs
    /// * `skill_base_dir` - Resolved target directory for skills
    /// * `temp_dir` - Temporary directory for GitHub downloads
    /// * `local_cache_only` - When true, read URL skills from the global cache only
    /// * `files_to_copy` - Accumulator for (source, target) file pairs
    #[allow(clippy::too_many_arguments)]
    fn install_skills<'b, I>(
        &self, skills: I, skill_base_dir: &Path, temp_dir: &Path, lang: &str, agent: &str, local_cache_only: bool, files_to_copy: &mut Vec<ResolvedFile>
    ) -> Result<()>
    where I: Iterator<Item = (&'b str, &'b str)>
    {
        for (skill_name, source) in skills
        {
            if github::is_url(source) == true
            {
                if local_cache_only == true
                {
                    let source_dir = self.cached_skill_dir(skill_name);
                    require!(
                        source_dir.is_dir() == true,
                        Err(anyhow::anyhow!("Skill '{}' not found in local template cache. Run 'slopctl templates --update' first.", skill_name))
                    );
                    let target_base = skill_base_dir.join(skill_name);
                    Self::collect_local_skill_files(&source_dir, &target_base, lang, agent, files_to_copy)?;
                    continue;
                }

                let parsed = github::parse_github_url(source).ok_or_else(|| anyhow::anyhow!("Invalid GitHub URL for skill '{}': {}", skill_name, source))?;

                println!("{} Installing skills from {}...", "→".blue(), source.yellow());

                let staging = temp_dir.join(format!("repo_{}_{}", parsed.owner, parsed.repo));
                let repo_root = github::fetch_repo_extracted_into(&parsed.owner, &parsed.repo, &parsed.branch, &staging)?;
                let search_root = if parsed.path.is_empty() == true
                {
                    repo_root
                }
                else
                {
                    repo_root.join(&parsed.path)
                };

                let discovered = github::discover_skills_in_dir(&search_root);
                if discovered.is_empty() == true
                {
                    println!("{} No skills found (no SKILL.md) at {}", "!".yellow(), source.yellow());
                    continue;
                }

                for (name, skill_path) in discovered
                {
                    let target_base = skill_base_dir.join(&name);
                    println!("{} Installing skill '{}' from GitHub...", "→".blue(), name.green());
                    Self::collect_local_skill_files(&skill_path, &target_base, lang, agent, files_to_copy)?;
                }
            }
            else
            {
                let source_path = Path::new(source);
                let source_dir = if source_path.is_absolute() == true
                {
                    source_path.to_path_buf()
                }
                else
                {
                    self.config_dir.join(source)
                };
                let label = if source_path.is_absolute() == true
                {
                    source
                }
                else
                {
                    "local templates"
                };

                if source_dir.is_dir() == true
                {
                    let target_base = skill_base_dir.join(skill_name);
                    if local_cache_only == false
                    {
                        println!("{} Installing skill '{}' from {}...", "→".blue(), skill_name.green(), label.yellow());
                    }
                    Self::collect_local_skill_files(&source_dir, &target_base, lang, agent, files_to_copy)?;
                }
                else if source_dir.is_file() == true
                {
                    let target_base = skill_base_dir.join(skill_name);
                    let target_path = source_dir.file_name().map(|f| target_base.join(f));
                    if let Some(target) = target_path
                    {
                        if local_cache_only == false
                        {
                            println!("{} Installing skill '{}' from {}...", "→".blue(), skill_name.green(), label.yellow());
                        }
                        files_to_copy.push(ResolvedFile {
                            source: source_dir,
                            target,
                            lang: Self::owner_list(lang, LANG_NONE),
                            agent: Self::owner_list(agent, AGENT_ALL),
                            category: "skill".to_string()
                        });
                    }
                }
                else if local_cache_only == true
                {
                    return Err(anyhow::anyhow!("Skill '{}' not found in local template cache. Run 'slopctl templates --update' first.", skill_name));
                }
                else
                {
                    println!("{} Skill source not found: {}", "!".yellow(), source.yellow());
                }
            }
        }

        Ok(())
    }

    /// Recursively collect all files from a local skill directory
    pub fn collect_local_skill_files(source_dir: &Path, target_base: &Path, lang: &str, agent: &str, files_to_copy: &mut Vec<ResolvedFile>) -> Result<()>
    {
        for entry in fs::read_dir(source_dir)?
        {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() == true
            {
                if let Some(dir_name) = path.file_name()
                {
                    Self::collect_local_skill_files(&path, &target_base.join(dir_name), lang, agent, files_to_copy)?;
                }
            }
            else if path.is_file() == true &&
                let Some(filename) = path.file_name()
            {
                files_to_copy.push(ResolvedFile {
                    source:   path.clone(),
                    target:   target_base.join(filename),
                    lang:     Self::owner_list(lang, LANG_NONE),
                    agent:    Self::owner_list(agent, AGENT_ALL),
                    category: "skill".to_string()
                });
            }
        }

        Ok(())
    }
}

/// Normalizes a path to its canonical form for map lookups
///
/// Falls back to the original path if canonicalization fails (e.g. file doesn't exist yet).
pub fn normalize_path(path: &Path) -> PathBuf
{
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

#[cfg(test)]
mod tests
{
    use std::{fs, path::PathBuf};

    use super::*;
    use crate::file_tracker::{AGENT_ALL, LANG_NONE};

    fn rf(source: &str, target: &str) -> ResolvedFile
    {
        ResolvedFile {
            source:   PathBuf::from(source),
            target:   PathBuf::from(target),
            lang:     Vec::new(),
            agent:    Vec::new(),
            category: "language".to_string()
        }
    }

    // -- file_contains_changelog_marker --

    #[test]
    fn test_file_contains_changelog_marker_present_true()
    {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("UPDATES.md");
        fs::write(&path, format!("# Log\n\n{}\n\n### entry\n", CHANGELOG_MARKER)).unwrap();
        assert!(file_contains_changelog_marker(&path) == true);
    }

    #[test]
    fn test_file_contains_changelog_marker_absent_false()
    {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("plain.md");
        fs::write(&path, "# Log\n\nno marker here\n").unwrap();
        assert!(file_contains_changelog_marker(&path) == false);
    }

    #[test]
    fn test_file_contains_changelog_marker_missing_file_false()
    {
        let dir = tempfile::TempDir::new().unwrap();
        assert!(file_contains_changelog_marker(&dir.path().join("missing.md")) == false);
    }

    #[test]
    fn test_file_contains_changelog_marker_inline_mention_false()
    {
        // Documentation that explains the marker syntax (e.g. the recent-updates
        // skill) mentions it inline, not as a standalone line; it must not be
        // mistaken for an actual changelog-marker file.
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("SKILL.md");
        fs::write(&path, format!("- `UPDATES.md` contains a title, a short intro, and a `{}` marker line\n", CHANGELOG_MARKER)).unwrap();
        assert!(file_contains_changelog_marker(&path) == false);
    }

    // -- is_changelog_protected --

    #[test]
    fn test_is_changelog_protected_marker_present_true()
    {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("UPDATES.md");
        fs::write(&path, format!("# Log\n\n{}\n\n### entry\n", CHANGELOG_MARKER)).unwrap();
        assert!(is_changelog_protected(&path) == true);
    }

    #[test]
    fn test_is_changelog_protected_marker_absent_false()
    {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("plain.md");
        fs::write(&path, "# Log\n\nno marker here\n").unwrap();
        assert!(is_changelog_protected(&path) == false);
    }

    #[test]
    fn test_is_changelog_protected_missing_file_false()
    {
        let dir = tempfile::TempDir::new().unwrap();
        assert!(is_changelog_protected(&dir.path().join("missing.md")) == false);
    }

    // -- load_template_config --

    #[test]
    fn test_load_template_config_valid() -> anyhow::Result<()>
    {
        let dir = tempfile::TempDir::new()?;
        fs::write(dir.path().join("templates.yml"), "version: 5\nlanguages: {}")?;

        let config = load_template_config(dir.path())?;
        assert_eq!(config.version, 5);
        Ok(())
    }

    #[test]
    fn test_load_template_config_missing() -> anyhow::Result<()>
    {
        let dir = tempfile::TempDir::new()?;
        let err = load_template_config(dir.path()).unwrap_err();
        assert!(err.to_string().contains("not found") == true);
        Ok(())
    }

    // -- is_file_customized --

    #[test]
    fn test_is_file_customized_with_marker() -> anyhow::Result<()>
    {
        let dir = tempfile::TempDir::new()?;
        let path = dir.path().join("test.md");
        fs::write(&path, format!("{}\n# Content", TEMPLATE_MARKER))?;

        assert!(is_file_customized(&path)? == false);
        Ok(())
    }

    #[test]
    fn test_is_file_customized_without_marker() -> anyhow::Result<()>
    {
        let dir = tempfile::TempDir::new()?;
        let path = dir.path().join("test.md");
        fs::write(&path, "# Custom content with no marker")?;

        assert!(is_file_customized(&path)? == true);
        Ok(())
    }

    #[test]
    fn test_is_file_customized_nonexistent() -> anyhow::Result<()>
    {
        assert!(is_file_customized(Path::new("/nonexistent/file.md"))? == false);
        Ok(())
    }

    // -- resolve_placeholder --

    #[test]
    fn test_resolve_placeholder_workspace()
    {
        let engine = TemplateEngine::new(Path::new("/config"));
        let workspace = PathBuf::from("/projects/myapp");
        let userprofile = PathBuf::from("/home/user");

        let result = engine.resolve_placeholder("$workspace/AGENTS.md", &workspace, &userprofile);
        assert_eq!(result, PathBuf::from("/projects/myapp/AGENTS.md"));
    }

    #[test]
    fn test_resolve_placeholder_userprofile()
    {
        let engine = TemplateEngine::new(Path::new("/config"));
        let workspace = PathBuf::from("/projects/myapp");
        let userprofile = PathBuf::from("/home/user");

        let result = engine.resolve_placeholder("$userprofile/.bogus/prompts/init.md", &workspace, &userprofile);
        assert_eq!(result, PathBuf::from("/home/user/.bogus/prompts/init.md"));
    }

    #[test]
    fn test_resolve_placeholder_no_placeholder()
    {
        let engine = TemplateEngine::new(Path::new("/config"));
        let workspace = PathBuf::from("/projects/myapp");
        let userprofile = PathBuf::from("/home/user");

        let result = engine.resolve_placeholder("relative/path.md", &workspace, &userprofile);
        assert_eq!(result, PathBuf::from("relative/path.md"));
    }

    // -- merge_fragments --

    fn write_template(dir: &Path, content: &str) -> anyhow::Result<PathBuf>
    {
        let path = dir.join("AGENTS.md");
        fs::write(&path, content)?;
        Ok(path)
    }

    fn write_fragment(dir: &Path, name: &str, content: &str) -> anyhow::Result<PathBuf>
    {
        let path = dir.join(name);
        fs::write(&path, content)?;
        Ok(path)
    }

    static TEMPLATE_BASE: &str = "\
# AGENTS.md

<!-- {mission} -->

<!-- {principles} -->

<!-- {languages} -->

<!-- {integration} -->
";

    #[test]
    fn test_merge_fragments_single_category() -> anyhow::Result<()>
    {
        let dir = tempfile::TempDir::new()?;
        let source = write_template(dir.path(), TEMPLATE_BASE)?;
        let target = dir.path().join("output/AGENTS.md");
        let frag = write_fragment(dir.path(), "rpp.md", "## Rust++ Conventions\n\nUse the configured build tool.")?;

        let engine = TemplateEngine::new(dir.path());
        let ctx = TemplateContext { source, target: target.clone(), fragments: vec![(frag, "languages".to_string())], template_version: 5 };
        let options = UpdateOptions {
            lang:             Some("Rust++"),
            agent:            None,
            mission:          None,
            force:            false,
            dry_run:          false,
            partial:          None,
            local_cache_only: false
        };

        engine.merge_fragments(&ctx, &options)?;

        let output = fs::read_to_string(&target)?;
        assert!(output.contains("## Rust++ Conventions") == true);
        assert!(output.contains("<!-- {languages} -->") == true);
        Ok(())
    }

    #[test]
    fn test_merge_fragments_multiple_categories() -> anyhow::Result<()>
    {
        let dir = tempfile::TempDir::new()?;
        let source = write_template(dir.path(), TEMPLATE_BASE)?;
        let target = dir.path().join("output/AGENTS.md");

        let mission_frag = write_fragment(dir.path(), "mission.md", "## Mission\n\nBuild great software.")?;
        let principles_frag = write_fragment(dir.path(), "principles.md", "## Principles\n\nKeep it simple.")?;
        let lang_frag = write_fragment(dir.path(), "lang.md", "## Rust++\n\nUse the configured linter.")?;

        let engine = TemplateEngine::new(dir.path());
        let ctx = TemplateContext {
            source,
            target: target.clone(),
            fragments: vec![(mission_frag, "mission".to_string()), (principles_frag, "principles".to_string()), (lang_frag, "languages".to_string())],
            template_version: 5
        };
        let options = UpdateOptions {
            lang:             Some("Rust++"),
            agent:            None,
            mission:          None,
            force:            false,
            dry_run:          false,
            partial:          None,
            local_cache_only: false
        };

        engine.merge_fragments(&ctx, &options)?;

        let output = fs::read_to_string(&target)?;
        assert!(output.contains("Build great software") == true);
        assert!(output.contains("Keep it simple") == true);
        assert!(output.contains("Use the configured linter") == true);
        Ok(())
    }

    #[test]
    fn test_merge_fragments_no_lang() -> anyhow::Result<()>
    {
        let dir = tempfile::TempDir::new()?;
        let source = write_template(dir.path(), TEMPLATE_BASE)?;
        let target = dir.path().join("output/AGENTS.md");

        let engine = TemplateEngine::new(dir.path());
        let ctx = TemplateContext { source, target: target.clone(), fragments: vec![], template_version: 5 };
        let options = UpdateOptions {
            lang:             None,
            agent:            None,
            mission:          None,
            force:            false,
            dry_run:          false,
            partial:          None,
            local_cache_only: false
        };

        engine.merge_fragments(&ctx, &options)?;

        let output = fs::read_to_string(&target)?;
        assert!(output.contains("<!-- {languages} -->") == true);
        // Languages insertion point should be followed by empty content (just newlines)
        assert!(output.contains("<!-- {languages} -->\n\n") == true);
        Ok(())
    }

    #[test]
    fn test_merge_fragments_custom_mission() -> anyhow::Result<()>
    {
        let dir = tempfile::TempDir::new()?;
        let source = write_template(dir.path(), TEMPLATE_BASE)?;
        let target = dir.path().join("output/AGENTS.md");

        let engine = TemplateEngine::new(dir.path());
        let ctx = TemplateContext { source, target: target.clone(), fragments: vec![], template_version: 5 };
        let options = UpdateOptions {
            lang:             None,
            agent:            None,
            mission:          Some("We build CLI tools."),
            force:            false,
            dry_run:          false,
            partial:          None,
            local_cache_only: false
        };

        engine.merge_fragments(&ctx, &options)?;

        let output = fs::read_to_string(&target)?;
        assert!(output.contains("## Mission Statement") == true);
        assert!(output.contains("We build CLI tools.") == true);
        Ok(())
    }

    #[test]
    fn test_merge_fragments_removes_template_marker() -> anyhow::Result<()>
    {
        let dir = tempfile::TempDir::new()?;
        let content_with_marker = format!("{}\n{}", TEMPLATE_MARKER, TEMPLATE_BASE);
        let source = write_template(dir.path(), &content_with_marker)?;
        let target = dir.path().join("output/AGENTS.md");

        let engine = TemplateEngine::new(dir.path());
        let ctx = TemplateContext { source, target: target.clone(), fragments: vec![], template_version: 5 };
        let options = UpdateOptions {
            lang:             None,
            agent:            None,
            mission:          None,
            force:            false,
            dry_run:          false,
            partial:          None,
            local_cache_only: false
        };

        engine.merge_fragments(&ctx, &options)?;

        let output = fs::read_to_string(&target)?;
        assert!(output.contains(TEMPLATE_MARKER) == false);
        Ok(())
    }

    // -- validate_no_duplicate_targets --

    #[test]
    fn test_validate_no_duplicates_empty()
    {
        assert!(validate_no_duplicate_targets(&[]).is_ok() == true);
    }

    #[test]
    fn test_validate_no_duplicates_unique_targets()
    {
        let files = vec![rf("a.txt", "/workspace/.gitignore"), rf("b.txt", "/workspace/.editorconfig")];
        assert!(validate_no_duplicate_targets(&files).is_ok() == true);
    }

    #[test]
    fn test_validate_duplicate_targets_rejected()
    {
        let files = vec![rf("lang-gitignore.txt", "/workspace/.gitignore"), rf("shared-gitignore.txt", "/workspace/.gitignore")];
        let err = validate_no_duplicate_targets(&files).unwrap_err();
        assert!(err.to_string().contains("Duplicate target") == true);
        assert!(err.to_string().contains(".gitignore") == true);
        assert!(err.to_string().contains("lang-gitignore.txt") == true);
        assert!(err.to_string().contains("shared-gitignore.txt") == true);
    }

    #[test]
    fn test_validate_same_source_different_targets()
    {
        let files = vec![rf("template.ini", "/workspace/.editorconfig"), rf("template.ini", "/workspace/.other-config")];
        assert!(validate_no_duplicate_targets(&files).is_ok() == true);
    }

    // -- cross-client skill directory --

    #[test]
    fn test_resolve_cross_client_skill_dir()
    {
        let engine = TemplateEngine::new(Path::new("/config"));
        let workspace = PathBuf::from("/projects/myapp");
        let userprofile = PathBuf::from("/home/user");

        let result = engine.resolve_placeholder(crate::agent_defaults::CROSS_CLIENT_SKILL_DIR, &workspace, &userprofile);
        assert_eq!(result, PathBuf::from("/projects/myapp/.agents/skills"));
    }

    #[test]
    fn test_skill_base_dir_with_agent_uses_agent_specific() -> anyhow::Result<()>
    {
        let engine = TemplateEngine::new(Path::new("/config"));
        let workspace = PathBuf::from("/projects/myapp");
        let userprofile = PathBuf::from("/home/user");

        let catalog = crate::agent_defaults::parse_agent_catalog(
            r#"
version: 1
agents:
  - name: bogus
    markers:
      - .bogus
    prompt_dir: '$workspace/.bogus/prompts'
    skill_dir: '$workspace/.bogus/skills'
    reads_cross_client_skills: true
"#
        )?;
        let dir_template = crate::agent_defaults::get_skill_dir_from_catalog(&catalog, "bogus").expect("bogus should have skill dir");
        let result = engine.resolve_placeholder(dir_template, &workspace, &userprofile);
        assert_eq!(result, PathBuf::from("/projects/myapp/.bogus/skills"));
        Ok(())
    }

    #[test]
    fn test_skill_base_dir_without_agent_uses_cross_client()
    {
        let engine = TemplateEngine::new(Path::new("/config"));
        let workspace = PathBuf::from("/projects/myapp");
        let userprofile = PathBuf::from("/home/user");

        let result = engine.resolve_placeholder(crate::agent_defaults::CROSS_CLIENT_SKILL_DIR, &workspace, &userprofile);
        assert!(result.to_string_lossy().contains(".agents/skills") == true);
    }

    // -- install_skills (unit) --

    #[test]
    fn test_install_skills_local_to_cross_client_dir() -> anyhow::Result<()>
    {
        let config_dir = tempfile::TempDir::new()?;
        let workspace_dir = tempfile::TempDir::new()?;
        let temp_dir = tempfile::TempDir::new()?;

        let skill_source = workspace_dir.path().join("test-skill");
        fs::create_dir_all(&skill_source)?;
        fs::write(skill_source.join("SKILL.md"), "---\nname: test-skill\ndescription: A test skill.\n---\n\n# Test Skill\n")?;

        let engine = TemplateEngine::new(config_dir.path());
        let skill_base_dir = workspace_dir.path().join(".agents/skills");
        let mut files_to_copy: Vec<ResolvedFile> = Vec::new();

        let source_str = skill_source.to_string_lossy().to_string();
        let skills_input = [("test-skill".to_string(), source_str)];
        engine.install_skills(
            skills_input.iter().map(|(n, s)| (n.as_str(), s.as_str())),
            &skill_base_dir,
            temp_dir.path(),
            LANG_NONE,
            AGENT_ALL,
            false,
            &mut files_to_copy
        )?;

        assert_eq!(files_to_copy.len(), 1);
        assert_eq!(files_to_copy[0].target, skill_base_dir.join("test-skill/SKILL.md"));
        Ok(())
    }

    #[test]
    fn test_install_skills_local_with_subdirectories() -> anyhow::Result<()>
    {
        let config_dir = tempfile::TempDir::new()?;
        let workspace_dir = tempfile::TempDir::new()?;
        let temp_dir = tempfile::TempDir::new()?;

        let skill_source = workspace_dir.path().join("my-skill");
        fs::create_dir_all(skill_source.join("scripts"))?;
        fs::write(skill_source.join("SKILL.md"), "---\nname: my-skill\ndescription: Test.\n---\n")?;
        fs::write(skill_source.join("scripts/setup.sh"), "#!/bin/bash\necho hello")?;

        let engine = TemplateEngine::new(config_dir.path());
        let skill_base_dir = workspace_dir.path().join(".agents/skills");
        let mut files_to_copy: Vec<ResolvedFile> = Vec::new();

        let source_str = skill_source.to_string_lossy().to_string();
        let skills_input = [("my-skill".to_string(), source_str)];
        engine.install_skills(
            skills_input.iter().map(|(n, s)| (n.as_str(), s.as_str())),
            &skill_base_dir,
            temp_dir.path(),
            LANG_NONE,
            AGENT_ALL,
            false,
            &mut files_to_copy
        )?;

        assert_eq!(files_to_copy.len(), 2);
        let targets: Vec<PathBuf> = files_to_copy.iter().map(|e| e.target.clone()).collect();
        assert!(targets.contains(&skill_base_dir.join("my-skill/SKILL.md")) == true);
        assert!(targets.contains(&skill_base_dir.join("my-skill/scripts/setup.sh")) == true);
        Ok(())
    }

    /// Write a minimal templates.yml with synthetic agents and languages
    fn write_minimal_templates_yml(dir: &std::path::Path) -> anyhow::Result<()>
    {
        let yml = r#"version: 5
main:
  source: AGENTS.md
  target: '$workspace/AGENTS.md'
agents:
  bogus:
    instructions:
      - source: bogus/instructions.md
        target: '$workspace/.bogus/instructions.md'
  fake:
    instructions:
      - source: fake/instructions.md
        target: '$workspace/.fake/instructions.md'
languages:
  Rust++:
    files:
      - source: rpp-format.toml
        target: '$workspace/.rpp.toml'
  CppScript:
    files:
      - source: cppscript-format.json
        target: '$workspace/.cppscript-format'
"#;
        fs::write(dir.join("templates.yml"), yml)?;
        fs::write(dir.join("AGENTS.md"), TEMPLATE_BASE)?;
        Ok(())
    }

    #[test]
    fn test_update_rejects_unknown_agent() -> anyhow::Result<()>
    {
        let config_dir = tempfile::TempDir::new()?;
        write_minimal_templates_yml(config_dir.path())?;

        let engine = TemplateEngine::new(config_dir.path());
        let options = UpdateOptions {
            lang:             None,
            agent:            Some("nonexistent"),
            mission:          None,
            force:            false,
            dry_run:          false,
            partial:          None,
            local_cache_only: false
        };

        let result = engine.update(&options);
        assert!(result.is_err() == true);
        let err = result.unwrap_err().to_string();
        assert!(err.contains("not found in templates.yml") == true);
        assert!(err.contains("nonexistent") == true);
        assert!(err.contains("bogus") == true);
        assert!(err.contains("fake") == true);
        Ok(())
    }

    #[test]
    fn test_update_rejects_unknown_language() -> anyhow::Result<()>
    {
        let config_dir = tempfile::TempDir::new()?;
        write_minimal_templates_yml(config_dir.path())?;

        let engine = TemplateEngine::new(config_dir.path());
        let options = UpdateOptions {
            lang:             Some("nonexistent"),
            agent:            None,
            mission:          None,
            force:            false,
            dry_run:          false,
            partial:          None,
            local_cache_only: false
        };

        let result = engine.update(&options);
        assert!(result.is_err() == true);
        let err = result.unwrap_err().to_string();
        assert!(err.contains("not found in templates.yml") == true);
        assert!(err.contains("nonexistent") == true);
        assert!(err.contains("Rust++") == true);
        assert!(err.contains("CppScript") == true);
        Ok(())
    }

    #[test]
    fn test_update_accepts_known_agent() -> anyhow::Result<()>
    {
        let config_dir = tempfile::TempDir::new()?;
        write_minimal_templates_yml(config_dir.path())?;

        fs::create_dir_all(config_dir.path().join("bogus"))?;
        fs::write(config_dir.path().join("bogus/instructions.md"), "test")?;

        let engine = TemplateEngine::new(config_dir.path());
        let options = UpdateOptions {
            lang:             None,
            agent:            Some("bogus"),
            mission:          None,
            force:            false,
            dry_run:          true,
            partial:          None,
            local_cache_only: false
        };

        let result = engine.update(&options);
        assert!(result.is_ok() == true);
        Ok(())
    }

    #[test]
    fn test_resolve_all_files_includes_agent_marker_directory() -> anyhow::Result<()>
    {
        let _cwd = crate::template_manager::CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let workspace = tempfile::TempDir::new()?;
        let config_dir = tempfile::TempDir::new()?;
        let original_cwd = std::env::current_dir()?;
        std::env::set_current_dir(workspace.path())?;

        let yaml = "version: 5\nmain:\n  source: AGENTS.md\n  target: '$workspace/AGENTS.md'\nagents:\n  bogus: {}\nlanguages: {}\nintegration: {}\n";
        fs::write(config_dir.path().join("templates.yml"), yaml)?;
        fs::write(config_dir.path().join("AGENTS.md"), "<!-- SLOPCTL-TEMPLATE -->\n# Project\n")?;
        write_synthetic_agent_defaults(config_dir.path(), &[("bogus", true, None)])?;

        let engine = TemplateEngine::new(config_dir.path());
        let options = UpdateOptions {
            lang:             None,
            agent:            Some("bogus"),
            mission:          None,
            force:            false,
            dry_run:          false,
            partial:          None,
            local_cache_only: false
        };
        let resolved = engine.resolve_all_files(&options);
        let _ = std::env::set_current_dir(&original_cwd);
        let resolved = resolved?;

        assert!(resolved.directories.iter().any(|path| path.ends_with(std::path::Path::new(".bogus"))) == true);
        assert!(resolved.directories.iter().any(|path| path.ends_with(std::path::Path::new("bogus.json"))) == false);
        Ok(())
    }

    #[test]
    fn test_update_accepts_known_language() -> anyhow::Result<()>
    {
        let _cwd = crate::template_manager::CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let workspace = tempfile::TempDir::new()?;
        let config_dir = tempfile::TempDir::new()?;
        let original_cwd = std::env::current_dir()?;
        std::env::set_current_dir(workspace.path())?;
        write_minimal_templates_yml(config_dir.path())?;

        fs::write(config_dir.path().join("rpp-format.toml"), "max_width = 100")?;

        let engine = TemplateEngine::new(config_dir.path());
        let options = UpdateOptions {
            lang:             Some("Rust++"),
            agent:            None,
            mission:          None,
            force:            false,
            dry_run:          true,
            partial:          None,
            local_cache_only: false
        };

        let result = engine.update(&options);
        let _ = std::env::set_current_dir(&original_cwd);
        assert!(result.is_ok() == true);
        Ok(())
    }

    /// Build a minimal config_dir with a templates.yml and AGENTS.md for skill routing tests.
    /// The agents list must include every agent name used in `--agent` for the test.
    fn setup_skill_routing_config(config_dir: &std::path::Path, skill_source_name: &str, agents: &[&str]) -> anyhow::Result<()>
    {
        use std::fs;
        let agents_yaml = agents.iter().map(|a| format!("  {}: {{}}", a)).collect::<Vec<_>>().join("\n");
        let yaml = format!(
            "version: 5\nmain:\n  source: AGENTS.md\n  target: '$workspace/AGENTS.md'\nagents:\n{}\nlanguages:\n  Rust++:\n    skills:\n      - source: \
             'skills/{}'\nintegration: {{}}\n",
            agents_yaml, skill_source_name
        );
        fs::write(config_dir.join("templates.yml"), yaml)?;
        fs::write(config_dir.join("AGENTS.md"), "<!-- SLOPCTL-TEMPLATE -->\n# Project\n")?;
        let skill_dir = config_dir.join("skills").join(skill_source_name);
        fs::create_dir_all(&skill_dir)?;
        fs::write(skill_dir.join("SKILL.md"), "# Skill")?;
        Ok(())
    }

    #[test]
    fn test_lang_skills_route_to_native_dir_for_bogus() -> anyhow::Result<()>
    {
        let _cwd = crate::template_manager::CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let workspace = tempfile::TempDir::new()?;
        let config_dir = tempfile::TempDir::new()?;
        let original_cwd = std::env::current_dir()?;
        std::env::set_current_dir(workspace.path())?;

        setup_skill_routing_config(config_dir.path(), "rpp-skill", &["bogus"])?;
        write_synthetic_agent_defaults(config_dir.path(), &[("bogus", false, None)])?;

        let engine = TemplateEngine::new(config_dir.path());
        let options = UpdateOptions {
            lang:             Some("Rust++"),
            agent:            Some("bogus"),
            mission:          None,
            force:            false,
            dry_run:          false,
            partial:          None,
            local_cache_only: false
        };
        let resolved = engine.resolve_all_files(&options);
        let _ = std::env::set_current_dir(&original_cwd);
        let resolved = resolved?;

        let skill_targets: Vec<String> =
            resolved.files.iter().filter(|f| f.target.to_string_lossy().contains("SKILL.md")).map(|f| f.target.to_string_lossy().into_owned()).collect();

        assert!(skill_targets.is_empty() == false, "expected at least one skill file");
        for t in &skill_targets
        {
            assert!(t.contains(".bogus/skills"), "skill target should be in .bogus/skills/, got: {}", t);
            assert!(t.contains(".agents/skills") == false, "skill must not go to .agents/skills/ for bogus, got: {}", t);
        }

        Ok(())
    }

    #[test]
    fn test_lang_skills_route_to_cross_client_for_fake() -> anyhow::Result<()>
    {
        let _cwd = crate::template_manager::CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let workspace = tempfile::TempDir::new()?;
        let config_dir = tempfile::TempDir::new()?;
        let original_cwd = std::env::current_dir()?;
        std::env::set_current_dir(workspace.path())?;

        setup_skill_routing_config(config_dir.path(), "rpp-skill", &["fake"])?;
        write_synthetic_agent_defaults(config_dir.path(), &[("fake", true, None)])?;

        let engine = TemplateEngine::new(config_dir.path());
        let options = UpdateOptions {
            lang:             Some("Rust++"),
            agent:            Some("fake"),
            mission:          None,
            force:            false,
            dry_run:          false,
            partial:          None,
            local_cache_only: false
        };
        let resolved = engine.resolve_all_files(&options);
        let _ = std::env::set_current_dir(&original_cwd);
        let resolved = resolved?;

        let skill_targets: Vec<String> =
            resolved.files.iter().filter(|f| f.target.to_string_lossy().contains("SKILL.md")).map(|f| f.target.to_string_lossy().into_owned()).collect();

        assert!(skill_targets.is_empty() == false, "expected at least one skill file");
        for t in &skill_targets
        {
            assert!(t.contains(".agents/skills"), "skill target should be in .agents/skills/ for fake, got: {}", t);
        }

        Ok(())
    }

    /// Build a minimal config_dir whose top-level `skills:` section contains one skill.
    /// The agents list must include every agent name used in `--agent` for the test.
    fn setup_toplevel_skill_routing_config(config_dir: &std::path::Path, skill_source_name: &str, agents: &[&str]) -> anyhow::Result<()>
    {
        use std::fs;
        let agents_yaml = agents.iter().map(|a| format!("  {}: {{}}", a)).collect::<Vec<_>>().join("\n");
        let yaml = format!(
            "version: 5\nmain:\n  source: AGENTS.md\n  target: '$workspace/AGENTS.md'\nagents:\n{}\nlanguages: {{}}\nskills:\n  - source: 'skills/{}'\nintegration: \
             {{}}\n",
            agents_yaml, skill_source_name
        );
        fs::write(config_dir.join("templates.yml"), yaml)?;
        fs::write(config_dir.join("AGENTS.md"), "<!-- SLOPCTL-TEMPLATE -->\n# Project\n")?;
        let skill_dir = config_dir.join("skills").join(skill_source_name);
        fs::create_dir_all(&skill_dir)?;
        fs::write(skill_dir.join("SKILL.md"), "# Skill")?;
        Ok(())
    }

    fn write_synthetic_agent_defaults(config_dir: &std::path::Path, agents: &[(&str, bool, Option<&str>)]) -> anyhow::Result<()>
    {
        let entries = agents
            .iter()
            .map(|(name, reads_cross_client_skills, skill_dir_override)| {
                let skill = skill_dir_override.map(|d| format!("'{d}'")).unwrap_or_else(|| format!("'$workspace/.{name}/skills'"));
                format!(
                    "  - name: {name}\n    markers:\n      - .{name}\n    prompt_dir: '$workspace/.{name}/prompts'\n    skill_dir: {skill}\n    \
                     reads_cross_client_skills: {reads_cross_client_skills}\n"
                )
            })
            .collect::<Vec<_>>()
            .join("");
        fs::write(config_dir.join(agent_defaults::AGENT_DEFAULTS_FILE), format!("version: 1\nagents:\n{entries}"))?;
        Ok(())
    }

    #[test]
    fn test_toplevel_skills_route_to_cross_client_for_fake_from_defaults() -> anyhow::Result<()>
    {
        let _cwd = crate::template_manager::CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let workspace = tempfile::TempDir::new()?;
        let config_dir = tempfile::TempDir::new()?;
        let original_cwd = std::env::current_dir()?;
        std::env::set_current_dir(workspace.path())?;

        setup_toplevel_skill_routing_config(config_dir.path(), "git-workflow", &["fake"])?;
        write_synthetic_agent_defaults(config_dir.path(), &[("fake", true, None)])?;

        let engine = TemplateEngine::new(config_dir.path());
        let options = UpdateOptions {
            lang:             None,
            agent:            Some("fake"),
            mission:          None,
            force:            false,
            dry_run:          false,
            partial:          None,
            local_cache_only: false
        };
        let resolved = engine.resolve_all_files(&options);
        let _ = std::env::set_current_dir(&original_cwd);
        let resolved = resolved?;

        let skill_targets: Vec<String> =
            resolved.files.iter().filter(|f| f.target.to_string_lossy().contains("SKILL.md")).map(|f| f.target.to_string_lossy().into_owned()).collect();

        assert!(skill_targets.is_empty() == false, "expected at least one skill file");
        for target in &skill_targets
        {
            assert!(target.contains(".agents/skills"), "skill target should be in .agents/skills/ for cross-client fake, got: {}", target);
            assert!(target.contains(".fake/skills") == false, "cross-client fake must not route top-level skills to native dir, got: {}", target);
        }

        Ok(())
    }

    #[test]
    fn test_toplevel_skills_route_to_native_dir_for_bogus_from_defaults() -> anyhow::Result<()>
    {
        let _cwd = crate::template_manager::CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let workspace = tempfile::TempDir::new()?;
        let config_dir = tempfile::TempDir::new()?;
        let original_cwd = std::env::current_dir()?;
        std::env::set_current_dir(workspace.path())?;

        setup_toplevel_skill_routing_config(config_dir.path(), "git-workflow", &["bogus"])?;
        write_synthetic_agent_defaults(config_dir.path(), &[("bogus", false, None)])?;

        let engine = TemplateEngine::new(config_dir.path());
        let options = UpdateOptions {
            lang:             None,
            agent:            Some("bogus"),
            mission:          None,
            force:            false,
            dry_run:          false,
            partial:          None,
            local_cache_only: false
        };
        let resolved = engine.resolve_all_files(&options);
        let _ = std::env::set_current_dir(&original_cwd);
        let resolved = resolved?;

        let skill_targets: Vec<String> =
            resolved.files.iter().filter(|f| f.target.to_string_lossy().contains("SKILL.md")).map(|f| f.target.to_string_lossy().into_owned()).collect();

        assert!(skill_targets.is_empty() == false, "expected at least one skill file");
        for target in &skill_targets
        {
            assert!(target.contains(".bogus/skills"), "skill target should use bogus native skill dir, got: {}", target);
            assert!(target.contains(".agents/skills") == false, "native-only bogus must not route top-level skills to .agents/skills/, got: {}", target);
        }

        Ok(())
    }

    #[test]
    fn test_toplevel_skills_for_native_agent_ignore_existing_cross_client_dir() -> anyhow::Result<()>
    {
        let _cwd = crate::template_manager::CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let workspace = tempfile::TempDir::new()?;
        let config_dir = tempfile::TempDir::new()?;
        let original_cwd = std::env::current_dir()?;
        std::env::set_current_dir(workspace.path())?;

        let cross_skill = workspace.path().join(".agents/skills/git-workflow");
        fs::create_dir_all(&cross_skill)?;
        fs::write(cross_skill.join("SKILL.md"), "# Existing Cross Skill")?;
        setup_toplevel_skill_routing_config(config_dir.path(), "git-workflow", &["bogus"])?;
        write_synthetic_agent_defaults(config_dir.path(), &[("bogus", false, None)])?;

        let engine = TemplateEngine::new(config_dir.path());
        let options = UpdateOptions {
            lang:             None,
            agent:            Some("bogus"),
            mission:          None,
            force:            false,
            dry_run:          false,
            partial:          None,
            local_cache_only: false
        };
        let resolved = engine.resolve_all_files(&options);
        let _ = std::env::set_current_dir(&original_cwd);
        let resolved = resolved?;

        let native_target = std::path::Path::new(".bogus/skills/git-workflow/SKILL.md");
        let native_targets: Vec<&ResolvedFile> = resolved.files.iter().filter(|f| f.target.ends_with(native_target)).collect();
        assert_eq!(native_targets.len(), 1);

        let cross_rel = std::path::Path::new(".agents/skills");
        let cross_targets: Vec<&ResolvedFile> = resolved.files.iter().filter(|f| f.target.ancestors().any(|a| a.ends_with(cross_rel))).collect();
        assert!(cross_targets.is_empty() == true, "top-level skills must not target .agents/skills for agents that do not read it");

        Ok(())
    }

    fn preflight_toplevel_cross_client_skill(
        existing_content: &str, tracked_sha: Option<String>, source_content: &str
    ) -> anyhow::Result<(PreflightPlan, Vec<ResolvedFile>)>
    {
        let workspace = tempfile::TempDir::new()?;
        let config_dir = tempfile::TempDir::new()?;
        let original_cwd = std::env::current_dir()?;
        std::env::set_current_dir(workspace.path())?;

        setup_toplevel_skill_routing_config(config_dir.path(), "git-workflow", &["fake"])?;
        write_synthetic_agent_defaults(config_dir.path(), &[("fake", true, None)])?;
        fs::write(config_dir.path().join("skills/git-workflow/SKILL.md"), source_content)?;

        let target = workspace.path().join(".agents/skills/git-workflow/SKILL.md");
        fs::create_dir_all(target.parent().ok_or_else(|| anyhow::anyhow!("missing parent"))?)?;
        fs::write(&target, existing_content)?;

        let mut tracker = FileTracker::new(workspace.path())?;
        if let Some(sha) = tracked_sha
        {
            tracker.record_installation(&target, sha, 5, LANG_NONE.into(), AGENT_ALL.into(), "skill".into());
        }

        let engine = TemplateEngine::new(config_dir.path());
        let options = UpdateOptions {
            lang:             None,
            agent:            Some("fake"),
            mission:          None,
            force:            false,
            dry_run:          false,
            partial:          None,
            local_cache_only: false
        };
        let resolved = engine.resolve_all_files(&options)?;
        let plan = engine.preflight_installation(&resolved.context, false, &options, &resolved.files, &resolved.directories, &tracker);
        let _ = std::env::set_current_dir(&original_cwd);
        Ok((plan?, resolved.files))
    }

    #[test]
    fn test_preflight_cross_client_skill_same_tracked_file_refreshes_tracker() -> anyhow::Result<()>
    {
        let _cwd = crate::template_manager::CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let content = "# Skill";
        let existing = tempfile::NamedTempFile::new()?;
        fs::write(existing.path(), content)?;
        let sha = FileTracker::calculate_sha256(existing.path())?;

        let (plan, files) = preflight_toplevel_cross_client_skill(content, Some(sha), content)?;
        let shared_actions: Vec<&PlannedFileAction> =
            plan.files.iter().filter(|planned| files[planned.index].target.ends_with(std::path::Path::new(".agents/skills/git-workflow/SKILL.md"))).collect();

        assert_eq!(shared_actions.len(), 1);
        assert_eq!(shared_actions[0].kind, PlannedFileActionKind::RefreshTracker);
        Ok(())
    }

    #[test]
    fn test_execute_refresh_tracker_skips_copy() -> anyhow::Result<()>
    {
        let workspace = tempfile::TempDir::new()?;
        let config_dir = tempfile::TempDir::new()?;
        let target = workspace.path().join(".agents/skills/git-workflow/SKILL.md");
        fs::create_dir_all(target.parent().ok_or_else(|| anyhow::anyhow!("missing parent"))?)?;
        fs::write(&target, "# Skill")?;

        let source = config_dir.path().join("SKILL.md");
        fs::write(&source, "# Skill")?;
        let source_sha = FileTracker::calculate_sha256(&source)?;

        let files = vec![ResolvedFile { source, target: target.clone(), lang: Vec::new(), agent: Vec::new(), category: "skill".to_string() }];
        let plan = PreflightPlan {
            files: vec![PlannedFileAction {
                index:      0,
                source_sha: source_sha.clone(),
                category:   "skill".to_string(),
                kind:       PlannedFileActionKind::RefreshTracker
            }]
        };
        let mut tracker = FileTracker::new(workspace.path())?;
        let engine = TemplateEngine::new(config_dir.path());

        engine.copy_files_with_tracking(&files, &plan, &mut tracker, 5)?;

        assert_eq!(fs::read_to_string(&target)?, "# Skill");
        let metadata = tracker.get_metadata(&target).ok_or_else(|| anyhow::anyhow!("expected tracker metadata"))?;
        assert_eq!(metadata.original_sha, source_sha);
        assert_eq!(metadata.template_version, 5);
        Ok(())
    }

    #[test]
    fn test_preflight_cross_client_skill_different_tracked_file_errors() -> anyhow::Result<()>
    {
        let _cwd = crate::template_manager::CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let existing = tempfile::NamedTempFile::new()?;
        fs::write(existing.path(), "# Existing")?;
        let sha = FileTracker::calculate_sha256(existing.path())?;

        let result = preflight_toplevel_cross_client_skill("# Existing", Some(sha), "# Skill");

        assert!(result.is_err() == true);
        let Err(err) = result
        else
        {
            panic!("expected preflight error");
        };
        assert!(err.to_string().contains("Installation preflight failed") == true);
        Ok(())
    }

    #[test]
    fn test_preflight_cross_client_skill_modified_tracked_file_errors() -> anyhow::Result<()>
    {
        let _cwd = crate::template_manager::CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let original = tempfile::NamedTempFile::new()?;
        fs::write(original.path(), "# Original")?;
        let sha = FileTracker::calculate_sha256(original.path())?;

        let result = preflight_toplevel_cross_client_skill("# Modified", Some(sha), "# Skill");

        assert!(result.is_err() == true);
        let Err(err) = result
        else
        {
            panic!("expected preflight error");
        };
        assert!(err.to_string().contains("local modifications") == true);
        Ok(())
    }

    #[test]
    fn test_preflight_cross_client_skill_untracked_file_errors() -> anyhow::Result<()>
    {
        let _cwd = crate::template_manager::CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        let result = preflight_toplevel_cross_client_skill("# Existing", None, "# Skill");

        assert!(result.is_err() == true);
        let Err(err) = result
        else
        {
            panic!("expected preflight error");
        };
        assert!(err.to_string().contains("not tracked") == true);
        Ok(())
    }

    #[test]
    fn test_update_preflight_conflict_writes_nothing() -> anyhow::Result<()>
    {
        let _cwd = crate::template_manager::CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let workspace = tempfile::TempDir::new()?;
        let config_dir = tempfile::TempDir::new()?;
        let original_cwd = std::env::current_dir()?;
        std::env::set_current_dir(workspace.path())?;

        setup_toplevel_skill_routing_config(config_dir.path(), "git-workflow", &["fake"])?;
        write_synthetic_agent_defaults(config_dir.path(), &[("fake", true, None)])?;
        let target = workspace.path().join(".agents/skills/git-workflow/SKILL.md");
        fs::create_dir_all(target.parent().ok_or_else(|| anyhow::anyhow!("missing parent"))?)?;
        fs::write(&target, "# Untracked")?;

        let engine = TemplateEngine::new(config_dir.path());
        let options = UpdateOptions {
            lang:             None,
            agent:            Some("fake"),
            mission:          None,
            force:            false,
            dry_run:          false,
            partial:          None,
            local_cache_only: false
        };
        let result = engine.update(&options);
        let _ = std::env::set_current_dir(&original_cwd);

        assert!(result.is_err() == true);
        assert!(workspace.path().join("AGENTS.md").exists() == false);
        assert!(workspace.path().join(".fake").exists() == false);
        Ok(())
    }

    #[test]
    fn test_lang_skills_with_no_agents_route_to_cross_client() -> anyhow::Result<()>
    {
        let _cwd = crate::template_manager::CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let workspace = tempfile::TempDir::new()?;
        let config_dir = tempfile::TempDir::new()?;
        let original_cwd = std::env::current_dir()?;
        std::env::set_current_dir(workspace.path())?;

        setup_skill_routing_config(config_dir.path(), "rpp-skill", &[])?;
        write_synthetic_agent_defaults(config_dir.path(), &[("bogus", false, None), ("fake", true, None)])?;

        let engine = TemplateEngine::new(config_dir.path());
        let options = UpdateOptions {
            lang:             Some("Rust++"),
            agent:            None,
            mission:          None,
            force:            false,
            dry_run:          false,
            partial:          None,
            local_cache_only: false
        };
        let resolved = engine.resolve_all_files(&options);
        let _ = std::env::set_current_dir(&original_cwd);
        let resolved = resolved?;

        let skill_targets: Vec<String> =
            resolved.files.iter().filter(|f| f.target.to_string_lossy().contains("SKILL.md")).map(|f| f.target.to_string_lossy().into_owned()).collect();

        assert_eq!(skill_targets.len(), 1);
        assert!(skill_targets[0].contains(".agents/skills") == true);
        Ok(())
    }

    #[test]
    fn test_lang_skills_with_installed_native_agent_route_to_native_dir() -> anyhow::Result<()>
    {
        let _cwd = crate::template_manager::CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let workspace = tempfile::TempDir::new()?;
        let config_dir = tempfile::TempDir::new()?;
        let original_cwd = std::env::current_dir()?;
        std::env::set_current_dir(workspace.path())?;

        fs::create_dir_all(workspace.path().join(".bogus"))?;
        setup_skill_routing_config(config_dir.path(), "rpp-skill", &[])?;
        write_synthetic_agent_defaults(config_dir.path(), &[("bogus", false, None), ("fake", true, None)])?;

        let engine = TemplateEngine::new(config_dir.path());
        let options = UpdateOptions {
            lang:             Some("Rust++"),
            agent:            None,
            mission:          None,
            force:            false,
            dry_run:          false,
            partial:          None,
            local_cache_only: false
        };
        let resolved = engine.resolve_all_files(&options);
        let _ = std::env::set_current_dir(&original_cwd);
        let resolved = resolved?;

        let skill_targets: Vec<String> =
            resolved.files.iter().filter(|f| f.target.to_string_lossy().contains("SKILL.md")).map(|f| f.target.to_string_lossy().into_owned()).collect();

        assert_eq!(skill_targets.len(), 1);
        assert!(skill_targets[0].contains(".bogus/skills") == true);
        assert!(skill_targets[0].contains(".agents/skills") == false);
        Ok(())
    }

    #[test]
    fn test_lang_skills_with_mixed_agents_route_to_both_dirs() -> anyhow::Result<()>
    {
        let _cwd = crate::template_manager::CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let workspace = tempfile::TempDir::new()?;
        let config_dir = tempfile::TempDir::new()?;
        let original_cwd = std::env::current_dir()?;
        std::env::set_current_dir(workspace.path())?;

        fs::create_dir_all(workspace.path().join(".bogus"))?;
        fs::create_dir_all(workspace.path().join(".fake"))?;
        setup_skill_routing_config(config_dir.path(), "rpp-skill", &[])?;
        write_synthetic_agent_defaults(config_dir.path(), &[("bogus", false, None), ("fake", true, None)])?;

        let engine = TemplateEngine::new(config_dir.path());
        let options = UpdateOptions {
            lang:             Some("Rust++"),
            agent:            None,
            mission:          None,
            force:            false,
            dry_run:          false,
            partial:          None,
            local_cache_only: false
        };
        let resolved = engine.resolve_all_files(&options);
        let _ = std::env::set_current_dir(&original_cwd);
        let resolved = resolved?;

        let skill_targets: Vec<String> =
            resolved.files.iter().filter(|f| f.target.to_string_lossy().contains("SKILL.md")).map(|f| f.target.to_string_lossy().into_owned()).collect();

        assert_eq!(skill_targets.len(), 2);
        assert!(skill_targets.iter().any(|t| t.contains(".agents/skills") == true) == true);
        assert!(skill_targets.iter().any(|t| t.contains(".bogus/skills") == true) == true);
        Ok(())
    }

    #[test]
    fn test_hydrate_language_skills_when_adding_native_agent() -> anyhow::Result<()>
    {
        let _cwd = crate::template_manager::CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let workspace = tempfile::TempDir::new()?;
        let config_dir = tempfile::TempDir::new()?;
        let original_cwd = std::env::current_dir()?;
        std::env::set_current_dir(workspace.path())?;

        setup_skill_routing_config(config_dir.path(), "rpp-skill", &["bogus"])?;
        write_synthetic_agent_defaults(config_dir.path(), &[("bogus", false, None)])?;

        // Simulate prior language install: tracker records rust++ lang skill in cross-client dir
        let cross_skill = workspace.path().join(".agents/skills/rpp-skill/SKILL.md");
        fs::create_dir_all(cross_skill.parent().ok_or_else(|| anyhow::anyhow!("missing parent"))?)?;
        fs::write(&cross_skill, "# Skill")?;
        let mut tracker = FileTracker::new(workspace.path())?;
        tracker.record_installation(&cross_skill, "sha1".into(), 5, "Rust++".into(), AGENT_ALL.into(), "skill".into());
        tracker.save()?;

        let engine = TemplateEngine::new(config_dir.path());
        let options = UpdateOptions {
            lang:             None,
            agent:            Some("bogus"),
            mission:          None,
            force:            false,
            dry_run:          false,
            partial:          None,
            local_cache_only: false
        };
        let resolved = engine.resolve_all_files(&options);
        let _ = std::env::set_current_dir(&original_cwd);
        let resolved = resolved?;

        let native_rel = std::path::Path::new(".bogus/skills/rpp-skill");
        let hydrated: Vec<&ResolvedFile> = resolved.files.iter().filter(|f| f.target.parent().is_some_and(|p| p.ends_with(native_rel))).collect();

        assert!(hydrated.is_empty() == false, "expected language skill to be hydrated into .bogus/skills/");
        assert!(hydrated[0].target.ends_with(std::path::Path::new(".bogus/skills/rpp-skill/SKILL.md")) == true);
        assert_eq!(hydrated[0].lang, vec!["Rust++".to_string()]);

        Ok(())
    }
}
