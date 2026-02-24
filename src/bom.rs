//! Bill of Materials (BoM) functionality for template file management

use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf}
};

use serde::{Deserialize, Serialize};

use crate::Result;

/// File mapping with source and target paths
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMapping
{
    pub source: String,
    pub target: String
}

/// Agent configuration with instructions, prompts, and skills
#[derive(Debug, Serialize, Deserialize)]
pub struct AgentConfig
{
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<Vec<FileMapping>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts:      Option<Vec<FileMapping>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skills:       Option<Vec<FileMapping>>
}

/// Language configuration with files and optional includes
///
/// Languages can include shared file groups or other languages via `includes`.
/// Resolution order: included files first (depth-first), then own `files`.
#[derive(Debug, Serialize, Deserialize)]
pub struct LanguageConfig
{
    #[serde(skip_serializing_if = "Option::is_none")]
    pub includes: Option<Vec<String>>,
    #[serde(default)]
    pub files:    Vec<FileMapping>
}

/// Integration configuration with files
#[derive(Debug, Serialize, Deserialize)]
pub struct IntegrationConfig
{
    pub files: Vec<FileMapping>
}

/// Main file configuration
#[derive(Debug, Serialize, Deserialize)]
pub struct MainConfig
{
    pub source: String,
    pub target: String
}

/// Agent-agnostic skill definition (top-level in templates.yml)
///
/// Skills are directories containing SKILL.md + optional supporting files.
/// The install target is resolved from `agent_defaults` based on the active agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDefinition
{
    pub name:   String,
    pub source: String
}

/// Default version for templates.yml (used when version field is missing)
///
/// Switched to version 3 in v9.0.0 (shared groups + includes)
fn default_version() -> u32
{
    3
}

/// Template configuration structure parsed from templates.yml
#[derive(Debug, Serialize, Deserialize)]
pub struct TemplateConfig
{
    #[serde(default = "default_version")]
    pub version:     u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub main:        Option<MainConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agents:      Option<HashMap<String, AgentConfig>>,
    pub languages:   HashMap<String, LanguageConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shared:      Option<HashMap<String, Vec<FileMapping>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub integration: Option<HashMap<String, IntegrationConfig>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub principles:  Option<Vec<FileMapping>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mission:     Option<Vec<FileMapping>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skills:      Option<Vec<SkillDefinition>>
}

/// Resolves a language's complete file list by recursively expanding includes
///
/// Looks up each include name in the `shared` section first, then in `languages`.
/// Included files are prepended (depth-first), the language's own `files` come last
/// so they can override earlier entries with the same target.
///
/// # Arguments
///
/// * `lang` - Language name to resolve
/// * `config` - Parsed template configuration
///
/// # Errors
///
/// Returns an error if a circular include is detected or a referenced name
/// is found in neither `shared` nor `languages`
pub fn resolve_language_files(lang: &str, config: &TemplateConfig) -> Result<Vec<FileMapping>>
{
    let mut visited = HashSet::new();
    resolve_language_files_inner(lang, config, &mut visited)
}

/// Recursive helper for `resolve_language_files` with cycle detection
fn resolve_language_files_inner(lang: &str, config: &TemplateConfig, visited: &mut HashSet<String>) -> Result<Vec<FileMapping>>
{
    if visited.contains(lang) == true
    {
        return Err(format!("Circular include detected: '{}'", lang).into());
    }
    visited.insert(lang.to_string());

    let lang_config = config.languages.get(lang).ok_or_else(|| format!("Language '{}' not found in templates.yml", lang))?;

    let mut files = Vec::new();

    if let Some(includes) = &lang_config.includes
    {
        for include_name in includes
        {
            // Check shared groups first
            if let Some(shared_map) = &config.shared &&
                let Some(shared_files) = shared_map.get(include_name.as_str())
            {
                files.extend(shared_files.iter().cloned());
                continue;
            }

            // Then check languages (recursive)
            if config.languages.contains_key(include_name.as_str()) == true
            {
                let included = resolve_language_files_inner(include_name, config, visited)?;
                files.extend(included);
            }
            else
            {
                return Err(format!("Include '{}' (referenced by '{}') not found in shared or languages", include_name, lang).into());
            }
        }
    }

    files.extend(lang_config.files.iter().cloned());

    Ok(files)
}

