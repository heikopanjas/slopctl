//! Smart (AI-assisted) features for init and doctor commands

use std::{fs, path::Path};

use owo_colors::OwoColorize;

use super::TemplateManager;
use crate::{
    Result,
    file_tracker::FileTracker,
    llm::{ChatMessage, LlmClient, Provider}
};

/// Kind of issue detected by the smart doctor analysis
#[derive(Debug)]
pub enum SmartIssueKind
{
    /// Two or more instructions that directly conflict with each other
    Contradiction,
    /// A reference to a tool, version, or URL that may be outdated
    StaleReference,
    /// An instruction that is ambiguous or difficult to follow correctly
    UnclearInstruction
}

/// An issue detected by AI-assisted linting of instruction files
#[derive(Debug)]
pub struct SmartIssue
{
    pub kind:        SmartIssueKind,
    pub description: String
}

/// Gathers workspace context for LLM prompts
///
/// Collects a top-level directory listing, README.md content (first 2000 chars),
/// and the first recognized project manifest (Cargo.toml, package.json, pyproject.toml,
/// go.mod, pom.xml; first 500 chars).
///
/// # Arguments
///
/// * `workspace` - Path to the workspace root directory
pub fn collect_workspace_context(workspace: &Path) -> String
{
    let mut parts: Vec<String> = Vec::new();

    // Top-level directory listing
    if let Ok(entries) = fs::read_dir(workspace)
    {
        let mut names: Vec<String> = entries.filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().to_string()).collect();
        names.sort();
        parts.push(format!("## Workspace structure\n{}", names.join("\n")));
    }

    // README.md
    let readme_path = workspace.join("README.md");
    if readme_path.exists() == true &&
        let Ok(content) = fs::read_to_string(&readme_path)
    {
        let excerpt = if content.len() > 2000
        {
            format!("{}...", &content[..2000])
        }
        else
        {
            content
        };
        parts.push(format!("## README.md\n{}", excerpt.trim()));
    }

    // Project manifest — first match wins
    let manifests = ["Cargo.toml", "package.json", "pyproject.toml", "go.mod", "pom.xml"];
    for manifest in manifests
    {
        let path = workspace.join(manifest);
        if path.exists() == true &&
            let Ok(content) = fs::read_to_string(&path)
        {
            let excerpt = if content.len() > 500
            {
                format!("{}...", &content[..500])
            }
            else
            {
                content
            };
            parts.push(format!("## {}\n{}", manifest, excerpt.trim()));
            break;
        }
    }

    parts.join("\n\n")
}

/// Parses the LLM JSON response into a list of SmartIssues
///
/// Extracts the JSON array from the response (tolerates surrounding prose),
/// deserializes each entry, and maps kind strings to `SmartIssueKind` variants.
/// Entries with unrecognized kinds or empty descriptions are silently skipped.
fn parse_smart_issues(response: &str) -> Result<Vec<SmartIssue>>
{
    let start = response.find('[').unwrap_or(0);
    let end = response.rfind(']').map(|i| i + 1).unwrap_or(response.len());
    let json_slice = &response[start..end];

    let raw: Vec<serde_json::Value> = serde_json::from_str(json_slice).map_err(|e| anyhow::anyhow!("Failed to parse smart issues JSON: {}", e))?;

    let mut issues: Vec<SmartIssue> = Vec::new();
    for item in raw
    {
        let kind_str = item["kind"].as_str().unwrap_or("").to_lowercase();
        let description = item["description"].as_str().unwrap_or("").to_string();

        let kind = match kind_str.as_str()
        {
            | "contradiction" => SmartIssueKind::Contradiction,
            | "stale_reference" => SmartIssueKind::StaleReference,
            | "unclear_instruction" => SmartIssueKind::UnclearInstruction,
            | _ => continue
        };

        if description.is_empty() == false
        {
            issues.push(SmartIssue { kind, description });
        }
    }

    Ok(issues)
}

