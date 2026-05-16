//! Default LLM provider configurations
//!
//! Provides a registry of provider-specific settings: API endpoints, API key
//! environment variables, and default model identifiers. The registry is loaded
//! from `model-defaults.yml` in the global template cache, with an embedded
//! fallback for first-run behavior.

use std::{collections::HashSet, fs, path::Path, sync::OnceLock};

use serde::{Deserialize, Serialize};

use crate::Result;

/// File name of the model defaults catalog
pub const MODEL_DEFAULTS_FILE: &str = "model-defaults.yml";

const EMBEDDED_MODEL_DEFAULTS: &str = include_str!("../templates/v5/model-defaults.yml");

/// Runtime representation of a provider's default configuration
///
/// Fields are `&'static str` so callers in `llm.rs` can return them without allocation.
#[derive(Debug, Clone)]
pub struct ProviderDefaults
{
    /// Provider identifier (lowercase, e.g. `openai`)
    pub name:            &'static str,
    /// Environment variable that holds the API key, or `None` for Ollama
    pub api_key_env:     Option<&'static str>,
    /// Chat-completions endpoint URL
    pub endpoint:        &'static str,
    /// Model-listing endpoint URL
    pub models_endpoint: &'static str,
    /// Default model identifier used when the user has not set `merge.model`
    pub default_model:   &'static str
}

/// Top-level YAML representation of the model defaults catalog
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCatalog
{
    /// Catalog schema version
    #[serde(default = "default_catalog_version")]
    pub version:   u32,
    /// Known provider configurations
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub providers: Vec<ProviderEntry>
}

/// YAML representation of a single provider entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderEntry
{
    /// Provider identifier
    pub name:            String,
    /// Environment variable for the API key (absent for Ollama)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key_env:     Option<String>,
    /// Chat-completions endpoint URL
    pub endpoint:        String,
    /// Model-listing endpoint URL
    pub models_endpoint: String,
    /// Default model identifier
    pub default_model:   String
}

static DEFAULT_MODEL_DEFAULTS: OnceLock<&'static [ProviderDefaults]> = OnceLock::new();

fn default_catalog_version() -> u32
{
    1
}

/// Load the model defaults catalog from a template cache directory
///
/// Falls back to the embedded catalog when `model-defaults.yml` is absent.
///
/// # Errors
///
/// Returns an error if the catalog file exists but cannot be read, parsed, or
/// validated, or if the embedded fallback is invalid.
pub fn load_model_catalog_from_dir(config_dir: &Path) -> Result<ModelCatalog>
{
    let path = config_dir.join(MODEL_DEFAULTS_FILE);
    if path.exists() == true
    {
        return load_model_catalog_file(&path);
    }
    load_embedded_model_catalog()
}

/// Load the cached model defaults catalog from a template cache directory
///
/// Unlike `load_model_catalog_from_dir`, this requires the cache file to exist.
///
/// # Errors
///
/// Returns an error if `model-defaults.yml` is missing or invalid.
pub fn load_cached_model_catalog_from_dir(config_dir: &Path) -> Result<ModelCatalog>
{
    let path = config_dir.join(MODEL_DEFAULTS_FILE);
    require!(path.exists() == true, Err(anyhow::anyhow!("{} not found in global template directory", MODEL_DEFAULTS_FILE)));
    load_model_catalog_file(&path)
}

/// Load the embedded fallback model defaults catalog
///
/// # Errors
///
/// Returns an error if the embedded catalog is invalid.
pub fn load_embedded_model_catalog() -> Result<ModelCatalog>
{
    parse_model_catalog(EMBEDDED_MODEL_DEFAULTS)
}

/// Load a model defaults catalog from a specific file
///
/// # Errors
///
/// Returns an error if the file cannot be read, parsed, or validated.
pub fn load_model_catalog_file(path: &Path) -> Result<ModelCatalog>
{
    let content = fs::read_to_string(path)?;
    parse_model_catalog(&content)
}

/// Parse and validate a model defaults YAML catalog
///
/// # Errors
///
/// Returns an error if YAML parsing or validation fails.
pub fn parse_model_catalog(content: &str) -> Result<ModelCatalog>
{
    let catalog: ModelCatalog = serde_yaml::from_str(content)?;
    validate_model_catalog(&catalog)?;
    Ok(catalog)
}

