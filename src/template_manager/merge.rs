//! AI-assisted merge of customized files with updated templates

use std::{
    collections::HashMap,
    fs,
    io::Write as _,
    path::{Path, PathBuf}
};

use owo_colors::OwoColorize;

use super::TemplateManager;
use crate::{
    EffectiveConfig, Result,
    file_tracker::FileTracker,
    llm::{ChatMessage, ChatResponse, LlmClient, Provider},
    template_engine::{self, ResolvedContent, TemplateEngine, UpdateOptions}
};

/// User-supplied overrides that control which templates are considered during merge
pub struct MergeOptions<'a>
{
    /// Language/framework override (falls back to installed language from tracker)
    pub lang:    Option<&'a str>,
    /// Agent override (falls back to installed agents detected in workspace)
    pub agent:   Option<&'a str>,
    /// Mission statement to use when generating the fresh template for comparison
    pub mission: Option<&'a str>,
    /// Additional skill sources from CLI `--skill` flags (local paths; URLs skipped)
    pub skills:  &'a [String]
}

/// Marker that separates template-managed content from user-owned changelog
const CHANGELOG_MARKER: &str = "<!-- {changelog} -->";

/// Classification of a file in the target-source map
enum FileClass
{
    /// File does not exist on disk; write template content directly
    New
    {
        target: PathBuf, content: String, lang: String, agent: String, display: String
    },
    /// File exists and content matches template; skip
    Unchanged
    {
        display: String
    },
    /// File exists and content differs from template; needs LLM merge.
    /// When the changelog marker is present, only the template half is stored
    /// for merging; the user's changelog tail is preserved separately.
    Diverged
    {
        target:           PathBuf,
        template_content: String,
        user_content:     String,
        user_changelog:   Option<String>,
        lang:             String,
        agent:            String,
        display:          String
    }
}

/// System prompt instructing the LLM how to perform the merge
const MERGE_SYSTEM_PROMPT: &str = "\
You are a file merge assistant that combines user-customized configuration files with updated templates. Follow these rules strictly:

1. PRESERVE all user customizations: added sections, modified content, custom notes, project-specific information, and any user-authored text. The merged output must \
                                   be ADDITIVE — never shorter than the user's current file unless removing an exact duplicate.
