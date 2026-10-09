//! User-defined agent overlays on top of the shipped agent catalogs
//!
//! Users can add agents that the shipped `agent-defaults.yml` and `templates.yml` do not know,
//! without forking the template repository. Each overlay agent lives in its own directory
//! `agents/<name>/agent.yml`, either in the global slopctl config directory or in the workspace
//! `.slopctl/` directory. A workspace overlay shadows a global overlay of the same name. Overlays are
//! add-only: a name that clashes with a shipped agent is an error.
//!
//! [`load_effective_catalogs`] is the single entry point that merges the shipped catalogs with all
//! overlays. It also hosts the cross-check between the two shipped catalogs, so the rule "an agent must
//! be known to both `templates.yml` and `agent-defaults.yml`" lives in exactly one place.

use std::{
    collections::BTreeMap,
    fs,
    path::{Component, Path, PathBuf}
};

use serde::Deserialize;

use crate::{
    Result,
    agent_defaults::{self, AgentCatalog, AgentDefaultsEntry, PLACEHOLDER_WORKSPACE},
    bom::{AgentConfig, TemplateConfig},
    config::ConfigScope,
    file_tracker::SLOPCTL_DIR,
    template_engine::load_template_config
};

/// Directory name (below the global config dir or the workspace `.slopctl/`) holding overlay agents
pub const AGENT_OVERLAY_DIR: &str = "agents";

/// File name of the overlay definition inside each agent directory
pub const AGENT_OVERLAY_FILE: &str = "agent.yml";

/// Origin of an overlay agent
#[derive(Debug, Clone)]
pub struct OverlayAgent
{
    /// Agent identifier
    pub name:  String,
    /// Whether the overlay came from the global config dir or the workspace
    pub scope: ConfigScope,
    /// Directory containing `agent.yml` and the agent's source files
    pub dir:   PathBuf
}

/// Shipped catalogs merged with all overlay agents
#[derive(Debug)]
pub struct EffectiveCatalogs
{
    /// `templates.yml` including overlay agent sections
    pub templates: TemplateConfig,
    /// `agent-defaults.yml` including overlay agent entries
    pub agents:    AgentCatalog,
    /// Overlay agents that were merged in, sorted by name
    pub overlays:  Vec<OverlayAgent>
}

/// One parsed overlay agent before it is merged into the catalogs
struct LoadedOverlay
{
    agent:    OverlayAgent,
    defaults: AgentDefaultsEntry,
    config:   AgentConfig
}

/// Raw `agent.yml` layout: catalog fields and template fields side by side
///
/// `unknown` must stay last so it only receives keys the two flattened structs did not consume.
#[derive(Deserialize)]
struct AgentOverlayFile
{
    #[serde(flatten)]
    defaults: AgentDefaultsEntry,
    #[serde(flatten)]
    config:   AgentConfig,
    #[serde(flatten)]
    unknown:  BTreeMap<String, serde_yaml::Value>
}

/// Loads the shipped catalogs from `config_dir` and merges global and workspace overlays into them
///
/// # Arguments
///
/// * `config_dir` - Global template cache directory holding `templates.yml` and `agent-defaults.yml`
/// * `workspace` - Workspace root whose `.slopctl/agents/` directory is scanned
///
/// # Errors
///
/// Returns an error if a shipped catalog cannot be loaded, an overlay is malformed, or an overlay
/// name clashes with a shipped agent.
pub fn load_effective_catalogs(config_dir: &Path, workspace: &Path) -> Result<EffectiveCatalogs>
{
    load_effective_catalogs_from(config_dir, global_overlay_root().as_deref(), &workspace.join(SLOPCTL_DIR).join(AGENT_OVERLAY_DIR), false)
}

/// Like [`load_effective_catalogs`], but a missing shipped catalog file counts as empty
///
/// `remove` uses this so it can still fall back to tracker records after the template cache was
/// cleared. Malformed shipped files and overlays are still errors.
///
/// # Errors
///
/// Returns an error if a shipped catalog exists but is invalid, an overlay is malformed, or an
/// overlay name clashes with a shipped agent.
pub fn load_effective_catalogs_lenient(config_dir: &Path, workspace: &Path) -> Result<EffectiveCatalogs>
{
    load_effective_catalogs_from(config_dir, global_overlay_root().as_deref(), &workspace.join(SLOPCTL_DIR).join(AGENT_OVERLAY_DIR), true)
}

