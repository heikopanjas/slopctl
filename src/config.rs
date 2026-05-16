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
    pub fallback_uri: Option<String>
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

    /// Returns the path to the **global** config file
    ///
    /// Uses `$XDG_CONFIG_HOME/slopctl/config.yml` if XDG_CONFIG_HOME is set,
    /// otherwise falls back to `$HOME/.config/slopctl/config.yml`
    pub fn get_global_path() -> Result<PathBuf>
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

        Ok(config_dir.join("slopctl").join("config.yml"))
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
        match key
        {
            | "templates.uri" => self.templates.uri.clone(),
            | "templates.fallbackUri" => self.templates.fallback_uri.clone(),
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

        values
    }

    /// Get list of all valid config keys
    pub fn valid_keys() -> Vec<&'static str>
    {
        vec!["templates.uri", "templates.fallbackUri", "agents.uri", "agents.fallbackUri", "models.uri", "models.fallbackUri", "merge.provider", "merge.model"]
    }
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
            "templates.uri", "templates.fallbackUri", "agents.uri", "agents.fallbackUri", "models.uri", "models.fallbackUri", "merge.provider", "merge.model"
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
}