/// Validate model defaults catalog structure
///
/// # Errors
///
/// Returns an error when required fields are empty, names are duplicated, or
/// endpoint URLs are missing.
pub fn validate_model_catalog(catalog: &ModelCatalog) -> Result<()>
{
    require!(catalog.version == 1, Err(anyhow::anyhow!("unsupported model defaults version: {}", catalog.version)));
    require!(catalog.providers.is_empty() == false, Err(anyhow::anyhow!("model defaults catalog must contain at least one provider")));

    let mut names = HashSet::new();
    for provider in &catalog.providers
    {
        require!(provider.name.trim().is_empty() == false, Err(anyhow::anyhow!("provider name cannot be empty")));
        require!(names.insert(provider.name.as_str()) == true, Err(anyhow::anyhow!("duplicate provider entry: {}", provider.name)));
        require!(provider.endpoint.trim().is_empty() == false, Err(anyhow::anyhow!("provider '{}' endpoint cannot be empty", provider.name)));
        require!(provider.models_endpoint.trim().is_empty() == false, Err(anyhow::anyhow!("provider '{}' models_endpoint cannot be empty", provider.name)));
        require!(provider.default_model.trim().is_empty() == false, Err(anyhow::anyhow!("provider '{}' default_model cannot be empty", provider.name)));
    }

    Ok(())
}

fn default_model_defaults() -> &'static [ProviderDefaults]
{
    DEFAULT_MODEL_DEFAULTS.get_or_init(|| {
        let catalog = load_default_model_catalog().or_else(|_| load_embedded_model_catalog()).expect("embedded model defaults catalog must be valid");
        leak_model_defaults(catalog)
    })
}

fn load_default_model_catalog() -> Result<ModelCatalog>
{
    let data_dir = dirs::data_local_dir().ok_or_else(|| anyhow::anyhow!("Could not determine local data directory"))?;
    load_model_catalog_from_dir(&data_dir.join("slopctl/templates"))
}

fn leak_model_defaults(catalog: ModelCatalog) -> &'static [ProviderDefaults]
{
    let providers: Vec<ProviderDefaults> = catalog
        .providers
        .into_iter()
        .map(|p| ProviderDefaults {
            name:            leak_str(p.name),
            api_key_env:     p.api_key_env.map(leak_str),
            endpoint:        leak_str(p.endpoint),
            models_endpoint: leak_str(p.models_endpoint),
            default_model:   leak_str(p.default_model)
        })
        .collect();
    Box::leak(providers.into_boxed_slice())
}

fn leak_str(value: String) -> &'static str
{
    Box::leak(value.into_boxed_str())
}

/// Look up defaults for a provider by name
pub fn get_provider_defaults(provider: &str) -> Option<&'static ProviderDefaults>
{
    default_model_defaults().iter().find(|p| p.name == provider)
}

/// Get the default model for a provider
///
/// Returns `None` if the provider is not in the catalog.
pub fn get_default_model(provider: &str) -> Option<&'static str>
{
    get_provider_defaults(provider).map(|p| p.default_model)
}

/// Get the chat-completions endpoint URL for a provider
///
/// Returns `None` if the provider is not in the catalog.
pub fn get_endpoint(provider: &str) -> Option<&'static str>
{
    get_provider_defaults(provider).map(|p| p.endpoint)
}

/// Get the model-listing endpoint URL for a provider
///
/// Returns `None` if the provider is not in the catalog.
pub fn get_models_endpoint(provider: &str) -> Option<&'static str>
{
    get_provider_defaults(provider).map(|p| p.models_endpoint)
}

/// Get the API key environment variable name for a provider
///
/// Returns `None` if the provider is not in the catalog or requires no key (Ollama).
pub fn get_api_key_env(provider: &str) -> Option<&'static str>
{
    get_provider_defaults(provider).and_then(|p| p.api_key_env)
}