impl TemplateManager
{
    /// Generates a mission statement for AGENTS.md using an LLM
    ///
    /// Collects workspace context (README, project manifest, directory listing)
    /// and asks the LLM to produce a concise 2-4 sentence mission statement.
    /// Provider and model are resolved using the same priority chain as the merge
    /// command: CLI > config `merge.provider`/`merge.model` > env auto-detect.
    ///
    /// # Arguments
    ///
    /// * `cli_provider` - CLI-supplied provider override (None = use config/env)
    /// * `cli_model` - CLI-supplied model override (None = use config/provider default)
    ///
    /// # Errors
    ///
    /// Returns an error if provider resolution or the LLM call fails
    pub fn generate_smart_mission(&self, cli_provider: Option<&str>, cli_model: Option<&str>) -> Result<String>
    {
        let (provider_name, model_name) = Self::resolve_provider_and_model(cli_provider, cli_model)?;
        let provider = Provider::from_name(&provider_name)?;
        let client = LlmClient::new(provider, model_name.as_deref())?;

        let workspace = std::env::current_dir()?;
        let context = collect_workspace_context(&workspace);

        println!("{} Generating mission statement from workspace context...", "→".blue());

        let messages = vec![
            ChatMessage {
                role:    "system".to_string(),
                content: "You are a technical writer creating mission statements for AI coding assistants. Given a workspace description, write a single concise \
                          paragraph (2-4 sentences) for an AI coding assistant mission statement describing what this project does and what the assistant should \
                          help with. Output only the paragraph text — no headers, no preamble, no markdown formatting."
                    .to_string()
            },
            ChatMessage { role: "user".to_string(), content: format!("Generate a mission statement for this workspace:\n\n{}", context) },
        ];

        let response = client.chat(&messages)?;
        Ok(response.content.trim().to_string())
    }

    /// Runs AI-assisted linting on the installed AGENTS.md
    ///
    /// Finds the installed AGENTS.md for the current workspace, reads its content,
    /// and asks the LLM to identify contradictions, stale references, and unclear
    /// instructions. Returns a list of detected issues.
    ///
    /// Provider and model are resolved using the same priority chain as the merge
    /// command: CLI > config `merge.provider`/`merge.model` > env auto-detect.
    ///
    /// # Arguments
    ///
    /// * `cli_provider` - CLI-supplied provider override (None = use config/env)
    /// * `cli_model` - CLI-supplied model override (None = use config/provider default)
    ///
    /// # Errors
    ///
    /// Returns an error if provider resolution, file reading, or the LLM call fails
    pub fn smart_doctor(&self, cli_provider: Option<&str>, cli_model: Option<&str>) -> Result<Vec<SmartIssue>>
    {
        let (provider_name, model_name) = Self::resolve_provider_and_model(cli_provider, cli_model)?;
        let provider = Provider::from_name(&provider_name)?;
        let client = LlmClient::new(provider, model_name.as_deref())?;

        let workspace = std::env::current_dir()?;
        let tracker = FileTracker::new(&self.config_dir)?;

        // Find the installed AGENTS.md (category "main") or fall back to workspace root
        let agents_md_path = tracker
            .get_workspace_entries(&workspace)
            .into_iter()
            .find(|(_, meta)| meta.category == "main")
            .map(|(path, _)| path)
            .unwrap_or_else(|| workspace.join("AGENTS.md"));

        require!(agents_md_path.exists() == true, Err(anyhow::anyhow!("No AGENTS.md found in workspace. Run 'slopctl init' first.")));

        let content = fs::read_to_string(&agents_md_path)?;

        println!("{} Analyzing AGENTS.md with AI...", "→".blue());

        let messages = vec![
            ChatMessage {
                role:    "system".to_string(),
                content: "You are a technical editor reviewing an AI coding assistant instruction file (AGENTS.md). Identify issues in exactly three categories: \
                          \"contradiction\" (two or more instructions that directly conflict), \"stale_reference\" (a tool name, version number, or URL that appears \
                          outdated), \"unclear_instruction\" (an instruction that is ambiguous or interpretable in multiple ways). Respond with a JSON array only — \
                          no other text. Each element must have exactly two string fields: \"kind\" (one of: contradiction, stale_reference, unclear_instruction) \
                          and \"description\" (a short explanation under 120 characters). If no issues are found, respond with an empty array: []"
                    .to_string()
            },
            ChatMessage { role: "user".to_string(), content: format!("Review this AGENTS.md for issues:\n\n{}", content) },
        ];

        let response = client.chat(&messages)?;
        parse_smart_issues(&response.content)
    }
}

