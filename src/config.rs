//! Configuration management for slopctl
//!
//! Handles persistent configuration stored in:
//! - `$XDG_CONFIG_HOME/slopctl/config.yml` (if XDG_CONFIG_HOME is set)
//! - `$HOME/.config/slopctl/config.yml` (fallback)

use std::{collections::HashMap, env, fs, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::Result;

/// Configuration structure for slopctl
///
/// Uses a nested HashMap to support dotted key access (e.g., "source.url")
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config
{
    #[serde(default)]
    pub source: SourceConfig,
    #[serde(default)]
    pub merge:  MergeConfig
}

/// Source-related configuration
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SourceConfig
{
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url:      Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback: Option<String>
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
    /// Returns the path to the config file
    ///
    /// Uses `$XDG_CONFIG_HOME/slopctl/config.yml` if XDG_CONFIG_HOME is set,
    /// otherwise falls back to `$HOME/.config/slopctl/config.yml`
    pub fn get_config_path() -> Result<PathBuf>
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

    /// Load configuration from file
    ///
    /// Returns default config if file doesn't exist
    pub fn load() -> Result<Self>
    {
        let config_path = Self::get_config_path()?;

        require!(config_path.exists() == true, Ok(Self::default()));

        let content = fs::read_to_string(&config_path)?;
        let config: Config = serde_yaml::from_str(&content)?;
        Ok(config)
    }

    /// Save configuration to file
    ///
    /// Creates parent directories if they don't exist
    pub fn save(&self) -> Result<()>
    {
        let config_path = Self::get_config_path()?;

        if let Some(parent) = config_path.parent()
        {
            fs::create_dir_all(parent)?;
        }

        let content = serde_yaml::to_string(self)?;
        fs::write(&config_path, content)?;
        Ok(())
    }

    /// Get a value by dotted key (e.g., "source.url")
    ///
    /// Returns None if key doesn't exist or path is invalid
    pub fn get(&self, key: &str) -> Option<String>
    {
        match key
        {
            | "source.url" => self.source.url.clone(),
            | "source.fallback" => self.source.fallback.clone(),
            | "merge.provider" => self.merge.provider.clone(),
            | "merge.model" => self.merge.model.clone(),
            | _ => None
        }
    }

    /// Set a value by dotted key (e.g., "source.url")
    ///
    /// Returns error if key is not recognized
    pub fn set(&mut self, key: &str, value: &str) -> Result<()>
    {
        match key
        {
            | "source.url" =>
            {
                self.source.url = Some(value.to_string());
                Ok(())
            }
            | "source.fallback" =>
            {
                self.source.fallback = Some(value.to_string());
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
            | "source.url" =>
            {
                self.source.url = None;
                Ok(())
            }
            | "source.fallback" =>
            {
                self.source.fallback = None;
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

        if let Some(url) = &self.source.url
        {
            values.insert("source.url".to_string(), url.clone());
        }

        if let Some(fallback) = &self.source.fallback
        {
            values.insert("source.fallback".to_string(), fallback.clone());
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
        vec!["source.url", "source.fallback", "merge.provider", "merge.model"]
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
        assert!(config.source.url.is_none() == true);
        assert!(config.source.fallback.is_none() == true);
    }

    #[test]
    fn test_config_get_set_url() -> anyhow::Result<()>
    {
        let mut config = Config::default();
        config.set("source.url", "https://example.com")?;
        assert_eq!(config.get("source.url").ok_or_else(|| anyhow::anyhow!("source.url not set"))?, "https://example.com");
        Ok(())
    }

    #[test]
    fn test_config_get_set_fallback() -> anyhow::Result<()>
    {
        let mut config = Config::default();
        config.set("source.fallback", "https://fallback.com")?;
        assert_eq!(config.get("source.fallback").ok_or_else(|| anyhow::anyhow!("source.fallback not set"))?, "https://fallback.com");
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
    fn test_config_unset_url() -> anyhow::Result<()>
    {
        let mut config = Config::default();
        config.set("source.url", "https://example.com")?;
        config.unset("source.url")?;
        assert!(config.get("source.url").is_none() == true);
        Ok(())
    }

    #[test]
    fn test_config_unset_fallback() -> anyhow::Result<()>
    {
        let mut config = Config::default();
        config.set("source.fallback", "https://fallback.com")?;
        config.unset("source.fallback")?;
        assert!(config.get("source.fallback").is_none() == true);
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
        config.set("source.url", "https://example.com")?;
        config.set("source.fallback", "https://fallback.com")?;

        let values = config.list();
        assert_eq!(values.len(), 2);
        assert_eq!(values.get("source.url").ok_or_else(|| anyhow::anyhow!("source.url not in list"))?, "https://example.com");
        assert_eq!(values.get("source.fallback").ok_or_else(|| anyhow::anyhow!("source.fallback not in list"))?, "https://fallback.com");
        Ok(())
    }

    #[test]
    fn test_config_valid_keys()
    {
        let keys = Config::valid_keys();
        assert_eq!(keys, vec!["source.url", "source.fallback", "merge.provider", "merge.model"]);
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
        config.set("source.url", "https://example.com")?;

        let yaml = serde_yaml::to_string(&config)?;
        let loaded: Config = serde_yaml::from_str(&yaml)?;
        assert_eq!(loaded.get("source.url").ok_or_else(|| anyhow::anyhow!("source.url not set"))?, "https://example.com");
        assert!(loaded.get("source.fallback").is_none() == true);
        Ok(())
    }

    #[test]
    fn test_config_save_and_load() -> anyhow::Result<()>
    {
        let _lock = ENV_LOCK.lock().map_err(|e| anyhow::anyhow!("env lock poisoned: {}", e))?;
        let dir = tempfile::TempDir::new()?;
        unsafe { env::set_var("XDG_CONFIG_HOME", dir.path()) };

        let mut config = Config::default();
        config.set("source.url", "https://test.com")?;
        config.save()?;

        let loaded = Config::load()?;
        assert_eq!(loaded.get("source.url").ok_or_else(|| anyhow::anyhow!("source.url not set"))?, "https://test.com");

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
        assert!(loaded.source.url.is_none() == true);

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
}
