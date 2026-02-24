//! Default filesystem paths and conventions for known AI coding agents
//!
//! Provides a registry of agent-specific paths for instruction files,
//! prompt/command directories, and skill directories. Used by the install
//! flow to resolve where skills and other agent-agnostic artifacts go.

use std::path::Path;

/// Placeholder for the project workspace root directory
pub const PLACEHOLDER_WORKSPACE: &str = "$workspace";

/// Placeholder for the user profile/home directory
pub const PLACEHOLDER_USERPROFILE: &str = "$userprofile";

/// An agent-specific instruction file with its conventional location
#[derive(Debug, Clone)]
pub struct InstructionFile
{
    /// Relative path within the placeholder root (e.g. `CLAUDE.md`, `.cursorrules`)
    pub path:        &'static str,
    /// Root placeholder: `$workspace` or `$userprofile`
    pub placeholder: &'static str
}

/// Default filesystem conventions for a known AI coding agent
#[derive(Debug, Clone)]
pub struct AgentDefaults
{
    /// Agent identifier (e.g. "cursor", "claude")
    pub name:              &'static str,
    /// Agent-specific instruction files (e.g. `.cursorrules`, `CLAUDE.md`)
    pub instruction_files: &'static [InstructionFile],
    /// Directory for agent prompts/commands, with placeholder prefix
    pub prompt_dir:        &'static str,
    /// Directory for agent skills, with placeholder prefix
    pub skill_dir:         &'static str
}

const CURSOR_INSTRUCTIONS: &[InstructionFile] = &[InstructionFile { path: ".cursorrules", placeholder: PLACEHOLDER_WORKSPACE }];

const CLAUDE_INSTRUCTIONS: &[InstructionFile] = &[InstructionFile { path: "CLAUDE.md", placeholder: PLACEHOLDER_WORKSPACE }];

const CODEX_INSTRUCTIONS: &[InstructionFile] = &[InstructionFile { path: "CODEX.md", placeholder: PLACEHOLDER_WORKSPACE }];

const COPILOT_INSTRUCTIONS: &[InstructionFile] = &[InstructionFile { path: ".github/copilot-instructions.md", placeholder: PLACEHOLDER_WORKSPACE }];

/// Built-in registry of known agents and their filesystem conventions
const KNOWN_AGENTS: &[AgentDefaults] = &[
    AgentDefaults {
        name:              "cursor",
        instruction_files: CURSOR_INSTRUCTIONS,
        prompt_dir:        "$workspace/.cursor/commands",
        skill_dir:         "$workspace/.cursor/skills"
    },
    AgentDefaults {
        name:              "claude",
        instruction_files: CLAUDE_INSTRUCTIONS,
        prompt_dir:        "$workspace/.claude/commands",
        skill_dir:         "$workspace/.claude/skills"
    },
    AgentDefaults {
        name:              "codex",
        instruction_files: CODEX_INSTRUCTIONS,
        prompt_dir:        "$userprofile/.codex/prompts",
        skill_dir:         "$userprofile/.codex/skills"
    },
    AgentDefaults {
        name:              "copilot",
        instruction_files: COPILOT_INSTRUCTIONS,
        prompt_dir:        "$workspace/.github/prompts",
        skill_dir:         "$workspace/.github/skills"
    }
];

/// Look up defaults for an agent by name
pub fn get_defaults(agent: &str) -> Option<&'static AgentDefaults>
{
    KNOWN_AGENTS.iter().find(|a| a.name == agent)
}

/// Get the skill installation directory for an agent
///
/// Returns the raw placeholder path (e.g. `$workspace/.cursor/skills`).
/// Caller must resolve the placeholder to an actual path.
pub fn get_skill_dir(agent: &str) -> Option<&'static str>
{
    get_defaults(agent).map(|d| d.skill_dir)
}

/// List all known agent names
pub fn known_agents() -> Vec<&'static str>
{
    KNOWN_AGENTS.iter().map(|a| a.name).collect()
}

/// Detect which agent is installed in a workspace by checking for known files
///
/// Scans the workspace for agent-specific instruction files.
/// Returns the first agent whose files are found.
///
/// # Arguments
///
/// * `workspace` - Path to the project workspace root
pub fn detect_installed_agent(workspace: &Path) -> Option<String>
{
    for agent in KNOWN_AGENTS
    {
        for instr in agent.instruction_files
        {
            if instr.placeholder == PLACEHOLDER_WORKSPACE
            {
                let file_path = workspace.join(instr.path);
                if file_path.exists() == true
                {
                    return Some(agent.name.to_string());
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn test_get_defaults_known_agent()
    {
        let defaults = get_defaults("cursor");
        assert!(defaults.is_some());
        let defaults = defaults.unwrap();
        assert_eq!(defaults.name, "cursor");
        assert_eq!(defaults.skill_dir, "$workspace/.cursor/skills");
        assert_eq!(defaults.prompt_dir, "$workspace/.cursor/commands");
    }

    #[test]
    fn test_get_defaults_unknown_agent()
    {
        assert!(get_defaults("unknown-agent").is_none());
    }

    #[test]
    fn test_get_skill_dir()
    {
        assert_eq!(get_skill_dir("claude"), Some("$workspace/.claude/skills"));
        assert_eq!(get_skill_dir("codex"), Some("$userprofile/.codex/skills"));
        assert_eq!(get_skill_dir("nonexistent"), None);
    }

    #[test]
    fn test_known_agents_contains_all()
    {
        let agents = known_agents();
        assert!(agents.contains(&"cursor"));
        assert!(agents.contains(&"claude"));
        assert!(agents.contains(&"codex"));
        assert!(agents.contains(&"copilot"));
        assert_eq!(agents.len(), 4);
    }

    #[test]
    fn test_detect_installed_agent()
    {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let workspace = temp_dir.path();

        // No agent files -> None
        assert!(detect_installed_agent(workspace).is_none());

        // Create .cursorrules -> detects cursor
        std::fs::write(workspace.join(".cursorrules"), b"test").unwrap();
        assert_eq!(detect_installed_agent(workspace), Some("cursor".to_string()));
    }

    #[test]
    fn test_detect_installed_agent_claude()
    {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let workspace = temp_dir.path();

        std::fs::write(workspace.join("CLAUDE.md"), b"test").unwrap();
        assert_eq!(detect_installed_agent(workspace), Some("claude".to_string()));
    }
}