#[cfg(test)]
mod tests
{
    use std::fs;

    use super::*;

    #[test]
    fn test_collect_workspace_context_readme() -> anyhow::Result<()>
    {
        let dir = tempfile::TempDir::new()?;
        fs::write(dir.path().join("README.md"), "# My Project\nA great CLI tool.")?;

        let context = collect_workspace_context(dir.path());
        assert!(context.contains("README.md") == true);
        assert!(context.contains("My Project") == true);
        Ok(())
    }

    #[test]
    fn test_collect_workspace_context_no_readme() -> anyhow::Result<()>
    {
        let dir = tempfile::TempDir::new()?;
        let context = collect_workspace_context(dir.path());
        assert!(context.contains("Workspace structure") == true);
        Ok(())
    }

    #[test]
    fn test_collect_workspace_context_manifest() -> anyhow::Result<()>
    {
        let dir = tempfile::TempDir::new()?;
        fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"myapp\"\nversion = \"1.0.0\"")?;

        let context = collect_workspace_context(dir.path());
        assert!(context.contains("Cargo.toml") == true);
        assert!(context.contains("myapp") == true);
        Ok(())
    }

    #[test]
    fn test_collect_workspace_context_readme_truncated() -> anyhow::Result<()>
    {
        let dir = tempfile::TempDir::new()?;
        let long_readme = "x".repeat(3000);
        fs::write(dir.path().join("README.md"), &long_readme)?;

        let context = collect_workspace_context(dir.path());
        assert!(context.contains("README.md") == true);
        assert!(context.contains("...") == true);
        Ok(())
    }

    #[test]
    fn test_parse_smart_issues_valid_json() -> anyhow::Result<()>
    {
        let json = r#"[
            {"kind": "contradiction", "description": "Rule A says X but Rule B says not X"},
            {"kind": "unclear_instruction", "description": "What does fully optimized mean exactly?"}
        ]"#;

        let issues = parse_smart_issues(json)?;
        assert!(issues.len() == 2);
        assert!(matches!(issues[0].kind, SmartIssueKind::Contradiction) == true);
        assert!(matches!(issues[1].kind, SmartIssueKind::UnclearInstruction) == true);
        Ok(())
    }

    #[test]
    fn test_parse_smart_issues_empty_array() -> anyhow::Result<()>
    {
        let issues = parse_smart_issues("[]")?;
        assert!(issues.is_empty() == true);
        Ok(())
    }

    #[test]
    fn test_parse_smart_issues_with_preamble() -> anyhow::Result<()>
    {
        let response = "Here are the issues I found:\n[{\"kind\": \"stale_reference\", \"description\": \"References deprecated API v1\"}]\nEnd.";

        let issues = parse_smart_issues(response)?;
        assert!(issues.len() == 1);
        assert!(matches!(issues[0].kind, SmartIssueKind::StaleReference) == true);
        Ok(())
    }

    #[test]
    fn test_parse_smart_issues_unknown_kind_skipped() -> anyhow::Result<()>
    {
        let json = r#"[
            {"kind": "unknown_type", "description": "Some unknown issue"},
            {"kind": "contradiction", "description": "A real contradiction"}
        ]"#;

        let issues = parse_smart_issues(json)?;
        assert!(issues.len() == 1);
        assert!(matches!(issues[0].kind, SmartIssueKind::Contradiction) == true);
        Ok(())
    }
}
