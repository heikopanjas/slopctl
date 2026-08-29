//! Default filesystem paths and conventions for known AI coding agents
//!
//! Provides a registry of agent-specific paths for workspace detection markers,
//! prompt/command directories, and skill directories. The registry is loaded
//! from `agent-defaults.yml` in the global template cache. There is no
//! compile-time embedded fallback: the catalog is authoritative, matching
//! `templates.yml` and `model-defaults.yml`, and callers must handle a missing
//! or unparsable cache by pointing the user at `slopctl templates --update`.

use std::{
    collections::HashSet,
    fs,
    path::{Component, Path, PathBuf}
};

use serde::{Deserialize, Serialize};

use crate::Result;

/// File name of the agent defaults catalog
pub const AGENT_DEFAULTS_FILE: &str = "agent-defaults.yml";

/// Placeholder for the project workspace root directory
pub const PLACEHOLDER_WORKSPACE: &str = "$workspace";

/// Placeholder for the user profile/home directory
pub const PLACEHOLDER_USERPROFILE: &str = "$userprofile";

/// Cross-client skill directory per the agentskills.io specification.
///
/// Scanned by Codex, Copilot, OpenCode, Mistral Vibe, Gemini CLI, and Cursor in
/// addition to their native skill directories. Claude Code does **not** read
/// this path — it only scans its own native skill directory.
pub const CROSS_CLIENT_SKILL_DIR: &str = "$workspace/.agents/skills";

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

fn default_catalog_version() -> u32
{
    1
}

/// Load the agent defaults catalog from a template cache directory
///
/// # Errors
///
/// Returns an error if `agent-defaults.yml` is missing, unreadable, unparsable,
/// or fails validation. Callers should point the user at
/// `slopctl templates --update` when this fails.
pub fn load_agent_catalog_from_dir(config_dir: &Path) -> Result<AgentCatalog>
{
    let path = config_dir.join(AGENT_DEFAULTS_FILE);
    require!(path.exists() == true, Err(anyhow::anyhow!("{} not found in global template directory", AGENT_DEFAULTS_FILE)));
    load_agent_catalog_file(&path)
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

fn get_catalog_entry<'a>(catalog: &'a AgentCatalog, agent: &str) -> Option<&'a AgentDefaultsEntry>
{
    catalog.agents.iter().find(|entry| entry.name == agent)
}

/// Get the skill installation directory for an agent from a specific catalog
pub fn get_skill_dir_from_catalog<'a>(catalog: &'a AgentCatalog, agent: &str) -> Option<&'a str>
{
    get_catalog_entry(catalog, agent).map(|entry| entry.skill_dir.as_str())
}

/// Get the prompt installation directory for an agent from a specific catalog
pub fn get_prompt_dir_from_catalog<'a>(catalog: &'a AgentCatalog, agent: &str) -> Option<&'a str>
{
    get_catalog_entry(catalog, agent).map(|entry| entry.prompt_dir.as_str())
}

/// Return whether an agent scans `.agents/skills/` according to a specific catalog
pub fn reads_cross_client_skills_from_catalog(catalog: &AgentCatalog, agent: &str) -> bool
{
    get_catalog_entry(catalog, agent).is_none_or(|entry| entry.reads_cross_client_skills)
}

/// Return the userprofile-scoped skill directory for an agent from a specific catalog
pub fn get_effective_userprofile_skill_dir_from_catalog(catalog: &AgentCatalog, agent: &str) -> String
{
    get_catalog_entry(catalog, agent).and_then(|entry| entry.userprofile_skill_dir.clone()).unwrap_or_else(|| "$userprofile/.agents/skills".to_string())
}

/// Return workspace marker directories for an agent from a specific catalog
pub fn get_workspace_marker_dirs_from_catalog(catalog: &AgentCatalog, agent: &str, workspace: &Path) -> Vec<PathBuf>
{
    get_catalog_entry(catalog, agent).map(|entry| entry.markers.iter().map(|marker| workspace.join(marker)).collect()).unwrap_or_default()
}

/// Return the names of all agents in a specific catalog
pub fn list_agent_names_from_catalog(catalog: &AgentCatalog) -> Vec<&str>
{
    catalog.agents.iter().map(|entry| entry.name.as_str()).collect()
}

