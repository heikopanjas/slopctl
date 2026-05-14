//! Default filesystem paths and conventions for known AI coding agents
//!
//! Provides a registry of agent-specific paths for workspace detection markers,
//! prompt/command directories, and skill directories. Used by the install
//! flow to resolve where skills and other agent-agnostic artifacts go.
//!
//! Detection markers are files or directories whose presence indicates a particular
//! agent is active in the workspace. Preferred markers are native agent-created
//! paths (e.g. `.claude/`, `.cursor/`). Copilot has no native marker so its
//! detection relies on the `.github/prompts/` directory that slopctl installs.

use std::path::{Path, PathBuf};

/// Placeholder for the project workspace root directory
pub const PLACEHOLDER_WORKSPACE: &str = "$workspace";

/// Placeholder for the user profile/home directory
pub const PLACEHOLDER_USERPROFILE: &str = "$userprofile";

/// Cross-client skill directory per the agentskills.io specification.
///
/// Scanned by Codex, Copilot, OpenCode, Gemini CLI, and Cursor in addition to
/// their native skill directories. Claude Code and Mistral Vibe do **not** read
/// this path — they only scan their own native skill directories.
pub const CROSS_CLIENT_SKILL_DIR: &str = "$workspace/.agents/skills";

/// A file or directory whose presence indicates a particular agent is active
/// in the workspace.
///
/// The path is joined to `placeholder` (workspace or userprofile root) and
/// tested with `Path::exists()`, which returns `true` for both files and
/// directories. Use a directory path (e.g. `.claude`) when the agent reliably
/// creates that directory itself; use a specific file when only a file works.
#[derive(Debug, Clone)]
pub struct WorkspaceMarker
{
    /// Relative path within the placeholder root (e.g. `.claude`, `opencode.json`)
    pub path:        &'static str,
    /// Root placeholder: `$workspace` or `$userprofile`
    pub placeholder: &'static str
}

/// Default filesystem conventions for a known AI coding agent
#[derive(Debug, Clone)]
pub struct AgentDefaults
{
    /// Agent identifier (e.g. "cursor", "claude")
    pub name:                      &'static str,
    /// Files or directories whose presence indicates this agent is active
    pub workspace_markers:         &'static [WorkspaceMarker],
    /// Directory for agent prompts/commands, with placeholder prefix
    pub prompt_dir:                &'static str,
    /// Primary skill installation directory, with placeholder prefix.
    /// Agent files are installed to the workspace by default when the agent supports it.
    pub skill_dir:                 &'static str,
    /// Explicit userprofile-scoped skill dir for opt-in global installs.
    /// Userprofile installs are the exception; use them only when a template explicitly
    /// requests `target: '$userprofile'`.
    pub userprofile_skill_dir:     Option<&'static str>,
    /// Whether this agent scans `.agents/skills/` in addition to its native skill dir.
    ///
    /// When `false` (Claude Code, Mistral Vibe), slopctl routes skill installation
    /// directly to `skill_dir` and migrates any pre-existing cross-client skills
    /// into that directory so they remain visible to the agent.
    pub reads_cross_client_skills: bool
}

// Claude Code creates `.claude/` when it initialises a project
const CLAUDE_MARKERS: &[WorkspaceMarker] = &[WorkspaceMarker { path: ".claude", placeholder: PLACEHOLDER_WORKSPACE }];

// Cursor IDE creates `.cursor/` when it opens a project
const CURSOR_MARKERS: &[WorkspaceMarker] = &[WorkspaceMarker { path: ".cursor", placeholder: PLACEHOLDER_WORKSPACE }];

// Copilot has no native workspace marker; the prompt directory is used as a proxy
// (created by slopctl when `init --agent copilot` installs the prompt files)
const COPILOT_MARKERS: &[WorkspaceMarker] = &[WorkspaceMarker { path: ".github/prompts", placeholder: PLACEHOLDER_WORKSPACE }];

// Codex creates `.codex/` when it initialises a project
const CODEX_MARKERS: &[WorkspaceMarker] = &[WorkspaceMarker { path: ".codex", placeholder: PLACEHOLDER_WORKSPACE }];