/// Bill of Materials - maps agent names to their target file paths
#[derive(Debug)]
pub struct BillOfMaterials
{
    agent_files: HashMap<String, Vec<PathBuf>>
}

impl Default for BillOfMaterials
{
    fn default() -> Self
    {
        Self::new()
    }
}

impl BillOfMaterials
{
    /// Create a new empty Bill of Materials
    pub fn new() -> Self
    {
        Self { agent_files: HashMap::new() }
    }

    /// Build a Bill of Materials from templates.yml configuration
    ///
    /// # Arguments
    ///
    /// * `config_path` - Path to templates.yml file in global storage
    ///
    /// # Returns
    ///
    /// A `BillOfMaterials` containing agent names mapped to their workspace file paths
    ///
    /// # Errors
    ///
    /// Returns an error if templates.yml cannot be read or parsed
    pub fn from_config(config_path: &Path) -> Result<Self>
    {
        let config_content = fs::read_to_string(config_path)?;
        let template_config: TemplateConfig = serde_yaml::from_str(&config_content)?;

        let mut bom = Self::new();

        // Process each agent's files (if agents section exists)
        if let Some(agents) = template_config.agents
        {
            for (agent_name, agent_config) in agents
            {
                let mut file_paths = Vec::new();

                // Collect instruction files
                if let Some(instructions) = agent_config.instructions
                {
                    for mapping in instructions
                    {
                        if let Some(path) = Self::resolve_workspace_path(&mapping.target)
                        {
                            file_paths.push(path);
                        }
                    }
                }

                // Collect prompt files
                if let Some(prompts) = agent_config.prompts
                {
                    for mapping in prompts
                    {
                        if let Some(path) = Self::resolve_workspace_path(&mapping.target)
                        {
                            file_paths.push(path);
                        }
                    }
                }

                // Collect skill files
                if let Some(skills) = agent_config.skills
                {
                    for mapping in skills
                    {
                        if let Some(path) = Self::resolve_workspace_path(&mapping.target)
                        {
                            file_paths.push(path);
                        }
                    }
                }

                if file_paths.is_empty() == false
                {
                    bom.agent_files.insert(agent_name, file_paths);
                }
            }
        }

        Ok(bom)
    }

    /// Resolve a target path placeholder to an actual workspace path
    ///
    /// Only resolves $workspace placeholders. Returns None for $userprofile
    /// and $instructions placeholders (those are not project-specific files).
    ///
    /// # Arguments
    ///
    /// * `target` - Target path with potential placeholder
    ///
    /// # Returns
    ///
    /// Some(PathBuf) if the path is workspace-relative, None otherwise
    fn resolve_workspace_path(target: &str) -> Option<PathBuf>
    {
        // Skip userprofile paths (user-global, not project-specific)
        if target.contains("$userprofile")
        {
            return None;
        }

        // Skip instruction fragments (merged into AGENTS.md, not standalone files)
        if target.contains("$instructions")
        {
            return None;
        }

        // Resolve workspace paths to current directory
        if target.contains("$workspace")
        {
            let resolved = target.replace("$workspace", ".");
            return Some(PathBuf::from(resolved));
        }

        // If no placeholder, treat as workspace-relative
        Some(PathBuf::from(target))
    }

    /// Get the list of file paths for a specific agent
    ///
    /// # Arguments
    ///
    /// * `agent_name` - Name of the agent (e.g., "claude", "copilot")
    ///
    /// # Returns
    ///
    /// Some(Vec<PathBuf>) if the agent exists in the BoM, None otherwise
    pub fn get_agent_files(&self, agent_name: &str) -> Option<&Vec<PathBuf>>
    {
        self.agent_files.get(agent_name)
    }

    /// Get all agent names in the Bill of Materials
    ///
    /// # Returns
    ///
    /// A vector of agent names
    pub fn get_agent_names(&self) -> Vec<String>
    {
        self.agent_files.keys().cloned().collect()
    }

    /// Check if an agent exists in the Bill of Materials
    ///
    /// # Arguments
    ///
    /// * `agent_name` - Name of the agent to check
    ///
    /// # Returns
    ///
    /// true if the agent has files in the BoM, false otherwise
    pub fn has_agent(&self, agent_name: &str) -> bool
    {
        self.agent_files.contains_key(agent_name)
    }
}
