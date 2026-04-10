//! AI-assisted merge of customized files with updated templates

use std::{
    fs,
    path::{Path, PathBuf}
};

use owo_colors::OwoColorize;

use super::TemplateManager;
use crate::{
    Config, Result,
    bom::{self, TemplateConfig},
    file_tracker::{FileStatus, FileTracker},
    llm::{ChatMessage, LlmClient, Provider},
    template_engine::{self, TemplateEngine}
};

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
You are a file merge assistant that combines user-customized configuration files \
with updated templates. Follow these rules strictly:

1. PRESERVE all user customizations: added sections, modified content, custom notes, \
   project-specific information, and any user-authored text.
2. INCORPORATE all new content from the updated template: new sections, updated \
   instructions, structural changes, and new conventions.
3. When both files define the same section, prefer the user's version but integrate \
   any genuinely new information from the template.
4. Maintain the overall document structure and formatting of the user's file.
5. Do NOT add commentary, explanations, or merge markers. Output ONLY the merged \
   file content, ready to save.
6. If the template introduces a new section that the user's file does not have, \
   insert it in a natural location that matches the template's ordering.
7. Do NOT remove any user content unless it directly contradicts a template change \
   (in which case prefer the template's factual updates but keep user customizations).";

impl TemplateManager
{
    /// Lists available models from the selected LLM provider
    ///
    /// Resolves the provider (CLI > config > env auto-detect), queries its
    /// models API, and prints the results. The currently configured default
    /// model is marked in the output.
    ///
    /// # Arguments
    ///
    /// * `provider` - CLI override for LLM provider name
    /// * `model` - CLI override for model name (used only to show the active default)
    ///
    /// # Errors
    ///
    /// Returns an error if provider resolution or the API call fails
    pub fn list_models(&self, provider: Option<&str>, model: Option<&str>) -> Result<()>
    {
        let (provider_name, model_name) = Self::resolve_provider_and_model(provider, model)?;
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
    /// has changed since installation, calls an LLM to produce a merged version
    /// and writes it as a `.merged` sidecar file.
    ///
    /// # Arguments
    ///
    /// * `provider` - CLI override for LLM provider name (falls back to config)
    /// * `model` - CLI override for model name (falls back to config, then provider default)
    /// * `dry_run` - If true, shows what would be merged without calling the LLM
    ///
    /// # Errors
    ///
    /// Returns an error if provider resolution fails, LLM calls fail, or file I/O fails
    pub fn merge(&self, provider: Option<&str>, model: Option<&str>, dry_run: bool) -> Result<()>
    {
        let (provider_name, model_name) = Self::resolve_provider_and_model(provider, model)?;
        let provider_enum = Provider::from_name(&provider_name)?;

        println!("{} Using {} / {}", "→".blue(), provider_name.green(), model_name.as_deref().unwrap_or(provider_enum.default_model()).green());

        let candidates = self.find_merge_candidates()?;

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

        for candidate in &candidates
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

            if dry_run == true
            {
                println!("  {} Would merge: {}", "→".blue(), candidate.display_name.yellow());
                continue;
            }

            print!("  {} Merging {}... ", "→".blue(), candidate.display_name.yellow());
            std::io::Write::flush(&mut std::io::stdout())?;

            let client = LlmClient::new(provider_enum.clone(), model_name.as_deref())?;
            let user_content = fs::read_to_string(&candidate.workspace_path)?;

            let messages = build_merge_messages(&user_content, &candidate.template_content);
            let merged = client.chat(&messages)?;

            fs::write(&sidecar, &merged)?;
            println!("{} wrote {}", "✓".green(), rel_sidecar.display().to_string().yellow());
        }

        if dry_run == true
        {
            println!();
            println!("{} Dry run complete. No files were modified.", "✓".green());
        }
        else
        {
            println!();
            println!("{} Review .merged files, then replace originals when satisfied", "→".blue());
        }

        Ok(())
    }

    /// Resolves the LLM provider and model from CLI args, config, env, or error
    ///
    /// Priority: CLI `--provider` > config `merge.provider` > auto-detect from
    /// environment API keys (`ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, `MISTRAL_API_KEY`).
    /// Model: CLI `--model` > config `merge.model` > None (provider default used later).
    fn resolve_provider_and_model(cli_provider: Option<&str>, cli_model: Option<&str>) -> Result<(String, Option<String>)>
    {
        let config = Config::load().ok();

        let provider = if let Some(p) = cli_provider
        {
            p.to_string()
        }
        else if let Some(ref c) = config &&
            let Some(p) = c.get("merge.provider")
        {
            p
        }
        else if let Some(detected) = Provider::detect_from_env()
        {
            println!("{} Auto-detected provider from environment: {}", "→".blue(), detected.name().green());
            detected.name().to_string()
        }
        else
        {
            return Err(anyhow::anyhow!(
                "No LLM provider specified and none auto-detected.\nSet an API key env var (OPENAI_API_KEY, ANTHROPIC_API_KEY, MISTRAL_API_KEY),\nor configure: \
                 vibe-cop config merge.provider openai\nor pass --provider on the command line.\nSupported: openai, anthropic, ollama, mistral"
            ));
        };

        let model = if let Some(m) = cli_model
        {
            Some(m.to_string())
        }
        else if let Some(ref c) = config
        {
            c.get("merge.model")
        }
        else
        {
            None
        };

        Ok((provider, model))
    }

    /// Finds workspace files that need AI-assisted merging
    ///
    /// A file is a merge candidate when:
    /// 1. It is tracked by FileTracker for this workspace
    /// 2. The user has modified it since installation (SHA changed)
    /// 3. The corresponding template source has also changed
    /// 4. It is not a "skill" category (skills are managed independently)
    fn find_merge_candidates(&self) -> Result<Vec<MergeCandidate>>
    {
        let workspace = std::env::current_dir()?;
        let tracker = FileTracker::new(&self.config_dir)?;
        let entries = tracker.get_workspace_entries(&workspace);

        require!(entries.is_empty() == false, Ok(Vec::new()));

        let config = template_engine::load_template_config(&self.config_dir)?;
        let engine = TemplateEngine::new(&self.config_dir);
        let userprofile = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;

        let target_source_map = build_target_source_map(&engine, &config, &workspace, &userprofile, &tracker)?;

        let mut candidates = Vec::new();

        for (path, metadata) in &entries
        {
            if metadata.category == "skill"
            {
                continue;
            }

            if tracker.check_modification(path)? != FileStatus::Modified
            {
                continue;
            }

            if let Some(template_content) = target_source_map.get(path)
            {
                let template_sha = sha256_string(template_content);

                // Both user AND template changed since install
                if template_sha != metadata.original_sha
                {
                    let display = path.strip_prefix(&workspace).unwrap_or(path).display().to_string();
                    candidates.push(MergeCandidate { workspace_path: path.clone(), template_content: template_content.clone(), display_name: display });
                }
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
fn build_target_source_map(
    engine: &TemplateEngine, config: &TemplateConfig, workspace: &Path, userprofile: &Path, tracker: &FileTracker
) -> Result<std::collections::HashMap<PathBuf, String>>
{
    let mut map = std::collections::HashMap::new();

    // Detect installed language from FileTracker
    let installed_lang = tracker.get_installed_language_for_workspace(workspace);

    // Main template (AGENTS.md): generate fresh merged content
    if let Some(ref main_config) = config.main
    {
        let source_path = engine.config_dir().join(&main_config.source);
        let target_path = resolve_target(engine, &main_config.target, workspace, userprofile);

        if source_path.exists() == true
        {
            let fresh_content = generate_fresh_main(&source_path, engine, config, installed_lang.as_deref())?;
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

/// Resolves a target placeholder string to a workspace path
fn resolve_target(engine: &TemplateEngine, target: &str, workspace: &Path, userprofile: &Path) -> PathBuf
{
    engine.resolve_target(target, workspace, userprofile)
}

/// Generates a fresh AGENTS.md by merging the base template with all fragment sections
///
/// Reproduces what `init` would produce without actually installing anything.
fn generate_fresh_main(source_path: &Path, engine: &TemplateEngine, config: &TemplateConfig, lang: Option<&str>) -> Result<String>
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

    // Mission fragments
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
    fn test_resolve_provider_cli_overrides_config() -> anyhow::Result<()>
    {
        let (provider, model) = TemplateManager::resolve_provider_and_model(Some("anthropic"), Some("claude-haiku"))?;
        assert_eq!(provider, "anthropic");
        assert_eq!(model, Some("claude-haiku".to_string()));
        Ok(())
    }

    #[test]
    fn test_resolve_provider_auto_detects_from_env()
    {
        // When no CLI or config provider is set, auto-detection kicks in.
        // This test verifies the function succeeds or fails depending on env state;
        // we test the explicit CLI override path separately above.
        let result = TemplateManager::resolve_provider_and_model(None, None);

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

        let result = generate_fresh_main(&source, &engine, &config, None)?;
        assert!(result.contains(marker) == false);
        assert!(result.contains("# Title") == true);
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

        let result = generate_fresh_main(&source, &engine, &config, None)?;
        assert!(result.contains("Build great things.") == true);
        assert!(result.contains("Be excellent.") == true);
        Ok(())
    }
}