/// List all configured provider names
pub fn known_providers() -> Vec<&'static str>
{
    default_model_defaults().iter().map(|p| p.name).collect()
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn test_load_model_catalog_from_dir_valid() -> anyhow::Result<()>
    {
        let temp_dir = tempfile::TempDir::new()?;
        std::fs::write(
            temp_dir.path().join(MODEL_DEFAULTS_FILE),
            r#"
version: 1
providers:
  - name: bogus-llm
    api_key_env: BOGUS_API_KEY
    endpoint: https://bogus.example.com/v1/chat
    models_endpoint: https://bogus.example.com/v1/models
    default_model: bogus-large
"#
        )?;

        let catalog = load_model_catalog_from_dir(temp_dir.path())?;
        assert_eq!(catalog.providers.len(), 1);
        assert_eq!(catalog.providers[0].name, "bogus-llm");
        Ok(())
    }

    #[test]
    fn test_load_model_catalog_from_dir_missing_uses_embedded() -> anyhow::Result<()>
    {
        let temp_dir = tempfile::TempDir::new()?;
        let catalog = load_model_catalog_from_dir(temp_dir.path())?;
        assert!(catalog.providers.is_empty() == false);
        Ok(())
    }

    #[test]
    fn test_parse_model_catalog_rejects_duplicate_names()
    {
        let err = parse_model_catalog(
            r#"
version: 1
providers:
  - name: fake-llm
    endpoint: https://fake.example.com/v1/chat
    models_endpoint: https://fake.example.com/v1/models
    default_model: fake-large
  - name: fake-llm
    endpoint: https://fake2.example.com/v1/chat
    models_endpoint: https://fake2.example.com/v1/models
    default_model: fake-small
"#
        )
        .unwrap_err();
        assert!(err.to_string().contains("duplicate provider entry") == true);
    }

    #[test]
    fn test_parse_model_catalog_rejects_empty_endpoint()
    {
        let err = parse_model_catalog(
            r#"
version: 1
providers:
  - name: bogus-llm
    endpoint: ''
    models_endpoint: https://bogus.example.com/v1/models
    default_model: bogus-large
"#
        )
        .unwrap_err();
        assert!(err.to_string().contains("endpoint cannot be empty") == true);
    }

    #[test]
    fn test_parse_model_catalog_rejects_empty_default_model()
    {
        let err = parse_model_catalog(
            r#"
version: 1
providers:
  - name: bogus-llm
    endpoint: https://bogus.example.com/v1/chat
    models_endpoint: https://bogus.example.com/v1/models
    default_model: ''
"#
        )
        .unwrap_err();
        assert!(err.to_string().contains("default_model cannot be empty") == true);
    }

    #[test]
    fn test_parse_model_catalog_rejects_unsupported_version()
    {
        let err = parse_model_catalog(
            r#"
version: 99
providers:
  - name: bogus-llm
    endpoint: https://bogus.example.com/v1/chat
    models_endpoint: https://bogus.example.com/v1/models
    default_model: bogus-large
"#
        )
        .unwrap_err();
        assert!(err.to_string().contains("unsupported model defaults version") == true);
    }

    #[test]
    fn test_parse_model_catalog_provider_without_api_key() -> anyhow::Result<()>
    {
        let catalog = parse_model_catalog(
            r#"
version: 1
providers:
  - name: bogus-local
    endpoint: http://localhost:9999/api/chat
    models_endpoint: http://localhost:9999/api/tags
    default_model: bogus-7b
"#
        )?;
        assert_eq!(catalog.providers[0].api_key_env, None);
        Ok(())
    }

    #[test]
    fn test_embedded_catalog_is_valid() -> anyhow::Result<()>
    {
        let catalog = load_embedded_model_catalog()?;
        assert!(catalog.providers.is_empty() == false);
        assert!(catalog.providers.iter().all(|p| p.name.trim().is_empty() == false) == true);
        Ok(())
    }

    #[test]
    fn test_catalog_lookup_present_provider() -> anyhow::Result<()>
    {
        let temp_dir = tempfile::TempDir::new()?;
        std::fs::write(
            temp_dir.path().join(MODEL_DEFAULTS_FILE),
            r#"
version: 1
providers:
  - name: bogus-llm
    api_key_env: BOGUS_API_KEY
    endpoint: https://bogus.example.com/v1/chat
    models_endpoint: https://bogus.example.com/v1/models
    default_model: bogus-large
"#
        )?;
        let catalog = load_model_catalog_from_dir(temp_dir.path())?;
        let entry = catalog.providers.iter().find(|p| p.name == "bogus-llm");
        assert!(entry.is_some() == true);
        assert_eq!(entry.expect("should exist").default_model, "bogus-large");
        Ok(())
    }

    #[test]
    fn test_catalog_lookup_absent_provider() -> anyhow::Result<()>
    {
        let temp_dir = tempfile::TempDir::new()?;
        std::fs::write(
            temp_dir.path().join(MODEL_DEFAULTS_FILE),
            r#"
version: 1
providers:
  - name: bogus-llm
    endpoint: https://bogus.example.com/v1/chat
    models_endpoint: https://bogus.example.com/v1/models
    default_model: bogus-large
"#
        )?;
        let catalog = load_model_catalog_from_dir(temp_dir.path())?;
        let entry = catalog.providers.iter().find(|p| p.name == "fake-provider");
        assert!(entry.is_none() == true);
        Ok(())
    }
}
