//! Default filesystem paths and conventions for known AI coding agents
//!
//! Provides a registry of agent-specific paths for workspace detection markers,
//! prompt/command directories, and skill directories. The registry is loaded
//! from `agent-defaults.yml` in the global template cache, with an embedded
//! fallback for first-run behavior.

use std::{
    collections::HashSet,
    fs,
    path::{Component, Path, PathBuf},
    sync::OnceLock
};

use serde::{Deserialize, Serialize};

use crate::Result;

/// File name of the agent defaults catalog
pub const AGENT_DEFAULTS_FILE: &str = "agent-defaults.yml";

const EMBEDDED_AGENT_DEFAULTS: &str = include_str!("../templates/v5/agent-defaults.yml");

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

/// A directory whose presence indicates a particular agent is active in the workspace.
///
/// Catalog markers are workspace-relative directory paths. The `placeholder`
/// field is synthesized as `$workspace` for source compatibility with existing
/// call sites that still iterate over `workspace_markers`.
#[derive(Debug, Clone)]
pub struct WorkspaceMarker
{
    /// Relative directory path within the workspace root (e.g. `.claude`)
    pub path:        &'static str,
    /// Root placeholder, always `$workspace` for catalog markers
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

/// YAML representation of the agent defaults catalog
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCatalog
{
    /// Catalog schema version
    #[serde(default = "default_catalog_version")]
    pub version: u32,
    /// Known agent defaults in detection order
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub agents:  Vec<AgentDefaultsEntry>
}

/// YAML representation of an agent default entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDefaultsEntry
{
    /// Agent identifier
    pub name:                      String,
    /// Workspace-relative marker directories whose presence indicates this agent is active
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub markers:                   Vec<String>,
    /// Directory for agent prompts/commands, with placeholder prefix
    pub prompt_dir:                String,
    /// Primary skill installation directory, with placeholder prefix
    pub skill_dir:                 String,
    /// Explicit userprofile-scoped skill dir for opt-in global installs
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub userprofile_skill_dir:     Option<String>,
    /// Whether this agent scans `.agents/skills/` in addition to its native skill dir
    pub reads_cross_client_skills: bool
}

static DEFAULT_AGENT_DEFAULTS: OnceLock<&'static [AgentDefaults]> = OnceLock::new();

fn default_catalog_version() -> u32
{
    1
}

/// Load the agent defaults catalog from a template cache directory
///
/// Falls back to the embedded catalog when `agent-defaults.yml` is absent.
///
/// # Errors
///
/// Returns an error if the catalog file exists but cannot be read, parsed, or
/// validated, or if the embedded fallback is invalid.
pub fn load_agent_catalog_from_dir(config_dir: &Path) -> Result<AgentCatalog>
{
    let path = config_dir.join(AGENT_DEFAULTS_FILE);
    if path.exists() == true
    {
        return load_agent_catalog_file(&path);
    }
    load_embedded_agent_catalog()
}

/// Load the cached agent defaults catalog from a template cache directory
///
/// Unlike `load_agent_catalog_from_dir`, this requires the cache file to exist.
///
/// # Errors
///
/// Returns an error if `agent-defaults.yml` is missing or invalid.
pub fn load_cached_agent_catalog_from_dir(config_dir: &Path) -> Result<AgentCatalog>
{
    let path = config_dir.join(AGENT_DEFAULTS_FILE);
    require!(path.exists() == true, Err(anyhow::anyhow!("{} not found in global template directory", AGENT_DEFAULTS_FILE)));
    load_agent_catalog_file(&path)
}

/// Load the embedded fallback agent defaults catalog
///
/// # Errors
///
/// Returns an error if the embedded catalog is invalid.
pub fn load_embedded_agent_catalog() -> Result<AgentCatalog>
{
    parse_agent_catalog(EMBEDDED_AGENT_DEFAULTS)
}

/// Load an agent defaults catalog from a specific file
///
/// # Errors
///
/// Returns an error if the file cannot be read, parsed, or validated.
pub fn load_agent_catalog_file(path: &Path) -> Result<AgentCatalog>
{
    let content = fs::read_to_string(path)?;
    parse_agent_catalog(&content)
}

/// Parse and validate an agent defaults YAML catalog
///
/// # Errors
///
/// Returns an error if YAML parsing or validation fails.
pub fn parse_agent_catalog(content: &str) -> Result<AgentCatalog>
{
    let catalog: AgentCatalog = serde_yaml::from_str(content)?;
    validate_agent_catalog(&catalog)?;
    Ok(catalog)
}

/// Validate agent defaults catalog structure and placeholder usage
///
/// # Errors
///
/// Returns an error when required fields are empty, names are duplicated, or
/// path placeholders are unsupported.
pub fn validate_agent_catalog(catalog: &AgentCatalog) -> Result<()>
{
    require!(catalog.version == 1, Err(anyhow::anyhow!("unsupported agent defaults version: {}", catalog.version)));
    require!(catalog.agents.is_empty() == false, Err(anyhow::anyhow!("agent defaults catalog must contain at least one agent")));

    let mut names = HashSet::new();
    for agent in &catalog.agents
    {
        require!(agent.name.trim().is_empty() == false, Err(anyhow::anyhow!("agent name cannot be empty")));
        require!(names.insert(agent.name.as_str()) == true, Err(anyhow::anyhow!("duplicate agent defaults entry: {}", agent.name)));
        require!(agent.markers.is_empty() == false, Err(anyhow::anyhow!("agent '{}' must declare at least one marker", agent.name)));
        validate_placeholder_path(&agent.prompt_dir, &format!("agent '{}'.prompt_dir", agent.name))?;
        validate_placeholder_path(&agent.skill_dir, &format!("agent '{}'.skill_dir", agent.name))?;
        if let Some(userprofile_skill_dir) = &agent.userprofile_skill_dir
        {
            validate_placeholder_path(userprofile_skill_dir, &format!("agent '{}'.userprofile_skill_dir", agent.name))?;
        }

        for marker in &agent.markers
        {
            validate_marker_path(marker, &agent.name)?;
        }
    }

    Ok(())
}

fn validate_placeholder_path(path: &str, field: &str) -> Result<()>
{
    require!(path.trim().is_empty() == false, Err(anyhow::anyhow!("{} cannot be empty", field)));
    require!(
        path.starts_with(PLACEHOLDER_WORKSPACE) == true || path.starts_with(PLACEHOLDER_USERPROFILE) == true,
        Err(anyhow::anyhow!("{} must start with {} or {}", field, PLACEHOLDER_WORKSPACE, PLACEHOLDER_USERPROFILE))
    );
    Ok(())
}

fn validate_marker_path(marker: &str, agent_name: &str) -> Result<()>
{
    require!(marker.trim().is_empty() == false, Err(anyhow::anyhow!("agent '{}' has an empty marker path", agent_name)));
    require!(marker.contains('$') == false, Err(anyhow::anyhow!("agent '{}' marker '{}' must not contain placeholders", agent_name, marker)));
    require!(marker.contains(':') == false, Err(anyhow::anyhow!("agent '{}' marker '{}' must be a relative directory path", agent_name, marker)));
    let path = Path::new(marker);
    require!(path.is_absolute() == false, Err(anyhow::anyhow!("agent '{}' marker '{}' must be relative", agent_name, marker)));

    for component in path.components()
    {
        match component
        {
            | Component::Normal(_) =>
            {}
            | _ => return Err(anyhow::anyhow!("agent '{}' marker '{}' must not escape the workspace", agent_name, marker))
        }
    }

    let Some(_file_name) = path.file_name().and_then(|name| name.to_str())
    else
    {
        return Err(anyhow::anyhow!("agent '{}' marker '{}' must be a directory path", agent_name, marker));
    };
    require!(path.extension().is_none() == true, Err(anyhow::anyhow!("agent '{}' marker '{}' must be a directory path, not a file", agent_name, marker)));

    Ok(())
}

fn default_agent_defaults() -> &'static [AgentDefaults]
{
    DEFAULT_AGENT_DEFAULTS.get_or_init(|| {
        let catalog = load_default_agent_catalog().or_else(|_| load_embedded_agent_catalog()).expect("embedded agent defaults catalog must be valid");
        leak_agent_defaults(catalog)
    })
}

fn load_default_agent_catalog() -> Result<AgentCatalog>
{
    let data_dir = dirs::data_local_dir().ok_or_else(|| anyhow::anyhow!("Could not determine local data directory"))?;
    load_agent_catalog_from_dir(&data_dir.join("slopctl/templates"))
}

fn leak_agent_defaults(catalog: AgentCatalog) -> &'static [AgentDefaults]
{
    let agents: Vec<AgentDefaults> = catalog
        .agents
        .into_iter()
        .map(|agent| {
            let markers: Vec<WorkspaceMarker> =
                agent.markers.into_iter().map(|marker| WorkspaceMarker { path: leak_str(marker), placeholder: PLACEHOLDER_WORKSPACE }).collect();
            AgentDefaults {
                name:                      leak_str(agent.name),
                workspace_markers:         Box::leak(markers.into_boxed_slice()),
                prompt_dir:                leak_str(agent.prompt_dir),
                skill_dir:                 leak_str(agent.skill_dir),
                userprofile_skill_dir:     agent.userprofile_skill_dir.map(leak_str),
                reads_cross_client_skills: agent.reads_cross_client_skills
            }
        })
        .collect();
    Box::leak(agents.into_boxed_slice())
}

fn leak_str(value: String) -> &'static str
{
    Box::leak(value.into_boxed_str())
}

/// Look up defaults for an agent by name
pub fn get_defaults(agent: &str) -> Option<&'static AgentDefaults>
{
    default_agent_defaults().iter().find(|a| a.name == agent)
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
    default_agent_defaults().iter().map(|a| a.name).collect()
}

/// Return workspace marker directories for an agent resolved under `workspace`
///
/// Markers in `agent-defaults.yml` are validated as relative directory paths, so
/// these paths are safe for `slopctl init --agent` to create.
///
/// # Arguments
///
/// * `agent` - Agent identifier
/// * `workspace` - Workspace root directory
pub fn get_workspace_marker_dirs(agent: &str, workspace: &Path) -> Vec<PathBuf>
{
    get_defaults(agent)
        .map(|defaults| {
            defaults.workspace_markers.iter().filter(|marker| marker.placeholder == PLACEHOLDER_WORKSPACE).map(|marker| workspace.join(marker.path)).collect()
        })
        .unwrap_or_default()
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
    for agent in default_agent_defaults()
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
    for agent in default_agent_defaults()
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
    fn test_load_agent_catalog_from_dir_valid() -> anyhow::Result<()>
    {
        let temp_dir = tempfile::TempDir::new()?;
        std::fs::write(
            temp_dir.path().join(AGENT_DEFAULTS_FILE),
            r#"
version: 1
agents:
  - name: test-agent
    markers:
      - .test-agent
    prompt_dir: '$workspace/.test-agent/prompts'
    skill_dir: '$workspace/.test-agent/skills'
    reads_cross_client_skills: true
"#
        )?;

        let catalog = load_agent_catalog_from_dir(temp_dir.path())?;
        assert_eq!(catalog.agents.len(), 1);
        assert_eq!(catalog.agents[0].name, "test-agent");
        Ok(())
    }

    #[test]
    fn test_load_agent_catalog_from_dir_missing_uses_embedded() -> anyhow::Result<()>
    {
        let temp_dir = tempfile::TempDir::new()?;
        let catalog = load_agent_catalog_from_dir(temp_dir.path())?;
        assert!(catalog.agents.iter().any(|agent| agent.name == "cursor") == true);
        assert!(catalog.agents.iter().any(|agent| agent.name == "claude") == true);
        Ok(())
    }

    #[test]
    fn test_parse_agent_catalog_rejects_duplicate_names()
    {
        let err = parse_agent_catalog(
            r#"
version: 1
agents:
  - name: duplicate
    markers:
      - .one
    prompt_dir: '$workspace/.one/prompts'
    skill_dir: '$workspace/.one/skills'
    reads_cross_client_skills: true
  - name: duplicate
    markers:
      - .two
    prompt_dir: '$workspace/.two/prompts'
    skill_dir: '$workspace/.two/skills'
    reads_cross_client_skills: true
"#
        )
        .unwrap_err();
        assert!(err.to_string().contains("duplicate agent defaults entry") == true);
    }

    #[test]
    fn test_parse_agent_catalog_rejects_invalid_placeholder()
    {
        let err = parse_agent_catalog(
            r#"
version: 1
agents:
  - name: invalid
    markers:
      - .invalid
    prompt_dir: '$project/.invalid/prompts'
    skill_dir: '$workspace/.invalid/skills'
    reads_cross_client_skills: true
"#
        )
        .unwrap_err();
        assert!(err.to_string().contains("prompt_dir must start") == true);
    }

    #[test]
    fn test_parse_agent_catalog_rejects_marker_placeholder()
    {
        let err = parse_agent_catalog(
            r#"
version: 1
agents:
  - name: invalid
    markers:
      - '$workspace/.invalid'
    prompt_dir: '$workspace/.invalid/prompts'
    skill_dir: '$workspace/.invalid/skills'
    reads_cross_client_skills: true
"#
        )
        .unwrap_err();
        assert!(err.to_string().contains("must not contain placeholders") == true);
    }

    #[test]
    fn test_parse_agent_catalog_rejects_absolute_marker()
    {
        let err = parse_agent_catalog(
            r#"
version: 1
agents:
  - name: invalid
    markers:
      - /tmp/invalid
    prompt_dir: '$workspace/.invalid/prompts'
    skill_dir: '$workspace/.invalid/skills'
    reads_cross_client_skills: true
"#
        )
        .unwrap_err();
        assert!(err.to_string().contains("must be relative") == true);
    }

    #[test]
    fn test_parse_agent_catalog_rejects_file_marker()
    {
        let err = parse_agent_catalog(
            r#"
version: 1
agents:
  - name: invalid
    markers:
      - opencode.json
    prompt_dir: '$workspace/.invalid/prompts'
    skill_dir: '$workspace/.invalid/skills'
    reads_cross_client_skills: true
"#
        )
        .unwrap_err();
        assert!(err.to_string().contains("not a file") == true);
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

        std::fs::create_dir(workspace.join(".opencode"))?;
        assert_eq!(detect_installed_agent(workspace), Some("opencode".to_string()));
        Ok(())
    }

    #[test]
    fn test_get_workspace_marker_dirs_resolves_marker_directories() -> anyhow::Result<()>
    {
        let temp_dir = tempfile::TempDir::new()?;
        let dirs = get_workspace_marker_dirs("opencode", temp_dir.path());
        assert_eq!(dirs, vec![temp_dir.path().join(".opencode")]);
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
