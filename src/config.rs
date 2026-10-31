//! Configuration management for slopctl
//!
//! Supports two scopes:
//! - **Workspace** — `<workspace>/.slopctl/config.yml` (next to `tracker.yml`)
//! - **Global** — `$XDG_CONFIG_HOME/slopctl/config.yml` or `$HOME/.config/slopctl/config.yml`
//!
//! Consumer commands read an *effective* config that merges both scopes with
//! per-key Git-style precedence: workspace wins, global is the fallback.

use std::{
    collections::{BTreeMap, HashMap},
    env, fmt, fs,
    path::{Path, PathBuf}
};

use serde::{Deserialize, Serialize};

use crate::{Result, file_tracker::SLOPCTL_DIR};

/// Which configuration file an operation targets
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigScope
{
    Global,
    Workspace
}

impl fmt::Display for ConfigScope
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            | Self::Global => write!(f, "global"),
            | Self::Workspace => write!(f, "workspace")
        }
    }
}

/// Configuration structure for slopctl
///
/// Uses dotted keys following the convention `<command>.<parameter>`,
/// e.g. `templates.uri`, `merge.provider`.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config
{
    #[serde(default)]
    pub templates: TemplatesConfig,
    #[serde(default)]
    pub agents:    AgentsConfig,
    #[serde(default)]
    pub models:    ModelsConfig,
    #[serde(default)]
    pub merge:     MergeConfig
}

/// Configuration for the `templates` command
///
/// `uri` and `fallback_uri` may be either a remote URL (e.g.
/// `https://github.com/owner/repo/tree/branch/templates`) or a local
/// filesystem path (e.g. `/path/to/templates` or `~/work/templates`).
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct TemplatesConfig
{
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri:          Option<String>,
    #[serde(rename = "fallbackUri", skip_serializing_if = "Option::is_none")]
    pub fallback_uri: Option<String>,
    /// Branch or tag of a git `uri` (the remote's default branch when unset)
    #[serde(rename = "ref", default, skip_serializing_if = "Option::is_none")]
    pub git_ref:      Option<String>,
    /// Subdirectory of `uri` holding `templates.yml`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path:         Option<String>,
    /// Additional template systems stacked on the baseline, in precedence order (later wins)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub layers:       Vec<LayerSpec>
}

/// One additional template system (layer) configured under `templates.layers`
///
/// `uri` accepts everything `templates.uri` does (local path, GitHub tree URL) plus git URLs.
/// `ref` and `path` select a branch/tag and a subdirectory; `ref` applies to git URLs only.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayerSpec
{
    pub id:      String,
    pub uri:     String,
    #[serde(rename = "ref", default, skip_serializing_if = "Option::is_none")]
    pub git_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path:    Option<String>
}

/// Config key prefix for template layers: `templates.layers.<id>.<field>`
const LAYER_KEY_PREFIX: &str = "templates.layers.";

/// Layer fields settable through `templates.layers.<id>.<field>`
const LAYER_FIELDS: [&str; 3] = ["uri", "ref", "path"];

/// Returns true when `id` is a valid template system id (`[a-z0-9_-]+`)
pub fn is_valid_system_id(id: &str) -> bool
{
    id.is_empty() == false && id.bytes().all(|b| b.is_ascii_lowercase() == true || b.is_ascii_digit() == true || b == b'-' || b == b'_')
}

/// Splits `templates.layers.<id>[.<field>]` into `(id, field)`; `None` for any other key
fn parse_layer_key(key: &str) -> Option<(&str, Option<&str>)>
{
    let rest = key.strip_prefix(LAYER_KEY_PREFIX)?;
    match rest.split_once('.')
    {
        | Some((id, field)) => Some((id, Some(field))),
        | None => Some((rest, None))
    }
}

/// Configuration for the `agents` command
///
/// `uri` and `fallback_uri` may be either a remote URL or a local filesystem path.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct AgentsConfig
{
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri:          Option<String>,
    #[serde(rename = "fallbackUri", skip_serializing_if = "Option::is_none")]
    pub fallback_uri: Option<String>
}

/// Configuration for the `models` command
///
/// `uri` and `fallback_uri` may be either a remote URL or a local filesystem path.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ModelsConfig
{
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri:          Option<String>,
    #[serde(rename = "fallbackUri", skip_serializing_if = "Option::is_none")]
    pub fallback_uri: Option<String>
}

/// Merge-related configuration for AI-assisted merging
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct MergeConfig
{
    /// LLM provider name (openai, anthropic, ollama, mistral)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    /// Model identifier (e.g. gpt-4o, claude-sonnet-4-20250514, llama3)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model:    Option<String>
}

impl Config
{
    // ── path helpers ──────────────────────────────────────────────

    /// Returns the **global** slopctl config directory
    ///
    /// Uses `$XDG_CONFIG_HOME/slopctl` if XDG_CONFIG_HOME is set,
    /// otherwise falls back to `$HOME/.config/slopctl`
    pub fn get_global_dir() -> Result<PathBuf>
    {
        let config_dir = if let Ok(xdg_config) = env::var("XDG_CONFIG_HOME")
        {
            PathBuf::from(xdg_config)
        }
        else if let Some(home) = dirs::home_dir()
        {
            home.join(".config")
        }
        else
        {
            return Err(anyhow::anyhow!("Could not determine config directory"));
        };

        Ok(config_dir.join("slopctl"))
    }

