//! Default filesystem paths and conventions for known AI coding agents
//!
//! Provides a registry of agent-specific paths for instruction files,
//! prompt/command directories, and skill directories. Used by the install
//! flow to resolve where skills and other agent-agnostic artifacts go.

use std::path::{Path, PathBuf};

/// Placeholder for the project workspace root directory
pub const PLACEHOLDER_WORKSPACE: &str = "$workspace";

/// Placeholder for the user profile/home directory
pub const PLACEHOLDER_USERPROFILE: &str = "$userprofile";

/// Cross-client skill directory per the agentskills.io specification.
/// All compliant agents scan this path alongside their native skill directories.
pub const CROSS_CLIENT_SKILL_DIR: &str = "$workspace/.agents/skills";

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

/// Resolve a placeholder path to an absolute filesystem path
///
/// Replaces `$workspace` and `$userprofile` prefixes with the supplied paths.
/// If neither prefix matches the string is treated as a literal path.
///
/// # Arguments
///
/// * `raw` - Placeholder path (e.g. `$workspace/.cursor/skills`)
/// * `workspace` - Absolute path to the project workspace root
/// * `userprofile` - Absolute path to the user home directory
pub fn resolve_placeholder_path(raw: &str, workspace: &Path, userprofile: &Path) -> PathBuf
{
    if raw.starts_with(PLACEHOLDER_WORKSPACE) == true
    {
        let suffix = raw[PLACEHOLDER_WORKSPACE.len()..].trim_start_matches('/').trim_start_matches('\\');
        return workspace.join(suffix);
    }
    if raw.starts_with(PLACEHOLDER_USERPROFILE) == true
    {
        let suffix = raw[PLACEHOLDER_USERPROFILE.len()..].trim_start_matches('/').trim_start_matches('\\');
        return userprofile.join(suffix);
    }
    PathBuf::from(raw)
}