/// Detect all agents installed in a workspace using a specific catalog
pub fn detect_all_installed_agents_from_catalog(catalog: &AgentCatalog, workspace: &Path) -> Vec<String>
{
    let mut found = Vec::new();
    for agent in &catalog.agents
    {
        for marker in &agent.markers
        {
            if workspace.join(marker).exists() == true
            {
                found.push(agent.name.clone());
                break;
            }
        }
    }
    found
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
    let path = Path::new(marker);
    require!(path.is_absolute() == false, Err(anyhow::anyhow!("agent '{}' marker '{}' must be relative", agent_name, marker)));
    require!(marker.contains(':') == false, Err(anyhow::anyhow!("agent '{}' marker '{}' must be a relative directory path", agent_name, marker)));

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

/// Resolve a placeholder path to an absolute filesystem path
///
/// Replaces `$workspace` and `$userprofile` prefixes with the supplied paths.
/// If neither prefix matches the string is treated as a literal path.
///
/// # Arguments
///
/// * `raw` - Placeholder path with a supported root placeholder
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

/// Return all skill directories to search using a specific agent catalog
pub fn get_all_skill_search_dirs_from_catalog(catalog: &AgentCatalog, workspace: &Path, userprofile: &Path) -> Vec<PathBuf>
{
    let mut dirs: Vec<PathBuf> = detect_all_installed_agents_from_catalog(catalog, workspace)
        .iter()
        .filter_map(|agent| get_skill_dir_from_catalog(catalog, agent).map(|raw| resolve_placeholder_path(raw, workspace, userprofile)))
        .collect();

    let cross_client = resolve_placeholder_path(CROSS_CLIENT_SKILL_DIR, workspace, userprofile);
    if dirs.contains(&cross_client) == false
    {
        dirs.push(cross_client);
    }

    dirs
}

/// Return workspace-scoped skill directories using a specific agent catalog
pub fn get_workspace_skill_search_dirs_from_catalog(catalog: &AgentCatalog, workspace: &Path, userprofile: &Path) -> Vec<PathBuf>
{
    let mut dirs: Vec<PathBuf> = detect_all_installed_agents_from_catalog(catalog, workspace)
        .iter()
        .filter_map(|agent| {
            let raw = get_skill_dir_from_catalog(catalog, agent)?;
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

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn test_load_agent_catalog_from_dir_valid() -> anyhow::Result<()>
    {
        let temp_dir = tempfile::TempDir::new()?;
        std::fs::write(
            temp_dir.path().join(AGENT_DEFAULTS_FILE),
            r#"
version: 1
agents:
  - name: bogus
    markers:
      - .bogus
    prompt_dir: '$workspace/.bogus/prompts'
    skill_dir: '$workspace/.bogus/skills'
    reads_cross_client_skills: true
"#
        )?;

        let catalog = load_agent_catalog_from_dir(temp_dir.path())?;
        assert_eq!(catalog.agents.len(), 1);
        assert_eq!(catalog.agents[0].name, "bogus");
        Ok(())
    }

    #[test]
    fn test_load_agent_catalog_from_dir_missing_errors() -> anyhow::Result<()>
    {
        let temp_dir = tempfile::TempDir::new()?;
        let err = load_agent_catalog_from_dir(temp_dir.path()).unwrap_err();
        assert!(err.to_string().contains(AGENT_DEFAULTS_FILE) == true);
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
        #[cfg(windows)]
        let marker_path =
            std::env::var_os("TEMP").or_else(|| std::env::var_os("TMP")).map(PathBuf::from).unwrap_or_else(std::env::temp_dir).join("slopctl-invalid-marker");
        #[cfg(not(windows))]
        let marker_path = PathBuf::from("/tmp/invalid");
        let marker = marker_path.to_string_lossy().replace('\'', "''");
        let err = parse_agent_catalog(&format!(
            r#"
version: 1
agents:
  - name: invalid
    markers:
      - '{marker}'
    prompt_dir: '$workspace/.invalid/prompts'
    skill_dir: '$workspace/.invalid/skills'
    reads_cross_client_skills: true
"#
        ))
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
      - bogus.json
    prompt_dir: '$workspace/.invalid/prompts'
    skill_dir: '$workspace/.invalid/skills'
    reads_cross_client_skills: true
"#
        )
        .unwrap_err();
        assert!(err.to_string().contains("not a file") == true);
    }

    #[test]
    fn test_get_skill_dir_from_catalog() -> anyhow::Result<()>
    {
        let catalog = parse_agent_catalog(
            r#"
version: 1
agents:
  - name: bogus
    markers:
      - .bogus
    prompt_dir: '$workspace/.bogus/prompts'
    skill_dir: '$workspace/.bogus/skills'
    reads_cross_client_skills: true
"#
        )?;
        assert_eq!(get_skill_dir_from_catalog(&catalog, "bogus"), Some("$workspace/.bogus/skills"));
        assert_eq!(get_skill_dir_from_catalog(&catalog, "nonexistent"), None);
        Ok(())
    }

    #[test]
    fn test_reads_cross_client_skills_per_agent()
    {
        let catalog = parse_agent_catalog(
            r#"
version: 1
agents:
  - name: bogus
    markers:
      - .bogus
    prompt_dir: '$workspace/.bogus/prompts'
    skill_dir: '$workspace/.bogus/skills'
    reads_cross_client_skills: true
  - name: fake
    markers:
      - .fake
    prompt_dir: '$workspace/.fake/prompts'
    skill_dir: '$workspace/.fake/skills'
    reads_cross_client_skills: false
"#
        )
        .expect("synthetic catalog should parse");

        assert!(reads_cross_client_skills_from_catalog(&catalog, "bogus") == true);
        assert!(reads_cross_client_skills_from_catalog(&catalog, "fake") == false);
        assert!(reads_cross_client_skills_from_catalog(&catalog, "unknown-agent") == true);
    }

    #[test]
    fn test_get_workspace_marker_dirs_resolves_marker_directories() -> anyhow::Result<()>
    {
        let temp_dir = tempfile::TempDir::new()?;
        let catalog = parse_agent_catalog(
            r#"
version: 1
agents:
  - name: bogus
    markers:
      - .bogus
    prompt_dir: '$workspace/.bogus/prompts'
    skill_dir: '$workspace/.bogus/skills'
    reads_cross_client_skills: true
"#
        )?;
        let dirs = get_workspace_marker_dirs_from_catalog(&catalog, "bogus", temp_dir.path());
        assert_eq!(dirs, vec![temp_dir.path().join(".bogus")]);
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
        let result = resolve_placeholder_path("$workspace/.bogus/skills", &workspace, &home);
        assert_eq!(result, workspace.join(".bogus/skills"));
        Ok(())
    }

    #[test]
    fn test_resolve_placeholder_path_userprofile() -> anyhow::Result<()>
    {
        let workspace = std::path::PathBuf::from("/proj");
        let home = std::path::PathBuf::from("/home/user");
        let result = resolve_placeholder_path("$userprofile/.bogus/skills", &workspace, &home);
        assert_eq!(result, home.join(".bogus/skills"));
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
        let catalog = parse_agent_catalog(
            r#"
version: 1
agents:
  - name: bogus
    markers:
      - .bogus
    prompt_dir: '$workspace/.bogus/prompts'
    skill_dir: '$workspace/.bogus/skills'
    reads_cross_client_skills: true
"#
        )?;

        let dirs = get_all_skill_search_dirs_from_catalog(&catalog, workspace, &home);
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
        let catalog = parse_agent_catalog(
            r#"
version: 1
agents:
  - name: bogus
    markers:
      - .bogus
    prompt_dir: '$workspace/.bogus/prompts'
    skill_dir: '$workspace/.bogus/skills'
    reads_cross_client_skills: true
"#
        )?;

        std::fs::create_dir(workspace.join(".bogus"))?;
        let dirs = get_all_skill_search_dirs_from_catalog(&catalog, workspace, &home);
        // agent skill dir + cross-client dir
        assert_eq!(dirs.len(), 2);
        assert!(dirs.contains(&workspace.join(".bogus/skills")) == true);
        assert!(dirs.contains(&workspace.join(".agents/skills")) == true);
        Ok(())
    }

    #[test]
    fn test_get_workspace_skill_search_dirs_includes_workspace_dir() -> anyhow::Result<()>
    {
        let temp_dir = tempfile::TempDir::new()?;
        let workspace = temp_dir.path();
        let home = std::path::PathBuf::from("/home/user");
        let catalog = parse_agent_catalog(
            r#"
version: 1
agents:
  - name: bogus
    markers:
      - .bogus
    prompt_dir: '$workspace/.bogus/prompts'
    skill_dir: '$workspace/.bogus/skills'
    userprofile_skill_dir: '$userprofile/.bogus/skills'
    reads_cross_client_skills: true
  - name: fake
    markers:
      - .fake
    prompt_dir: '$workspace/.fake/prompts'
    skill_dir: '$workspace/.fake/skills'
    reads_cross_client_skills: true
"#
        )?;

        std::fs::create_dir(workspace.join(".bogus"))?;
        std::fs::create_dir(workspace.join(".fake"))?;

        let all_dirs = get_all_skill_search_dirs_from_catalog(&catalog, workspace, &home);
        let ws_dirs = get_workspace_skill_search_dirs_from_catalog(&catalog, workspace, &home);

        assert!(all_dirs.contains(&workspace.join(".bogus/skills")) == true);
        assert!(ws_dirs.contains(&workspace.join(".bogus/skills")) == true);
        assert!(ws_dirs.contains(&home.join(".bogus/skills")) == false);
        // workspace-scoped dirs are still present
        assert!(ws_dirs.contains(&workspace.join(".fake/skills")) == true);
        assert!(ws_dirs.contains(&workspace.join(".agents/skills")) == true);
        Ok(())
    }

    #[test]
    fn test_detect_all_installed_agents_none() -> anyhow::Result<()>
    {
        let temp_dir = tempfile::TempDir::new()?;
        let workspace = temp_dir.path();
        let catalog = parse_agent_catalog(
            r#"
version: 1
agents:
  - name: bogus
    markers:
      - .bogus
    prompt_dir: '$workspace/.bogus/prompts'
    skill_dir: '$workspace/.bogus/skills'
    reads_cross_client_skills: true
"#
        )?;

        assert!(detect_all_installed_agents_from_catalog(&catalog, workspace).is_empty() == true);
        Ok(())
    }

    #[test]
    fn test_detect_all_installed_agents_single() -> anyhow::Result<()>
    {
        let temp_dir = tempfile::TempDir::new()?;
        let workspace = temp_dir.path();
        let catalog = parse_agent_catalog(
            r#"
version: 1
agents:
  - name: bogus
    markers:
      - .bogus
    prompt_dir: '$workspace/.bogus/prompts'
    skill_dir: '$workspace/.bogus/skills'
    reads_cross_client_skills: true
"#
        )?;

        std::fs::create_dir(workspace.join(".bogus"))?;
        let agents = detect_all_installed_agents_from_catalog(&catalog, workspace);
        assert_eq!(agents, vec!["bogus".to_string()]);
        Ok(())
    }

    #[test]
    fn test_detect_all_installed_agents_multiple() -> anyhow::Result<()>
    {
        let temp_dir = tempfile::TempDir::new()?;
        let workspace = temp_dir.path();
        let catalog = parse_agent_catalog(
            r#"
version: 1
agents:
  - name: bogus
    markers:
      - .bogus
    prompt_dir: '$workspace/.bogus/prompts'
    skill_dir: '$workspace/.bogus/skills'
    reads_cross_client_skills: true
  - name: fake
    markers:
      - .fake
    prompt_dir: '$workspace/.fake/prompts'
    skill_dir: '$workspace/.fake/skills'
    reads_cross_client_skills: true
"#
        )?;

        std::fs::create_dir(workspace.join(".bogus"))?;
        std::fs::create_dir(workspace.join(".fake"))?;

        let agents = detect_all_installed_agents_from_catalog(&catalog, workspace);
        assert!(agents.contains(&"bogus".to_string()) == true);
        assert!(agents.contains(&"fake".to_string()) == true);
        assert_eq!(agents.len(), 2);
        Ok(())
    }
}
