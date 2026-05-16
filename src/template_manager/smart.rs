//! Smart (AI-assisted) features for init and doctor commands

use std::fs;

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
    /// Runs AI-assisted linting on the installed AGENTS.md
    ///
    /// Finds the installed AGENTS.md for the current workspace, reads its content,
    /// and asks the LLM to identify contradictions, stale references, and unclear
    /// instructions. Returns a list of detected issues.
    ///
    /// Provider and model are resolved from config (`merge.provider`/`merge.model`)
    /// or auto-detected from environment API keys.
    ///
    /// # Errors
    ///
    /// Returns an error if provider resolution, file reading, or the LLM call fails
    pub fn smart_doctor(&self) -> Result<Vec<SmartIssue>>
    {
        let (provider_name, model_name) = Self::resolve_provider_and_model()?;
        let provider = Provider::from_name(&provider_name)?;
        let client = LlmClient::new(provider, model_name.as_deref())?;

        let workspace = std::env::current_dir()?;
        let _ = self.try_migrate_tracker(&workspace);
        let tracker = FileTracker::new(&workspace)?;

        // Find the installed AGENTS.md (category "main") or fall back to workspace root
        let agents_md_path = tracker
            .get_entries()
            .into_iter()
            .find(|(_, meta)| meta.category == "main")
            .map(|(rel_path, _)| workspace.join(rel_path))
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
    use super::*;

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

    #[test]
    fn test_smart_doctor_returns_parsed_issues_via_hook() -> anyhow::Result<()>
    {
        let _cwd = crate::template_manager::cwd_test_guard();
        let workspace = tempfile::TempDir::new()?;
        let config_dir = tempfile::TempDir::new()?;
        std::env::set_current_dir(workspace.path())?;

        std::fs::write(workspace.path().join("AGENTS.md"), "# Project\n## Rules\nDo X.\nNever do X.\n")?;
        std::fs::write(
            config_dir.path().join(crate::agent_defaults::AGENT_DEFAULTS_FILE),
            "version: 1\nagents:\n  - name: bogus\n    markers:\n      - .bogus\n    prompt_dir: '$workspace/.bogus/prompts'\n    skill_dir: \
             '$workspace/.bogus/skills'\n    reads_cross_client_skills: false\n"
        )?;

        let mut tracker = crate::FileTracker::new(workspace.path())?;
        tracker.record_installation(
            &workspace.path().join("AGENTS.md"),
            "sha".into(),
            5,
            crate::file_tracker::LANG_NONE.into(),
            crate::file_tracker::AGENT_ALL.into(),
            "main".into()
        );
        tracker.save()?;

        // Write workspace config with merge.provider = ollama (no API key required)
        let slopctl_dir = workspace.path().join(".slopctl");
        std::fs::create_dir_all(&slopctl_dir)?;
        std::fs::write(slopctl_dir.join("config.yml"), "merge:\n  provider: ollama\n")?;

        let _hook = crate::llm::set_chat_test_hook(Box::new(|_msgs| {
            Ok(crate::llm::ChatResponse {
                content:       r#"[{"kind":"contradiction","description":"Do X conflicts with Never do X"}]"#.to_string(),
                input_tokens:  Some(50),
                output_tokens: Some(20),
                stop_reason:   Some("stop".to_string())
            })
        }));

        let manager = crate::TemplateManager { config_dir: config_dir.path().to_path_buf() };
        let issues = manager.smart_doctor()?;

        assert_eq!(issues.len(), 1);
        assert!(matches!(issues[0].kind, SmartIssueKind::Contradiction) == true);
        assert!(issues[0].description.contains("Do X") == true);
        Ok(())
    }

    #[test]
    fn test_smart_doctor_handles_empty_response_via_hook() -> anyhow::Result<()>
    {
        let _cwd = crate::template_manager::cwd_test_guard();
        let workspace = tempfile::TempDir::new()?;
        let config_dir = tempfile::TempDir::new()?;
        std::env::set_current_dir(workspace.path())?;

        std::fs::write(workspace.path().join("AGENTS.md"), "# Clean file\n")?;
        std::fs::write(
            config_dir.path().join(crate::agent_defaults::AGENT_DEFAULTS_FILE),
            "version: 1\nagents:\n  - name: bogus\n    markers:\n      - .bogus\n    prompt_dir: '$workspace/.bogus/prompts'\n    skill_dir: \
             '$workspace/.bogus/skills'\n    reads_cross_client_skills: false\n"
        )?;

        let mut tracker = crate::FileTracker::new(workspace.path())?;
        tracker.record_installation(
            &workspace.path().join("AGENTS.md"),
            "sha".into(),
            5,
            crate::file_tracker::LANG_NONE.into(),
            crate::file_tracker::AGENT_ALL.into(),
            "main".into()
        );
        tracker.save()?;

        let slopctl_dir = workspace.path().join(".slopctl");
        std::fs::create_dir_all(&slopctl_dir)?;
        std::fs::write(slopctl_dir.join("config.yml"), "merge:\n  provider: ollama\n")?;

        let _hook = crate::llm::set_chat_test_hook(Box::new(|_msgs| {
            Ok(crate::llm::ChatResponse { content: "[]".to_string(), input_tokens: Some(30), output_tokens: Some(5), stop_reason: Some("stop".to_string()) })
        }));

        let manager = crate::TemplateManager { config_dir: config_dir.path().to_path_buf() };
        let issues = manager.smart_doctor()?;

        assert!(issues.is_empty() == true, "clean file must produce no issues");
        Ok(())
    }
}