/// Return all skill directories to search for a given workspace
///
/// Includes the skill directory of every installed agent (detected via their
/// instruction files) and always appends the cross-client `.agents/skills`
/// directory. Duplicates are removed before returning.
///
/// # Arguments
///
/// * `workspace` - Absolute path to the project workspace root
/// * `userprofile` - Absolute path to the user home directory
pub fn get_all_skill_search_dirs(workspace: &Path, userprofile: &Path) -> Vec<PathBuf>
{
    let mut dirs: Vec<PathBuf> = detect_all_installed_agents(workspace)
        .iter()
        .filter_map(|agent| get_skill_dir(agent).map(|raw| resolve_placeholder_path(raw, workspace, userprofile)))
        .collect();

    let cross_client = resolve_placeholder_path(CROSS_CLIENT_SKILL_DIR, workspace, userprofile);
    if dirs.contains(&cross_client) == false
    {
        dirs.push(cross_client);
    }

    dirs
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

/// Detect all agents installed in a workspace by checking for known files
///
/// Scans the workspace for agent-specific instruction files.
/// Returns every agent whose files are found (may be empty).
///
/// # Arguments
///
/// * `workspace` - Path to the project workspace root
pub fn detect_all_installed_agents(workspace: &Path) -> Vec<String>
{
    let mut found = Vec::new();
    for agent in KNOWN_AGENTS
    {
        for instr in agent.instruction_files
        {
            if instr.placeholder == PLACEHOLDER_WORKSPACE
            {
                let file_path = workspace.join(instr.path);
                if file_path.exists() == true
                {
                    found.push(agent.name.to_string());
                    break;
                }
            }
        }
    }
    found
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn test_get_defaults_known_agent() -> anyhow::Result<()>
    {
        let defaults = get_defaults("cursor");
        assert!(defaults.is_some());
        let defaults = defaults.ok_or_else(|| anyhow::anyhow!("expected defaults"))?;
        assert_eq!(defaults.name, "cursor");
        assert_eq!(defaults.skill_dir, "$workspace/.cursor/skills");
        assert_eq!(defaults.prompt_dir, "$workspace/.cursor/commands");
        Ok(())
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
    fn test_detect_installed_agent() -> anyhow::Result<()>
    {
        let temp_dir = tempfile::TempDir::new()?;
        let workspace = temp_dir.path();

        // No agent files -> None
        assert!(detect_installed_agent(workspace).is_none());

        // Create .cursorrules -> detects cursor
        std::fs::write(workspace.join(".cursorrules"), b"test")?;
        assert_eq!(detect_installed_agent(workspace), Some("cursor".to_string()));
        Ok(())
    }

    #[test]
    fn test_detect_installed_agent_claude() -> anyhow::Result<()>
    {
        let temp_dir = tempfile::TempDir::new()?;
        let workspace = temp_dir.path();

        std::fs::write(workspace.join("CLAUDE.md"), b"test")?;
        assert_eq!(detect_installed_agent(workspace), Some("claude".to_string()));
        Ok(())
    }

    #[test]
    fn test_cross_client_skill_dir_uses_workspace_placeholder()
    {
        assert!(CROSS_CLIENT_SKILL_DIR.starts_with("$workspace"));
        assert!(CROSS_CLIENT_SKILL_DIR.contains(".agents/skills"));
    }

    #[test]
    fn test_resolve_placeholder_path_workspace() -> anyhow::Result<()>
    {
        let workspace = std::path::PathBuf::from("/proj");
        let home = std::path::PathBuf::from("/home/user");
        let result = resolve_placeholder_path("$workspace/.cursor/skills", &workspace, &home);
        assert_eq!(result, workspace.join(".cursor/skills"));
        Ok(())
    }

    #[test]
    fn test_resolve_placeholder_path_userprofile() -> anyhow::Result<()>
    {
        let workspace = std::path::PathBuf::from("/proj");
        let home = std::path::PathBuf::from("/home/user");
        let result = resolve_placeholder_path("$userprofile/.codex/skills", &workspace, &home);
        assert_eq!(result, home.join(".codex/skills"));
        Ok(())
    }

    #[test]
    fn test_resolve_placeholder_path_literal() -> anyhow::Result<()>
    {
        let workspace = std::path::PathBuf::from("/proj");
        let home = std::path::PathBuf::from("/home/user");
        let result = resolve_placeholder_path("/absolute/path", &workspace, &home);
        assert_eq!(result, std::path::PathBuf::from("/absolute/path"));
        Ok(())
    }

    #[test]
    fn test_get_all_skill_search_dirs_no_agents() -> anyhow::Result<()>
    {
        let temp_dir = tempfile::TempDir::new()?;
        let workspace = temp_dir.path();
        let home = std::path::PathBuf::from("/home/user");

        let dirs = get_all_skill_search_dirs(workspace, &home);
        // Only cross-client dir when no agents installed
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0], workspace.join(".agents/skills"));
        Ok(())
    }

    #[test]
    fn test_get_all_skill_search_dirs_with_agent() -> anyhow::Result<()>
    {
        let temp_dir = tempfile::TempDir::new()?;
        let workspace = temp_dir.path();
        let home = std::path::PathBuf::from("/home/user");

        std::fs::write(workspace.join(".cursorrules"), b"test")?;
        let dirs = get_all_skill_search_dirs(workspace, &home);
        // cursor skill dir + cross-client dir
        assert_eq!(dirs.len(), 2);
        assert!(dirs.contains(&workspace.join(".cursor/skills")) == true);
        assert!(dirs.contains(&workspace.join(".agents/skills")) == true);
        Ok(())
    }

    #[test]
    fn test_detect_all_installed_agents_none() -> anyhow::Result<()>
    {
        let temp_dir = tempfile::TempDir::new()?;
        let workspace = temp_dir.path();

        assert!(detect_all_installed_agents(workspace).is_empty() == true);
        Ok(())
    }

    #[test]
    fn test_detect_all_installed_agents_single() -> anyhow::Result<()>
    {
        let temp_dir = tempfile::TempDir::new()?;
        let workspace = temp_dir.path();

        std::fs::write(workspace.join("CLAUDE.md"), b"test")?;
        let agents = detect_all_installed_agents(workspace);
        assert_eq!(agents, vec!["claude".to_string()]);
        Ok(())
    }

    #[test]
    fn test_detect_all_installed_agents_multiple() -> anyhow::Result<()>
    {
        let temp_dir = tempfile::TempDir::new()?;
        let workspace = temp_dir.path();

        std::fs::write(workspace.join(".cursorrules"), b"test")?;
        std::fs::write(workspace.join("CLAUDE.md"), b"test")?;

        let agents = detect_all_installed_agents(workspace);
        assert!(agents.contains(&"cursor".to_string()) == true);
        assert!(agents.contains(&"claude".to_string()) == true);
        assert_eq!(agents.len(), 2);
        Ok(())
    }
}