/// Global overlay root; disabled in unit tests so they never read the developer's real config
fn global_overlay_root() -> Option<PathBuf>
{
    #[cfg(test)]
    {
        None
    }
    #[cfg(not(test))]
    {
        crate::config::Config::get_global_dir().ok().map(|dir| dir.join(AGENT_OVERLAY_DIR))
    }
}

fn load_effective_catalogs_from(config_dir: &Path, global_root: Option<&Path>, workspace_root: &Path, lenient: bool) -> Result<EffectiveCatalogs>
{
    let mut templates = if lenient == true && config_dir.join("templates.yml").exists() == false
    {
        TemplateConfig::default()
    }
    else
    {
        load_template_config(config_dir)?
    };
    let mut agents = if lenient == true && config_dir.join(agent_defaults::AGENT_DEFAULTS_FILE).exists() == false
    {
        AgentCatalog::default()
    }
    else
    {
        agent_defaults::load_agent_catalog_from_dir(config_dir)?
    };

    // Workspace entries are inserted last so they shadow global ones with the same name.
    let mut loaded = BTreeMap::new();
    if let Some(root) = global_root
    {
        loaded.extend(load_overlay_root(root, ConfigScope::Global)?);
    }
    loaded.extend(load_overlay_root(workspace_root, ConfigScope::Workspace)?);

    let mut overlays = Vec::new();
    for (name, overlay) in loaded
    {
        require!(
            templates.agents.contains_key(&name) == false && agents.agents.iter().any(|entry| entry.name == name) == false,
            Err(anyhow::anyhow!("Overlay agent '{}' ({}) clashes with a shipped agent; overlays are add-only", name, overlay.agent.dir.display()))
        );
        agents.agents.push(overlay.defaults);
        templates.agents.insert(name, overlay.config);
        overlays.push(overlay.agent);
    }

    Ok(EffectiveCatalogs { templates, agents, overlays })
}

/// Loads every `<root>/<name>/agent.yml`; a missing root yields no overlays
fn load_overlay_root(root: &Path, scope: ConfigScope) -> Result<BTreeMap<String, LoadedOverlay>>
{
    let mut loaded = BTreeMap::new();
    require!(root.is_dir() == true, Ok(loaded));

    let mut dirs: Vec<PathBuf> = fs::read_dir(root)?.collect::<std::io::Result<Vec<_>>>()?.into_iter().map(|entry| entry.path()).collect();
    dirs.sort();

    for agent_dir in dirs
    {
        if agent_dir.is_dir() == true
        {
            let dir_name = agent_dir
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| anyhow::anyhow!("Overlay directory name is not valid UTF-8: {}", agent_dir.display()))?;
            let file = agent_dir.join(AGENT_OVERLAY_FILE);
            require!(file.is_file() == true, Err(anyhow::anyhow!("Overlay agent directory {} has no {}", agent_dir.display(), AGENT_OVERLAY_FILE)));

            let content = fs::read_to_string(&file)?;
            let (defaults, config) = parse_overlay(&content, dir_name, &agent_dir, scope).map_err(|e| anyhow::anyhow!("Invalid overlay {}: {}", file.display(), e))?;
            loaded.insert(defaults.name.clone(), LoadedOverlay { agent: OverlayAgent { name: defaults.name.clone(), scope, dir: agent_dir }, defaults, config });
        }
    }

    Ok(loaded)
}

/// Parses, validates and rebases one `agent.yml`
fn parse_overlay(content: &str, dir_name: &str, agent_dir: &Path, scope: ConfigScope) -> Result<(AgentDefaultsEntry, AgentConfig)>
{
    let mut mapping: serde_yaml::Mapping = serde_yaml::from_str(content)?;
    let name_key = serde_yaml::Value::String("name".to_string());
    match mapping.get(&name_key)
    {
        | Some(serde_yaml::Value::String(name)) => require!(name == dir_name, Err(anyhow::anyhow!("name '{}' does not match directory name '{}'", name, dir_name))),
        | Some(_) => return Err(anyhow::anyhow!("name must be a string")),
        | None =>
        {
            mapping.insert(name_key, serde_yaml::Value::String(dir_name.to_string()));
        }
    }

    let AgentOverlayFile { defaults, mut config, unknown } = serde_yaml::from_value(serde_yaml::Value::Mapping(mapping))?;
    require!(unknown.is_empty() == true, Err(anyhow::anyhow!("unknown keys: {}", unknown.keys().map(String::as_str).collect::<Vec<_>>().join(", "))));

    agent_defaults::validate_agent_catalog(&AgentCatalog { version: 1, agents: vec![defaults.clone()] })?;

    validate_target(&defaults.prompt_dir, "prompt_dir", scope)?;
    validate_target(&defaults.skill_dir, "skill_dir", scope)?;
    if let Some(dir) = &defaults.userprofile_skill_dir
    {
        validate_target(dir, "userprofile_skill_dir", scope)?;
    }

    let base = std::path::absolute(agent_dir)?;
    for mapping in config.instructions.iter_mut().chain(config.prompts.iter_mut())
    {
        validate_target(&mapping.target, "target", scope)?;
        mapping.source = rebase_source(&mapping.source, &base)?;
    }
    for skill in &mut config.skills
    {
        if let Some(target) = &skill.target
        {
            validate_target(target, "skill target", scope)?;
        }
        skill.source = rebase_source(&skill.source, &base)?;
    }
    for directory in &config.directories
    {
        validate_target(&directory.target, "directory target", scope)?;
    }

    Ok((defaults, config))
}

