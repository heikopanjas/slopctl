//! AI-assisted merge of customized files with updated templates

use std::{
    fs,
    path::{Path, PathBuf}
};

use owo_colors::OwoColorize;

use super::TemplateManager;
use crate::{
    Config, Result, agent_defaults,
    bom::{self, TemplateConfig},
    file_tracker::{FileStatus, FileTracker},
    github,
    llm::{ChatMessage, ChatResponse, LlmClient, Provider},
    template_engine::{self, TemplateEngine}
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

/// A file where the user's customizations need merging with template updates
struct MergeCandidate
{
    /// Absolute path to the user's current (customized) file in the workspace
    workspace_path:   PathBuf,
    /// Content of the fresh template (what init would produce now)
    template_content: String,
    /// Human-readable label for display (relative path)
    display_name:     String
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
            let config = Config::load().ok();
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
    /// For each tracked file that the user has modified AND whose template source
    /// has changed since installation, calls an LLM to produce a merged version.
    /// By default, the merged content replaces the original file. With `preview`,
    /// a `.merged` sidecar file is written instead for manual review.
    ///
    /// # Arguments
    ///
    /// * `provider` - CLI override for LLM provider name (falls back to config)
    /// * `lang` - Language/framework override (falls back to installed language from tracker)
    /// * `agent` - Agent override (falls back to installed agents detected in workspace)
    /// * `mission` - Mission statement to use when generating the fresh template for comparison
    /// * `skills` - Additional skill sources to include in the fresh template
    /// * `dry_run` - If true, shows what would be merged without calling the LLM
    /// * `preview` - If true, writes `.merged` sidecar files instead of replacing originals
    /// * `verbose` - If true, prints token usage summary after merging
    ///
    /// # Errors
    ///
    /// Returns an error if provider resolution fails, LLM calls fail, or file I/O fails
    pub fn merge(&self, options: &MergeOptions, dry_run: bool, preview: bool, verbose: bool) -> Result<()>
    {
        let (provider_name, model_name) = Self::resolve_provider_and_model()?;
        let provider_enum = Provider::from_name(&provider_name)?;

        let candidates = self.find_merge_candidates(options)?;

        if candidates.is_empty() == true
        {
            println!("{} No files need merging", "✓".green());
            println!("{} Files are either unmodified or templates have not changed", "→".blue());
            return Ok(());
        }

        println!(
            "{} Found {} file{} to merge",
            "→".blue(),
            candidates.len(),
            if candidates.len() == 1
            {
                ""
            }
            else
            {
                "s"
            }
        );
        println!();

        let mut total_input: u64 = 0;
        let mut total_output: u64 = 0;
        let mut truncated_count: u64 = 0;

        for candidate in &candidates
        {
            if dry_run == true
            {
                println!("  {} Would merge: {}", "→".blue(), candidate.display_name.yellow());
                continue;
            }

            if preview == true
            {
                let sidecar = sidecar_path(&candidate.workspace_path);
                let rel_sidecar = sidecar.strip_prefix(std::env::current_dir().unwrap_or_default()).unwrap_or(&sidecar);

                if sidecar.exists() == true
                {
                    println!(
                        "  {} {} {} {}",
                        "!".yellow(),
                        "Skipped:".yellow(),
                        candidate.display_name.yellow(),
                        format!("(.merged already exists: {})", rel_sidecar.display()).dimmed()
                    );
                    continue;
                }

                print!("  {} Merging {}... ", "→".blue(), candidate.display_name.yellow());
                std::io::Write::flush(&mut std::io::stdout())?;

                let client = LlmClient::new(provider_enum.clone(), model_name.as_deref())?;
                let user_content = fs::read_to_string(&candidate.workspace_path)?;
                let messages = build_merge_messages(&user_content, &candidate.template_content);
                let response = client.chat(&messages)?;

                accumulate_usage(&response, &mut total_input, &mut total_output, &mut truncated_count);
                fs::write(&sidecar, &response.content)?;
                println!("{} wrote {}", "✓".green(), rel_sidecar.display().to_string().yellow());
            }
            else
            {
                print!("  {} Merging {}... ", "→".blue(), candidate.display_name.yellow());
                std::io::Write::flush(&mut std::io::stdout())?;

                let client = LlmClient::new(provider_enum.clone(), model_name.as_deref())?;
                let user_content = fs::read_to_string(&candidate.workspace_path)?;
                let messages = build_merge_messages(&user_content, &candidate.template_content);
                let response = client.chat(&messages)?;

                accumulate_usage(&response, &mut total_input, &mut total_output, &mut truncated_count);
                fs::write(&candidate.workspace_path, &response.content)?;
                println!("{} merged {}", "✓".green(), candidate.display_name.yellow());
            }
        }

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
            println!("{} Merge complete. Merged files replaced originals.", "✓".green());
        }

        if verbose == true && dry_run == false
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

    /// Resolves the LLM provider and model from CLI args, config, env, or error
    ///
    /// Priority: CLI `--provider` > config `merge.provider` > auto-detect from
    /// environment API keys (`ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, `MISTRAL_API_KEY`).
    /// Model: CLI `--model` > config `merge.model` > None (provider default used later).
    pub(super) fn resolve_provider_and_model() -> Result<(String, Option<String>)>
    {
        let config = Config::load().ok();

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
                 config --add merge.provider openai\nSupported: openai, anthropic, ollama, mistral"
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

    /// Finds workspace files that need AI-assisted merging
    ///
    /// A file is a merge candidate when either:
    /// - It is tracked, user-modified, AND the template source has also changed
    /// - It exists on disk but is untracked, and its content differs from the current template (e.g. AGENTS.md was customized before tracking began, or was skipped by
    ///   init because it was already customized)
    fn find_merge_candidates(&self, options: &MergeOptions) -> Result<Vec<MergeCandidate>>
    {
        let workspace = std::env::current_dir()?;
        let tracker = FileTracker::new(&self.config_dir)?;
        let entries = tracker.get_workspace_entries(&workspace);

        let config = template_engine::load_template_config(&self.config_dir)?;
        let engine = TemplateEngine::new(&self.config_dir);
        let userprofile = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;

        let target_source_map = build_target_source_map(&engine, &config, &workspace, &userprofile, &tracker, options)?;

        let mut candidates = Vec::new();
        let mut seen_paths = std::collections::HashSet::new();

        // Tracked files: both user AND template must have changed since install
        for (path, metadata) in &entries
        {
            seen_paths.insert(path.clone());

            if tracker.check_modification(path)? != FileStatus::Modified
            {
                continue;
            }

            if let Some(template_content) = target_source_map.get(path)
            {
                let template_sha = sha256_string(template_content);

                if template_sha != metadata.original_sha
                {
                    let display = path.strip_prefix(&workspace).unwrap_or(path).display().to_string();
                    candidates.push(MergeCandidate { workspace_path: path.clone(), template_content: template_content.clone(), display_name: display });
                }
            }
        }

        // Untracked files: exist on disk and differ from current template
        for (target_path, template_content) in &target_source_map
        {
            if seen_paths.contains(target_path) || target_path.exists() == false
            {
                continue;
            }

            let current_sha = FileTracker::calculate_sha256(target_path)?;
            let template_sha = sha256_string(template_content);

            if current_sha != template_sha
            {
                let display = target_path.strip_prefix(&workspace).unwrap_or(target_path).display().to_string();
                candidates.push(MergeCandidate { workspace_path: target_path.clone(), template_content: template_content.clone(), display_name: display });
            }
        }

        Ok(candidates)
    }
}

/// Builds a map from resolved workspace target path → fresh template content
///
/// Walks all sections of templates.yml and resolves each entry's source content
/// and target path. For the "main" file (AGENTS.md), produces a fresh merged
/// version with all fragment sections filled in.
///
/// Fields in `options` override auto-detected values when set.
fn build_target_source_map(
    engine: &TemplateEngine, config: &TemplateConfig, workspace: &Path, userprofile: &Path, tracker: &FileTracker, options: &MergeOptions
) -> Result<std::collections::HashMap<PathBuf, String>>
{
    let mut map = std::collections::HashMap::new();

    // Resolve language: CLI override > tracker detection
    let installed_lang = options.lang.map(|s| s.to_string()).or_else(|| tracker.get_installed_language_for_workspace(workspace));

    // Main template (AGENTS.md): generate fresh merged content
    if let Some(ref main_config) = config.main
    {
        let source_path = engine.config_dir().join(&main_config.source);
        let target_path = resolve_target(engine, &main_config.target, workspace, userprofile);

        if source_path.exists() == true
        {
            let fresh_content = generate_fresh_main(&source_path, engine, config, installed_lang.as_deref(), options.mission)?;
            map.insert(normalize_path(&target_path), fresh_content);
        }
    }

    // Principles, mission entries → direct source files
    for entry in config.principles.iter().chain(config.mission.iter())
    {
        if entry.target.starts_with("$instructions") == true
        {
            continue;
        }
        insert_source_content(engine, &entry.source, &entry.target, workspace, userprofile, &mut map);
    }

    // Integration entries
    for int_config in config.integration.values()
    {
        for entry in &int_config.files
        {
            if entry.target.starts_with("$instructions") == true
            {
                continue;
            }
            insert_source_content(engine, &entry.source, &entry.target, workspace, userprofile, &mut map);
        }
    }

    // Agent entries (instructions + prompts for all agents)
    for agent_config in config.agents.values()
    {
        for entry in agent_config.instructions.iter().chain(agent_config.prompts.iter())
        {
            if entry.target.starts_with("$userprofile") == true
            {
                continue;
            }
            insert_source_content(engine, &entry.source, &entry.target, workspace, userprofile, &mut map);
        }
    }

    // Language files (if a language is installed)
    if let Some(ref lang) = installed_lang &&
        let Ok(files) = bom::resolve_language_files(lang, config)
    {
        for entry in &files
        {
            if entry.target.starts_with("$instructions") == true || entry.target.starts_with("$userprofile") == true
            {
                continue;
            }
            insert_source_content(engine, &entry.source, &entry.target, workspace, userprofile, &mut map);
        }
    }

    // Skill files: walk local skill sources and map each file to its installed target
    // Agent: CLI override > workspace detection
    let active_agents: Vec<String> = if let Some(a) = options.agent
    {
        vec![a.to_string()]
    }
    else
    {
        agent_defaults::detect_all_installed_agents(workspace)
    };
    let cross_client_raw = agent_defaults::CROSS_CLIENT_SKILL_DIR;

    // Top-level skills → each active agent's skill dir + cross-client fallback
    if active_agents.is_empty() == true
    {
        insert_skill_sources(engine, &config.skills, cross_client_raw, workspace, userprofile, &mut map);
    }
    else
    {
        for agent_name in &active_agents
        {
            if let Some(skill_dir) = agent_defaults::get_skill_dir(agent_name)
            {
                insert_skill_sources(engine, &config.skills, skill_dir, workspace, userprofile, &mut map);
            }
        }
    }

    // Agent-specific skills
    for agent_name in &active_agents
    {
        if let Some(agent_config) = config.agents.get(agent_name.as_str()) &&
            let Some(skill_dir) = agent_defaults::get_skill_dir(agent_name)
        {
            insert_skill_sources(engine, &agent_config.skills, skill_dir, workspace, userprofile, &mut map);
        }
    }

    // Language skills → cross-client dir
    if let Some(ref lang) = installed_lang &&
        let Ok(lang_skills) = bom::resolve_language_skills(lang, config)
    {
        insert_skill_sources(engine, &lang_skills, cross_client_raw, workspace, userprofile, &mut map);
    }

    // Extra skills from CLI --skill flags (local paths only; URL-based skills are skipped)
    if options.skills.is_empty() == false
    {
        let extra_defs: Vec<bom::SkillDefinition> = options.skills.iter().map(|s| bom::SkillDefinition { name: s.clone(), source: s.clone() }).collect();
        let skill_dir = active_agents.first().and_then(|a| agent_defaults::get_skill_dir(a)).unwrap_or(cross_client_raw);
        insert_skill_sources(engine, &extra_defs, skill_dir, workspace, userprofile, &mut map);
    }

    Ok(map)
}

/// Reads a template source file and inserts it into the target→content map
fn insert_source_content(
    engine: &TemplateEngine, source: &str, target: &str, workspace: &Path, userprofile: &Path, map: &mut std::collections::HashMap<PathBuf, String>
)
{
    let source_path = engine.config_dir().join(source);
    let target_path = resolve_target(engine, target, workspace, userprofile);

    if source_path.exists() == true &&
        let Ok(content) = fs::read_to_string(&source_path)
    {
        map.insert(normalize_path(&target_path), content);
    }
}

/// Walks local skill source directories and inserts each file into the target→content map
///
/// URL-based skill sources are skipped (they are fetched at install time, not cached locally).
fn insert_skill_sources(
    engine: &TemplateEngine, skills: &[bom::SkillDefinition], skill_dir_placeholder: &str, workspace: &Path, userprofile: &Path,
    map: &mut std::collections::HashMap<PathBuf, String>
)
{
    let skill_base = agent_defaults::resolve_placeholder_path(skill_dir_placeholder, workspace, userprofile);

    for skill in skills
    {
        if github::is_url(&skill.source) == true
        {
            continue;
        }

        let source_dir = engine.config_dir().join(&skill.source);
        if source_dir.is_dir() == false
        {
            continue;
        }

        let target_base = skill_base.join(&skill.name);
        insert_skill_dir_recursive(&source_dir, &target_base, map);
    }
}

/// Recursively reads files from a skill source directory and inserts them into the map
fn insert_skill_dir_recursive(source_dir: &Path, target_base: &Path, map: &mut std::collections::HashMap<PathBuf, String>)
{
    let Ok(entries) = fs::read_dir(source_dir)
    else
    {
        return;
    };

    for entry in entries.flatten()
    {
        let path = entry.path();

        if path.is_dir() == true
        {
            if let Some(dir_name) = path.file_name()
            {
                insert_skill_dir_recursive(&path, &target_base.join(dir_name), map);
            }
        }
        else if path.is_file() == true &&
            let Some(filename) = path.file_name() &&
            let Ok(content) = fs::read_to_string(&path)
        {
            let target_path = target_base.join(filename);
            map.insert(normalize_path(&target_path), content);
        }
    }
}

/// Resolves a target placeholder string to a workspace path
fn resolve_target(engine: &TemplateEngine, target: &str, workspace: &Path, userprofile: &Path) -> PathBuf
{
    engine.resolve_target(target, workspace, userprofile)
}

/// Generates a fresh AGENTS.md by merging the base template with all fragment sections
///
/// Reproduces what `init` would produce without actually installing anything.
/// When `mission` is provided it replaces any template-defined mission fragments.
fn generate_fresh_main(source_path: &Path, engine: &TemplateEngine, config: &TemplateConfig, lang: Option<&str>, mission: Option<&str>) -> Result<String>
{
    let mut content = fs::read_to_string(source_path)?;

    // Strip the template marker
    let marker_line = format!("{}\n", template_engine::TEMPLATE_MARKER);
    content = content.replace(&marker_line, "");

    // Collect fragments by category
    let mut fragments_by_category: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();

    // Principles fragments
    for entry in &config.principles
    {
        if entry.target.starts_with("$instructions") == true
        {
            let frag_path = engine.config_dir().join(&entry.source);
            if let Ok(frag) = fs::read_to_string(&frag_path)
            {
                fragments_by_category.entry("principles".to_string()).or_default().push(frag);
            }
        }
    }

    // Mission fragments — CLI override takes precedence over template fragments
    if let Some(m) = mission
    {
        fragments_by_category.entry("mission".to_string()).or_default().push(m.to_string());
    }
    else
    {
        for entry in &config.mission
        {
            if entry.target.starts_with("$instructions") == true
            {
                let frag_path = engine.config_dir().join(&entry.source);
                if let Ok(frag) = fs::read_to_string(&frag_path)
                {
                    fragments_by_category.entry("mission".to_string()).or_default().push(frag);
                }
            }
        }
    }

    // Language fragments
    if let Some(lang_name) = lang &&
        let Ok(files) = bom::resolve_language_files(lang_name, config)
    {
        for entry in &files
        {
            if entry.target.starts_with("$instructions") == true
            {
                let frag_path = engine.config_dir().join(&entry.source);
                if let Ok(frag) = fs::read_to_string(&frag_path)
                {
                    let category = entry.target.strip_prefix("$instructions/").unwrap_or("languages");
                    fragments_by_category.entry(category.to_string()).or_default().push(frag);
                }
            }
        }
    }

    // Integration fragments
    for int_config in config.integration.values()
    {
        for entry in &int_config.files
        {
            if entry.target.starts_with("$instructions") == true
            {
                let frag_path = engine.config_dir().join(&entry.source);
                if let Ok(frag) = fs::read_to_string(&frag_path)
                {
                    let category = entry.target.strip_prefix("$instructions/").unwrap_or("integration");
                    fragments_by_category.entry(category.to_string()).or_default().push(frag);
                }
            }
        }
    }

    // If no language installed, insert empty entry to clear the placeholder
    if lang.is_none() == true
    {
        fragments_by_category.entry("languages".to_string()).or_default();
    }

    // Merge fragments into content at insertion points
    for (category, contents) in &fragments_by_category
    {
        let insertion_point = format!("<!-- {{{}}} -->", category);
        let combined = contents.iter().map(|c| c.trim()).collect::<Vec<_>>().join("\n\n");
        let replacement = format!("<!-- {{{}}} -->\n\n{}", category, combined);
        content = content.replace(&insertion_point, &replacement);
    }

    Ok(content)
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

/// Returns the `.merged` sidecar path for a given file
fn sidecar_path(path: &Path) -> PathBuf
{
    let mut sidecar = path.as_os_str().to_owned();
    sidecar.push(".merged");
    PathBuf::from(sidecar)
}

/// Computes SHA-256 of a string (for comparing template content against stored hashes)
fn sha256_string(content: &str) -> String
{
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(content.as_bytes());
    format!("{:x}", hash)
}

/// Normalizes a path to its canonical form for map lookups
///
/// Falls back to the original path if canonicalization fails (e.g. file doesn't exist yet).
fn normalize_path(path: &Path) -> PathBuf
{
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
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

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn test_sidecar_path()
    {
        assert_eq!(sidecar_path(Path::new("/project/AGENTS.md")), PathBuf::from("/project/AGENTS.md.merged"));
        assert_eq!(sidecar_path(Path::new("relative.txt")), PathBuf::from("relative.txt.merged"));
    }

    #[test]
    fn test_sha256_string()
    {
        let sha = sha256_string("Hello, World!");
        assert_eq!(sha, "dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f");
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
    fn test_resolve_provider_auto_detects_from_env()
    {
        let result = TemplateManager::resolve_provider_and_model();

        if std::env::var("ANTHROPIC_API_KEY").is_ok() == true || std::env::var("OPENAI_API_KEY").is_ok() == true || std::env::var("MISTRAL_API_KEY").is_ok() == true
        {
            assert!(result.is_ok() == true);
        }
        else
        {
            assert!(result.is_err() == true);
            assert!(result.unwrap_err().to_string().contains("No LLM provider") == true);
        }
    }

    #[test]
    fn test_generate_fresh_main_strips_marker() -> anyhow::Result<()>
    {
        let dir = tempfile::tempdir()?;
        let source = dir.path().join("AGENTS.md");
        let marker = template_engine::TEMPLATE_MARKER;
        fs::write(&source, format!("{}\n# Title\n\n<!-- {{mission}} -->\n<!-- {{languages}} -->\n", marker))?;

        let config = TemplateConfig {
            version:     5,
            main:        None,
            agents:      std::collections::HashMap::new(),
            languages:   std::collections::HashMap::new(),
            shared:      std::collections::HashMap::new(),
            integration: std::collections::HashMap::new(),
            principles:  Vec::new(),
            mission:     Vec::new(),
            skills:      Vec::new()
        };

        let engine = TemplateEngine::new(dir.path());

        let result = generate_fresh_main(&source, &engine, &config, None, None)?;
        assert!(result.contains(marker) == false);
        assert!(result.contains("# Title") == true);
        Ok(())
    }

    #[test]
    fn test_insert_skill_dir_recursive_maps_files() -> anyhow::Result<()>
    {
        let dir = tempfile::tempdir()?;
        let source_dir = dir.path().join("skills/my-skill");
        fs::create_dir_all(&source_dir)?;
        fs::write(source_dir.join("SKILL.md"), "# My Skill")?;
        fs::write(source_dir.join("extra.md"), "Extra content")?;

        let target_base = dir.path().join("workspace/.cursor/skills/my-skill");
        fs::create_dir_all(&target_base)?;

        let mut map = std::collections::HashMap::new();
        insert_skill_dir_recursive(&source_dir, &target_base, &mut map);

        assert_eq!(map.len(), 2);
        let skill_md = normalize_path(&target_base.join("SKILL.md"));
        let extra_md = normalize_path(&target_base.join("extra.md"));
        assert_eq!(map.get(&skill_md).map(|s| s.as_str()), Some("# My Skill"));
        assert_eq!(map.get(&extra_md).map(|s| s.as_str()), Some("Extra content"));
        Ok(())
    }

    #[test]
    fn test_insert_skill_dir_recursive_handles_subdirs() -> anyhow::Result<()>
    {
        let dir = tempfile::tempdir()?;
        let source_dir = dir.path().join("skills/my-skill");
        let sub_dir = source_dir.join("scripts");
        fs::create_dir_all(&sub_dir)?;
        fs::write(source_dir.join("SKILL.md"), "# Skill")?;
        fs::write(sub_dir.join("helper.sh"), "#!/bin/bash")?;

        let target_base = dir.path().join("workspace/.agents/skills/my-skill");
        fs::create_dir_all(target_base.join("scripts"))?;

        let mut map = std::collections::HashMap::new();
        insert_skill_dir_recursive(&source_dir, &target_base, &mut map);

        assert_eq!(map.len(), 2);
        let helper = normalize_path(&target_base.join("scripts/helper.sh"));
        assert_eq!(map.get(&helper).map(|s| s.as_str()), Some("#!/bin/bash"));
        Ok(())
    }

    #[test]
    fn test_insert_skill_sources_skips_urls()
    {
        let dir = tempfile::tempdir().expect("tempdir");
        let engine = TemplateEngine::new(dir.path());
        let workspace = dir.path().join("workspace");
        let userprofile = dir.path().join("home");

        let skills = vec![bom::SkillDefinition { name: "remote-skill".into(), source: "https://github.com/user/repo".into() }];

        let mut map = std::collections::HashMap::new();
        insert_skill_sources(&engine, &skills, agent_defaults::CROSS_CLIENT_SKILL_DIR, &workspace, &userprofile, &mut map);

        assert!(map.is_empty() == true);
    }

    #[test]
    fn test_insert_skill_sources_includes_local_skills() -> anyhow::Result<()>
    {
        let dir = tempfile::tempdir()?;
        let skill_dir = dir.path().join("skills/git-workflow");
        fs::create_dir_all(&skill_dir)?;
        fs::write(skill_dir.join("SKILL.md"), "# Git Workflow")?;

        let workspace = dir.path().join("workspace");
        fs::create_dir_all(&workspace)?;
        let engine = TemplateEngine::new(dir.path());
        let userprofile = dir.path().join("home");

        let skills = vec![bom::SkillDefinition { name: "git-workflow".into(), source: "skills/git-workflow".into() }];

        let mut map = std::collections::HashMap::new();
        insert_skill_sources(&engine, &skills, agent_defaults::CROSS_CLIENT_SKILL_DIR, &workspace, &userprofile, &mut map);

        assert_eq!(map.len(), 1);
        let expected_target = workspace.join(".agents/skills/git-workflow/SKILL.md");
        let key = normalize_path(&expected_target);
        assert_eq!(map.get(&key).map(|s| s.as_str()), Some("# Git Workflow"));
        Ok(())
    }

    #[test]
    fn test_generate_fresh_main_with_fragments() -> anyhow::Result<()>
    {
        let dir = tempfile::tempdir()?;
        let source = dir.path().join("AGENTS.md");
        fs::write(&source, "# Title\n\n<!-- {mission} -->\n\n<!-- {principles} -->\n")?;

        let mission_frag = dir.path().join("mission.md");
        fs::write(&mission_frag, "## Mission\n\nBuild great things.")?;

        let principles_frag = dir.path().join("principles.md");
        fs::write(&principles_frag, "## Principles\n\nBe excellent.")?;

        let config = TemplateConfig {
            version:     5,
            main:        None,
            agents:      std::collections::HashMap::new(),
            languages:   std::collections::HashMap::new(),
            shared:      std::collections::HashMap::new(),
            integration: std::collections::HashMap::new(),
            principles:  vec![bom::FileMapping { source: "principles.md".into(), target: "$instructions/principles".into() }],
            mission:     vec![bom::FileMapping { source: "mission.md".into(), target: "$instructions/mission".into() }],
            skills:      Vec::new()
        };

        let engine = TemplateEngine::new(dir.path());

        let result = generate_fresh_main(&source, &engine, &config, None, None)?;
        assert!(result.contains("Build great things.") == true);
        assert!(result.contains("Be excellent.") == true);
        Ok(())
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
}
