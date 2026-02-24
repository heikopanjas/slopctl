//! Template engine for vibe-check
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
    file_tracker::{FileStatus, FileTracker},
    github,
    utils::{FileActionResponse, copy_file_with_mkdir, prompt_file_modification}
};

/// Template marker comment used to detect unmerged template files
pub const TEMPLATE_MARKER: &str = "<!-- VIBE-CHECK-TEMPLATE: This marker indicates an unmerged template. Do not remove manually. -->";

/// Options for the template update operation
///
/// Aggregates CLI parameters that are passed through the update call chain.
#[derive(Clone, Copy)]
pub struct UpdateOptions<'a>
{
    /// Programming language or framework identifier
    pub lang:    &'a str,
    /// AI coding agent identifier (optional)
    pub agent:   Option<&'a str>,
    /// Skip language-specific setup
    pub no_lang: bool,
    /// Custom mission statement to override template default
    pub mission: Option<&'a str>,
    /// Ad-hoc skill URLs from CLI `--skill` flags
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

    if config_path.exists() == false
    {
        return Err("templates.yml not found in global template directory".into());
    }

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
    if local_path.exists() == false
    {
        return Ok(false);
    }

    let content = fs::read_to_string(local_path)?;
    Ok(content.contains(TEMPLATE_MARKER) == false)
}