/// Validates a target placeholder path; workspace overlays may only write below `$workspace`
fn validate_target(target: &str, field: &str, scope: ConfigScope) -> Result<()>
{
    agent_defaults::validate_placeholder_path(target, field)?;
    require!(
        (scope == ConfigScope::Global || target.starts_with(PLACEHOLDER_WORKSPACE) == true) == true,
        Err(anyhow::anyhow!("{} '{}' must start with {} in a workspace overlay", field, target, PLACEHOLDER_WORKSPACE))
    );
    Ok(())
}

/// Rewrites a relative source into an absolute path inside the agent directory
///
/// Absolute paths are used so every existing `config_dir.join(source)` site resolves overlay files
/// unchanged. URLs, absolute paths and `..` components are rejected to confine sources to the agent
/// directory. `std::path::absolute` is used on the base instead of `canonicalize` to avoid `\\?\` paths
/// on Windows.
fn rebase_source(source: &str, base: &Path) -> Result<String>
{
    require!(source.contains("://") == false, Err(anyhow::anyhow!("source '{}' must be a relative path, URLs are not allowed", source)));

    let mut joined = base.to_path_buf();
    let mut components = 0;
    for component in Path::new(source).components()
    {
        match component
        {
            | Component::Normal(part) =>
            {
                joined.push(part);
                components += 1;
            }
            | Component::CurDir =>
            {}
            | Component::ParentDir | Component::RootDir | Component::Prefix(_) =>
            {
                return Err(anyhow::anyhow!("source '{}' must stay inside the agent directory", source));
            }
        }
    }
    require!(components > 0, Err(anyhow::anyhow!("source '{}' must name a file or directory", source)));

    // Components never keep a trailing separator, which `derive_name` relies on.
    joined.to_str().map(str::to_string).ok_or_else(|| anyhow::anyhow!("source '{}' resolves to a non-UTF-8 path", source))
}

impl EffectiveCatalogs
{
    /// Returns the overlay record for an agent, if it came from an overlay
    pub fn overlay(&self, name: &str) -> Option<&OverlayAgent>
    {
        self.overlays.iter().find(|overlay| overlay.name == name)
    }

    /// Display label for an agent: the bare name, or the name annotated with its overlay origin
    pub fn agent_label(&self, name: &str) -> String
    {
        match self.overlay(name)
        {
            | Some(overlay) => format!("{} (overlay: {})", name, overlay.scope),
            | None => name.to_string()
        }
    }

    fn in_templates(&self, name: &str) -> bool
    {
        self.templates.agents.contains_key(name)
    }

    fn in_defaults(&self, name: &str) -> bool
    {
        self.agents.agents.iter().any(|entry| entry.name == name)
    }

    /// Fails when an agent is known to only one of the two catalogs
    ///
    /// An agent in neither catalog passes: `remove` still falls back to tracker records for those.
    ///
    /// # Errors
    ///
    /// Returns an error naming the catalog the agent is missing from.
    pub fn require_agent_consistent(&self, agent: &str) -> Result<()>
    {
        match (self.in_templates(agent), self.in_defaults(agent))
        {
            | (true, false) => Err(anyhow::anyhow!(
                "Agent '{}' is defined in templates.yml but missing from agent-defaults.yml.\nRun 'slopctl agents --update' to refresh the agent catalog.",
                agent
            )),
            | (false, true) => Err(anyhow::anyhow!(
                "Agent '{}' is defined in agent-defaults.yml but missing from templates.yml.\nRun 'slopctl templates --update' to refresh the templates.",
                agent
            )),
            | (true, true) | (false, false) => Ok(())
        }
    }

