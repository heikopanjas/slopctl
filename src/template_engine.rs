//! Template engine for slopctl
//!
//! This module provides the `TemplateEngine` struct and supporting types for
//! template generation, fragment merging, and placeholder resolution.
//! Follows the agents.md standard: one AGENTS.md file that works across all agents.

use std::{
    collections::HashMap,
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
    utils::{FileActionResponse, copy_file_with_mkdir, prompt_file_modification}
};

/// Template marker comment used to detect unmerged template files
pub const TEMPLATE_MARKER: &str = "<!-- SLOPCTL-TEMPLATE: This marker indicates an unmerged template. Do not remove manually. -->";

/// Options for the template update operation
///
/// Aggregates CLI parameters that are passed through the update call chain.
#[derive(Clone, Copy)]
pub struct UpdateOptions<'a>
{
    /// Programming language or framework identifier (None = no language setup)
    pub lang:    Option<&'a str>,
    /// AI coding agent identifier (None = no agent-specific files)
    pub agent:   Option<&'a str>,
    /// Custom mission statement to override template default
    pub mission: Option<&'a str>,
    /// Ad-hoc skill sources from CLI `--skill` flags (GitHub URLs, shorthand, or local paths)
    pub skills:  &'a [String],
    /// Force overwrite of local modifications without warning
    pub force:   bool,
    /// Preview changes without applying them
    pub dry_run: bool
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
    pub source: PathBuf,
    pub target: PathBuf,
    /// Language name or `LANG_NONE` for language-agnostic files
    pub lang:   String,
    /// Agent name or `AGENT_ALL` for agent-agnostic files
    pub agent:  String
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
    pub content: String,
    pub lang:    String,
    pub agent:   String
}

/// Result of the file copy operation
pub enum CopyFilesResult
{
    /// Completed successfully with a list of skipped files
    Done
    {
        skipped: Vec<PathBuf>
    },
    /// User cancelled the operation
    Cancelled
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

