//! Bill of Materials (BoM) functionality for template file management

use std::{
    collections::HashMap,
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

/// Agent configuration with instructions and prompts
#[derive(Debug, Serialize, Deserialize)]
pub struct AgentConfig
{
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<Vec<FileMapping>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts:      Option<Vec<FileMapping>>
}

/// Language configuration with files
#[derive(Debug, Serialize, Deserialize)]
pub struct LanguageConfig
{
    pub files: Vec<FileMapping>
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

/// Default version for templates.yml (used when version field is missing)
///
/// Note: Default will switch to version 2 with v7.0.0
fn default_version() -> u32
{
    1
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
    pub integration: Option<HashMap<String, IntegrationConfig>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub principles:  Option<Vec<FileMapping>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mission:     Option<Vec<FileMapping>>
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