    /// Fails unless an agent is known to both catalogs
    ///
    /// # Errors
    ///
    /// Returns an error listing the available agents when the agent is unknown, or the cross-check
    /// error when it is known to only one catalog.
    pub fn require_known_agent(&self, agent: &str) -> Result<()>
    {
        self.require_agent_consistent(agent)?;
        require!(
            self.in_templates(agent) == true,
            Err(anyhow::anyhow!("Agent '{}' not found in templates.yml.\nAvailable agents: {}", agent, self.available_agents().join(", ")))
        );
        Ok(())
    }

    /// Sorted names of all agents in the templates catalog, including overlays
    fn available_agents(&self) -> Vec<&str>
    {
        let mut names: Vec<&str> = self.templates.agents.keys().map(String::as_str).collect();
        names.sort_unstable();
        names
    }

    /// Describes every agent that is known to only one of the two catalogs
    pub fn catalog_mismatches(&self) -> Vec<String>
    {
        let mut mismatches = Vec::new();
        for name in self.available_agents()
        {
            if self.in_defaults(name) == false
            {
                mismatches.push(format!("Agent '{}' is in templates.yml but missing from agent-defaults.yml", name));
            }
        }
        let mut default_names: Vec<&str> = self.agents.agents.iter().map(|entry| entry.name.as_str()).collect();
        default_names.sort_unstable();
        for name in default_names
        {
            if self.in_templates(name) == false
            {
                mismatches.push(format!("Agent '{}' is in agent-defaults.yml but missing from templates.yml", name));
            }
        }
        mismatches
    }
}

#[cfg(test)]
mod tests
{
    use tempfile::TempDir;

    use super::*;

    const SHIPPED_TEMPLATES: &str = "version: 5\nagents:\n  bogus: {}\nlanguages: {}\n";
    const SHIPPED_DEFAULTS: &str = "version: 1\nagents:\n  - name: bogus\n    markers: [.bogus]\n    prompt_dir: $workspace/.bogus/prompts\n    skill_dir: \
                                    $workspace/.bogus/skills\n    reads_cross_client_skills: true\n";
    const FAKE_OVERLAY: &str = "markers: [.fake]\nprompt_dir: $workspace/.fake/prompts\nskill_dir: $workspace/.fake/skills\nreads_cross_client_skills: \
                                false\ninstructions:\n  - source: instructions.md\n    target: $workspace/FAKE.md\nskills:\n  - source: skills/helper/\n";