// Mistral Vibe creates `.vibe/` when it initialises a project
const VIBE_MARKERS: &[WorkspaceMarker] = &[WorkspaceMarker { path: ".vibe", placeholder: PLACEHOLDER_WORKSPACE }];

// OpenCode writes `opencode.json` to the workspace root when initialised
const OPENCODE_MARKERS: &[WorkspaceMarker] = &[WorkspaceMarker { path: "opencode.json", placeholder: PLACEHOLDER_WORKSPACE }];

/// Built-in registry of known agents and their filesystem conventions
const KNOWN_AGENTS: &[AgentDefaults] = &[
    AgentDefaults {
        name:                      "cursor",
        workspace_markers:         CURSOR_MARKERS,
        prompt_dir:                "$workspace/.cursor/commands",
        skill_dir:                 "$workspace/.cursor/skills",
        userprofile_skill_dir:     None,
        reads_cross_client_skills: true
    },
    AgentDefaults {
        name:                      "claude",
        workspace_markers:         CLAUDE_MARKERS,
        prompt_dir:                "$workspace/.claude/commands",
        skill_dir:                 "$workspace/.claude/skills",
        userprofile_skill_dir:     Some("$userprofile/.claude/skills"),
        reads_cross_client_skills: false
    },
    AgentDefaults {
        name:                      "codex",
        workspace_markers:         CODEX_MARKERS,
        prompt_dir:                "$workspace/.codex/prompts",
        skill_dir:                 "$workspace/.codex/skills",
        userprofile_skill_dir:     Some("$userprofile/.codex/skills"),
        reads_cross_client_skills: true
    },
    AgentDefaults {
        name:                      "copilot",
        workspace_markers:         COPILOT_MARKERS,
        prompt_dir:                "$workspace/.github/prompts",
        skill_dir:                 "$workspace/.github/skills",
        userprofile_skill_dir:     Some("$userprofile/.copilot/skills"),
        reads_cross_client_skills: true
    },
    AgentDefaults {
        name:                      "vibe",
        workspace_markers:         VIBE_MARKERS,
        prompt_dir:                "$userprofile/.vibe/prompts",
        skill_dir:                 "$workspace/.vibe/skills",
        userprofile_skill_dir:     Some("$userprofile/.vibe/skills"),
        reads_cross_client_skills: false
    },
    AgentDefaults {
        name:                      "opencode",
        workspace_markers:         OPENCODE_MARKERS,
        prompt_dir:                "$workspace/.opencode/commands",
        skill_dir:                 "$workspace/.opencode/skills",
        userprofile_skill_dir:     Some("$userprofile/.config/opencode/skills"),
        reads_cross_client_skills: true
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

/// Return whether an agent scans `.agents/skills/` for skills
///
/// Returns `true` for Cursor, Codex, Copilot, and OpenCode (which all follow the
/// agentskills.io cross-client convention). Returns `false` for Claude Code and
/// Mistral Vibe, which only scan their own native skill directories.
/// Unknown agents default to `true` (assume cross-client compliance).
pub fn reads_cross_client_skills(agent: &str) -> bool
{
    get_defaults(agent).is_none_or(|d| d.reads_cross_client_skills)
}

/// Return the userprofile-scoped skill directory for an agent
///
/// Returns the raw placeholder path (e.g. `$userprofile/.codex/skills`).
/// Unknown agents fall back to `$userprofile/.agents/skills`. Caller must resolve
/// the placeholder to an actual path.
pub fn get_effective_userprofile_skill_dir(agent: &str) -> &'static str
{
    get_defaults(agent).map(|d| d.userprofile_skill_dir.unwrap_or("$userprofile/.agents/skills")).unwrap_or("$userprofile/.agents/skills")
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
/// workspace markers) and always appends the cross-client `.agents/skills`
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

/// Return only workspace-scoped skill directories safe for filesystem scanning
///
/// Like `get_all_skill_search_dirs` but excludes `$userprofile`-based skill
/// directories. Those directories are user-global and may contain agent-internal files or skills from other
/// workspaces. Use `FileTracker` to manage userprofile skills instead.
///
/// # Arguments
///
/// * `workspace` - Absolute path to the project workspace root
/// * `userprofile` - Absolute path to the user home directory
pub fn get_workspace_skill_search_dirs(workspace: &Path, userprofile: &Path) -> Vec<PathBuf>
{
    let mut dirs: Vec<PathBuf> = detect_all_installed_agents(workspace)
        .iter()
        .filter_map(|agent| {
            let raw = get_skill_dir(agent)?;
            if raw.starts_with(PLACEHOLDER_WORKSPACE) == true
            {
                Some(resolve_placeholder_path(raw, workspace, userprofile))
            }
            else
            {
                None
            }
        })
        .collect();

    let cross_client = resolve_placeholder_path(CROSS_CLIENT_SKILL_DIR, workspace, userprofile);
    if dirs.contains(&cross_client) == false
    {
        dirs.push(cross_client);
    }

    dirs
}

/// Detect which agent is installed in a workspace by checking for known markers
///
/// Scans the workspace for agent-specific files or directories.
/// Returns the first agent whose marker is found.
///
/// # Arguments
///
/// * `workspace` - Path to the project workspace root
pub fn detect_installed_agent(workspace: &Path) -> Option<String>
{
    for agent in KNOWN_AGENTS
    {
        for marker in agent.workspace_markers
        {
            if marker.placeholder == PLACEHOLDER_WORKSPACE
            {
                let marker_path = workspace.join(marker.path);
                if marker_path.exists() == true
                {
                    return Some(agent.name.to_string());
                }
            }
        }
    }
    None
}

/// Detect all agents installed in a workspace by checking for known markers
///
/// Scans the workspace for agent-specific files or directories.
/// Returns every agent whose marker is found (may be empty).
///
/// # Arguments
///
/// * `workspace` - Path to the project workspace root
pub fn detect_all_installed_agents(workspace: &Path) -> Vec<String>
{
    let mut found = Vec::new();
    for agent in KNOWN_AGENTS
    {
        for marker in agent.workspace_markers
        {
            if marker.placeholder == PLACEHOLDER_WORKSPACE
            {
                let marker_path = workspace.join(marker.path);
                if marker_path.exists() == true
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
        assert_eq!(get_skill_dir("codex"), Some("$workspace/.codex/skills"));
        assert_eq!(get_skill_dir("vibe"), Some("$workspace/.vibe/skills"));
        assert_eq!(get_skill_dir("opencode"), Some("$workspace/.opencode/skills"));
        assert_eq!(get_skill_dir("nonexistent"), None);
    }

    #[test]
    fn test_codex_defaults_use_workspace_dirs()
    {
        let defaults = get_defaults("codex").expect("codex defaults should exist");
        assert_eq!(defaults.prompt_dir, "$workspace/.codex/prompts");
        assert_eq!(defaults.skill_dir, "$workspace/.codex/skills");
        assert_eq!(defaults.userprofile_skill_dir, Some("$userprofile/.codex/skills"));
    }

    #[test]
    fn test_opencode_prompt_dir_uses_commands()
    {
        let defaults = get_defaults("opencode").expect("opencode defaults should exist");
        assert_eq!(defaults.prompt_dir, "$workspace/.opencode/commands");
    }

    #[test]
    fn test_known_agents_contains_all()
    {
        let agents = known_agents();
        assert!(agents.contains(&"cursor"));
        assert!(agents.contains(&"claude"));
        assert!(agents.contains(&"codex"));
        assert!(agents.contains(&"copilot"));
        assert!(agents.contains(&"vibe"));
        assert!(agents.contains(&"opencode"));
        assert_eq!(agents.len(), 6);
    }

    #[test]
    fn test_reads_cross_client_skills_per_agent()
    {
        // Agents that DO scan .agents/skills/
        assert!(reads_cross_client_skills("cursor") == true);
        assert!(reads_cross_client_skills("codex") == true);
        assert!(reads_cross_client_skills("copilot") == true);
        assert!(reads_cross_client_skills("opencode") == true);
        // Agents that do NOT scan .agents/skills/
        assert!(reads_cross_client_skills("claude") == false);
        assert!(reads_cross_client_skills("vibe") == false);
        // Unknown agent defaults to true (assume compliant)
        assert!(reads_cross_client_skills("unknown-agent") == true);
    }

    #[test]
    fn test_detect_installed_agent_cursor() -> anyhow::Result<()>
    {
        let temp_dir = tempfile::TempDir::new()?;
        let workspace = temp_dir.path();

        // No agent markers -> None
        assert!(detect_installed_agent(workspace).is_none());

        // Create .cursor/ directory -> detects cursor
        std::fs::create_dir(workspace.join(".cursor"))?;
        assert_eq!(detect_installed_agent(workspace), Some("cursor".to_string()));
        Ok(())
    }

    #[test]
    fn test_detect_installed_agent_claude() -> anyhow::Result<()>
    {
        let temp_dir = tempfile::TempDir::new()?;
        let workspace = temp_dir.path();

        std::fs::create_dir(workspace.join(".claude"))?;
        assert_eq!(detect_installed_agent(workspace), Some("claude".to_string()));
        Ok(())
    }

    #[test]
    fn test_detect_installed_agent_codex() -> anyhow::Result<()>
    {
        let temp_dir = tempfile::TempDir::new()?;
        let workspace = temp_dir.path();

        std::fs::create_dir(workspace.join(".codex"))?;
        assert_eq!(detect_installed_agent(workspace), Some("codex".to_string()));
        Ok(())
    }

    #[test]
    fn test_detect_installed_agent_vibe() -> anyhow::Result<()>
    {
        let temp_dir = tempfile::TempDir::new()?;
        let workspace = temp_dir.path();

        std::fs::create_dir(workspace.join(".vibe"))?;
        assert_eq!(detect_installed_agent(workspace), Some("vibe".to_string()));
        Ok(())
    }

    #[test]
    fn test_detect_installed_agent_opencode() -> anyhow::Result<()>
    {
        let temp_dir = tempfile::TempDir::new()?;
        let workspace = temp_dir.path();

        std::fs::write(workspace.join("opencode.json"), b"{}")?;
        assert_eq!(detect_installed_agent(workspace), Some("opencode".to_string()));
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

        std::fs::create_dir(workspace.join(".cursor"))?;
        let dirs = get_all_skill_search_dirs(workspace, &home);
        // cursor skill dir + cross-client dir
        assert_eq!(dirs.len(), 2);
        assert!(dirs.contains(&workspace.join(".cursor/skills")) == true);
        assert!(dirs.contains(&workspace.join(".agents/skills")) == true);
        Ok(())
    }

    #[test]
    fn test_get_workspace_skill_search_dirs_includes_codex_workspace_dir() -> anyhow::Result<()>
    {
        let temp_dir = tempfile::TempDir::new()?;
        let workspace = temp_dir.path();
        let home = std::path::PathBuf::from("/home/user");

        std::fs::create_dir(workspace.join(".cursor"))?;
        std::fs::create_dir(workspace.join(".codex"))?;

        let all_dirs = get_all_skill_search_dirs(workspace, &home);
        let ws_dirs = get_workspace_skill_search_dirs(workspace, &home);

        // all_dirs includes codex's workspace-local skill dir
        assert!(all_dirs.contains(&workspace.join(".codex/skills")) == true);
        // workspace-only dirs include codex because its native dir is project-scoped
        assert!(ws_dirs.contains(&workspace.join(".codex/skills")) == true);
        assert!(ws_dirs.contains(&home.join(".codex/skills")) == false);
        // workspace-scoped dirs are still present
        assert!(ws_dirs.contains(&workspace.join(".cursor/skills")) == true);
        assert!(ws_dirs.contains(&workspace.join(".agents/skills")) == true);
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

        std::fs::create_dir(workspace.join(".claude"))?;
        let agents = detect_all_installed_agents(workspace);
        assert_eq!(agents, vec!["claude".to_string()]);
        Ok(())
    }

    #[test]
    fn test_detect_all_installed_agents_multiple() -> anyhow::Result<()>
    {
        let temp_dir = tempfile::TempDir::new()?;
        let workspace = temp_dir.path();

        std::fs::create_dir(workspace.join(".cursor"))?;
        std::fs::create_dir(workspace.join(".claude"))?;

        let agents = detect_all_installed_agents(workspace);
        assert!(agents.contains(&"cursor".to_string()) == true);
        assert!(agents.contains(&"claude".to_string()) == true);
        assert_eq!(agents.len(), 2);
        Ok(())
    }
}
