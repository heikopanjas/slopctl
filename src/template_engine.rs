//! Shared template engine functionality for vibe-check
//!
//! This module provides the `TemplateEngine` trait with default implementations
//! for common template operations shared between v1 and v2 engines, as well as
//! free functions used by both engines and the `TemplateManager`.

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf}
};

use owo_colors::OwoColorize;

use crate::{
    Result,
    bom::TemplateConfig,
    file_tracker::{FileStatus, FileTracker},
    utils::{FileActionResponse, copy_file_with_mkdir, prompt_file_modification}
};

/// Template marker comment used to detect unmerged template files
pub const TEMPLATE_MARKER: &str = "<!-- VIBE-CHECK-TEMPLATE: This marker indicates an unmerged template. Do not remove manually. -->";

/// Options for the template update operation
///
/// Aggregates CLI parameters that are passed through the update call chain.
pub struct UpdateOptions<'a>
{
    /// Programming language or framework identifier
    pub lang:    &'a str,
    /// AI coding agent identifier (required for v1, optional for v2)
    pub agent:   Option<&'a str>,
    /// Skip language-specific setup
    pub no_lang: bool,
    /// Custom mission statement to override template default
    pub mission: Option<&'a str>,
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
/// Loads and parses templates.yml from the given config directory.
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

/// Shared trait for template engine operations
///
/// Provides default implementations for common template operations
/// shared between v1 and v2 template engines. Each engine only needs
/// to implement `config_dir()` and its version-specific `update()` method.
pub trait TemplateEngine
{
    /// Returns the path to the global template storage directory
    fn config_dir(&self) -> &Path;

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
        // Read main AGENTS.md template
        let mut main_content = fs::read_to_string(&ctx.source)?;

        // Remove the template marker to indicate this is a merged/customized file
        let marker_with_newline = format!("{}\n", TEMPLATE_MARKER);
        main_content = main_content.replace(&marker_with_newline, "");

        // Group fragments by category to handle multiple fragments per insertion point
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

        // If custom mission is provided, add it to the fragments
        if let Some(mission_content) = options.mission
        {
            let formatted_mission = format!("## Mission Statement\n\n{}", mission_content.trim());
            fragments_by_category.entry("mission".to_string()).or_default().push(formatted_mission);
            println!("{} Using custom mission statement", "→".blue());
        }

        // Process each category
        for (category, contents) in fragments_by_category
        {
            let insertion_point = format!("<!-- {{{}}} -->", category);

            // Combine all fragments for this category
            let combined_content = contents.iter().map(|c| c.trim()).collect::<Vec<_>>().join("\n\n");

            // Replace insertion point with comment + fragment content (keep single insertion point)
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

        // Write merged content to target
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

        // Show main AGENTS.md status
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

        // Show other files
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
        // Skip AGENTS.md if customized and force is false
        if skip_agents_md && options.force == false
        {
            println!("{} Skipping AGENTS.md (customized)", "→".blue());
            return Ok(());
        }

        if ctx.fragments.is_empty() == false || options.mission.is_some() == true
        {
            // Merge fragments into AGENTS.md
            println!("{} Merging fragments into AGENTS.md", "→".blue());
            self.merge_fragments(ctx, options)?;
        }
        else
        {
            // No fragments, just copy main file as-is
            if let Some(parent) = ctx.target.parent()
            {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&ctx.source, &ctx.target)?;
        }

        println!("  {} {}", "✓".green(), ctx.target.display().to_string().yellow());

        // Record installation in file tracker
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
            // Calculate new template SHA
            let new_template_sha = FileTracker::calculate_sha256(source)?;

            // Check if file needs to be processed
            let should_copy = if target.exists() == false
            {
                // File doesn't exist, safe to copy
                true
            }
            else if options.force == true
            {
                // Force flag set, always overwrite
                true
            }
            else
            {
                // Check modification status
                match file_tracker.check_modification(target)?
                {
                    | FileStatus::NotTracked =>
                    {
                        // Not tracked, could be user file - prompt for safety
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
                    | FileStatus::Unmodified =>
                    {
                        // User didn't modify, safe to update
                        true
                    }
                    | FileStatus::Modified =>
                    {
                        // User modified, prompt
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
                            // Shouldn't happen, but treat as safe to update
                            true
                        }
                    }
                    | FileStatus::Deleted =>
                    {
                        // Was tracked but deleted, safe to recreate
                        true
                    }
                }
            };

            if should_copy == true
            {
                copy_file_with_mkdir(source, target)?;
                println!("  {} {}", "✓".green(), target.display().to_string().yellow());

                // Determine category based on target path
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

                // Record installation in file tracker
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
}