2. INCORPORATE all new content from the updated template: new sections, updated instructions, structural changes, and new conventions.
3. When both files define the same section, prefer the user's version but integrate any genuinely new information from the template.
4. Maintain the overall document structure and formatting of the user's file.
5. Do NOT add commentary, explanations, or merge markers. Output ONLY the merged file content, ready to save.
6. If the template introduces a new section that the user's file does not have, insert it in a natural location that matches the template's ordering.
7. Do NOT remove any user content unless it directly contradicts a template change (in which case prefer the template's factual updates but keep user customizations).
8. CRITICAL: Changelog, history, and log sections (such as 'Recent Updates & Decisions') must be preserved IN FULL. Every single entry must appear in the output \
                                   exactly as it was in the user's file. These sections are append-only records and must never be summarized, truncated, or \
                                   condensed.";

impl TemplateManager
{
    /// Lists available models from the selected LLM provider
    ///
    /// When `provider_override` is supplied it is used directly; otherwise the
    /// provider is resolved from config (`merge.provider`) or env auto-detect.
    /// The currently configured default model is marked in the output.
    ///
    /// # Arguments
    ///
    /// * `provider_override` - Optional CLI-supplied provider name (overrides config/env)
    ///
    /// # Errors
    ///
    /// Returns an error if provider resolution or the API call fails
    pub fn list_models(&self, provider_override: Option<&str>) -> Result<()>
    {
        let (provider_name, model_name) = if let Some(p) = provider_override
        {
            let config = std::env::current_dir().ok().and_then(|cwd| EffectiveConfig::load(&cwd).ok());
            let model = config.as_ref().and_then(|c| c.get("merge.model"));
            let provider_enum = Provider::from_name(p)?;
            let effective_model = model.clone().unwrap_or_else(|| provider_enum.default_model().to_string());
            println!("{} Using provider: {} ({})", "→".blue(), p.green(), effective_model.yellow());
            (p.to_string(), model)
        }
        else
        {
            Self::resolve_provider_and_model()?
        };
        let provider_enum = Provider::from_name(&provider_name)?;
        let default_model = model_name.as_deref().unwrap_or(provider_enum.default_model());

        let client = LlmClient::new(provider_enum, None)?;
        let models = client.list_models()?;

        if models.is_empty() == true
        {
            println!("{} No models found from {}", "→".blue(), provider_name.yellow());
            return Ok(());
        }

        println!("{}", format!("Models available from {}:", provider_name).bold());
        for m in &models
        {
            if m == default_model
            {
                println!("  {} {}", m.green(), "(default)".dimmed());
            }
            else
            {
                println!("  {}", m);
            }
        }

        Ok(())
    }

    /// AI-assisted merge of customized workspace files with updated templates
    ///
    /// Follows the same file-resolution pipeline as `init`, but differs in conflict
    /// strategy: new files are written directly, unchanged files are skipped, and
    /// diverged files are sent to an LLM for AI-assisted merging.
    ///
    /// # Arguments
    ///
    /// * `options` - User-supplied overrides (lang, agent, mission, skills)
    /// * `dry_run` - If true, shows what would happen without making changes
    /// * `preview` - If true, writes `.merged` sidecar files instead of replacing originals
    /// * `verbose` - If true, prints token usage, reports unchanged files, and dumps the outgoing chat messages plus streams the incoming agent response to stdout
    ///
    /// # Errors
    ///
    /// Returns an error if provider resolution fails, LLM calls fail, or file I/O fails
    pub fn merge(&self, options: &MergeOptions, dry_run: bool, preview: bool, verbose: bool) -> Result<()>
    {
        let engine = TemplateEngine::new(&self.config_dir);

        let update_options =
            UpdateOptions { lang: options.lang, agent: options.agent, mission: options.mission, skills: options.skills, force: false, dry_run: false };

        let content_map = engine.build_target_content_map(&update_options)?;

        let workspace = std::env::current_dir()?;
        let _ = self.try_migrate_tracker(&workspace);
        let classified = classify_files(&content_map, &workspace);

        let new_count = classified.iter().filter(|c| matches!(c, FileClass::New { .. })).count();
        let diverged_count = classified.iter().filter(|c| matches!(c, FileClass::Diverged { .. })).count();
        let unchanged_count = classified.iter().filter(|c| matches!(c, FileClass::Unchanged { .. })).count();

        if new_count == 0 && diverged_count == 0
        {
            println!("{} All files are up to date", "✓".green());
            if verbose == true && unchanged_count > 0
            {
                println!("{} {} file{} unchanged", "→".blue(), unchanged_count, plural(unchanged_count));
            }
            return Ok(());
        }

        println!(
            "{} {} new, {} to merge, {} unchanged",
            "→".blue(),
            new_count.to_string().green(),
            diverged_count.to_string().yellow(),
            unchanged_count.to_string().dimmed()
        );
        println!();

        let needs_llm = diverged_count > 0 && dry_run == false;
        let client = if needs_llm == true
        {
            let (provider_name, model_name) = Self::resolve_provider_and_model()?;
            let provider_enum = Provider::from_name(&provider_name)?;
            Some(LlmClient::new(provider_enum, model_name.as_deref())?)
        }
        else
        {
            None
        };

        let mut file_tracker = FileTracker::new(&workspace)?;
        let template_version = template_engine::load_template_config(&self.config_dir).map(|c| c.version).unwrap_or(0);

        let mut total_input: u64 = 0;
        let mut total_output: u64 = 0;
        let mut truncated_count: u64 = 0;

        for entry in &classified
        {
            match entry
            {
                | FileClass::New { target, content, lang, agent, display } =>
                {
                    if dry_run == true
                    {
                        println!("  {} Would create: {}", "●".green(), display.green());
                    }
                    else
                    {
                        if let Some(parent) = target.parent()
                        {
                            fs::create_dir_all(parent)?;
                        }
                        fs::write(target, content)?;
                        println!("  {} Created {}", "✓".green(), display.green());

                        let sha = FileTracker::calculate_sha256(target)?;
                        let category = categorize_path(target, options);
                        file_tracker.record_installation(target, sha, template_version, lang.clone(), agent.clone(), category);
                    }
                }
                | FileClass::Unchanged { display } =>
                {
                    if verbose == true
                    {
                        println!("  {} {} (unchanged)", "○".dimmed(), display.dimmed());
                    }
                }
                | FileClass::Diverged { target, template_content, user_content, user_changelog, lang, agent, display } =>
                {
                    if dry_run == true
                    {
                        println!("  {} Would merge: {}", "→".yellow(), display.yellow());
                        continue;
                    }

                    let llm = client.as_ref().expect("LlmClient required for diverged files");

                    let output_path = if preview == true
                    {
                        let sidecar = sidecar_path(target);

                        if sidecar.exists() == true
                        {
                            let rel_sidecar = sidecar.strip_prefix(&workspace).unwrap_or(&sidecar);
                            println!(
                                "  {} {} {} {}",
                                "!".yellow(),
                                "Skipped:".yellow(),
                                display.yellow(),
                                format!("(.merged already exists: {})", rel_sidecar.display()).dimmed()
                            );
                            continue;
                        }

                        sidecar
                    }
                    else
                    {
                        target.clone()
                    };

                    let partial = partial_path(target);

                    if partial.exists() == true
                    {
                        let rel_partial = partial.strip_prefix(&workspace).unwrap_or(&partial);
                        println!(
                            "  {} {} {} {}",
                            "!".yellow(),
                            "Skipped:".yellow(),
                            display.yellow(),
                            format!("(.partial exists from previous run: {})", rel_partial.display()).dimmed()
                        );
                        continue;
                    }

                    let messages = build_merge_messages(user_content, template_content);

                    if verbose == true
                    {
                        print_outgoing_messages(display, &messages);
                    }
                    else
                    {
                        print!("  {} Merging {}... ", "→".blue(), display.yellow());
                        std::io::stdout().flush()?;
                    }

                    let mut partial_file = fs::File::create(&partial)?;
                    let mut char_count: usize = 0;
                    let start = std::time::Instant::now();

                    let response = llm.chat_stream(&messages, |chunk| {
                        let _ = partial_file.write_all(chunk.as_bytes());
                        char_count += chunk.len();
                        if verbose == true
                        {
                            let _ = write!(std::io::stdout(), "{}", chunk);
                            let _ = std::io::stdout().flush();
                        }
                        else
                        {
                            let elapsed = start.elapsed().as_secs();
                            let _ = write!(
                                std::io::stdout(),
                                "\r  {} Merging {}... {}s ({} chars)",
                                "→".blue(),
                                display.yellow(),
                                elapsed,
                                format_number(char_count as u64)
                            );
                            let _ = std::io::stdout().flush();
                        }
                    });

                    if verbose == true
                    {
                        print_incoming_footer();
                    }

                    match response
                    {
                        | Ok(response) =>
                        {
                            accumulate_usage(&response, &mut total_input, &mut total_output, &mut truncated_count);

                            let is_truncated = response.stop_reason.as_deref().is_some_and(|r| matches!(r, "max_tokens" | "length"));

                            if is_truncated == true
                            {
                                let rel_partial = partial.strip_prefix(&workspace).unwrap_or(&partial);
                                print!("\r\x1b[2K");
                                println!(
                                    "  {} {} {} (truncated, partial saved: {})",
                                    "!".yellow(),
                                    display.yellow(),
                                    "hit max_tokens limit".red(),
                                    rel_partial.display().to_string().dimmed()
                                );
                            }
                            else
                            {
                                let final_content = reassemble(&response.content, user_changelog);
                                fs::write(&output_path, &final_content)?;
                                let _ = fs::remove_file(&partial);

                                let rel = output_path.strip_prefix(&workspace).unwrap_or(&output_path);
                                print!("\r\x1b[2K");
                                if preview == true
                                {
                                    println!("  {} wrote {}", "✓".green(), rel.display().to_string().yellow());
                                }
                                else
                                {
                                    println!("  {} merged {}", "✓".green(), display.yellow());
                                    let sha = FileTracker::calculate_sha256(target)?;
                                    let category = categorize_path(target, options);
                                    file_tracker.record_installation(target, sha, template_version, lang.clone(), agent.clone(), category);
                                }
                            }
                        }
                        | Err(e) =>
                        {
                            let rel_partial = partial.strip_prefix(&workspace).unwrap_or(&partial);
                            print!("\r\x1b[2K");
                            println!("  {} {} failed: {} (partial saved: {})", "!".red(), display.yellow(), e, rel_partial.display().to_string().dimmed());
                        }
                    }
                }
            }
        }

        file_tracker.save()?;

        if dry_run == true
        {
            println!();
            println!("{} Dry run complete. No files were modified.", "✓".green());
        }
        else if preview == true
        {
            println!();
            println!("{} Review .merged files, then replace originals when satisfied", "→".blue());
        }
        else
        {
            println!();
            println!("{} Merge complete.", "✓".green());
        }

        if verbose == true && dry_run == false && (total_input > 0 || total_output > 0)
        {
            let total = total_input + total_output;
            println!("{} Tokens: {} input, {} output ({} total)", "→".blue(), format_number(total_input), format_number(total_output), format_number(total));

            if truncated_count > 0
            {
                println!(
                    "{} {} {} truncated (hit max_tokens limit)",
                    "!".yellow(),
                    truncated_count,
                    if truncated_count == 1
                    {
                        "file was"
                    }
                    else
                    {
                        "files were"
                    }
                );
            }
        }

        Ok(())
    }

    /// Resolves the LLM provider and model from config or env
    ///
    /// Priority: config `merge.provider` > auto-detect from environment API keys
    /// (`ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, `MISTRAL_API_KEY`).
    /// Model: config `merge.model` > None (provider default used later).
    pub(super) fn resolve_provider_and_model() -> Result<(String, Option<String>)>
    {
        let config = std::env::current_dir().ok().and_then(|cwd| EffectiveConfig::load(&cwd).ok());

        let provider = if let Some(ref c) = config &&
            let Some(p) = c.get("merge.provider")
        {
            p
        }
        else if let Some(detected) = Provider::detect_from_env()
        {
            detected.name().to_string()
        }
        else
        {
            return Err(anyhow::anyhow!(
                "No LLM provider configured or auto-detected.\nSet an API key env var (OPENAI_API_KEY, ANTHROPIC_API_KEY, MISTRAL_API_KEY),\nor configure: slopctl \
                 config --set merge.provider openai\nSupported: openai, anthropic, ollama, mistral"
            ));
        };

        let model = if let Some(ref c) = config
        {
            c.get("merge.model")
        }
        else
        {
            None
        };

        let effective_model = model.clone().unwrap_or_else(|| Provider::from_name(&provider).map(|p| p.default_model().to_string()).unwrap_or_default());
        println!("{} Using provider: {} ({})", "→".blue(), provider.green(), effective_model.yellow());

        Ok((provider, model))
    }
}

/// Splits content at the changelog marker, returning (template_half, changelog_half).
/// If the marker is absent, returns None.
fn split_at_changelog(content: &str) -> Option<(&str, &str)>
{
    content.find(CHANGELOG_MARKER).map(|pos| {
        let template_half = &content[..pos];
        let changelog_half = &content[pos + CHANGELOG_MARKER.len()..];
        (template_half, changelog_half)
    })
}

/// Classifies every entry in the content map as New, Unchanged, or Diverged.
/// When the changelog marker is present in both files, only the template half
/// is compared -- changelog-only differences are classified as Unchanged.
fn classify_files(content_map: &HashMap<PathBuf, ResolvedContent>, workspace: &Path) -> Vec<FileClass>
{
    let mut classified = Vec::with_capacity(content_map.len());

    for (target, resolved) in content_map
    {
        let display = target.strip_prefix(workspace).unwrap_or(target).display().to_string();
        let template_content = &resolved.content;

        if target.exists() == false
        {
            classified.push(FileClass::New {
                target: target.clone(),
                content: template_content.clone(),
                lang: resolved.lang.clone(),
                agent: resolved.agent.clone(),
                display
            });
        }
        else if let Ok(current_content) = fs::read_to_string(target)
        {
            if current_content == *template_content
            {
                classified.push(FileClass::Unchanged { display });
            }
            else if let (Some((tmpl_upper, _)), Some((user_upper, user_lower))) = (split_at_changelog(template_content), split_at_changelog(&current_content))
            {
                if tmpl_upper == user_upper
                {
                    classified.push(FileClass::Unchanged { display });
                }
                else
                {
                    classified.push(FileClass::Diverged {
                        target: target.clone(),
                        template_content: tmpl_upper.to_string(),
                        user_content: user_upper.to_string(),
                        user_changelog: Some(user_lower.to_string()),
                        lang: resolved.lang.clone(),
                        agent: resolved.agent.clone(),
                        display
                    });
                }
            }
            else
            {
                classified.push(FileClass::Diverged {
                    target: target.clone(),
                    template_content: template_content.clone(),
                    user_content: current_content,
                    user_changelog: None,
                    lang: resolved.lang.clone(),
                    agent: resolved.agent.clone(),
                    display
                });
            }
        }
        else
        {
            classified.push(FileClass::Diverged {
                target: target.clone(),
                template_content: template_content.clone(),
                user_content: String::new(),
                user_changelog: None,
                lang: resolved.lang.clone(),
                agent: resolved.agent.clone(),
                display
            });
        }
    }

    classified.sort_by(|a, b| {
        let display_a = match a
        {
            | FileClass::New { display, .. } | FileClass::Unchanged { display } | FileClass::Diverged { display, .. } => display
        };
        let display_b = match b
        {
            | FileClass::New { display, .. } | FileClass::Unchanged { display } | FileClass::Diverged { display, .. } => display
        };
        display_a.cmp(display_b)
    });

    classified
}

/// Determines the tracking category for a target file path
fn categorize_path(target: &Path, options: &MergeOptions) -> String
{
    let target_str = target.to_string_lossy();
    if target_str.contains("SKILL.md") || target_str.contains("/skills/") || target_str.contains("\\skills\\")
    {
        "skill".to_string()
    }
    else if target_str.contains(".git")
    {
        "integration".to_string()
    }
    else if target_str.contains("AGENTS.md")
    {
        "main".to_string()
    }
    else if let Some(name) = options.agent
    {
        if target_str.contains(&format!(".{}", name)) || target_str.contains(name)
        {
            "agent".to_string()
        }
        else
        {
            "language".to_string()
        }
    }
    else
    {
        "language".to_string()
    }
}

/// Prints the outgoing chat messages to stdout for verbose mode
///
/// Emits a header with the file display name, then each message preceded by a
/// `[role]` tag. The format is meant for human inspection, not machine parsing.
fn print_outgoing_messages(display: &str, messages: &[ChatMessage])
{
    println!();
    println!("{} {} {}", "──".dimmed(), format!("Merging {}", display).yellow().bold(), "──".dimmed());
    println!("{}", "── Outgoing messages ──".dimmed());
    for msg in messages
    {
        println!();
        println!("{}", format!("[{}]", msg.role).cyan().bold());
        println!("{}", msg.content);
    }
    println!();
    println!("{}", "── Incoming response ──".dimmed());
    println!();
}

/// Prints a closing separator after the streamed agent response
fn print_incoming_footer()
{
    println!();
    println!("{}", "── End response ──".dimmed());
}

/// Builds the LLM messages for a merge operation
fn build_merge_messages(user_content: &str, template_content: &str) -> Vec<ChatMessage>
{
    vec![ChatMessage { role: "system".to_string(), content: MERGE_SYSTEM_PROMPT.to_string() }, ChatMessage {
        role:    "user".to_string(),
        content: format!(
            "<current_file>\n{}\n</current_file>\n\n<updated_template>\n{}\n</updated_template>\n\nMerge these files. Preserve all user customizations while \
             incorporating template updates. Output ONLY the merged file content.",
            user_content, template_content
        )
    }]
}

/// Reassembles a merged template half with the user's original changelog.
/// If `user_changelog` is None (no marker was present), returns the merged content as-is.
fn reassemble(merged_template_half: &str, user_changelog: &Option<String>) -> String
{
    match user_changelog
    {
        | Some(changelog) => format!("{}{}{}", merged_template_half, CHANGELOG_MARKER, changelog),
        | None => merged_template_half.to_string()
    }
}

/// Returns the `.merged` sidecar path for a given file
fn sidecar_path(path: &Path) -> PathBuf
{
    let mut sidecar = path.as_os_str().to_owned();
    sidecar.push(".merged");
    PathBuf::from(sidecar)
}

/// Returns the `.partial` recovery path for a given file
fn partial_path(path: &Path) -> PathBuf
{
    let mut partial = path.as_os_str().to_owned();
    partial.push(".partial");
    PathBuf::from(partial)
}

/// Accumulates token counts and detects truncation from a chat response
fn accumulate_usage(response: &ChatResponse, total_input: &mut u64, total_output: &mut u64, truncated_count: &mut u64)
{
    if let Some(input) = response.input_tokens
    {
        *total_input += input;
    }
    if let Some(output) = response.output_tokens
    {
        *total_output += output;
    }
    if let Some(ref reason) = response.stop_reason &&
        matches!(reason.as_str(), "max_tokens" | "length")
    {
        *truncated_count += 1;
    }
}

/// Formats a number with comma separators for display (e.g. 12345 → "12,345")
fn format_number(n: u64) -> String
{
    let s = n.to_string();
    let mut result = String::with_capacity(s.len() + s.len() / 3);
    for (i, c) in s.chars().enumerate()
    {
        if i > 0 && (s.len() - i).is_multiple_of(3) == true
        {
            result.push(',');
        }
        result.push(c);
    }
    result
}

/// Returns "s" for plural counts, empty for singular
fn plural(count: usize) -> &'static str
{
    if count == 1
    {
        ""
    }
    else
    {
        "s"
    }
}

#[cfg(test)]
mod tests
{
    use std::collections::HashMap;

    use super::*;
    use crate::{
        file_tracker::{AGENT_ALL, LANG_NONE},
        template_engine::normalize_path
    };

    fn rc(content: &str) -> ResolvedContent
    {
        ResolvedContent { content: content.to_string(), lang: LANG_NONE.to_string(), agent: AGENT_ALL.to_string() }
    }

    #[test]
    fn test_sidecar_path()
    {
        assert_eq!(sidecar_path(Path::new("/project/AGENTS.md")), PathBuf::from("/project/AGENTS.md.merged"));
        assert_eq!(sidecar_path(Path::new("relative.txt")), PathBuf::from("relative.txt.merged"));
    }

    #[test]
    fn test_build_merge_messages()
    {
        let messages = build_merge_messages("user content", "template content");
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, "system");
        assert!(messages[0].content.contains("file merge assistant") == true);
        assert_eq!(messages[1].role, "user");
        assert!(messages[1].content.contains("user content") == true);
        assert!(messages[1].content.contains("template content") == true);
    }

    #[test]
    fn test_resolve_provider_no_env_no_config_returns_error()
    {
        let _cwd = crate::template_manager::CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _env = crate::ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        let saved_cwd = std::env::current_dir().unwrap();
        let temp = tempfile::tempdir().unwrap();
        std::env::set_current_dir(&temp).unwrap();

        let saved_a = std::env::var("ANTHROPIC_API_KEY").ok();
        let saved_o = std::env::var("OPENAI_API_KEY").ok();
        let saved_m = std::env::var("MISTRAL_API_KEY").ok();
        unsafe { std::env::remove_var("ANTHROPIC_API_KEY") };
        unsafe { std::env::remove_var("OPENAI_API_KEY") };
        unsafe { std::env::remove_var("MISTRAL_API_KEY") };

        let result = TemplateManager::resolve_provider_and_model();
        assert!(result.is_err() == true);
        assert!(result.unwrap_err().to_string().contains("No LLM provider") == true);

        if let Some(k) = saved_a
        {
            unsafe { std::env::set_var("ANTHROPIC_API_KEY", k) };
        }
        if let Some(k) = saved_o
        {
            unsafe { std::env::set_var("OPENAI_API_KEY", k) };
        }
        if let Some(k) = saved_m
        {
            unsafe { std::env::set_var("MISTRAL_API_KEY", k) };
        }
        std::env::set_current_dir(saved_cwd).unwrap();
    }

    #[test]
    fn test_resolve_provider_detects_openai_from_env()
    {
        let _cwd = crate::template_manager::CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _env = crate::ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        let saved_cwd = std::env::current_dir().unwrap();
        let temp = tempfile::tempdir().unwrap();
        std::env::set_current_dir(&temp).unwrap();

        let saved_a = std::env::var("ANTHROPIC_API_KEY").ok();
        let saved_o = std::env::var("OPENAI_API_KEY").ok();
        let saved_m = std::env::var("MISTRAL_API_KEY").ok();
        unsafe { std::env::remove_var("ANTHROPIC_API_KEY") };
        unsafe { std::env::set_var("OPENAI_API_KEY", "test-key") };
        unsafe { std::env::remove_var("MISTRAL_API_KEY") };

        let result = TemplateManager::resolve_provider_and_model();
        assert!(result.is_ok() == true);
        let (provider, _model) = result.unwrap();
        assert_eq!(provider, "openai");

        unsafe { std::env::remove_var("OPENAI_API_KEY") };
        if let Some(k) = saved_a
        {
            unsafe { std::env::set_var("ANTHROPIC_API_KEY", k) };
        }
        if let Some(k) = saved_o
        {
            unsafe { std::env::set_var("OPENAI_API_KEY", k) };
        }
        if let Some(k) = saved_m
        {
            unsafe { std::env::set_var("MISTRAL_API_KEY", k) };
        }
        std::env::set_current_dir(saved_cwd).unwrap();
    }

    #[test]
    fn test_format_number_no_commas()
    {
        assert_eq!(format_number(0), "0");
        assert_eq!(format_number(1), "1");
        assert_eq!(format_number(999), "999");
    }

    #[test]
    fn test_format_number_with_commas()
    {
        assert_eq!(format_number(1_000), "1,000");
        assert_eq!(format_number(12_345), "12,345");
        assert_eq!(format_number(1_000_000), "1,000,000");
    }

    #[test]
    fn test_accumulate_usage_adds_tokens()
    {
        let mut total_input: u64 = 100;
        let mut total_output: u64 = 50;
        let mut truncated: u64 = 0;

        let response = ChatResponse { content: String::new(), input_tokens: Some(200), output_tokens: Some(300), stop_reason: Some("end_turn".to_string()) };
        accumulate_usage(&response, &mut total_input, &mut total_output, &mut truncated);

        assert_eq!(total_input, 300);
        assert_eq!(total_output, 350);
        assert_eq!(truncated, 0);
    }

    #[test]
    fn test_accumulate_usage_detects_truncation()
    {
        let mut total_input: u64 = 0;
        let mut total_output: u64 = 0;
        let mut truncated: u64 = 0;

        let response_max =
            ChatResponse { content: String::new(), input_tokens: Some(100), output_tokens: Some(50), stop_reason: Some("max_tokens".to_string()) };
        accumulate_usage(&response_max, &mut total_input, &mut total_output, &mut truncated);
        assert_eq!(truncated, 1);

        let response_len = ChatResponse { content: String::new(), input_tokens: Some(100), output_tokens: Some(50), stop_reason: Some("length".to_string()) };
        accumulate_usage(&response_len, &mut total_input, &mut total_output, &mut truncated);
        assert_eq!(truncated, 2);
    }

    #[test]
    fn test_accumulate_usage_handles_none()
    {
        let mut total_input: u64 = 10;
        let mut total_output: u64 = 20;
        let mut truncated: u64 = 0;

        let response = ChatResponse { content: String::new(), input_tokens: None, output_tokens: None, stop_reason: None };
        accumulate_usage(&response, &mut total_input, &mut total_output, &mut truncated);

        assert_eq!(total_input, 10);
        assert_eq!(total_output, 20);
        assert_eq!(truncated, 0);
    }

    #[test]
    fn test_classify_files_new() -> anyhow::Result<()>
    {
        let dir = tempfile::tempdir()?;
        let workspace = dir.path();

        let target = workspace.join("new_file.md");
        let mut map = HashMap::new();
        map.insert(target.clone(), rc("template content"));

        let classified = classify_files(&map, workspace);
        assert_eq!(classified.len(), 1);
        assert!(matches!(&classified[0], FileClass::New { display, .. } if display == "new_file.md") == true);
        Ok(())
    }

    #[test]
    fn test_classify_files_unchanged() -> anyhow::Result<()>
    {
        let dir = tempfile::tempdir()?;
        let workspace = dir.path();

        let target = workspace.join("existing.md");
        fs::write(&target, "same content")?;

        let mut map = HashMap::new();
        map.insert(normalize_path(&target), rc("same content"));

        let classified = classify_files(&map, workspace);
        assert_eq!(classified.len(), 1);
        assert!(matches!(&classified[0], FileClass::Unchanged { .. }) == true);
        Ok(())
    }

    #[test]
    fn test_classify_files_diverged() -> anyhow::Result<()>
    {
        let dir = tempfile::tempdir()?;
        let workspace = dir.path();

        let target = workspace.join("modified.md");
        fs::write(&target, "user customized content")?;

        let mut map = HashMap::new();
        map.insert(normalize_path(&target), rc("new template content"));

        let classified = classify_files(&map, workspace);
        assert_eq!(classified.len(), 1);
        assert!(matches!(&classified[0], FileClass::Diverged { .. }) == true);
        Ok(())
    }

    #[test]
    fn test_classify_files_mixed() -> anyhow::Result<()>
    {
        let dir = tempfile::tempdir()?;
        let workspace = dir.path();

        let new_file = workspace.join("new.md");
        let unchanged_file = workspace.join("unchanged.md");
        let diverged_file = workspace.join("diverged.md");

        fs::write(&unchanged_file, "same")?;
        fs::write(&diverged_file, "user version")?;

        let mut map = HashMap::new();
        map.insert(new_file, rc("template"));
        map.insert(normalize_path(&unchanged_file), rc("same"));
        map.insert(normalize_path(&diverged_file), rc("template version"));

        let classified = classify_files(&map, workspace);
        assert_eq!(classified.len(), 3);

        let new_count = classified.iter().filter(|c| matches!(c, FileClass::New { .. })).count();
        let unchanged_count = classified.iter().filter(|c| matches!(c, FileClass::Unchanged { .. })).count();
        let diverged_count = classified.iter().filter(|c| matches!(c, FileClass::Diverged { .. })).count();

        assert_eq!(new_count, 1);
        assert_eq!(unchanged_count, 1);
        assert_eq!(diverged_count, 1);
        Ok(())
    }

    #[test]
    fn test_categorize_path_main()
    {
        let options = MergeOptions { lang: None, agent: None, mission: None, skills: &[] };
        assert_eq!(categorize_path(Path::new("/project/AGENTS.md"), &options), "main");
    }

    #[test]
    fn test_categorize_path_skill()
    {
        let options = MergeOptions { lang: None, agent: None, mission: None, skills: &[] };
        assert_eq!(categorize_path(Path::new("/project/.cursor/skills/my-skill/SKILL.md"), &options), "skill");
    }

    #[test]
    fn test_categorize_path_integration()
    {
        let options = MergeOptions { lang: None, agent: None, mission: None, skills: &[] };
        assert_eq!(categorize_path(Path::new("/project/.gitignore"), &options), "integration");
    }

    #[test]
    fn test_categorize_path_agent()
    {
        let options = MergeOptions { lang: None, agent: Some("cursor"), mission: None, skills: &[] };
        assert_eq!(categorize_path(Path::new("/project/.cursorrules"), &options), "agent");
    }

    #[test]
    fn test_categorize_path_language()
    {
        let options = MergeOptions { lang: Some("rust"), agent: None, mission: None, skills: &[] };
        assert_eq!(categorize_path(Path::new("/project/.rustfmt.toml"), &options), "language");
    }

    #[test]
    fn test_plural()
    {
        assert_eq!(plural(0), "s");
        assert_eq!(plural(1), "");
        assert_eq!(plural(2), "s");
        assert_eq!(plural(100), "s");
    }

    #[test]
    fn test_classify_files_sorted_by_display_name() -> anyhow::Result<()>
    {
        let dir = tempfile::tempdir()?;
        let workspace = dir.path();

        let file_c = workspace.join("charlie.md");
        let file_a = workspace.join("alpha.md");
        let file_b = workspace.join("bravo.md");

        let mut map = HashMap::new();
        map.insert(file_c, ResolvedContent { content: "c".into(), lang: LANG_NONE.into(), agent: AGENT_ALL.into() });
        map.insert(file_a, ResolvedContent { content: "a".into(), lang: LANG_NONE.into(), agent: AGENT_ALL.into() });
        map.insert(file_b, ResolvedContent { content: "b".into(), lang: LANG_NONE.into(), agent: AGENT_ALL.into() });

        let classified = classify_files(&map, workspace);
        let displays: Vec<&str> = classified
            .iter()
            .map(|c| match c
            {
                | FileClass::New { display, .. } | FileClass::Unchanged { display } | FileClass::Diverged { display, .. } => display.as_str()
            })
            .collect();

        assert_eq!(displays, vec!["alpha.md", "bravo.md", "charlie.md"]);
        Ok(())
    }

    #[test]
    fn test_partial_path()
    {
        assert_eq!(partial_path(Path::new("/project/AGENTS.md")), PathBuf::from("/project/AGENTS.md.partial"));
        assert_eq!(partial_path(Path::new("relative.txt")), PathBuf::from("relative.txt.partial"));
    }

    #[test]
    fn test_split_at_changelog_present()
    {
        let content = "template half\n<!-- {changelog} -->\nchangelog half";
        let (upper, lower) = split_at_changelog(content).expect("marker present");
        assert_eq!(upper, "template half\n");
        assert_eq!(lower, "\nchangelog half");
    }

    #[test]
    fn test_split_at_changelog_absent()
    {
        let content = "no marker here";
        assert!(split_at_changelog(content).is_none() == true);
    }

    #[test]
    fn test_reassemble_with_changelog()
    {
        let merged = "merged template";
        let changelog = Some("\nuser changelog".to_string());
        let result = reassemble(merged, &changelog);
        assert_eq!(result, "merged template<!-- {changelog} -->\nuser changelog");
    }

    #[test]
    fn test_reassemble_without_changelog()
    {
        let merged = "full merged content";
        let result = reassemble(merged, &None);
        assert_eq!(result, "full merged content");
    }

    #[test]
    fn test_classify_files_changelog_only_diff_is_unchanged() -> anyhow::Result<()>
    {
        let dir = tempfile::tempdir()?;
        let workspace = dir.path();

        let target = workspace.join("AGENTS.md");
        let template_upper = "template content\n";
        let template = format!("{}<!-- {{changelog}} -->\n## Log\n\n- initial", template_upper);
        let user = format!("{}<!-- {{changelog}} -->\n## Log\n\n- initial\n- user entry", template_upper);
        fs::write(&target, &user)?;

        let mut map = HashMap::new();
        map.insert(normalize_path(&target), rc(&template));

        let classified = classify_files(&map, workspace);
        assert_eq!(classified.len(), 1);
        assert!(matches!(&classified[0], FileClass::Unchanged { .. }) == true);
        Ok(())
    }

    #[test]
    fn test_classify_files_template_half_differs_is_diverged() -> anyhow::Result<()>
    {
        let dir = tempfile::tempdir()?;
        let workspace = dir.path();

        let target = workspace.join("AGENTS.md");
        let template = "NEW template\n<!-- {changelog} -->\n## Log\n\n- initial";
        let user = "OLD template\n<!-- {changelog} -->\n## Log\n\n- initial\n- user entry";
        fs::write(&target, user)?;

        let mut map = HashMap::new();
        map.insert(normalize_path(&target), rc(template));

        let classified = classify_files(&map, workspace);
        assert_eq!(classified.len(), 1);
        match &classified[0]
        {
            | FileClass::Diverged { template_content, user_content, user_changelog, .. } =>
            {
                assert_eq!(template_content, "NEW template\n");
                assert_eq!(user_content, "OLD template\n");
                assert!(user_changelog.is_some() == true);
                assert!(user_changelog.as_ref().unwrap().contains("user entry") == true);
            }
            | other => panic!("expected Diverged, got {:?}", std::mem::discriminant(other))
        }
        Ok(())
    }
}