    /// Resolves a source string to a local file path
    ///
    /// If the source is a URL, downloads it to the temp directory and returns
    /// the temp path. Otherwise, joins it with config_dir for local lookup.
    fn resolve_source_to_path(&self, source: &str, temp_dir: &Path) -> Result<PathBuf>
    {
        if github::is_url(source) == true
        {
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
        let main_source = self.resolve_source_to_path(&main_config.source, temp_dir.path())?;
        if main_source.exists() == false
        {
            return Err(anyhow::anyhow!("Main template not found: {}", main_source.display()));
        }
        let main_target = self.resolve_placeholder(&main_config.target, &workspace, &userprofile);

        let mut files_to_copy: Vec<ResolvedFile> = Vec::new();
        let mut fragments: Vec<(PathBuf, String)> = Vec::new();

        let temp_path = temp_dir.path();
        let mut process_errors: Vec<String> = Vec::new();
        let mut process_entry = |source: &str, target: &str, category: &str, lang: &str, agent: &str| {
            let source_path = if github::is_url(source) == true
            {
                match self.resolve_source_to_path(source, temp_path)
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
                files_to_copy.push(ResolvedFile { source: source_path, target: target_path, lang: lang.to_string(), agent: agent.to_string() });
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

        let mut directories_to_create: Vec<PathBuf> = Vec::new();

        if let Some(agent_name) = options.agent &&
            let Some(agent_config) = config.agents.get(agent_name)
        {
            for entry in agent_config.instructions.iter().chain(&agent_config.prompts)
            {
                let source_path = match self.resolve_source_to_path(&entry.source, temp_path)
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
                    files_to_copy.push(ResolvedFile { source: source_path, target: target_path, lang: LANG_NONE.to_string(), agent: agent_name.to_string() });
                }
            }

            for dir_entry in &agent_config.directories
            {
                let dir_path = self.resolve_placeholder(&dir_entry.target, &workspace, &userprofile);
                directories_to_create.push(dir_path);
            }
        }

        for err in &process_errors
        {
            println!("{} {}", "!".yellow(), err.yellow());
        }

        let agent_skill_dir = options.agent.and_then(agent_defaults::get_skill_dir).map(|dir| self.resolve_placeholder(dir, &workspace, &userprofile));
        let cross_client_skill_dir = self.resolve_placeholder(agent_defaults::CROSS_CLIENT_SKILL_DIR, &workspace, &userprofile);

        // For agents that don't read .agents/skills/ (Claude, Vibe), route lang and
        // top-level skills directly to the agent's native skill directory so they are
        // visible to the agent. For all other agents keep the cross-client dir.
        let native_only_agent = options.agent.is_some_and(|a| agent_defaults::reads_cross_client_skills(a) == false);
        let lang_skill_dir = if native_only_agent == true
        {
            agent_skill_dir.as_ref().unwrap_or(&cross_client_skill_dir)
        }
        else
        {
            &cross_client_skill_dir
        };

        if let Some(agent_name) = options.agent &&
            let Some(agent_config) = config.agents.get(agent_name) &&
            agent_config.skills.is_empty() == false &&
            let Some(ref dir) = agent_skill_dir
        {
            self.install_skills(agent_config.skills.iter().map(|s| (s.name.as_str(), s.source.as_str())), dir, temp_path, LANG_NONE, agent_name, &mut files_to_copy)?;
        }

        if let Some(lang) = options.lang
        {
            let lang_skills = bom::resolve_language_skills(lang, &config)?;
            if lang_skills.is_empty() == false
            {
                self.install_skills(lang_skills.iter().map(|s| (s.name.as_str(), s.source.as_str())), lang_skill_dir, temp_path, lang, AGENT_ALL, &mut files_to_copy)?;
            }
        }

        if config.skills.is_empty() == false
        {
            let skill_dir = agent_skill_dir.as_ref().unwrap_or(&cross_client_skill_dir);
            self.install_skills(config.skills.iter().map(|s| (s.name.as_str(), s.source.as_str())), skill_dir, temp_path, LANG_NONE, AGENT_ALL, &mut files_to_copy)?;
        }

        if options.skills.is_empty() == false
        {
            let skill_dir = agent_skill_dir.as_ref().unwrap_or(&cross_client_skill_dir);
            let adhoc_skills = Self::resolve_adhoc_skills(options.skills);
            self.install_skills(adhoc_skills.iter().map(|(n, s)| (n.as_str(), s.as_str())), skill_dir, temp_path, LANG_NONE, AGENT_ALL, &mut files_to_copy)?;
        }

        // Migrate any skills already in .agents/skills/ into the agent's native skill
        // dir when the agent doesn't read the cross-client path. This ensures Claude and
        // Vibe users don't lose access to skills that were previously installed without
        // an agent specified (e.g. via `slopctl init --lang rust`).
        if native_only_agent == true &&
            let Some(ref native_dir) = agent_skill_dir &&
            cross_client_skill_dir.is_dir() == true &&
            let Ok(entries) = std::fs::read_dir(&cross_client_skill_dir)
        {
            for entry in entries.flatten()
            {
                if entry.path().is_dir() == true
                {
                    let mut skill_files: Vec<PathBuf> = Vec::new();
                    crate::utils::collect_files_recursive(&entry.path(), &mut skill_files)?;
                    for src in skill_files
                    {
                        // Preserve the full relative path under the skill name:
                        // .agents/skills/foo/SKILL.md → .claude/skills/foo/SKILL.md
                        if let Ok(relative) = src.strip_prefix(&cross_client_skill_dir)
                        {
                            let target = native_dir.join(relative);
                            if target.exists() == false
                            {
                                files_to_copy.push(ResolvedFile { source: src, target, lang: LANG_NONE.to_string(), agent: AGENT_ALL.to_string() });
                            }
                        }
                    }
                }
            }
        }

        validate_no_duplicate_targets(&files_to_copy)?;

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
        map.insert(main_target, ResolvedContent { content: fresh_main, lang: options.lang.unwrap_or(LANG_NONE).to_string(), agent: AGENT_ALL.to_string() });

        for entry in &resolved.files
        {
            if entry.source.exists() == true &&
                let Ok(content) = fs::read_to_string(&entry.source)
            {
                map.insert(normalize_path(&entry.target), ResolvedContent { content, lang: entry.lang.clone(), agent: entry.agent.clone() });
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

    /// Updates local templates from global storage
    ///
    /// This method:
    /// 1. Resolves all files from templates.yml
    /// 2. Detects local modifications to AGENTS.md
    /// 3. Copies templates to current directory
    /// 4. Installs skills from templates.yml and CLI args
    ///
    /// Single AGENTS.md works for all agents. Agent-specific instruction files
    /// (e.g. CLAUDE.md) and prompts are copied if agent is specified.
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
            self.show_dry_run_files(ctx, skip_agents_md, options, &resolved.files, &resolved.directories);
            return Ok(());
        }

        self.handle_main_template(ctx, options, skip_agents_md, &mut file_tracker)?;

        for dir_path in &resolved.directories
        {
            fs::create_dir_all(dir_path)?;
            println!("  {} {} (directory)", "✓".green(), dir_path.display().to_string().yellow());
        }

        let copy_result = self.copy_files_with_tracking(&resolved.files, &mut file_tracker, ctx.template_version, options)?;

        match copy_result
        {
            | CopyFilesResult::Done { skipped } =>
            {
                self.show_skipped_files_summary(&skipped);
            }
            | CopyFilesResult::Cancelled =>
            {
                return Ok(());
            }
        }

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
    fn show_dry_run_files(&self, ctx: &TemplateContext, skip_agents_md: bool, options: &UpdateOptions, files_to_copy: &[ResolvedFile], directories: &[PathBuf])
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

        for entry in files_to_copy
        {
            if entry.target.exists()
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
        file_tracker.record_installation(
            &ctx.target,
            sha,
            ctx.template_version,
            options.lang.unwrap_or(LANG_NONE).to_string(),
            AGENT_ALL.to_string(),
            "main".to_string()
        );

        Ok(())
    }

    /// Copies template files to targets with modification checking
    ///
    /// Iterates over source/target file pairs, checking each target for user
    /// modifications before copying. Prompts the user when modifications are
    /// detected (unless force mode is enabled). Records each installation
    /// in the file tracker.
    ///
    /// # Arguments
    ///
    /// * `files_to_copy` - List of (source, target) file pairs
    /// * `file_tracker` - File tracker for checking modifications and recording installations
    /// * `template_version` - Template version for file tracking (0 for standalone skill installs)
    /// * `options` - Update options containing lang, agent, and force settings
    ///
    /// # Returns
    ///
    /// Returns `CopyFilesResult::Done` with skipped files, or `CopyFilesResult::Cancelled` if user quits
    ///
    /// # Errors
    ///
    /// Returns an error if file operations fail
    fn copy_files_with_tracking(
        &self, files_to_copy: &[ResolvedFile], file_tracker: &mut FileTracker, template_version: u32, options: &UpdateOptions
    ) -> Result<CopyFilesResult>
    {
        println!("{} Copying templates to target directories", "→".blue());

        let mut skipped_files = Vec::new();

        for entry in files_to_copy
        {
            let source = &entry.source;
            let target = &entry.target;
            let new_template_sha = FileTracker::calculate_sha256(source)?;

            let should_copy = if target.exists() == false || options.force == true
            {
                true
            }
            else
            {
                match file_tracker.check_modification(target)?
                {
                    | FileStatus::NotTracked =>
                    {
                        let response = prompt_file_modification(target, "<not tracked>", "<current file>", source)?;
                        match response
                        {
                            | FileActionResponse::Overwrite => true,
                            | FileActionResponse::Skip =>
                            {
                                skipped_files.push(target.clone());
                                false
                            }
                            | FileActionResponse::Quit =>
                            {
                                println!("\n{} Operation cancelled by user", "!".yellow());
                                return Ok(CopyFilesResult::Cancelled);
                            }
                        }
                    }
                    | FileStatus::Unmodified => true,
                    | FileStatus::Modified =>
                    {
                        if let Some(metadata) = file_tracker.get_metadata(target)
                        {
                            let current_sha = FileTracker::calculate_sha256(target)?;
                            let response = prompt_file_modification(target, &metadata.original_sha, &current_sha, source)?;
                            match response
                            {
                                | FileActionResponse::Overwrite => true,
                                | FileActionResponse::Skip =>
                                {
                                    skipped_files.push(target.clone());
                                    false
                                }
                                | FileActionResponse::Quit =>
                                {
                                    println!("\n{} Operation cancelled by user", "!".yellow());
                                    return Ok(CopyFilesResult::Cancelled);
                                }
                            }
                        }
                        else
                        {
                            true
                        }
                    }
                    | FileStatus::Deleted => true
                }
            };

            if should_copy == true
            {
                copy_file_with_mkdir(source, target)?;
                println!("  {} {}", "✓".green(), target.display().to_string().yellow());

                let target_str = target.to_string_lossy();
                let category = if target_str.contains("SKILL.md") || target_str.contains("/skills/") || target_str.contains("\\skills\\")
                {
                    "skill"
                }
                else if target_str.contains(".git")
                {
                    "integration"
                }
                else if let Some(name) = options.agent
                {
                    if target_str.contains(&format!(".{}", name)) || target_str.contains(name)
                    {
                        "agent"
                    }
                    else
                    {
                        "language"
                    }
                }
                else
                {
                    "language"
                };

                file_tracker.record_installation(target, new_template_sha, template_version, entry.lang.clone(), entry.agent.clone(), category.to_string());
            }
        }

        Ok(CopyFilesResult::Done { skipped: skipped_files })
    }

    /// Shows summary of skipped files after a copy operation
    ///
    /// # Arguments
    ///
    /// * `skipped_files` - List of file paths that were skipped
    fn show_skipped_files_summary(&self, skipped_files: &[PathBuf])
    {
        if skipped_files.is_empty() == false
        {
            println!("\n{} Skipped {} modified file(s):", "!".yellow(), skipped_files.len());
            for file in skipped_files
            {
                println!("  {} {}", "○".yellow(), file.display());
            }
            println!("{} Use --force to overwrite modified files", "→".blue());
        }
    }

    /// Check whether a `--skill` value looks like a local filesystem path
    ///
    /// Returns true for absolute paths (Unix `/` or Windows drive letter `C:\`),
    /// explicit relative paths (`./`, `../`), and home-relative paths (`~/`).
    /// Recognizes both `/` and `\` separators for cross-platform support.
    /// Everything else is treated as GitHub shorthand.
    fn is_local_path(input: &str) -> bool
    {
        input.starts_with('/') ||
            input.starts_with("./") ||
            input.starts_with(".\\") ||
            input.starts_with("../") ||
            input.starts_with("..\\") ||
            input.starts_with("~/") ||
            input.starts_with("~\\") ||
            Self::starts_with_drive_letter(input)
    }

    /// Check for a Windows drive letter prefix (e.g. `C:\`, `D:\`)
    fn starts_with_drive_letter(input: &str) -> bool
    {
        let bytes = input.as_bytes();
        bytes.len() >= 3 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' && (bytes[2] == b'\\' || bytes[2] == b'/')
    }

    /// Resolve a local skill path to an absolute `PathBuf`
    ///
    /// Expands `~` to the user's home directory via the `dirs` crate.
    /// Relative paths (`./`, `../`) are resolved against `std::env::current_dir()`.
    /// Handles both `/` and `\` separators for cross-platform support.
    fn resolve_local_skill_path(input: &str) -> PathBuf
    {
        let home_suffix = input.strip_prefix("~/").or_else(|| input.strip_prefix("~\\"));
        if let Some(suffix) = home_suffix &&
            let Some(home) = dirs::home_dir()
        {
            return home.join(suffix);
        }

        let path = PathBuf::from(input);
        if path.is_absolute() == true
        {
            path
        }
        else
        {
            std::env::current_dir().unwrap_or_default().join(path)
        }
    }

    /// Resolve ad-hoc skill sources from CLI `--skill` values into (name, source) pairs
    ///
    /// Local paths are resolved to absolute paths. GitHub shorthand is expanded to full URLs.
    fn resolve_adhoc_skills(skills: &[String]) -> Vec<(String, String)>
    {
        skills
            .iter()
            .map(|s| {
                if Self::is_local_path(s) == true
                {
                    let resolved = Self::resolve_local_skill_path(s);
                    let name = resolved.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_else(|| s.clone());
                    (name, resolved.to_string_lossy().to_string())
                }
                else
                {
                    let url = github::expand_shorthand(s);
                    let name = Self::skill_name_from_url(&url).unwrap_or_else(|| s.clone());
                    (name, url)
                }
            })
            .collect()
    }

    /// Install ad-hoc skills without requiring global templates
    ///
    /// Standalone skill installation that bypasses all template processing (AGENTS.md,
    /// language files, agent files). Skills are installed to the cross-client
    /// `.agents/skills/` directory per the agentskills.io specification.
    ///
    /// # Arguments
    ///
    /// * `options` - Update options (only `skills`, `force`, and `dry_run` are used)
    ///
    /// # Errors
    ///
    /// Returns an error if skill resolution, download, or copy operations fail
    pub fn install_skills_only(&self, options: &UpdateOptions) -> Result<()>
    {
        require!(options.skills.is_empty() == false, Ok(()));

        let workspace = std::env::current_dir()?;
        let userprofile = dirs::home_dir().ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "Could not determine home directory"))?;

        // Detect all installed agents; install into each agent's skill dir.
        // Fall back to the cross-client dir when no agent is present.
        let installed_agents = agent_defaults::detect_all_installed_agents(&workspace);

        let skill_dirs: Vec<(String, PathBuf)> = if installed_agents.is_empty() == false
        {
            installed_agents
                .iter()
                .filter_map(|agent| agent_defaults::get_skill_dir(agent).map(|dir| (agent.clone(), self.resolve_placeholder(dir, &workspace, &userprofile))))
                .collect()
        }
        else
        {
            let cross_client = self.resolve_placeholder(agent_defaults::CROSS_CLIENT_SKILL_DIR, &workspace, &userprofile);
            vec![("cross-client".to_string(), cross_client)]
        };

        let mut file_tracker = FileTracker::new(&workspace)?;
        let temp_dir = tempfile::TempDir::new()?;

        let mut files_to_copy: Vec<ResolvedFile> = Vec::new();

        let adhoc_skills = Self::resolve_adhoc_skills(options.skills);
        for (_, skill_dir) in &skill_dirs
        {
            self.install_skills(adhoc_skills.iter().map(|(n, s)| (n.as_str(), s.as_str())), skill_dir, temp_dir.path(), LANG_NONE, AGENT_ALL, &mut files_to_copy)?;
        }

        validate_no_duplicate_targets(&files_to_copy)?;

        if options.dry_run == true
        {
            println!("\n{} Files that would be created/modified:", "→".blue());
            for entry in &files_to_copy
            {
                if entry.target.exists() == true
                {
                    println!("  {} {} (would be overwritten)", "●".yellow(), entry.target.display());
                }
                else
                {
                    println!("  {} {} (would be created)", "●".green(), entry.target.display());
                }
            }
            println!("\n{} Dry run complete. No files were modified.", "✓".green());
            return Ok(());
        }

        println!("{} Copying skill files", "→".blue());

        let copy_result = self.copy_files_with_tracking(&files_to_copy, &mut file_tracker, 0, options)?;

        match copy_result
        {
            | CopyFilesResult::Done { skipped } =>
            {
                self.show_skipped_files_summary(&skipped);
            }
            | CopyFilesResult::Cancelled =>
            {
                return Ok(());
            }
        }

        file_tracker.save()?;

        println!("{} Skills installed successfully", "✓".green());
        if installed_agents.is_empty() == true
        {
            println!("{} Installed to cross-client directory: {}", "→".blue(), skill_dirs[0].1.display().to_string().yellow());
        }
        else
        {
            for (agent, skill_dir) in &skill_dirs
            {
                println!("{} Installed to {} ({})", "→".blue(), skill_dir.display().to_string().yellow(), agent.green());
            }
        }

        Ok(())
    }

    /// Install skills into the given skill directory
    ///
    /// For each skill, resolves the source (local or GitHub) and adds file entries
    /// to the files_to_copy list. GitHub skills are discovered via SKILL.md scanning
    /// and downloaded recursively (including subdirectories). Local skills are copied
    /// recursively; absolute paths are used directly (ad-hoc local installs) while
    /// relative paths are resolved against the global template cache.
    ///
    /// # Arguments
    ///
    /// * `skills` - Iterator of (name, source) pairs
    /// * `skill_base_dir` - Resolved target directory for skills (e.g. `.cursor/skills` or `.agents/skills`)
    /// * `temp_dir` - Temporary directory for GitHub downloads
    /// * `files_to_copy` - Accumulator for (source, target) file pairs
    fn install_skills<'b, I>(&self, skills: I, skill_base_dir: &Path, temp_dir: &Path, lang: &str, agent: &str, files_to_copy: &mut Vec<ResolvedFile>) -> Result<()>
    where I: Iterator<Item = (&'b str, &'b str)>
    {
        for (skill_name, source) in skills
        {
            if github::is_url(source) == true
            {
                let parsed = github::parse_github_url(source).ok_or_else(|| anyhow::anyhow!("Invalid GitHub URL for skill '{}': {}", skill_name, source))?;

                println!("{} Discovering skills at {}...", "→".blue(), source.yellow());

                match github::discover_skills(&parsed)
                {
                    | Ok(discovered) if discovered.is_empty() == true =>
                    {
                        println!("{} No skills found (no SKILL.md) at {}", "!".yellow(), source.yellow());
                    }
                    | Ok(discovered) =>
                    {
                        for skill in discovered
                        {
                            let target_base = skill_base_dir.join(&skill.name);
                            let prefix = format!("skill_{}", skill.name);

                            println!("{} Installing skill '{}' from GitHub...", "→".blue(), skill.name.green());

                            match github::download_directory_from_entries(skill.entries, &skill.url, temp_dir, &prefix, "")
                            {
                                | Ok(downloaded) =>
                                {
                                    for (temp_path, rel_path) in downloaded
                                    {
                                        files_to_copy.push(ResolvedFile {
                                            source: temp_path,
                                            target: target_base.join(rel_path),
                                            lang:   lang.to_string(),
                                            agent:  agent.to_string()
                                        });
                                    }
                                }
                                | Err(e) =>
                                {
                                    println!("{} Failed to download skill '{}': {}", "!".yellow(), skill.name, e);
                                }
                            }
                        }
                    }
                    | Err(e) =>
                    {
                        println!("{} Failed to discover skills at '{}': {}", "!".yellow(), skill_name, e);
                    }
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
                    println!("{} Installing skill '{}' from {}...", "→".blue(), skill_name.green(), label.yellow());
                    Self::collect_local_skill_files(&source_dir, &target_base, lang, agent, files_to_copy)?;
                }
                else if source_dir.is_file() == true
                {
                    let target_base = skill_base_dir.join(skill_name);
                    let target_path = source_dir.file_name().map(|f| target_base.join(f));
                    if let Some(target) = target_path
                    {
                        println!("{} Installing skill '{}' from {}...", "→".blue(), skill_name.green(), label.yellow());
                        files_to_copy.push(ResolvedFile { source: source_dir, target, lang: lang.to_string(), agent: agent.to_string() });
                    }
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
                files_to_copy.push(ResolvedFile { source: path.clone(), target: target_base.join(filename), lang: lang.to_string(), agent: agent.to_string() });
            }
        }

        Ok(())
    }

    /// Extract a skill name from a GitHub URL or expanded shorthand
    ///
    /// Parses as a GitHubUrl to derive the name from the path (last segment)
    /// or repo name when the path is empty (bare `user/repo` shorthand).
    /// Falls back to the last URL segment for non-GitHub URLs.
    fn skill_name_from_url(url: &str) -> Option<String>
    {
        if let Some(parsed) = github::parse_github_url(url)
        {
            let name = parsed.skill_name();
            if name.is_empty() == false
            {
                return Some(name);
            }
        }

        let trimmed = url.trim_end_matches('/');
        trimmed.rsplit('/').next().map(|s| s.to_string()).filter(|s| s.is_empty() == false)
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
        ResolvedFile { source: PathBuf::from(source), target: PathBuf::from(target), lang: LANG_NONE.to_string(), agent: AGENT_ALL.to_string() }
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

        let result = engine.resolve_placeholder("$userprofile/.codex/prompts/init.md", &workspace, &userprofile);
        assert_eq!(result, PathBuf::from("/home/user/.codex/prompts/init.md"));
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

    // -- skill_name_from_url --

    #[test]
    fn test_skill_name_from_url_simple() -> anyhow::Result<()>
    {
        assert_eq!(
            TemplateEngine::skill_name_from_url("https://github.com/user/repo/tree/main/my-skill").ok_or_else(|| anyhow::anyhow!("expected skill name"))?,
            "my-skill"
        );
        Ok(())
    }

    #[test]
    fn test_skill_name_from_url_trailing_slash() -> anyhow::Result<()>
    {
        assert_eq!(
            TemplateEngine::skill_name_from_url("https://github.com/user/repo/tree/main/skill/").ok_or_else(|| anyhow::anyhow!("expected skill name"))?,
            "skill"
        );
        Ok(())
    }

    #[test]
    fn test_skill_name_from_url_empty()
    {
        assert!(TemplateEngine::skill_name_from_url("").is_none() == true);
    }

    #[test]
    fn test_skill_name_from_url_bare_repo() -> anyhow::Result<()>
    {
        assert_eq!(
            TemplateEngine::skill_name_from_url("https://github.com/twostraws/swiftui-agent-skill/tree/main").ok_or_else(|| anyhow::anyhow!("expected skill name"))?,
            "swiftui-agent-skill"
        );
        Ok(())
    }

    #[test]
    fn test_skill_name_from_url_bare_repo_no_tree() -> anyhow::Result<()>
    {
        assert_eq!(TemplateEngine::skill_name_from_url("https://github.com/user/my-skill").ok_or_else(|| anyhow::anyhow!("expected skill name"))?, "my-skill");
        Ok(())
    }

    // -- is_local_path --

    #[test]
    fn test_is_local_path_absolute()
    {
        assert!(TemplateEngine::is_local_path("/Users/heiko/skills/my-skill") == true);
    }

    #[test]
    fn test_is_local_path_relative_dot()
    {
        assert!(TemplateEngine::is_local_path("./my-skill") == true);
    }

    #[test]
    fn test_is_local_path_relative_dotdot()
    {
        assert!(TemplateEngine::is_local_path("../shared/my-skill") == true);
    }

    #[test]
    fn test_is_local_path_home()
    {
        assert!(TemplateEngine::is_local_path("~/skills/my-skill") == true);
    }

    #[test]
    fn test_is_local_path_github_shorthand()
    {
        assert!(TemplateEngine::is_local_path("user/repo") == false);
    }

    #[test]
    fn test_is_local_path_url()
    {
        assert!(TemplateEngine::is_local_path("https://github.com/user/repo") == false);
    }

    #[test]
    fn test_is_local_path_bare_name()
    {
        assert!(TemplateEngine::is_local_path("my-skill") == false);
    }

    #[test]
    fn test_is_local_path_windows_drive_letter()
    {
        assert!(TemplateEngine::is_local_path("C:\\Users\\heiko\\skills") == true);
    }

    #[test]
    fn test_is_local_path_windows_drive_letter_forward_slash()
    {
        assert!(TemplateEngine::is_local_path("D:/skills/my-skill") == true);
    }

    #[test]
    fn test_is_local_path_windows_relative_dot()
    {
        assert!(TemplateEngine::is_local_path(".\\my-skill") == true);
    }

    #[test]
    fn test_is_local_path_windows_relative_dotdot()
    {
        assert!(TemplateEngine::is_local_path("..\\shared\\skill") == true);
    }

    #[test]
    fn test_is_local_path_windows_home()
    {
        assert!(TemplateEngine::is_local_path("~\\skills\\my-skill") == true);
    }

    #[test]
    fn test_starts_with_drive_letter_lowercase()
    {
        assert!(TemplateEngine::starts_with_drive_letter("c:\\projects") == true);
    }

    #[test]
    fn test_starts_with_drive_letter_too_short()
    {
        assert!(TemplateEngine::starts_with_drive_letter("C:") == false);
    }

    #[test]
    fn test_starts_with_drive_letter_no_separator()
    {
        assert!(TemplateEngine::starts_with_drive_letter("C:foo") == false);
    }

    // -- resolve_local_skill_path --

    #[test]
    fn test_resolve_local_skill_path_absolute()
    {
        #[cfg(windows)]
        {
            let result = TemplateEngine::resolve_local_skill_path("C:\\opt\\skills\\my-skill");
            assert_eq!(result, PathBuf::from("C:\\opt\\skills\\my-skill"));
        }
        #[cfg(not(windows))]
        {
            let result = TemplateEngine::resolve_local_skill_path("/opt/skills/my-skill");
            assert_eq!(result, PathBuf::from("/opt/skills/my-skill"));
        }
    }

    #[test]
    fn test_resolve_local_skill_path_home()
    {
        let result = TemplateEngine::resolve_local_skill_path("~/skills/my-skill");
        if let Some(home) = dirs::home_dir()
        {
            assert_eq!(result, home.join("skills/my-skill"));
        }
    }

    #[test]
    fn test_resolve_local_skill_path_relative()
    {
        let result = TemplateEngine::resolve_local_skill_path("./my-skill");
        let expected = std::env::current_dir().unwrap().join("./my-skill");
        assert_eq!(result, expected);
    }

    #[test]
    fn test_resolve_local_skill_path_home_backslash()
    {
        let result = TemplateEngine::resolve_local_skill_path("~\\skills\\my-skill");
        if let Some(home) = dirs::home_dir()
        {
            assert_eq!(result, home.join("skills\\my-skill"));
        }
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
        let frag = write_fragment(dir.path(), "rust.md", "## Rust Conventions\n\nUse cargo.")?;

        let engine = TemplateEngine::new(dir.path());
        let ctx = TemplateContext { source, target: target.clone(), fragments: vec![(frag, "languages".to_string())], template_version: 5 };
        let options = UpdateOptions { lang: Some("rust"), agent: None, mission: None, skills: &[], force: false, dry_run: false };

        engine.merge_fragments(&ctx, &options)?;

        let output = fs::read_to_string(&target)?;
        assert!(output.contains("## Rust Conventions") == true);
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
        let lang_frag = write_fragment(dir.path(), "lang.md", "## Rust\n\nUse clippy.")?;

        let engine = TemplateEngine::new(dir.path());
        let ctx = TemplateContext {
            source,
            target: target.clone(),
            fragments: vec![(mission_frag, "mission".to_string()), (principles_frag, "principles".to_string()), (lang_frag, "languages".to_string())],
            template_version: 5
        };
        let options = UpdateOptions { lang: Some("rust"), agent: None, mission: None, skills: &[], force: false, dry_run: false };

        engine.merge_fragments(&ctx, &options)?;

        let output = fs::read_to_string(&target)?;
        assert!(output.contains("Build great software") == true);
        assert!(output.contains("Keep it simple") == true);
        assert!(output.contains("Use clippy") == true);
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
        let options = UpdateOptions { lang: None, agent: None, mission: None, skills: &[], force: false, dry_run: false };

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
        let options = UpdateOptions { lang: None, agent: None, mission: Some("We build CLI tools."), skills: &[], force: false, dry_run: false };

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
        let options = UpdateOptions { lang: None, agent: None, mission: None, skills: &[], force: false, dry_run: false };

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

    // -- resolve_adhoc_skills --

    #[test]
    fn test_resolve_adhoc_skills_github_shorthand()
    {
        let skills = vec!["user/my-skill".to_string()];
        let resolved = TemplateEngine::resolve_adhoc_skills(&skills);
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].0, "my-skill");
        assert!(resolved[0].1.contains("github.com") == true);
    }

    #[test]
    fn test_resolve_adhoc_skills_local_path()
    {
        let skills = vec!["./my-local-skill".to_string()];
        let resolved = TemplateEngine::resolve_adhoc_skills(&skills);
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].0, "my-local-skill");
        assert!(Path::new(&resolved[0].1).is_absolute() == true);
    }

    #[test]
    fn test_resolve_adhoc_skills_mixed()
    {
        let skills = vec!["user/remote-skill".to_string(), "./local-skill".to_string()];
        let resolved = TemplateEngine::resolve_adhoc_skills(&skills);
        assert_eq!(resolved.len(), 2);
        assert_eq!(resolved[0].0, "remote-skill");
        assert_eq!(resolved[1].0, "local-skill");
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
    fn test_skill_base_dir_with_agent_uses_agent_specific()
    {
        let engine = TemplateEngine::new(Path::new("/config"));
        let workspace = PathBuf::from("/projects/myapp");
        let userprofile = PathBuf::from("/home/user");

        let dir_template = crate::agent_defaults::get_skill_dir("cursor").expect("cursor should have skill dir");
        let result = engine.resolve_placeholder(dir_template, &workspace, &userprofile);
        assert_eq!(result, PathBuf::from("/projects/myapp/.cursor/skills"));
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
            &mut files_to_copy
        )?;

        assert_eq!(files_to_copy.len(), 2);
        let targets: Vec<PathBuf> = files_to_copy.iter().map(|e| e.target.clone()).collect();
        assert!(targets.contains(&skill_base_dir.join("my-skill/SKILL.md")) == true);
        assert!(targets.contains(&skill_base_dir.join("my-skill/scripts/setup.sh")) == true);
        Ok(())
    }

    #[test]
    fn test_install_skills_only_empty_skills_is_noop() -> anyhow::Result<()>
    {
        let config_dir = tempfile::TempDir::new()?;
        let engine = TemplateEngine::new(config_dir.path());
        let skills: Vec<String> = vec![];
        let options = UpdateOptions { lang: None, agent: None, mission: None, skills: &skills, force: false, dry_run: false };

        engine.install_skills_only(&options)?;
        Ok(())
    }

    /// Write a minimal templates.yml with known agents and languages
    fn write_minimal_templates_yml(dir: &std::path::Path) -> anyhow::Result<()>
    {
        let yml = r#"version: 5
main:
  source: AGENTS.md
  target: '$workspace/AGENTS.md'
agents:
  cursor:
    instructions:
      - source: cursor/cursorrules
        target: '$workspace/.cursorrules'
  claude:
    instructions:
      - source: claude/CLAUDE.md
        target: '$workspace/CLAUDE.md'
languages:
  rust:
    files:
      - source: rust-format.toml
        target: '$workspace/.rustfmt.toml'
  swift:
    files:
      - source: swift-format.json
        target: '$workspace/.swift-format'
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
        let skills: Vec<String> = vec![];
        let options = UpdateOptions { lang: None, agent: Some("nonexistent"), mission: None, skills: &skills, force: false, dry_run: false };

        let result = engine.update(&options);
        assert!(result.is_err() == true);
        let err = result.unwrap_err().to_string();
        assert!(err.contains("not found in templates.yml") == true);
        assert!(err.contains("nonexistent") == true);
        assert!(err.contains("cursor") == true);
        assert!(err.contains("claude") == true);
        Ok(())
    }

    #[test]
    fn test_update_rejects_unknown_language() -> anyhow::Result<()>
    {
        let config_dir = tempfile::TempDir::new()?;
        write_minimal_templates_yml(config_dir.path())?;

        let engine = TemplateEngine::new(config_dir.path());
        let skills: Vec<String> = vec![];
        let options = UpdateOptions { lang: Some("nonexistent"), agent: None, mission: None, skills: &skills, force: false, dry_run: false };

        let result = engine.update(&options);
        assert!(result.is_err() == true);
        let err = result.unwrap_err().to_string();
        assert!(err.contains("not found in templates.yml") == true);
        assert!(err.contains("nonexistent") == true);
        assert!(err.contains("rust") == true);
        assert!(err.contains("swift") == true);
        Ok(())
    }

    #[test]
    fn test_update_accepts_known_agent() -> anyhow::Result<()>
    {
        let config_dir = tempfile::TempDir::new()?;
        write_minimal_templates_yml(config_dir.path())?;

        fs::create_dir_all(config_dir.path().join("cursor"))?;
        fs::write(config_dir.path().join("cursor/cursorrules"), "test")?;

        let engine = TemplateEngine::new(config_dir.path());
        let skills: Vec<String> = vec![];
        let options = UpdateOptions { lang: None, agent: Some("cursor"), mission: None, skills: &skills, force: false, dry_run: true };

        let result = engine.update(&options);
        assert!(result.is_ok() == true);
        Ok(())
    }

    #[test]
    fn test_update_accepts_known_language() -> anyhow::Result<()>
    {
        let config_dir = tempfile::TempDir::new()?;
        write_minimal_templates_yml(config_dir.path())?;

        fs::write(config_dir.path().join("rust-format.toml"), "max_width = 100")?;

        let engine = TemplateEngine::new(config_dir.path());
        let skills: Vec<String> = vec![];
        let options = UpdateOptions { lang: Some("rust"), agent: None, mission: None, skills: &skills, force: false, dry_run: true };

        let result = engine.update(&options);
        assert!(result.is_ok() == true);
        Ok(())
    }

    /// Regression test for v17.0.2: GitHub-downloaded source files must remain on disk
    /// after `resolve_all_files()` returns
    ///
    /// Before the fix, the `tempfile::TempDir` created inside `resolve_all_files()` was
    /// dropped at function exit, deleting every GitHub-downloaded file referenced by
    /// `ResolvedFiles.files`. The subsequent `copy_files_with_tracking()` then failed
    /// with a bare `os error 2` (No such file or directory).
    ///
    /// The fix moves ownership of the `TempDir` into `ResolvedFiles._temp_dir` so it
    /// lives until the consumer (init or merge) finishes. This test installs HTTP
    /// hooks via `github::set_test_hooks`, drives the GitHub-skill code path through
    /// `resolve_all_files()`, and asserts the source files still exist on the
    /// returned `ResolvedFiles`.
    #[test]
    fn test_resolve_all_files_keeps_github_skill_temp_files_alive() -> anyhow::Result<()>
    {
        struct CwdGuard
        {
            original: PathBuf
        }
        impl Drop for CwdGuard
        {
            fn drop(&mut self)
            {
                let _ = std::env::set_current_dir(&self.original);
            }
        }

        let _cwd = crate::template_manager::CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        let workspace = tempfile::TempDir::new()?;
        let config_dir = tempfile::TempDir::new()?;

        let _restore = CwdGuard { original: std::env::current_dir()? };
        std::env::set_current_dir(workspace.path())?;

        fs::write(config_dir.path().join("AGENTS.md"), format!("{}\n# Test\n", TEMPLATE_MARKER))?;
        let yml = "version: 5\nmain:\n  source: AGENTS.md\n  target: '$workspace/AGENTS.md'\nlanguages: {}\n";
        fs::write(config_dir.path().join("templates.yml"), yml)?;

        let _hooks = github::set_test_hooks(
            Box::new(|_url| {
                Ok(vec![github::GitHubContentEntry {
                    name:         "SKILL.md".into(),
                    entry_type:   "file".into(),
                    download_url: Some("https://raw.githubusercontent.com/foo/bar/main/SKILL.md".into()),
                    path:         "SKILL.md".into()
                }])
            }),
            Box::new(|_url| Ok(b"# Mock Skill\n".to_vec()))
        );

        let engine = TemplateEngine::new(config_dir.path());
        let skills: Vec<String> = vec!["foo/bar".to_string()];
        let options = UpdateOptions { lang: None, agent: None, mission: None, skills: &skills, force: false, dry_run: false };

        let resolved = engine.resolve_all_files(&options)?;

        let github_files: Vec<&ResolvedFile> = resolved.files.iter().filter(|f| f.target.to_string_lossy().contains("SKILL.md")).collect();

        assert!(github_files.is_empty() == false, "expected GitHub-sourced skill file in ResolvedFiles");

        for entry in &github_files
        {
            assert!(
                entry.source.exists() == true,
                "GitHub-sourced temp file missing after resolve_all_files() returned: {} (this is the v17.0.2 TempDir-lifetime bug)",
                entry.source.display()
            );
        }

        Ok(())
    }

    /// Build a minimal config_dir with a templates.yml and AGENTS.md for skill routing tests.
    /// The agents list must include every agent name used in `--agent` for the test.
    fn setup_skill_routing_config(config_dir: &std::path::Path, skill_source_name: &str, agents: &[&str]) -> anyhow::Result<()>
    {
        use std::fs;
        let agents_yaml = agents.iter().map(|a| format!("  {}: {{}}", a)).collect::<Vec<_>>().join("\n");
        let yaml = format!(
            "version: 5\nmain:\n  source: AGENTS.md\n  target: '$workspace/AGENTS.md'\nagents:\n{}\nlanguages:\n  rust:\n    skills:\n      - name: {}\n        \
             source: 'skills/{}'\nintegration: {{}}\n",
            agents_yaml, skill_source_name, skill_source_name
        );
        fs::write(config_dir.join("templates.yml"), yaml)?;
        fs::write(config_dir.join("AGENTS.md"), "<!-- SLOPCTL-TEMPLATE -->\n# Project\n")?;
        let skill_dir = config_dir.join("skills").join(skill_source_name);
        fs::create_dir_all(&skill_dir)?;
        fs::write(skill_dir.join("SKILL.md"), "# Skill")?;
        Ok(())
    }

    #[test]
    fn test_lang_skills_route_to_native_dir_for_claude() -> anyhow::Result<()>
    {
        let _cwd = crate::template_manager::CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let workspace = tempfile::TempDir::new()?;
        let config_dir = tempfile::TempDir::new()?;
        let original_cwd = std::env::current_dir()?;
        std::env::set_current_dir(workspace.path())?;

        setup_skill_routing_config(config_dir.path(), "rust-coding-conventions", &["claude"])?;

        let engine = TemplateEngine::new(config_dir.path());
        let options = UpdateOptions { lang: Some("rust"), agent: Some("claude"), mission: None, skills: &[], force: false, dry_run: false };
        let resolved = engine.resolve_all_files(&options);
        let _ = std::env::set_current_dir(&original_cwd);
        let resolved = resolved?;

        let skill_targets: Vec<String> =
            resolved.files.iter().filter(|f| f.target.to_string_lossy().contains("SKILL.md")).map(|f| f.target.to_string_lossy().into_owned()).collect();

        // Lang skills for claude must go to .claude/skills/, never to .agents/skills/
        assert!(skill_targets.is_empty() == false, "expected at least one skill file");
        for t in &skill_targets
        {
            assert!(t.contains(".claude/skills"), "skill target should be in .claude/skills/, got: {}", t);
            assert!(t.contains(".agents/skills") == false, "skill must not go to .agents/skills/ for claude, got: {}", t);
        }

        Ok(())
    }

    #[test]
    fn test_lang_skills_route_to_cross_client_for_cursor() -> anyhow::Result<()>
    {
        let _cwd = crate::template_manager::CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let workspace = tempfile::TempDir::new()?;
        let config_dir = tempfile::TempDir::new()?;
        let original_cwd = std::env::current_dir()?;
        std::env::set_current_dir(workspace.path())?;

        setup_skill_routing_config(config_dir.path(), "rust-coding-conventions", &["cursor"])?;

        let engine = TemplateEngine::new(config_dir.path());
        let options = UpdateOptions { lang: Some("rust"), agent: Some("cursor"), mission: None, skills: &[], force: false, dry_run: false };
        let resolved = engine.resolve_all_files(&options);
        let _ = std::env::set_current_dir(&original_cwd);
        let resolved = resolved?;

        let skill_targets: Vec<String> =
            resolved.files.iter().filter(|f| f.target.to_string_lossy().contains("SKILL.md")).map(|f| f.target.to_string_lossy().into_owned()).collect();

        // Cursor reads .agents/skills/ so lang skills should go there
        assert!(skill_targets.is_empty() == false, "expected at least one skill file");
        for t in &skill_targets
        {
            assert!(t.contains(".agents/skills"), "skill target should be in .agents/skills/ for cursor, got: {}", t);
        }

        Ok(())
    }

    #[test]
    fn test_adopt_cross_client_skills_to_claude() -> anyhow::Result<()>
    {
        let _cwd = crate::template_manager::CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let workspace = tempfile::TempDir::new()?;
        let config_dir = tempfile::TempDir::new()?;
        let original_cwd = std::env::current_dir()?;
        std::env::set_current_dir(workspace.path())?;

        // Pre-populate .agents/skills/ as if a previous agent-less install had run
        let cross_skill = workspace.path().join(".agents/skills/git-workflow");
        std::fs::create_dir_all(&cross_skill)?;
        std::fs::write(cross_skill.join("SKILL.md"), "# Git Workflow")?;

        // Minimal config with no language skills so we can isolate the adoption behaviour
        let yaml = "version: 5\nmain:\n  source: AGENTS.md\n  target: '$workspace/AGENTS.md'\nagents:\n  claude: {}\nlanguages: {}\nintegration: {}\n";
        std::fs::write(config_dir.path().join("templates.yml"), yaml)?;
        std::fs::write(config_dir.path().join("AGENTS.md"), "<!-- SLOPCTL-TEMPLATE -->\n# Project\n")?;

        let engine = TemplateEngine::new(config_dir.path());
        let options = UpdateOptions { lang: None, agent: Some("claude"), mission: None, skills: &[], force: false, dry_run: false };
        let resolved = engine.resolve_all_files(&options);
        let _ = std::env::set_current_dir(&original_cwd);
        let resolved = resolved?;

        // Use Path::ends_with with relative components to avoid symlink / path-separator
        // mismatches between workspace.path() (raw TempDir) and the canonicalized cwd
        // that resolve_all_files uses internally (macOS /var vs /private/var, Windows \).
        let native_rel = std::path::Path::new(".claude/skills/git-workflow");
        let adopted: Vec<&ResolvedFile> = resolved.files.iter().filter(|f| f.target.parent().is_some_and(|p| p.ends_with(native_rel))).collect();

        assert!(adopted.is_empty() == false, "expected cross-client skill to be adopted into .claude/skills/");
        assert!(adopted[0].target.ends_with(std::path::Path::new(".claude/skills/git-workflow/SKILL.md")) == true);
        // The .agents/skills/ path must not appear in copy targets
        let cross_rel = std::path::Path::new(".agents/skills");
        let cross_targets: Vec<&ResolvedFile> = resolved.files.iter().filter(|f| f.target.ancestors().any(|a| a.ends_with(cross_rel))).collect();
        assert!(cross_targets.is_empty() == true, "cross-client skills should not be copy targets when agent uses native dir");

        Ok(())
    }
}
