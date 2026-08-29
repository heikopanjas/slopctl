//! Default LLM provider configurations
//!
//! Provides a registry of provider-specific settings: API endpoints, API key
//! environment variables, and default model identifiers. The registry is loaded
//! from `model-defaults.yml` in the global template cache. There is no
//! compile-time embedded fallback: the catalog is authoritative, matching
//! `templates.yml` and `agent-defaults.yml`, and callers must handle a missing
//! or unparsable cache by pointing the user at `slopctl templates --update`.

use std::{collections::HashSet, fs, path::Path};

use serde::{Deserialize, Serialize};

use crate::Result;

/// File name of the model defaults catalog
pub const MODEL_DEFAULTS_FILE: &str = "model-defaults.yml";

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

fn default_catalog_version() -> u32
{
    1
}

fn get_provider_entry<'a>(catalog: &'a ModelCatalog, provider: &str) -> Option<&'a ProviderEntry>
{
    catalog.providers.iter().find(|p| p.name == provider)
}

/// Get the API key environment variable name for a provider from a specific catalog
///
/// Returns `None` if the provider is not in the catalog or requires no key (Ollama).
pub fn get_api_key_env_from_catalog<'a>(catalog: &'a ModelCatalog, provider: &str) -> Option<&'a str>
{
    get_provider_entry(catalog, provider).and_then(|p| p.api_key_env.as_deref())
}

/// Get the default model for a provider from a specific catalog
pub fn get_default_model_from_catalog<'a>(catalog: &'a ModelCatalog, provider: &str) -> Option<&'a str>
{
    get_provider_entry(catalog, provider).map(|p| p.default_model.as_str())
}

/// Get the chat-completions endpoint URL for a provider from a specific catalog
pub fn get_endpoint_from_catalog<'a>(catalog: &'a ModelCatalog, provider: &str) -> Option<&'a str>
{
    get_provider_entry(catalog, provider).map(|p| p.endpoint.as_str())
}

/// Get the model-listing endpoint URL for a provider from a specific catalog
pub fn get_models_endpoint_from_catalog<'a>(catalog: &'a ModelCatalog, provider: &str) -> Option<&'a str>
{
    get_provider_entry(catalog, provider).map(|p| p.models_endpoint.as_str())
}

/// Load the model defaults catalog from a template cache directory
///
/// # Errors
///
/// Returns an error if `model-defaults.yml` is missing, unreadable, unparsable,
/// or fails validation. Callers should point the user at
/// `slopctl templates --update` when this fails.
pub fn load_model_catalog_from_dir(config_dir: &Path) -> Result<ModelCatalog>
{
    let path = config_dir.join(MODEL_DEFAULTS_FILE);
    require!(path.exists() == true, Err(anyhow::anyhow!("{} not found in global template directory", MODEL_DEFAULTS_FILE)));
    load_model_catalog_file(&path)
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
    fn test_load_model_catalog_from_dir_missing_errors() -> anyhow::Result<()>
    {
        let temp_dir = tempfile::TempDir::new()?;
        let err = load_model_catalog_from_dir(temp_dir.path()).unwrap_err();
        assert!(err.to_string().contains(MODEL_DEFAULTS_FILE) == true);
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