    /// Returns the path to the **global** config file (`config.yml` inside [`Self::get_global_dir`])
    pub fn get_global_path() -> Result<PathBuf>
    {
        Ok(Self::get_global_dir()?.join("config.yml"))
    }

    /// Returns the path to the **workspace-local** config file
    pub fn get_workspace_path(workspace: &Path) -> PathBuf
    {
        workspace.join(SLOPCTL_DIR).join("config.yml")
    }

    /// Compatibility shim — delegates to [`Self::get_global_path`]
    pub fn get_config_path() -> Result<PathBuf>
    {
        Self::get_global_path()
    }

    // ── load / save (global) ──────────────────────────────────────

    /// Load the global configuration file (returns default if missing)
    pub fn load_global() -> Result<Self>
    {
        let path = Self::get_global_path()?;
        Self::load_from(&path)
    }

    /// Save to the global configuration file
    pub fn save_global(&self) -> Result<()>
    {
        let path = Self::get_global_path()?;
        Self::save_to(self, &path)
    }

    // ── load / save (workspace) ───────────────────────────────────

    /// Load the workspace-local configuration file (returns default if missing)
    pub fn load_workspace(workspace: &Path) -> Result<Self>
    {
        let path = Self::get_workspace_path(workspace);
        Self::load_from(&path)
    }

    /// Save to the workspace-local configuration file
    pub fn save_workspace(&self, workspace: &Path) -> Result<()>
    {
        let path = Self::get_workspace_path(workspace);
        Self::save_to(self, &path)
    }

    // ── compatibility shims ───────────────────────────────────────

    /// Compatibility shim — delegates to [`Self::load_global`]
    pub fn load() -> Result<Self>
    {
        Self::load_global()
    }

    /// Compatibility shim — delegates to [`Self::save_global`]
    pub fn save(&self) -> Result<()>
    {
        self.save_global()
    }

    // ── internal helpers ──────────────────────────────────────────

    fn load_from(path: &Path) -> Result<Self>
    {
        require!(path.exists() == true, Ok(Self::default()));

        let content = fs::read_to_string(path)?;
        let config: Config = serde_yaml::from_str(&content)?;
        Ok(config)
    }