/// Template engine for vibe-check (agents.md standard)
///
/// Handles template generation, fragment merging, placeholder resolution,
/// and skill installation. Supports V2 and V3 template formats.
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
            let parsed = github::parse_github_url(source).ok_or_else(|| format!("Invalid GitHub URL: {}", source))?;

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

    /// Updates local templates from global storage
    ///
    /// This method:
    /// 1. Verifies global templates exist
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
        let templates_yml_path = self.config_dir.join("templates.yml");

        if self.config_dir.exists() == false || templates_yml_path.exists() == false
        {
            return Err("Global templates not found. Please run 'vibe-check update' first to download templates.".into());
        }

        let config = load_template_config(self.config_dir)?;

        let workspace = std::env::current_dir()?;
        let userprofile = dirs::home_dir().ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "Could not determine home directory"))?;

        let mut file_tracker = FileTracker::new(self.config_dir)?;

        let temp_dir = tempfile::TempDir::new()?;

        let main_config = config.main.as_ref().ok_or("Missing 'main' section in templates.yml")?;
        let main_source = self.resolve_source_to_path(&main_config.source, temp_dir.path())?;
        if main_source.exists() == false
        {
            return Err(format!("Main template not found: {}", main_source.display()).into());
        }
        let main_target = self.resolve_placeholder(&main_config.target, &workspace, &userprofile);

        let mut files_to_copy: Vec<(PathBuf, PathBuf)> = Vec::new();
        let mut fragments: Vec<(PathBuf, String)> = Vec::new();

        let temp_path = temp_dir.path();
        let mut process_errors: Vec<String> = Vec::new();
        let mut process_entry = |source: &str, target: &str, category: &str| {
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
                files_to_copy.push((source_path, target_path));
            }
        };

        if let Some(principles_entries) = &config.principles
        {
            for entry in principles_entries
            {
                process_entry(&entry.source, &entry.target, "principles");
            }
        }

        if options.mission.is_none() == true &&
            let Some(mission_entries) = &config.mission
        {
            for entry in mission_entries
            {
                process_entry(&entry.source, &entry.target, "mission");
            }
        }

        if options.no_lang == false
        {
            let resolved_files = bom::resolve_language_files(options.lang, &config)?;
            for file_entry in &resolved_files
            {
                process_entry(&file_entry.source, &file_entry.target, "languages");
            }
        }

        if let Some(integration_map) = &config.integration
        {
            for integration_config in integration_map.values()
            {
                for file_entry in &integration_config.files
                {
                    process_entry(&file_entry.source, &file_entry.target, "integration");
                }
            }
        }

        if let Some(agent_name) = options.agent &&
            let Some(agents) = config.agents.as_ref()
        {
            if let Some(agent_config) = agents.get(agent_name)
            {
                let all_mappings = [&agent_config.instructions, &agent_config.prompts, &agent_config.skills];
                for entries in all_mappings.iter().copied().flatten()
                {
                    for entry in entries
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
                            files_to_copy.push((source_path, target_path));
                        }
                    }
                }
            }
            else
            {
                println!("{} Agent '{}' not found in templates.yml", "!".yellow(), agent_name.yellow());
            }
        }

        for err in &process_errors
        {
            println!("{} {}", "!".yellow(), err.yellow());
        }

        let skill_agent = options.agent.map(|a| a.to_string()).or_else(|| agent_defaults::detect_installed_agent(&workspace));

        if let Some(ref agent_name) = skill_agent &&
            let Some(template_skills) = &config.skills &&
            template_skills.is_empty() == false
        {
            self.install_skills(
                template_skills.iter().map(|s| (s.name.as_str(), s.source.as_str())),
                agent_name,
                &workspace,
                &userprofile,
                temp_path,
                &mut files_to_copy
            )?;
        }

        if options.skills.is_empty() == false
        {
            let resolved_agent = skill_agent.as_deref().ok_or("Cannot install skills: no --agent specified and no agent detected in workspace")?;

            let adhoc_skills: Vec<(String, String)> = options
                .skills
                .iter()
                .map(|s| {
                    let url = github::expand_shorthand(s);
                    let name = Self::skill_name_from_url(&url).unwrap_or_else(|| s.clone());
                    (name, url)
                })
                .collect();

            self.install_skills(adhoc_skills.iter().map(|(n, s)| (n.as_str(), s.as_str())), resolved_agent, &workspace, &userprofile, temp_path, &mut files_to_copy)?;
        }

        let ctx = TemplateContext { source: main_source, target: main_target, fragments, template_version: config.version };

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
            self.show_dry_run_files(&ctx, skip_agents_md, options, &files_to_copy);
            return Ok(());
        }

        self.handle_main_template(&ctx, options, skip_agents_md, &mut file_tracker)?;

        let copy_result = self.copy_files_with_tracking(&files_to_copy, &mut file_tracker, &ctx, options)?;

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

    /// Merges fragment files into main AGENTS.md at insertion points
    ///
    /// Reads fragments that have `$instructions` placeholder in their target path
    /// and inserts them into the main AGENTS.md template at the corresponding
    /// insertion points: `<!-- {mission} -->`, `<!-- {principles} -->`,
    /// `<!-- {languages} -->`, `<!-- {integration} -->`
    ///
    /// The insertion point comments are preserved in the final merged file.
    ///
    /// # Arguments
    ///
    /// * `ctx` - Main template context containing source, target, and fragments
    /// * `options` - Update options containing no_lang and mission settings
    ///
    /// # Errors
    ///
    /// Returns an error if file reading or writing fails
    fn merge_fragments(&self, ctx: &TemplateContext, options: &UpdateOptions) -> Result<()>
    {
        let mut main_content = fs::read_to_string(&ctx.source)?;

        let marker_with_newline = format!("{}\n", TEMPLATE_MARKER);
        main_content = main_content.replace(&marker_with_newline, "");

        let mut fragments_by_category: HashMap<String, Vec<String>> = HashMap::new();

        if options.no_lang == true
        {
            fragments_by_category.entry("languages".to_string()).or_default();
        }

        for (fragment_path, category) in &ctx.fragments
        {
            let fragment_content = fs::read_to_string(fragment_path)?;
            fragments_by_category.entry(category.clone()).or_default().push(fragment_content);
        }

        if let Some(mission_content) = options.mission
        {
            let formatted_mission = format!("## Mission Statement\n\n{}", mission_content.trim());
            fragments_by_category.entry("mission".to_string()).or_default().push(formatted_mission);
            println!("{} Using custom mission statement", "→".blue());
        }

        for (category, contents) in fragments_by_category
        {
            let insertion_point = format!("<!-- {{{}}} -->", category);

            let combined_content = contents.iter().map(|c| c.trim()).collect::<Vec<_>>().join("\n\n");

            if main_content.contains(&insertion_point)
            {
                let replacement = format!("<!-- {{{}}} -->\n\n{}", category, combined_content);
                main_content = main_content.replace(&insertion_point, &replacement);
            }
            else
            {
                println!("{} Warning: Insertion point {} not found in AGENTS.md", "!".yellow(), insertion_point.yellow());
            }
        }

        if let Some(parent) = ctx.target.parent()
        {
            fs::create_dir_all(parent)?;
        }
        fs::write(&ctx.target, main_content)?;

        Ok(())
    }

    /// Shows dry-run preview of files that would be created/modified
    ///
    /// # Arguments
    ///
    /// * `ctx` - Template context for main AGENTS.md
    /// * `skip_agents_md` - Whether AGENTS.md is customized and should be skipped
    /// * `options` - Update options containing force and dry_run settings
    /// * `files_to_copy` - List of (source, target) file pairs
    fn show_dry_run_files(&self, ctx: &TemplateContext, skip_agents_md: bool, options: &UpdateOptions, files_to_copy: &[(PathBuf, PathBuf)])
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

        for (_, target) in files_to_copy
        {
            if target.exists()
            {
                println!("  {} {} (would be overwritten)", "●".yellow(), target.display());
            }
            else
            {
                println!("  {} {} (would be created)", "●".green(), target.display());
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
    /// * `options` - Update options containing mission, no_lang, lang, and force settings
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
            if options.no_lang
            {
                None
            }
            else
            {
                Some(options.lang.to_string())
            },
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
    /// * `ctx` - Template context containing the template version for file tracking
    /// * `options` - Update options containing lang, no_lang, agent, and force settings
    ///
    /// # Returns
    ///
    /// Returns `CopyFilesResult::Done` with skipped files, or `CopyFilesResult::Cancelled` if user quits
    ///
    /// # Errors
    ///
    /// Returns an error if file operations fail
    fn copy_files_with_tracking(
        &self, files_to_copy: &[(PathBuf, PathBuf)], file_tracker: &mut FileTracker, ctx: &TemplateContext, options: &UpdateOptions
    ) -> Result<CopyFilesResult>
    {
        println!("{} Copying templates to target directories", "→".blue());

        let mut skipped_files = Vec::new();

        for (source, target) in files_to_copy
        {
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

                file_tracker.record_installation(
                    target,
                    new_template_sha,
                    ctx.template_version,
                    if options.no_lang
                    {
                        None
                    }
                    else
                    {
                        Some(options.lang.to_string())
                    },
                    category.to_string()
                );
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

    /// Install skills into the agent's skill directory
    ///
    /// For each skill, resolves the source (local or GitHub) and adds file entries
    /// to the files_to_copy list. GitHub directory skills are downloaded via the
    /// Contents API; local skills are copied from the global template cache.
    fn install_skills<'b, I>(
        &self, skills: I, agent_name: &str, workspace: &Path, userprofile: &Path, temp_dir: &Path, files_to_copy: &mut Vec<(PathBuf, PathBuf)>
    ) -> Result<()>
    where I: Iterator<Item = (&'b str, &'b str)>
    {
        let skill_dir_template = agent_defaults::get_skill_dir(agent_name).ok_or_else(|| format!("Unknown agent '{}': no skill directory defined", agent_name))?;

        for (skill_name, source) in skills
        {
            let target_base = self.resolve_placeholder(skill_dir_template, workspace, userprofile).join(skill_name);

            if github::is_url(source) == true
            {
                let parsed = github::parse_github_url(source).ok_or_else(|| format!("Invalid GitHub URL for skill '{}': {}", skill_name, source))?;

                println!("{} Installing skill '{}' from GitHub...", "→".blue(), skill_name.green());

                match github::list_directory_contents(&parsed)
                {
                    | Ok(entries) =>
                    {
                        for entry in &entries
                        {
                            if entry.entry_type != "file"
                            {
                                continue;
                            }
                            if let Some(ref dl_url) = entry.download_url
                            {
                                let temp_path = temp_dir.join(format!("skill_{}_{}", skill_name, entry.name));

                                print!("  {} Downloading {}... ", "→".blue(), entry.name.yellow());
                                io::stdout().flush()?;

                                match github::download_file(dl_url, &temp_path)
                                {
                                    | Ok(_) =>
                                    {
                                        println!("{}", "✓".green());
                                        files_to_copy.push((temp_path, target_base.join(&entry.name)));
                                    }
                                    | Err(e) =>
                                    {
                                        println!("{} ({})", "✗".red(), e);
                                    }
                                }
                            }
                        }
                    }
                    | Err(e) =>
                    {
                        println!("{} Failed to list skill directory '{}': {}", "!".yellow(), skill_name, e);
                    }
                }
            }
            else
            {
                let source_dir = self.config_dir.join(source);
                if source_dir.is_dir() == true
                {
                    println!("{} Installing skill '{}' from local templates...", "→".blue(), skill_name.green());

                    if let Ok(entries) = std::fs::read_dir(&source_dir)
                    {
                        for entry in entries.flatten()
                        {
                            let path = entry.path();
                            if path.is_file() == true &&
                                let Some(filename) = path.file_name()
                            {
                                files_to_copy.push((path.clone(), target_base.join(filename)));
                            }
                        }
                    }
                }
                else if source_dir.is_file() == true
                {
                    let filename = source_dir.file_name().map(|f| f.to_os_string());
                    if let Some(fname) = filename
                    {
                        files_to_copy.push((source_dir, target_base.join(fname)));
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

    /// Extract a skill name from a GitHub URL or expanded shorthand
    fn skill_name_from_url(url: &str) -> Option<String>
    {
        let trimmed = url.trim_end_matches('/');
        trimmed.rsplit('/').next().map(|s| s.to_string()).filter(|s| s.is_empty() == false)
    }
}
