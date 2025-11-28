//! Configuration management for vibe-check
//!
//! Handles persistent configuration stored in:
//! - `$XDG_CONFIG_HOME/vibe-check/config.yml` (if XDG_CONFIG_HOME is set)
//! - `$HOME/.config/vibe-check/config.yml` (fallback)

use std::{collections::HashMap, env, fs, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::Result;

/// Configuration structure for vibe-check
///
/// Uses a nested HashMap to support dotted key access (e.g., "source.url")
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config
{
    #[serde(default)]
    pub source: SourceConfig
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

impl Config
{
    /// Returns the path to the config file
    ///
    /// Uses `$XDG_CONFIG_HOME/vibe-check/config.yml` if XDG_CONFIG_HOME is set,
    /// otherwise falls back to `$HOME/.config/vibe-check/config.yml`
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
            return Err("Could not determine config directory".into());
        };

        Ok(config_dir.join("vibe-check").join("config.yml"))
    }

    /// Load configuration from file
    ///
    /// Returns default config if file doesn't exist
    pub fn load() -> Result<Self>
    {
        let config_path = Self::get_config_path()?;

        if config_path.exists() == false
        {
            return Ok(Self::default());
        }

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
            | _ => Err(format!("Unknown config key: {}", key).into())
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
            | _ => Err(format!("Unknown config key: {}", key).into())
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

        values
    }

    /// Get list of all valid config keys
    pub fn valid_keys() -> Vec<&'static str>
    {
        vec!["source.url", "source.fallback"]
    }
}