    fn save_to(&self, path: &Path) -> Result<()>
    {
        if let Some(parent) = path.parent()
        {
            fs::create_dir_all(parent)?;
        }

        let content = serde_yaml::to_string(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    /// Get a value by dotted key (e.g., "templates.uri")
    ///
    /// Returns None if key doesn't exist or path is invalid
    pub fn get(&self, key: &str) -> Option<String>
    {
        if let Some((id, field)) = parse_layer_key(key)
        {
            let layer = self.templates.layers.iter().find(|layer| layer.id == id)?;
            // The bare `templates.layers.<id>` key reads as the layer's uri so it can be deleted as a whole.
            return match field.unwrap_or("uri")
            {
                | "uri" => Some(layer.uri.clone()),
                | "ref" => layer.git_ref.clone(),
                | "path" => layer.path.clone(),
                | _ => None
            };
        }

        match key
        {
            | "templates.uri" => self.templates.uri.clone(),
            | "templates.fallbackUri" => self.templates.fallback_uri.clone(),
            | "templates.ref" => self.templates.git_ref.clone(),
            | "templates.path" => self.templates.path.clone(),
            | "agents.uri" => self.agents.uri.clone(),
            | "agents.fallbackUri" => self.agents.fallback_uri.clone(),
            | "models.uri" => self.models.uri.clone(),
            | "models.fallbackUri" => self.models.fallback_uri.clone(),
            | "merge.provider" => self.merge.provider.clone(),
            | "merge.model" => self.merge.model.clone(),
            | _ => None
        }
    }

    /// Set a value by dotted key (e.g., "templates.uri")
    ///
    /// Returns error if key is not recognized
    pub fn set(&mut self, key: &str, value: &str) -> Result<()>
    {
        if let Some((id, field)) = parse_layer_key(key)
        {
            return self.set_layer_field(key, id, field, value);
        }

        match key
        {
            | "templates.uri" =>
            {
                self.templates.uri = Some(value.to_string());
                Ok(())
            }
            | "templates.fallbackUri" =>
            {
                self.templates.fallback_uri = Some(value.to_string());
                Ok(())
            }
            | "templates.ref" =>
            {
                self.templates.git_ref = Some(value.to_string());
                Ok(())
            }
            | "templates.path" =>
            {
                self.templates.path = Some(value.to_string());
                Ok(())
            }
            | "agents.uri" =>
            {
                self.agents.uri = Some(value.to_string());
                Ok(())
            }
            | "agents.fallbackUri" =>
            {
                self.agents.fallback_uri = Some(value.to_string());
                Ok(())
            }
            | "models.uri" =>
            {
                self.models.uri = Some(value.to_string());
                Ok(())
            }
            | "models.fallbackUri" =>
            {
                self.models.fallback_uri = Some(value.to_string());
                Ok(())
            }
            | "merge.provider" =>
            {
                self.merge.provider = Some(value.to_string());
                Ok(())
            }
            | "merge.model" =>
            {
                self.merge.model = Some(value.to_string());
                Ok(())
            }
            | _ => Err(anyhow::anyhow!("Unknown config key: {}", key))
        }
    }

    /// Unset (remove) a value by dotted key
    ///
    /// Returns error if key is not recognized
    pub fn unset(&mut self, key: &str) -> Result<()>
    {
        if let Some((id, field)) = parse_layer_key(key)
        {
            let position = self.templates.layers.iter().position(|layer| layer.id == id).ok_or_else(|| anyhow::anyhow!("No template layer '{}' configured", id))?;
            match field
            {
                | None | Some("uri") =>
                {
                    self.templates.layers.remove(position);
                }
                | Some("ref") => self.templates.layers[position].git_ref = None,
                | Some("path") => self.templates.layers[position].path = None,
                | Some(_) => return Err(anyhow::anyhow!("Unknown config key: {}", key))
            }
            return Ok(());
        }

        match key
        {
            | "templates.uri" =>
            {
                self.templates.uri = None;
                Ok(())
            }
            | "templates.fallbackUri" =>
            {
                self.templates.fallback_uri = None;
                Ok(())
            }
            | "templates.ref" =>
            {
                self.templates.git_ref = None;
                Ok(())
            }
            | "templates.path" =>
            {
                self.templates.path = None;
                Ok(())
            }
            | "agents.uri" =>
            {
                self.agents.uri = None;
                Ok(())
            }
            | "agents.fallbackUri" =>
            {
                self.agents.fallback_uri = None;
                Ok(())
            }
            | "models.uri" =>
            {
                self.models.uri = None;
                Ok(())
            }
            | "models.fallbackUri" =>
            {
                self.models.fallback_uri = None;
                Ok(())
            }
            | "merge.provider" =>
            {
                self.merge.provider = None;
                Ok(())
            }
            | "merge.model" =>
            {
                self.merge.model = None;
                Ok(())
            }
            | _ => Err(anyhow::anyhow!("Unknown config key: {}", key))
        }
    }

    /// List all configuration values as key-value pairs
    ///
    /// Returns a HashMap of dotted keys to their values
    pub fn list(&self) -> HashMap<String, String>
    {
        let mut values = HashMap::new();

        if let Some(uri) = &self.templates.uri
        {
            values.insert("templates.uri".to_string(), uri.clone());
        }

        if let Some(fallback_uri) = &self.templates.fallback_uri
        {
            values.insert("templates.fallbackUri".to_string(), fallback_uri.clone());
        }

        if let Some(git_ref) = &self.templates.git_ref
        {
            values.insert("templates.ref".to_string(), git_ref.clone());
        }

        if let Some(path) = &self.templates.path
        {
            values.insert("templates.path".to_string(), path.clone());
        }

        if let Some(uri) = &self.agents.uri
        {
            values.insert("agents.uri".to_string(), uri.clone());
        }

        if let Some(fallback_uri) = &self.agents.fallback_uri
        {
            values.insert("agents.fallbackUri".to_string(), fallback_uri.clone());
        }

        if let Some(uri) = &self.models.uri
        {
            values.insert("models.uri".to_string(), uri.clone());
        }

        if let Some(fallback_uri) = &self.models.fallback_uri
        {
            values.insert("models.fallbackUri".to_string(), fallback_uri.clone());
        }

        if let Some(provider) = &self.merge.provider
        {
            values.insert("merge.provider".to_string(), provider.clone());
        }

        if let Some(model) = &self.merge.model
        {
            values.insert("merge.model".to_string(), model.clone());
        }

        for layer in &self.templates.layers
        {
            values.insert(format!("{}{}.uri", LAYER_KEY_PREFIX, layer.id), layer.uri.clone());
            if let Some(git_ref) = &layer.git_ref
            {
                values.insert(format!("{}{}.ref", LAYER_KEY_PREFIX, layer.id), git_ref.clone());
            }
            if let Some(path) = &layer.path
            {
                values.insert(format!("{}{}.path", LAYER_KEY_PREFIX, layer.id), path.clone());
            }
        }

        values
    }

    /// Sets one field of a template layer; `uri` on an unknown id appends a new layer
    fn set_layer_field(&mut self, key: &str, id: &str, field: Option<&str>, value: &str) -> Result<()>
    {
        let field = field.ok_or_else(|| anyhow::anyhow!("Config key '{}' needs a field: {}", key, LAYER_FIELDS.join(", ")))?;
        require!(LAYER_FIELDS.contains(&field) == true, Err(anyhow::anyhow!("Unknown config key: {}", key)));
        require!(is_valid_system_id(id) == true, Err(anyhow::anyhow!("Invalid template layer id '{}': use lowercase letters, digits, '-' and '_'", id)));

        let position = match self.templates.layers.iter().position(|layer| layer.id == id)
        {
            | Some(position) => position,
            | None =>
            {
                require!(field == "uri", Err(anyhow::anyhow!("Template layer '{}' does not exist; set {}{}.uri first", id, LAYER_KEY_PREFIX, id)));
                self.templates.layers.push(LayerSpec { id: id.to_string(), ..Default::default() });
                self.templates.layers.len() - 1
            }
        };

        let layer = &mut self.templates.layers[position];
        match field
        {
            | "uri" => layer.uri = value.to_string(),
            | "ref" => layer.git_ref = Some(value.to_string()),
            | _ => layer.path = Some(value.to_string())
        }
        Ok(())
    }

    /// Get list of all valid config keys
    pub fn valid_keys() -> Vec<&'static str>
    {
        vec![
            "templates.uri", "templates.fallbackUri", "templates.ref", "templates.path", "templates.layers.<id>.uri", "templates.layers.<id>.ref",
            "templates.layers.<id>.path", "agents.uri", "agents.fallbackUri", "models.uri", "models.fallbackUri", "merge.provider", "merge.model",
        ]
    }
}

/// The baseline template source resolved from config (see [`EffectiveConfig::baseline_source`])
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaselineSource
{
    pub uri:     String,
    pub git_ref: Option<String>,
    pub path:    Option<String>
}

/// Merged view of workspace + global config with per-key precedence
///
/// For every key the workspace value wins; if not set there, the global
/// value is returned.  Consumer commands (`templates --update`, `merge`,
/// `init`, `list-models`) use this to read configuration.
pub struct EffectiveConfig
{
    pub workspace: Config,
    pub global:    Config
}

impl EffectiveConfig
{
    /// Load both workspace and global config files
    pub fn load(workspace: &Path) -> Result<Self>
    {
        let ws = Config::load_workspace(workspace)?;
        let gl = Config::load_global()?;
        Ok(Self { workspace: ws, global: gl })
    }

    /// Get a value with workspace-wins-over-global precedence
    pub fn get(&self, key: &str) -> Option<String>
    {
        self.workspace.get(key).or_else(|| self.global.get(key))
    }

    /// Get a value together with its origin scope
    pub fn get_with_origin(&self, key: &str) -> Option<(String, ConfigScope)>
    {
        if let Some(v) = self.workspace.get(key)
        {
            Some((v, ConfigScope::Workspace))
        }
        else
        {
            self.global.get(key).map(|v| (v, ConfigScope::Global))
        }
    }

    /// The configured baseline template source: `templates.uri` with its `ref` and `path`
    ///
    /// `ref` and `path` are read from the same scope as `uri`, so a workspace uri is never combined
    /// with a global path meant for a different source. `None` when no `templates.uri` is set.
    pub fn baseline_source(&self) -> Option<BaselineSource>
    {
        let (uri, scope) = self.get_with_origin("templates.uri")?;
        let config = match scope
        {
            | ConfigScope::Workspace => &self.workspace,
            | ConfigScope::Global => &self.global
        };
        Some(BaselineSource { uri, git_ref: config.templates.git_ref.clone(), path: config.templates.path.clone() })
    }

    /// Configured template layers in precedence order: global layers, then workspace layers
    ///
    /// Unlike scalar keys, layers from both scopes are concatenated rather than shadowed.
    ///
    /// # Errors
    ///
    /// Returns an error if a layer id is invalid, a layer has no uri, or the same id is configured
    /// in both scopes (the layer cache is keyed by id)
    pub fn layers(&self) -> Result<Vec<(LayerSpec, ConfigScope)>>
    {
        let mut layers: Vec<(LayerSpec, ConfigScope)> = Vec::new();
        for (scope, config) in [(ConfigScope::Global, &self.global), (ConfigScope::Workspace, &self.workspace)]
        {
            for layer in &config.templates.layers
            {
                require!(is_valid_system_id(&layer.id) == true, Err(anyhow::anyhow!("Invalid template layer id '{}' in {} config", layer.id, scope)));
                require!(layer.uri.is_empty() == false, Err(anyhow::anyhow!("Template layer '{}' in {} config has no uri", layer.id, scope)));
                require!(
                    layers.iter().any(|(existing, _)| existing.id == layer.id) == false,
                    Err(anyhow::anyhow!("Template layer '{}' is configured more than once (global and workspace); ids must be unique", layer.id))
                );
                layers.push((layer.clone(), scope));
            }
        }
        Ok(layers)
    }

    /// Merged list of all set keys with their origin scope (deterministic order)
    pub fn list_with_origin(&self) -> BTreeMap<String, (String, ConfigScope)>
    {
        let mut map = BTreeMap::new();

        for (k, v) in self.global.list()
        {
            map.insert(k, (v, ConfigScope::Global));
        }
        for (k, v) in self.workspace.list()
        {
            map.insert(k, (v, ConfigScope::Workspace));
        }

        map
    }
}

#[cfg(test)]
mod tests
{
    use std::sync::Mutex;

    use super::*;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn test_config_default()
    {
        let config = Config::default();
        assert!(config.templates.uri.is_none() == true);
        assert!(config.templates.fallback_uri.is_none() == true);
        assert!(config.agents.uri.is_none() == true);
        assert!(config.agents.fallback_uri.is_none() == true);
    }

    #[test]
    fn test_config_get_set_uri() -> anyhow::Result<()>
    {
        let mut config = Config::default();
        config.set("templates.uri", "https://example.com")?;
        assert_eq!(config.get("templates.uri").ok_or_else(|| anyhow::anyhow!("templates.uri not set"))?, "https://example.com");
        Ok(())
    }

    #[test]
    fn test_config_get_set_fallback_uri() -> anyhow::Result<()>
    {
        let mut config = Config::default();
        config.set("templates.fallbackUri", "https://fallback.com")?;
        assert_eq!(config.get("templates.fallbackUri").ok_or_else(|| anyhow::anyhow!("templates.fallbackUri not set"))?, "https://fallback.com");
        Ok(())
    }

    #[test]
    fn test_config_get_set_agents_uri() -> anyhow::Result<()>
    {
        let mut config = Config::default();
        config.set("agents.uri", "https://example.com/agents")?;
        assert_eq!(config.get("agents.uri").ok_or_else(|| anyhow::anyhow!("agents.uri not set"))?, "https://example.com/agents");
        Ok(())
    }

    #[test]
    fn test_config_get_set_agents_fallback_uri() -> anyhow::Result<()>
    {
        let mut config = Config::default();
        config.set("agents.fallbackUri", "https://fallback.com/agents")?;
        assert_eq!(config.get("agents.fallbackUri").ok_or_else(|| anyhow::anyhow!("agents.fallbackUri not set"))?, "https://fallback.com/agents");
        Ok(())
    }

    #[test]
    fn test_config_uri_accepts_local_path() -> anyhow::Result<()>
    {
        let mut config = Config::default();
        config.set("templates.uri", "/local/path/to/templates")?;
        assert_eq!(config.get("templates.uri").ok_or_else(|| anyhow::anyhow!("templates.uri not set"))?, "/local/path/to/templates");
        Ok(())
    }

    #[test]
    fn test_config_get_unknown_key()
    {
        let config = Config::default();
        assert!(config.get("unknown.key").is_none() == true);
    }

    #[test]
    fn test_config_set_unknown_key()
    {
        let mut config = Config::default();
        let err = config.set("unknown.key", "value").unwrap_err();
        assert!(err.to_string().contains("Unknown config key") == true);
    }

    #[test]
    fn test_config_unset_uri() -> anyhow::Result<()>
    {
        let mut config = Config::default();
        config.set("templates.uri", "https://example.com")?;
        config.unset("templates.uri")?;
        assert!(config.get("templates.uri").is_none() == true);
        Ok(())
    }

    #[test]
    fn test_config_unset_fallback_uri() -> anyhow::Result<()>
    {
        let mut config = Config::default();
        config.set("templates.fallbackUri", "https://fallback.com")?;
        config.unset("templates.fallbackUri")?;
        assert!(config.get("templates.fallbackUri").is_none() == true);
        Ok(())
    }

    #[test]
    fn test_config_unset_agents_uri() -> anyhow::Result<()>
    {
        let mut config = Config::default();
        config.set("agents.uri", "https://example.com/agents")?;
        config.unset("agents.uri")?;
        assert!(config.get("agents.uri").is_none() == true);
        Ok(())
    }

    #[test]
    fn test_config_unset_unknown_key()
    {
        let mut config = Config::default();
        let err = config.unset("unknown.key").unwrap_err();
        assert!(err.to_string().contains("Unknown config key") == true);
    }

    #[test]
    fn test_config_list_empty()
    {
        let config = Config::default();
        assert!(config.list().is_empty() == true);
    }

    #[test]
    fn test_config_list_populated() -> anyhow::Result<()>
    {
        let mut config = Config::default();
        config.set("templates.uri", "https://example.com")?;
        config.set("templates.fallbackUri", "https://fallback.com")?;

        let values = config.list();
        assert_eq!(values.len(), 2);
        assert_eq!(values.get("templates.uri").ok_or_else(|| anyhow::anyhow!("templates.uri not in list"))?, "https://example.com");
        assert_eq!(values.get("templates.fallbackUri").ok_or_else(|| anyhow::anyhow!("templates.fallbackUri not in list"))?, "https://fallback.com");
        Ok(())
    }

    #[test]
    fn test_config_valid_keys()
    {
        let keys = Config::valid_keys();
        assert_eq!(keys, vec![
            "templates.uri", "templates.fallbackUri", "templates.ref", "templates.path", "templates.layers.<id>.uri", "templates.layers.<id>.ref",
            "templates.layers.<id>.path", "agents.uri", "agents.fallbackUri", "models.uri", "models.fallbackUri", "merge.provider", "merge.model"
        ]);
    }

    #[test]
    fn test_config_get_set_merge_provider() -> anyhow::Result<()>
    {
        let mut config = Config::default();
        config.set("merge.provider", "openai")?;
        assert_eq!(config.get("merge.provider").ok_or_else(|| anyhow::anyhow!("merge.provider not set"))?, "openai");
        Ok(())
    }

    #[test]
    fn test_config_get_set_merge_model() -> anyhow::Result<()>
    {
        let mut config = Config::default();
        config.set("merge.model", "gpt-4o")?;
        assert_eq!(config.get("merge.model").ok_or_else(|| anyhow::anyhow!("merge.model not set"))?, "gpt-4o");
        Ok(())
    }

    #[test]
    fn test_config_unset_merge_provider() -> anyhow::Result<()>
    {
        let mut config = Config::default();
        config.set("merge.provider", "anthropic")?;
        config.unset("merge.provider")?;
        assert!(config.get("merge.provider").is_none() == true);
        Ok(())
    }

    #[test]
    fn test_config_list_includes_merge() -> anyhow::Result<()>
    {
        let mut config = Config::default();
        config.set("merge.provider", "openai")?;
        config.set("merge.model", "gpt-4o")?;

        let values = config.list();
        assert_eq!(values.get("merge.provider").ok_or_else(|| anyhow::anyhow!("merge.provider not in list"))?, "openai");
        assert_eq!(values.get("merge.model").ok_or_else(|| anyhow::anyhow!("merge.model not in list"))?, "gpt-4o");
        Ok(())
    }

    #[test]
    fn test_config_serde_round_trip() -> anyhow::Result<()>
    {
        let mut config = Config::default();
        config.set("templates.uri", "https://example.com")?;

        let yaml = serde_yaml::to_string(&config)?;
        let loaded: Config = serde_yaml::from_str(&yaml)?;
        assert_eq!(loaded.get("templates.uri").ok_or_else(|| anyhow::anyhow!("templates.uri not set"))?, "https://example.com");
        assert!(loaded.get("templates.fallbackUri").is_none() == true);
        Ok(())
    }

    #[test]
    fn test_config_serde_uses_camel_case_for_fallback_uri() -> anyhow::Result<()>
    {
        let mut config = Config::default();
        config.set("templates.fallbackUri", "https://fallback.com")?;
        let yaml = serde_yaml::to_string(&config)?;
        assert!(yaml.contains("fallbackUri:") == true, "expected serialized YAML to use camelCase key, got: {}", yaml);
        Ok(())
    }

    #[test]
    fn test_config_save_and_load() -> anyhow::Result<()>
    {
        let _lock = ENV_LOCK.lock().map_err(|e| anyhow::anyhow!("env lock poisoned: {}", e))?;
        let dir = tempfile::TempDir::new()?;
        unsafe { env::set_var("XDG_CONFIG_HOME", dir.path()) };

        let mut config = Config::default();
        config.set("templates.uri", "https://test.com")?;
        config.save()?;

        let loaded = Config::load()?;
        assert_eq!(loaded.get("templates.uri").ok_or_else(|| anyhow::anyhow!("templates.uri not set"))?, "https://test.com");

        unsafe { env::remove_var("XDG_CONFIG_HOME") };
        Ok(())
    }

    #[test]
    fn test_config_load_missing_file() -> anyhow::Result<()>
    {
        let _lock = ENV_LOCK.lock().map_err(|e| anyhow::anyhow!("env lock poisoned: {}", e))?;
        let dir = tempfile::TempDir::new()?;
        unsafe { env::set_var("XDG_CONFIG_HOME", dir.path()) };

        let loaded = Config::load()?;
        assert!(loaded.templates.uri.is_none() == true);

        unsafe { env::remove_var("XDG_CONFIG_HOME") };
        Ok(())
    }

    #[test]
    fn test_config_get_config_path_xdg() -> anyhow::Result<()>
    {
        let _lock = ENV_LOCK.lock().map_err(|e| anyhow::anyhow!("env lock poisoned: {}", e))?;
        unsafe { env::set_var("XDG_CONFIG_HOME", "/tmp/test-xdg") };
        let path = Config::get_config_path()?;
        assert_eq!(path, PathBuf::from("/tmp/test-xdg/slopctl/config.yml"));
        unsafe { env::remove_var("XDG_CONFIG_HOME") };
        Ok(())
    }

    // ── workspace-scoped and EffectiveConfig tests ────────────────

    #[test]
    fn test_config_workspace_path()
    {
        let ws = PathBuf::from("/tmp/my-project");
        let path = Config::get_workspace_path(&ws);
        assert_eq!(path, PathBuf::from("/tmp/my-project/.slopctl/config.yml"));
    }

    #[test]
    fn test_config_workspace_save_and_load() -> anyhow::Result<()>
    {
        let dir = tempfile::TempDir::new()?;
        let ws = dir.path();

        let mut config = Config::default();
        config.set("merge.provider", "anthropic")?;
        config.save_workspace(ws)?;

        let loaded = Config::load_workspace(ws)?;
        assert_eq!(loaded.get("merge.provider").ok_or_else(|| anyhow::anyhow!("not set"))?, "anthropic");
        Ok(())
    }

    #[test]
    fn test_config_workspace_load_missing_returns_default() -> anyhow::Result<()>
    {
        let dir = tempfile::TempDir::new()?;
        let loaded = Config::load_workspace(dir.path())?;
        assert!(loaded.templates.uri.is_none() == true);
        Ok(())
    }

    #[test]
    fn test_effective_config_workspace_wins() -> anyhow::Result<()>
    {
        let _lock = ENV_LOCK.lock().map_err(|e| anyhow::anyhow!("env lock poisoned: {}", e))?;
        let global_dir = tempfile::TempDir::new()?;
        let ws_dir = tempfile::TempDir::new()?;
        unsafe { env::set_var("XDG_CONFIG_HOME", global_dir.path()) };

        let mut global = Config::default();
        global.set("templates.uri", "https://global.example.com")?;
        global.set("merge.provider", "openai")?;
        global.save_global()?;

        let mut ws = Config::default();
        ws.set("templates.uri", "https://workspace.example.com")?;
        ws.save_workspace(ws_dir.path())?;

        let effective = EffectiveConfig::load(ws_dir.path())?;
        assert_eq!(effective.get("templates.uri").ok_or_else(|| anyhow::anyhow!("not set"))?, "https://workspace.example.com");
        assert_eq!(effective.get("merge.provider").ok_or_else(|| anyhow::anyhow!("not set"))?, "openai");

        unsafe { env::remove_var("XDG_CONFIG_HOME") };
        Ok(())
    }

    #[test]
    fn test_effective_config_get_with_origin() -> anyhow::Result<()>
    {
        let _lock = ENV_LOCK.lock().map_err(|e| anyhow::anyhow!("env lock poisoned: {}", e))?;
        let global_dir = tempfile::TempDir::new()?;
        let ws_dir = tempfile::TempDir::new()?;
        unsafe { env::set_var("XDG_CONFIG_HOME", global_dir.path()) };

        let mut global = Config::default();
        global.set("merge.provider", "openai")?;
        global.set("merge.model", "gpt-4o")?;
        global.save_global()?;

        let mut ws = Config::default();
        ws.set("merge.model", "claude-opus-4-6")?;
        ws.save_workspace(ws_dir.path())?;

        let effective = EffectiveConfig::load(ws_dir.path())?;
        let (val, scope) = effective.get_with_origin("merge.model").ok_or_else(|| anyhow::anyhow!("not set"))?;
        assert_eq!(val, "claude-opus-4-6");
        assert_eq!(scope, ConfigScope::Workspace);

        let (val, scope) = effective.get_with_origin("merge.provider").ok_or_else(|| anyhow::anyhow!("not set"))?;
        assert_eq!(val, "openai");
        assert_eq!(scope, ConfigScope::Global);

        assert!(effective.get_with_origin("templates.uri").is_none() == true);

        unsafe { env::remove_var("XDG_CONFIG_HOME") };
        Ok(())
    }

    #[test]
    fn test_effective_config_list_with_origin_deterministic_order() -> anyhow::Result<()>
    {
        let _lock = ENV_LOCK.lock().map_err(|e| anyhow::anyhow!("env lock poisoned: {}", e))?;
        let global_dir = tempfile::TempDir::new()?;
        let ws_dir = tempfile::TempDir::new()?;
        unsafe { env::set_var("XDG_CONFIG_HOME", global_dir.path()) };

        let mut global = Config::default();
        global.set("merge.provider", "openai")?;
        global.set("templates.uri", "https://global.example.com")?;
        global.save_global()?;

        let mut ws = Config::default();
        ws.set("templates.uri", "/local/path")?;
        ws.set("merge.model", "claude-opus-4-6")?;
        ws.save_workspace(ws_dir.path())?;

        let effective = EffectiveConfig::load(ws_dir.path())?;
        let list = effective.list_with_origin();

        let keys: Vec<&String> = list.keys().collect();
        assert_eq!(keys, vec!["merge.model", "merge.provider", "templates.uri"]);

        let (val, scope) = list.get("templates.uri").ok_or_else(|| anyhow::anyhow!("missing"))?;
        assert_eq!(val, "/local/path");
        assert_eq!(*scope, ConfigScope::Workspace);

        let (val, scope) = list.get("merge.provider").ok_or_else(|| anyhow::anyhow!("missing"))?;
        assert_eq!(val, "openai");
        assert_eq!(*scope, ConfigScope::Global);

        unsafe { env::remove_var("XDG_CONFIG_HOME") };
        Ok(())
    }

    #[test]
    fn test_config_scope_display()
    {
        assert_eq!(format!("{}", ConfigScope::Global), "global");
        assert_eq!(format!("{}", ConfigScope::Workspace), "workspace");
    }

    #[test]
    fn test_config_get_set_models_uri() -> anyhow::Result<()>
    {
        let mut config = Config::default();
        config.set("models.uri", "https://example.com/models")?;
        assert_eq!(config.get("models.uri").ok_or_else(|| anyhow::anyhow!("models.uri not set"))?, "https://example.com/models");
        Ok(())
    }

    #[test]
    fn test_config_get_set_models_fallback_uri() -> anyhow::Result<()>
    {
        let mut config = Config::default();
        config.set("models.fallbackUri", "https://fallback.com/models")?;
        assert_eq!(config.get("models.fallbackUri").ok_or_else(|| anyhow::anyhow!("not set"))?, "https://fallback.com/models");
        Ok(())
    }

    #[test]
    fn test_config_unset_models_uri() -> anyhow::Result<()>
    {
        let mut config = Config::default();
        config.set("models.uri", "https://example.com/models")?;
        config.unset("models.uri")?;
        assert!(config.get("models.uri").is_none() == true);
        Ok(())
    }

    #[test]
    fn test_config_unset_models_fallback_uri() -> anyhow::Result<()>
    {
        let mut config = Config::default();
        config.set("models.fallbackUri", "https://fallback.com/models")?;
        config.unset("models.fallbackUri")?;
        assert!(config.get("models.fallbackUri").is_none() == true);
        Ok(())
    }

    #[test]
    fn test_config_unset_merge_model() -> anyhow::Result<()>
    {
        let mut config = Config::default();
        config.set("merge.model", "gpt-4o")?;
        config.unset("merge.model")?;
        assert!(config.get("merge.model").is_none() == true);
        Ok(())
    }

    #[test]
    fn test_get_global_dir_is_parent_of_global_path() -> Result<()>
    {
        assert_eq!(Config::get_global_dir()?, Config::get_global_path()?.parent().unwrap());
        Ok(())
    }

    // -- template layers --

    #[test]
    fn test_config_layers_set_appends_in_order_and_updates_in_place() -> Result<()>
    {
        let mut config = Config::default();
        config.set("templates.layers.fake-layer.uri", "/tmp/fake")?;
        config.set("templates.layers.bogus_layer.uri", "git@example.com:bogus.git")?;
        config.set("templates.layers.bogus_layer.ref", "main")?;
        config.set("templates.layers.fake-layer.uri", "/tmp/fake2")?;

        let ids: Vec<&str> = config.templates.layers.iter().map(|layer| layer.id.as_str()).collect();
        assert_eq!(ids, vec!["fake-layer", "bogus_layer"]);
        assert_eq!(config.get("templates.layers.fake-layer.uri"), Some("/tmp/fake2".to_string()));
        assert_eq!(config.get("templates.layers.bogus_layer.ref"), Some("main".to_string()));
        assert_eq!(config.list().get("templates.layers.bogus_layer.ref"), Some(&"main".to_string()));

        let yaml = serde_yaml::to_string(&config)?;
        let reloaded: Config = serde_yaml::from_str(&yaml)?;
        assert_eq!(reloaded.templates.layers, config.templates.layers);
        assert!(yaml.contains("ref: main") == true);
        Ok(())
    }

    #[test]
    fn test_config_layers_field_before_uri_errors()
    {
        let mut config = Config::default();
        let err = config.set("templates.layers.fake-layer.ref", "main").unwrap_err().to_string();
        assert!(err.contains("set templates.layers.fake-layer.uri first") == true);
    }

    #[test]
    fn test_config_layers_invalid_id_and_field_error()
    {
        let mut config = Config::default();
        assert!(config.set("templates.layers.Bad.Id.uri", "/tmp").is_err() == true);
        assert!(config.set("templates.layers.UPPER.uri", "/tmp").is_err() == true);
        assert!(config.set("templates.layers.fake.branch", "main").is_err() == true);
        assert!(config.set("templates.layers.fake", "/tmp").is_err() == true);
    }

    #[test]
    fn test_config_layers_unset_field_and_layer() -> Result<()>
    {
        let mut config = Config::default();
        config.set("templates.layers.fake.uri", "/tmp/fake")?;
        config.set("templates.layers.fake.path", "templates")?;
        config.unset("templates.layers.fake.path")?;
        assert_eq!(config.get("templates.layers.fake.path"), None);
        assert_eq!(config.get("templates.layers.fake"), Some("/tmp/fake".to_string()));
        config.unset("templates.layers.fake")?;
        assert!(config.templates.layers.is_empty() == true);
        assert!(config.unset("templates.layers.fake").is_err() == true);
        Ok(())
    }

    #[test]
    fn test_effective_config_layers_concatenates_global_then_workspace() -> Result<()>
    {
        let mut global = Config::default();
        global.set("templates.layers.bogus.uri", "/tmp/bogus")?;
        let mut workspace = Config::default();
        workspace.set("templates.layers.fake.uri", "/tmp/fake")?;
        let effective = EffectiveConfig { workspace, global };

        let layers = effective.layers()?;
        assert_eq!(layers.len(), 2);
        assert_eq!((layers[0].0.id.as_str(), layers[0].1), ("bogus", ConfigScope::Global));
        assert_eq!((layers[1].0.id.as_str(), layers[1].1), ("fake", ConfigScope::Workspace));
        Ok(())
    }

    #[test]
    fn test_effective_config_layers_same_id_in_both_scopes_errors() -> Result<()>
    {
        let mut global = Config::default();
        global.set("templates.layers.fake.uri", "/tmp/a")?;
        let mut workspace = Config::default();
        workspace.set("templates.layers.fake.uri", "/tmp/b")?;
        let err = EffectiveConfig { workspace, global }.layers().unwrap_err().to_string();
        assert!(err.contains("more than once") == true);
        Ok(())
    }

    #[test]
    fn test_baseline_source_takes_ref_and_path_from_the_uri_scope() -> Result<()>
    {
        let mut global = Config::default();
        global.set("templates.uri", "git@example.com:bogus/templates.git")?;
        global.set("templates.path", "templates")?;
        global.set("templates.ref", "develop")?;
        let effective = EffectiveConfig { workspace: Config::default(), global };
        assert_eq!(
            effective.baseline_source(),
            Some(BaselineSource {
                uri:     "git@example.com:bogus/templates.git".to_string(),
                git_ref: Some("develop".to_string()),
                path:    Some("templates".to_string())
            })
        );

        let mut workspace = Config::default();
        workspace.set("templates.uri", "/tmp/fake")?;
        let effective = EffectiveConfig { workspace, global: effective.global };
        assert_eq!(effective.baseline_source(), Some(BaselineSource { uri: "/tmp/fake".to_string(), git_ref: None, path: None }));
        Ok(())
    }
}