    fn shipped_cache() -> TempDir
    {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("templates.yml"), SHIPPED_TEMPLATES).unwrap();
        fs::write(dir.path().join("agent-defaults.yml"), SHIPPED_DEFAULTS).unwrap();
        dir
    }

    fn write_overlay(root: &Path, name: &str, content: &str)
    {
        let dir = root.join(name);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(AGENT_OVERLAY_FILE), content).unwrap();
    }

    fn parse(content: &str, scope: ConfigScope) -> Result<(AgentDefaultsEntry, AgentConfig)>
    {
        parse_overlay(content, "fake", Path::new("/tmp/overlay/fake"), scope)
    }

    #[test]
    fn test_load_effective_catalogs_no_overlays_returns_shipped()
    {
        let cache = shipped_cache();
        let ws = TempDir::new().unwrap();
        let catalogs = load_effective_catalogs_from(cache.path(), None, &ws.path().join(".slopctl/agents"), false).unwrap();
        assert_eq!(catalogs.agents.agents.len(), 1);
        assert_eq!(catalogs.templates.agents.len(), 1);
        assert!(catalogs.overlays.is_empty() == true);
    }

    #[test]
    fn test_load_effective_catalogs_global_overlay_adds_agent()
    {
        let cache = shipped_cache();
        let global = TempDir::new().unwrap();
        let ws = TempDir::new().unwrap();
        write_overlay(global.path(), "fake", FAKE_OVERLAY);
        let catalogs = load_effective_catalogs_from(cache.path(), Some(global.path()), &ws.path().join("agents"), false).unwrap();
        assert!(catalogs.templates.agents.contains_key("fake") == true);
        assert!(catalogs.in_defaults("fake") == true);
        assert_eq!(catalogs.overlay("fake").unwrap().scope, ConfigScope::Global);
    }

    #[test]
    fn test_load_effective_catalogs_workspace_overlay_adds_agent()
    {
        let cache = shipped_cache();
        let ws = TempDir::new().unwrap();
        let root = ws.path().join("agents");
        write_overlay(&root, "fake", FAKE_OVERLAY);
        let catalogs = load_effective_catalogs_from(cache.path(), None, &root, false).unwrap();
        assert_eq!(catalogs.overlay("fake").unwrap().scope, ConfigScope::Workspace);
        assert!(catalogs.require_known_agent("fake").is_ok() == true);
    }

    #[test]
    fn test_load_effective_catalogs_workspace_overlay_shadows_global()
    {
        let cache = shipped_cache();
        let global = TempDir::new().unwrap();
        let ws = TempDir::new().unwrap();
        write_overlay(global.path(), "fake", FAKE_OVERLAY);
        write_overlay(ws.path(), "fake", &FAKE_OVERLAY.replace("FAKE.md", "WS.md"));
        let catalogs = load_effective_catalogs_from(cache.path(), Some(global.path()), ws.path(), false).unwrap();
        assert_eq!(catalogs.overlays.len(), 1);
        assert_eq!(catalogs.overlays[0].scope, ConfigScope::Workspace);
        assert!(catalogs.templates.agents["fake"].instructions[0].target.ends_with("WS.md") == true);
    }

    #[test]
    fn test_load_effective_catalogs_overlay_clashes_with_shipped_errors()
    {
        let cache = shipped_cache();
        let ws = TempDir::new().unwrap();
        write_overlay(ws.path(), "bogus", FAKE_OVERLAY);
        let err = load_effective_catalogs_from(cache.path(), None, ws.path(), false).unwrap_err().to_string();
        assert!(err.contains("add-only") == true);
    }

    #[test]
    fn test_load_effective_catalogs_lenient_missing_files_yield_empty()
    {
        let dir = TempDir::new().unwrap();
        let ws = TempDir::new().unwrap();
        assert!(load_effective_catalogs_from(dir.path(), None, ws.path(), false).is_err() == true);
        let catalogs = load_effective_catalogs_from(dir.path(), None, ws.path(), true).unwrap();
        assert!(catalogs.templates.agents.is_empty() == true);
        assert!(catalogs.agents.agents.is_empty() == true);
    }

    #[test]
    fn test_load_overlay_root_absent_dir_returns_empty()
    {
        let ws = TempDir::new().unwrap();
        assert!(load_overlay_root(&ws.path().join("missing"), ConfigScope::Workspace).unwrap().is_empty() == true);
    }

    #[test]
    fn test_load_overlay_root_missing_agent_yml_errors()
    {
        let ws = TempDir::new().unwrap();
        fs::create_dir_all(ws.path().join("fake")).unwrap();
        assert!(load_overlay_root(ws.path(), ConfigScope::Workspace).is_err() == true);
    }

    #[test]
    fn test_parse_overlay_missing_name_uses_directory_name()
    {
        let (defaults, _) = parse(FAKE_OVERLAY, ConfigScope::Workspace).unwrap();
        assert_eq!(defaults.name, "fake");
    }

    #[test]
    fn test_parse_overlay_name_mismatch_errors()
    {
        assert!(parse(&format!("name: other\n{}", FAKE_OVERLAY), ConfigScope::Workspace).is_err() == true);
    }

    #[test]
    fn test_parse_overlay_unknown_key_errors()
    {
        let err = parse(&format!("{}skils: []\n", FAKE_OVERLAY), ConfigScope::Workspace).unwrap_err().to_string();
        assert!(err.contains("skils") == true);
    }

    #[test]
    fn test_parse_overlay_missing_markers_errors()
    {
        let content = FAKE_OVERLAY.replace("markers: [.fake]\n", "");
        assert!(parse(&content, ConfigScope::Workspace).is_err() == true);
    }

    #[test]
    fn test_parse_overlay_invalid_skill_dir_placeholder_errors()
    {
        let content = FAKE_OVERLAY.replace("$workspace/.fake/skills", "/etc/skills");
        assert!(parse(&content, ConfigScope::Global).is_err() == true);
    }

    #[test]
    fn test_parse_overlay_url_source_errors()
    {
        let content = FAKE_OVERLAY.replace("skills/helper/", "https://example.com/skill");
        assert!(parse(&content, ConfigScope::Workspace).is_err() == true);
    }

    #[test]
    fn test_parse_overlay_parent_dir_source_errors()
    {
        let content = FAKE_OVERLAY.replace("instructions.md", "../secret.md");
        assert!(parse(&content, ConfigScope::Workspace).is_err() == true);
    }

    #[test]
    fn test_parse_overlay_absolute_source_errors()
    {
        let content = FAKE_OVERLAY.replace("instructions.md", "/etc/passwd");
        assert!(parse(&content, ConfigScope::Workspace).is_err() == true);
    }

    #[test]
    fn test_parse_overlay_relative_sources_rebased_absolute()
    {
        let (_, config) = parse(FAKE_OVERLAY, ConfigScope::Workspace).unwrap();
        let source = Path::new(&config.instructions[0].source);
        assert!(source.is_absolute() == true);
        assert!(source.ends_with("fake/instructions.md") == true);
    }

    #[test]
    fn test_parse_overlay_skill_trailing_separator_derives_name()
    {
        let (_, config) = parse(FAKE_OVERLAY, ConfigScope::Workspace).unwrap();
        assert_eq!(config.skills[0].derive_name(), "helper");
    }

    #[test]
    fn test_parse_workspace_overlay_userprofile_target_errors()
    {
        let content = FAKE_OVERLAY.replace("$workspace/FAKE.md", "$userprofile/FAKE.md");
        assert!(parse(&content, ConfigScope::Workspace).is_err() == true);
        assert!(parse(&content, ConfigScope::Global).is_ok() == true);
    }

    #[test]
    fn test_agent_label_marks_overlay_origin()
    {
        let cache = shipped_cache();
        let ws = TempDir::new().unwrap();
        write_overlay(ws.path(), "fake", FAKE_OVERLAY);
        let catalogs = load_effective_catalogs_from(cache.path(), None, ws.path(), false).unwrap();
        assert_eq!(catalogs.agent_label("bogus"), "bogus");
        assert_eq!(catalogs.agent_label("fake"), "fake (overlay: workspace)");
    }

    #[test]
    fn test_require_known_agent_unknown_lists_overlay_names()
    {
        let cache = shipped_cache();
        let ws = TempDir::new().unwrap();
        write_overlay(ws.path(), "fake", FAKE_OVERLAY);
        let catalogs = load_effective_catalogs_from(cache.path(), None, ws.path(), false).unwrap();
        let err = catalogs.require_known_agent("nope").unwrap_err().to_string();
        assert!(err.contains("bogus, fake") == true);
    }

    #[test]
    fn test_require_agent_consistent_templates_only_errors()
    {
        let cache = shipped_cache();
        let mut catalogs = load_effective_catalogs_from(cache.path(), None, Path::new("/nonexistent"), false).unwrap();
        catalogs.templates.agents.insert("fake".to_string(), AgentConfig::default());
        let err = catalogs.require_agent_consistent("fake").unwrap_err().to_string();
        assert!(err.contains("agents --update") == true);
        assert!(catalogs.require_known_agent("fake").is_err() == true);
    }

    #[test]
    fn test_require_agent_consistent_defaults_only_errors()
    {
        let cache = shipped_cache();
        let mut catalogs = load_effective_catalogs_from(cache.path(), None, Path::new("/nonexistent"), false).unwrap();
        catalogs.templates.agents.clear();
        assert!(catalogs.require_agent_consistent("bogus").is_err() == true);
    }

    #[test]
    fn test_require_agent_consistent_unknown_ok()
    {
        let cache = shipped_cache();
        let catalogs = load_effective_catalogs_from(cache.path(), None, Path::new("/nonexistent"), false).unwrap();
        assert!(catalogs.require_agent_consistent("nope").is_ok() == true);
    }

    #[test]
    fn test_catalog_mismatches_reports_both_directions()
    {
        let cache = shipped_cache();
        let mut catalogs = load_effective_catalogs_from(cache.path(), None, Path::new("/nonexistent"), false).unwrap();
        catalogs.templates.agents.insert("fake".to_string(), AgentConfig::default());
        catalogs.agents.agents.push(AgentDefaultsEntry {
            name:                      "extra".to_string(),
            markers:                   vec![".extra".to_string()],
            prompt_dir:                "$workspace/.extra".to_string(),
            skill_dir:                 "$workspace/.extra".to_string(),
            userprofile_skill_dir:     None,
            reads_cross_client_skills: true
        });
        let mismatches = catalogs.catalog_mismatches();
        assert_eq!(mismatches.len(), 2);
        assert!(mismatches[0].contains("'fake'") == true);
        assert!(mismatches[1].contains("'extra'") == true);
    }
}
