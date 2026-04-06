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

/// Directory entry that an agent declares for creation during install
#[derive(Debug, Serialize, Deserialize)]
pub struct DirectoryEntry
{
    pub target: String
}

/// Agent configuration with instructions, prompts, skills, and directories
#[derive(Debug, Serialize, Deserialize)]
pub struct AgentConfig
{
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub instructions: Vec<FileMapping>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prompts:      Vec<FileMapping>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skills:       Vec<SkillDefinition>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub directories:  Vec<DirectoryEntry>
}

/// Language configuration with files, optional includes, and optional skills
///
/// Languages can include shared file groups or other languages via `includes`.
/// Resolution order: included files first (depth-first), then own `files`.
/// Skills are installed to the cross-client `.agents/skills/` directory when
/// the language is selected. Skills from included `shared` groups are propagated;
/// skills from included *languages* are NOT propagated.
#[derive(Debug, Serialize, Deserialize)]
pub struct LanguageConfig
{
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub includes: Vec<String>,
    #[serde(default)]
    pub files:    Vec<FileMapping>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skills:   Vec<SkillDefinition>
}

/// Shared file group with files and optional skills
///
/// Shared groups are referenced by languages via `includes`. When a language
/// includes a shared group, the group's files are prepended and its skills
/// are propagated to the language's resolved skill list.
#[derive(Debug, Serialize, Deserialize)]
pub struct SharedConfig
{
    #[serde(default)]
    pub files:  Vec<FileMapping>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skills: Vec<SkillDefinition>
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

/// Skill definition used in agents, languages, and top-level skills sections
///
/// Skills are directories containing SKILL.md + optional supporting files.
/// The install target is resolved based on context: agent-specific dir for agent
/// skills, cross-client `.agents/skills/` for language and top-level skills.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDefinition
{
    pub name:   String,
    pub source: String
}

/// Default version for templates.yml (used when version field is missing)
///
/// Switched to version 5 in v12.0.0 (extensive skill handling improvements)
fn default_version() -> u32
{
    5
}

/// Template configuration structure parsed from templates.yml
#[derive(Debug, Serialize, Deserialize)]
pub struct TemplateConfig
{
    #[serde(default = "default_version")]
    pub version:     u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub main:        Option<MainConfig>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub agents:      HashMap<String, AgentConfig>,
    pub languages:   HashMap<String, LanguageConfig>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub shared:      HashMap<String, SharedConfig>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub integration: HashMap<String, IntegrationConfig>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub principles:  Vec<FileMapping>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mission:     Vec<FileMapping>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skills:      Vec<SkillDefinition>
}

/// Resolves a language's complete file list by recursively expanding includes
///
/// Looks up each include name in the `shared` section first, then in `languages`.
/// Included files are prepended (depth-first), the language's own `files` come last.
///
/// After resolution, validates that no two entries target the same disk file.
/// Entries targeting `$instructions` (AGENTS.md fragments) are exempt since
/// multiple fragments are expected and merged.
///
/// # Arguments
///
/// * `lang` - Language name to resolve
/// * `config` - Parsed template configuration
///
/// # Errors
///
/// Returns an error if a circular include is detected, a referenced name
/// is found in neither `shared` nor `languages`, or two entries target
/// the same disk file
pub fn resolve_language_files(lang: &str, config: &TemplateConfig) -> Result<Vec<FileMapping>>
{
    let mut visited = HashSet::new();
    let files = resolve_language_files_inner(lang, config, &mut visited)?;

    let mut seen_targets: HashMap<&str, &str> = HashMap::new();
    for entry in &files
    {
        if entry.target.starts_with("$instructions") == false &&
            let Some(previous_source) = seen_targets.insert(&entry.target, &entry.source)
        {
            return Err(anyhow::anyhow!(
                "Duplicate target '{}' in language '{}': '{}' and '{}' both write to the same file", entry.target, lang, previous_source, entry.source
            ));
        }
    }

    Ok(files)
}

/// Recursive helper for `resolve_language_files` with cycle detection
fn resolve_language_files_inner(lang: &str, config: &TemplateConfig, visited: &mut HashSet<String>) -> Result<Vec<FileMapping>>
{
    require!(visited.contains(lang) == false, Err(anyhow::anyhow!("Circular include detected: '{}'", lang)));
    visited.insert(lang.to_string());

    let lang_config = config.languages.get(lang).ok_or_else(|| anyhow::anyhow!("Language '{}' not found in templates.yml", lang))?;

    let mut files = Vec::new();

    for include_name in &lang_config.includes
    {
        if let Some(shared_config) = config.shared.get(include_name.as_str())
        {
            files.extend(shared_config.files.iter().cloned());
        }
        else if config.languages.contains_key(include_name.as_str()) == true
        {
            let included = resolve_language_files_inner(include_name, config, visited)?;
            files.extend(included);
        }
        else
        {
            return Err(anyhow::anyhow!("Include '{}' (referenced by '{}') not found in shared or languages", include_name, lang));
        }
    }

    files.extend(lang_config.files.iter().cloned());

    Ok(files)
}

/// Resolves a language's complete skill list including skills from shared groups
/// and included languages
///
/// Collects the language's own `skills` plus skills from any `shared` groups or
/// other languages referenced via `includes`. Recurses into included languages
/// depth-first with cycle detection. Included skills are prepended; the
/// language's own skills come last.
///
/// # Arguments
///
/// * `lang` - Language name to resolve
/// * `config` - Parsed template configuration
///
/// # Errors
///
/// Returns an error if the language is not found in templates.yml or a circular
/// include is detected
pub fn resolve_language_skills(lang: &str, config: &TemplateConfig) -> Result<Vec<SkillDefinition>>
{
    let mut visited = HashSet::new();
    resolve_language_skills_inner(lang, config, &mut visited)
}

/// Recursive helper for `resolve_language_skills` with cycle detection
fn resolve_language_skills_inner(lang: &str, config: &TemplateConfig, visited: &mut HashSet<String>) -> Result<Vec<SkillDefinition>>
{
    require!(visited.contains(lang) == false, Err(anyhow::anyhow!("Circular include detected in skills: '{}'", lang)));
    visited.insert(lang.to_string());

    let lang_config = config.languages.get(lang).ok_or_else(|| anyhow::anyhow!("Language '{}' not found in templates.yml", lang))?;

    let mut skills = Vec::new();

    for include_name in &lang_config.includes
    {
        if let Some(shared_config) = config.shared.get(include_name.as_str()) &&
            shared_config.skills.is_empty() == false
        {
            skills.extend(shared_config.skills.iter().cloned());
        }
        else if config.languages.contains_key(include_name.as_str()) == true
        {
            let included = resolve_language_skills_inner(include_name, config, visited)?;
            skills.extend(included);
        }
    }

    skills.extend(lang_config.skills.iter().cloned());

    Ok(skills)
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

        for (agent_name, agent_config) in template_config.agents
        {
            let mut file_paths = Vec::new();

            for mapping in agent_config.instructions.iter().chain(&agent_config.prompts)
            {
                if let Some(path) = Self::resolve_workspace_path(&mapping.target)
                {
                    file_paths.push(path);
                }
            }

            if file_paths.is_empty() == false
            {
                bom.agent_files.insert(agent_name, file_paths);
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
    pub fn resolve_workspace_path(target: &str) -> Option<PathBuf>
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
    /// Some(&[PathBuf]) if the agent exists in the BoM, None otherwise
    pub fn get_agent_files(&self, agent_name: &str) -> Option<&[PathBuf]>
    {
        self.agent_files.get(agent_name).map(|v| v.as_slice())
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

#[cfg(test)]
mod tests
{
    use super::*;

    fn make_mapping(source: &str, target: &str) -> FileMapping
    {
        FileMapping { source: source.to_string(), target: target.to_string() }
    }

    fn make_lang(includes: Vec<String>, files: Vec<FileMapping>) -> LanguageConfig
    {
        LanguageConfig { includes, files, skills: vec![] }
    }

    fn make_shared(files: Vec<FileMapping>) -> SharedConfig
    {
        SharedConfig { files, skills: vec![] }
    }

    fn minimal_config() -> TemplateConfig
    {
        TemplateConfig {
            version:     5,
            main:        None,
            agents:      HashMap::new(),
            languages:   HashMap::new(),
            shared:      HashMap::new(),
            integration: HashMap::new(),
            principles:  vec![],
            mission:     vec![],
            skills:      vec![]
        }
    }

    // -- default_version --

    #[test]
    fn test_default_version_returns_5()
    {
        assert_eq!(default_version(), 5);
    }

    // -- TemplateConfig serde --

    #[test]
    fn test_template_config_version_defaults_to_5() -> anyhow::Result<()>
    {
        let yaml = "languages: {}";
        let config: TemplateConfig = serde_yaml::from_str(yaml)?;
        assert_eq!(config.version, 5);
        Ok(())
    }

    #[test]
    fn test_template_config_explicit_version() -> anyhow::Result<()>
    {
        let yaml = "version: 2\nlanguages: {}";
        let config: TemplateConfig = serde_yaml::from_str(yaml)?;
        assert_eq!(config.version, 2);
        Ok(())
    }

    #[test]
    fn test_template_config_optional_fields_absent() -> anyhow::Result<()>
    {
        let yaml = "languages: {}";
        let config: TemplateConfig = serde_yaml::from_str(yaml)?;
        assert!(config.main.is_none() == true);
        assert!(config.agents.is_empty() == true);
        assert!(config.shared.is_empty() == true);
        assert!(config.integration.is_empty() == true);
        assert!(config.principles.is_empty() == true);
        assert!(config.mission.is_empty() == true);
        assert!(config.skills.is_empty() == true);
        Ok(())
    }

    // -- LanguageConfig serde --

    #[test]
    fn test_language_config_files_defaults_empty() -> anyhow::Result<()>
    {
        let yaml = "includes: [foo]";
        let config: LanguageConfig = serde_yaml::from_str(yaml)?;
        assert!(config.files.is_empty() == true);
        assert_eq!(config.includes.len(), 1);
        Ok(())
    }

    #[test]
    fn test_language_config_includes_absent() -> anyhow::Result<()>
    {
        let yaml = "files:\n  - source: a.md\n    target: '$instructions'";
        let config: LanguageConfig = serde_yaml::from_str(yaml)?;
        assert!(config.includes.is_empty() == true);
        assert_eq!(config.files.len(), 1);
        Ok(())
    }

    // -- resolve_language_files: basic --

    #[test]
    fn test_resolve_simple_language_no_includes() -> anyhow::Result<()>
    {
        let mut config = minimal_config();
        config
            .languages
            .insert("rust".to_string(), make_lang(vec![], vec![make_mapping("rust.md", "$instructions"), make_mapping("rust.toml", "$workspace/.rustfmt.toml")]));

        let files = resolve_language_files("rust", &config)?;
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].source, "rust.md");
        assert_eq!(files[1].source, "rust.toml");
        Ok(())
    }

    #[test]
    fn test_resolve_language_not_found()
    {
        let config = minimal_config();
        let err = resolve_language_files("nonexistent", &config).unwrap_err();
        assert!(err.to_string().contains("not found in templates.yml") == true);
    }

    // -- resolve_language_files: shared includes --

    #[test]
    fn test_resolve_includes_shared_group() -> anyhow::Result<()>
    {
        let mut config = minimal_config();
        let mut shared = HashMap::new();
        shared
            .insert("cmake".to_string(), make_shared(vec![make_mapping("cmake-build.md", "$instructions"), make_mapping("cmake.gitignore", "$workspace/.gitignore")]));
        config.shared = shared;

        config.languages.insert("c".to_string(), make_lang(vec!["cmake".to_string()], vec![make_mapping("c.md", "$instructions")]));

        let files = resolve_language_files("c", &config)?;
        assert_eq!(files.len(), 3);
        assert_eq!(files[0].source, "cmake-build.md");
        assert_eq!(files[1].source, "cmake.gitignore");
        assert_eq!(files[2].source, "c.md");
        Ok(())
    }

    // -- resolve_language_files: language includes --

    #[test]
    fn test_resolve_includes_another_language() -> anyhow::Result<()>
    {
        let mut config = minimal_config();
        config
            .languages
            .insert("swift".to_string(), make_lang(vec![], vec![make_mapping("swift.md", "$instructions"), make_mapping("swift.ini", "$workspace/.editorconfig")]));
        config.languages.insert("swiftui".to_string(), make_lang(vec!["swift".to_string()], vec![make_mapping("swiftui.md", "$instructions")]));

        let files = resolve_language_files("swiftui", &config)?;
        assert_eq!(files.len(), 3);
        assert_eq!(files[0].source, "swift.md");
        assert_eq!(files[1].source, "swift.ini");
        assert_eq!(files[2].source, "swiftui.md");
        Ok(())
    }

    // -- resolve_language_files: transitive includes --

    #[test]
    fn test_resolve_transitive_includes() -> anyhow::Result<()>
    {
        let mut config = minimal_config();
        let mut shared = HashMap::new();
        shared.insert("base".to_string(), make_shared(vec![make_mapping("base.gitignore", "$workspace/.gitignore")]));
        config.shared = shared;

        config.languages.insert("a".to_string(), make_lang(vec!["base".to_string()], vec![make_mapping("a.md", "$instructions")]));
        config.languages.insert("b".to_string(), make_lang(vec!["a".to_string()], vec![make_mapping("b.md", "$instructions")]));

        let files = resolve_language_files("b", &config)?;
        assert_eq!(files.len(), 3);
        assert_eq!(files[0].source, "base.gitignore");
        assert_eq!(files[1].source, "a.md");
        assert_eq!(files[2].source, "b.md");
        Ok(())
    }

    // -- resolve_language_files: mixed shared + language includes --

    #[test]
    fn test_resolve_mixed_shared_and_language_includes() -> anyhow::Result<()>
    {
        let mut config = minimal_config();
        let mut shared = HashMap::new();
        shared.insert("cmake".to_string(), make_shared(vec![make_mapping("cmake.md", "$instructions")]));
        config.shared = shared;

        config.languages.insert("c".to_string(), make_lang(vec![], vec![make_mapping("c.md", "$instructions")]));
        config.languages.insert("c-ext".to_string(), make_lang(vec!["cmake".to_string(), "c".to_string()], vec![make_mapping("ext.md", "$instructions")]));

        let files = resolve_language_files("c-ext", &config)?;
        assert_eq!(files.len(), 3);
        assert_eq!(files[0].source, "cmake.md");
        assert_eq!(files[1].source, "c.md");
        assert_eq!(files[2].source, "ext.md");
        Ok(())
    }

    // -- resolve_language_files: include-only language (empty files) --

    #[test]
    fn test_resolve_include_only_language() -> anyhow::Result<()>
    {
        let mut config = minimal_config();
        config.languages.insert("base".to_string(), make_lang(vec![], vec![make_mapping("base.md", "$instructions")]));
        config.languages.insert("alias".to_string(), make_lang(vec!["base".to_string()], vec![]));

        let files = resolve_language_files("alias", &config)?;
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].source, "base.md");
        Ok(())
    }

    // -- resolve_language_files: error cases --

    #[test]
    fn test_resolve_circular_include()
    {
        let mut config = minimal_config();
        config.languages.insert("a".to_string(), make_lang(vec!["b".to_string()], vec![]));
        config.languages.insert("b".to_string(), make_lang(vec!["a".to_string()], vec![]));

        let err = resolve_language_files("a", &config).unwrap_err();
        assert!(err.to_string().contains("Circular include") == true);
    }

    #[test]
    fn test_resolve_include_not_found()
    {
        let mut config = minimal_config();
        config.languages.insert("lang".to_string(), make_lang(vec!["nonexistent".to_string()], vec![]));

        let err = resolve_language_files("lang", &config).unwrap_err();
        assert!(err.to_string().contains("not found in shared or languages") == true);
    }

    #[test]
    fn test_resolve_include_not_found_no_shared_section()
    {
        let mut config = minimal_config();
        config.shared = HashMap::new();
        config.languages.insert("lang".to_string(), make_lang(vec!["missing".to_string()], vec![]));

        let err = resolve_language_files("lang", &config).unwrap_err();
        assert!(err.to_string().contains("not found in shared or languages") == true);
    }

    // -- resolve_language_files: duplicate target detection --

    #[test]
    fn test_resolve_duplicate_disk_target_rejected()
    {
        let mut config = minimal_config();
        let mut shared = HashMap::new();
        shared.insert("group".to_string(), make_shared(vec![make_mapping("a.ini", "$workspace/.editorconfig")]));
        config.shared = shared;

        config.languages.insert("lang".to_string(), make_lang(vec!["group".to_string()], vec![make_mapping("b.ini", "$workspace/.editorconfig")]));

        let err = resolve_language_files("lang", &config).unwrap_err();
        assert!(err.to_string().contains("Duplicate target") == true);
        assert!(err.to_string().contains(".editorconfig") == true);
    }

    #[test]
    fn test_resolve_multiple_instructions_targets_allowed() -> anyhow::Result<()>
    {
        let mut config = minimal_config();
        config.languages.insert(
            "rust".to_string(),
            make_lang(vec![], vec![make_mapping("coding.md", "$instructions"), make_mapping("build.md", "$instructions"), make_mapping("extra.md", "$instructions")])
        );

        let files = resolve_language_files("rust", &config)?;
        assert_eq!(files.len(), 3);
        Ok(())
    }

    #[test]
    fn test_resolve_duplicate_instructions_from_include_allowed() -> anyhow::Result<()>
    {
        let mut config = minimal_config();
        let mut shared = HashMap::new();
        shared.insert("group".to_string(), make_shared(vec![make_mapping("shared.md", "$instructions")]));
        config.shared = shared;

        config.languages.insert("lang".to_string(), make_lang(vec!["group".to_string()], vec![make_mapping("own.md", "$instructions")]));

        let files = resolve_language_files("lang", &config)?;
        assert_eq!(files.len(), 2);
        Ok(())
    }

    // -- BillOfMaterials --

    #[test]
    fn test_bom_new_is_empty()
    {
        let bom = BillOfMaterials::new();
        assert!(bom.get_agent_names().is_empty() == true);
    }

    #[test]
    fn test_bom_default_is_empty()
    {
        let bom = BillOfMaterials::default();
        assert!(bom.get_agent_names().is_empty() == true);
    }

    #[test]
    fn test_bom_has_agent()
    {
        let mut bom = BillOfMaterials::new();
        bom.agent_files.insert("claude".to_string(), vec![PathBuf::from("./CLAUDE.md")]);

        assert!(bom.has_agent("claude") == true);
        assert!(bom.has_agent("copilot") == false);
    }

    #[test]
    fn test_bom_get_agent_files() -> anyhow::Result<()>
    {
        let mut bom = BillOfMaterials::new();
        bom.agent_files.insert("cursor".to_string(), vec![PathBuf::from("./.cursorrules")]);

        assert!(bom.get_agent_files("cursor").is_some() == true);
        assert_eq!(bom.get_agent_files("cursor").ok_or_else(|| anyhow::anyhow!("missing cursor agent files"))?.len(), 1);
        assert!(bom.get_agent_files("unknown").is_none() == true);
        Ok(())
    }

    #[test]
    fn test_bom_get_agent_names()
    {
        let mut bom = BillOfMaterials::new();
        bom.agent_files.insert("a".to_string(), vec![PathBuf::from("a")]);
        bom.agent_files.insert("b".to_string(), vec![PathBuf::from("b")]);

        let mut names = bom.get_agent_names();
        names.sort();
        assert_eq!(names, vec!["a", "b"]);
    }

    // -- BillOfMaterials::resolve_workspace_path --

    #[test]
    fn test_resolve_workspace_path_userprofile()
    {
        assert!(BillOfMaterials::resolve_workspace_path("$userprofile/.codex/prompts/init.md").is_none() == true);
    }

    #[test]
    fn test_resolve_workspace_path_instructions()
    {
        assert!(BillOfMaterials::resolve_workspace_path("$instructions").is_none() == true);
    }

    #[test]
    fn test_resolve_workspace_path_workspace() -> anyhow::Result<()>
    {
        let result = BillOfMaterials::resolve_workspace_path("$workspace/CLAUDE.md");
        assert_eq!(result.ok_or_else(|| anyhow::anyhow!("expected workspace path"))?, PathBuf::from("./CLAUDE.md"));
        Ok(())
    }

    #[test]
    fn test_resolve_workspace_path_no_placeholder() -> anyhow::Result<()>
    {
        let result = BillOfMaterials::resolve_workspace_path("relative/path.md");
        assert_eq!(result.ok_or_else(|| anyhow::anyhow!("expected relative path"))?, PathBuf::from("relative/path.md"));
        Ok(())
    }

    // -- BillOfMaterials::from_config --

    #[test]
    fn test_bom_from_config_with_agents() -> anyhow::Result<()>
    {
        let dir = tempfile::TempDir::new()?;
        let config_path = dir.path().join("templates.yml");

        let yaml = r#"
languages: {}
agents:
  claude:
    instructions:
      - source: claude/CLAUDE.md
        target: '$workspace/CLAUDE.md'
    prompts:
      - source: claude/commands/init.md
        target: '$workspace/.claude/commands/init.md'
  codex:
    prompts:
      - source: codex/init.md
        target: '$userprofile/.codex/prompts/init.md'
"#;
        fs::write(&config_path, yaml)?;

        let bom = BillOfMaterials::from_config(&config_path)?;
        assert!(bom.has_agent("claude") == true);
        assert_eq!(bom.get_agent_files("claude").ok_or_else(|| anyhow::anyhow!("missing claude agent files"))?.len(), 2);
        // codex has only $userprofile paths, so all are skipped -> no entry
        assert!(bom.has_agent("codex") == false);
        Ok(())
    }

    #[test]
    fn test_bom_from_config_no_agents() -> anyhow::Result<()>
    {
        let dir = tempfile::TempDir::new()?;
        let config_path = dir.path().join("templates.yml");

        let yaml = "languages: {}";
        fs::write(&config_path, yaml)?;

        let bom = BillOfMaterials::from_config(&config_path)?;
        assert!(bom.get_agent_names().is_empty() == true);
        Ok(())
    }

    #[test]
    fn test_bom_from_config_agent_with_skills() -> anyhow::Result<()>
    {
        let dir = tempfile::TempDir::new()?;
        let config_path = dir.path().join("templates.yml");

        let yaml = r#"
languages: {}
agents:
  cursor:
    instructions:
      - source: cursor/cursorrules
        target: '$workspace/.cursorrules'
    skills:
      - name: create-rule
        source: 'https://github.com/user/cursor-skills/tree/main/create-rule'
"#;
        fs::write(&config_path, yaml)?;

        let bom = BillOfMaterials::from_config(&config_path)?;
        assert!(bom.has_agent("cursor") == true);
        // Skills are SkillDefinition (no target), so only instructions contribute to BoM
        assert_eq!(bom.get_agent_files("cursor").ok_or_else(|| anyhow::anyhow!("missing cursor agent files"))?.len(), 1);
        Ok(())
    }

    #[test]
    fn test_bom_from_config_invalid_file()
    {
        let result = BillOfMaterials::from_config(Path::new("/nonexistent/templates.yml"));
        assert!(result.is_err() == true);
    }

    // -- Full YAML round-trip --

    #[test]
    fn test_full_template_config_parse() -> anyhow::Result<()>
    {
        let yaml = r#"
version: 5
main:
  source: AGENTS.md
  target: '$workspace/AGENTS.md'
agents:
  claude:
    instructions:
      - source: claude/CLAUDE.md
        target: '$workspace/CLAUDE.md'
    skills:
      - name: claude-skill
        source: 'https://github.com/user/claude-skills/tree/main/skill-a'
    directories:
      - target: '$workspace/.claude/plans'
shared:
  cmake:
    files:
      - source: cmake-build.md
        target: '$instructions'
    skills:
      - name: cmake-skill
        source: 'https://github.com/user/cmake-skills/tree/main/cmake-skill'
languages:
  c:
    includes: [cmake]
    files:
      - source: c.md
        target: '$instructions'
  rust:
    files:
      - source: rust.md
        target: '$instructions'
    skills:
      - name: rust-analyzer
        source: 'https://github.com/user/rust-skills/tree/main/rust-analyzer'
integration:
  git:
    files:
      - source: git.md
        target: '$instructions'
principles:
  - source: core.md
    target: '$instructions'
mission:
  - source: mission.md
    target: '$instructions'
skills:
  - name: my-skill
    source: 'https://github.com/user/repo/tree/main/skills/my-skill'
"#;
        let config: TemplateConfig = serde_yaml::from_str(yaml)?;
        assert_eq!(config.version, 5);
        assert!(config.main.is_some() == true);
        assert_eq!(config.main.as_ref().ok_or_else(|| anyhow::anyhow!("missing main config"))?.source, "AGENTS.md");
        assert!(config.agents.is_empty() == false);
        let claude_agent = config.agents.get("claude").ok_or_else(|| anyhow::anyhow!("missing claude agent"))?;
        assert_eq!(claude_agent.skills.len(), 1);
        assert_eq!(claude_agent.directories.len(), 1);
        assert!(config.shared.is_empty() == false);
        let cmake_shared = config.shared.get("cmake").ok_or_else(|| anyhow::anyhow!("missing cmake group"))?;
        assert_eq!(cmake_shared.files.len(), 1);
        assert_eq!(cmake_shared.skills.len(), 1);
        assert_eq!(cmake_shared.skills[0].name, "cmake-skill");
        assert_eq!(config.languages.len(), 2);
        assert!(config.languages.get("c").ok_or_else(|| anyhow::anyhow!("missing c language"))?.includes.is_empty() == false);
        assert!(config.languages.get("c").ok_or_else(|| anyhow::anyhow!("missing c language"))?.skills.is_empty() == true);
        assert!(config.languages.get("rust").ok_or_else(|| anyhow::anyhow!("missing rust language"))?.includes.is_empty() == true);
        assert_eq!(config.languages.get("rust").ok_or_else(|| anyhow::anyhow!("missing rust language"))?.skills.len(), 1);
        assert!(config.integration.is_empty() == false);
        assert!(config.principles.is_empty() == false);
        assert!(config.mission.is_empty() == false);
        assert!(config.skills.is_empty() == false);
        assert_eq!(config.skills[0].name, "my-skill");
        Ok(())
    }

    // -- LanguageConfig skills serde --

    #[test]
    fn test_language_config_skills_defaults_empty() -> anyhow::Result<()>
    {
        let yaml = "files:\n  - source: a.md\n    target: '$instructions'";
        let config: LanguageConfig = serde_yaml::from_str(yaml)?;
        assert!(config.skills.is_empty() == true);
        Ok(())
    }

    #[test]
    fn test_language_config_with_skills() -> anyhow::Result<()>
    {
        let yaml = r#"
files:
  - source: rust.md
    target: '$instructions'
skills:
  - name: rust-analyzer
    source: 'https://github.com/user/rust-skills/tree/main/rust-analyzer'
"#;
        let config: LanguageConfig = serde_yaml::from_str(yaml)?;
        assert_eq!(config.skills.len(), 1);
        assert_eq!(config.skills[0].name, "rust-analyzer");
        Ok(())
    }

    // -- AgentConfig skills serde --

    #[test]
    fn test_agent_config_skills_as_skill_definition() -> anyhow::Result<()>
    {
        let yaml = r#"
instructions:
  - source: cursor/cursorrules
    target: '$workspace/.cursorrules'
skills:
  - name: create-rule
    source: 'https://github.com/user/cursor-skills/tree/main/create-rule'
"#;
        let config: AgentConfig = serde_yaml::from_str(yaml)?;
        assert_eq!(config.skills.len(), 1);
        assert_eq!(config.skills[0].name, "create-rule");
        assert_eq!(config.instructions.len(), 1);
        Ok(())
    }

    // -- DirectoryEntry serde --

    #[test]
    fn test_directory_entry_basic() -> anyhow::Result<()>
    {
        let yaml = "target: '$workspace/.cursor/plans'";
        let entry: DirectoryEntry = serde_yaml::from_str(yaml)?;
        assert_eq!(entry.target, "$workspace/.cursor/plans");
        Ok(())
    }

    // -- AgentConfig directories serde --

    #[test]
    fn test_agent_config_directories_defaults_empty() -> anyhow::Result<()>
    {
        let yaml = "instructions:\n  - source: cursor/cursorrules\n    target: '$workspace/.cursorrules'";
        let config: AgentConfig = serde_yaml::from_str(yaml)?;
        assert!(config.directories.is_empty() == true);
        Ok(())
    }

    #[test]
    fn test_agent_config_with_directories() -> anyhow::Result<()>
    {
        let yaml = r#"
instructions:
  - source: cursor/cursorrules
    target: '$workspace/.cursorrules'
directories:
  - target: '$workspace/.cursor/plans'
"#;
        let config: AgentConfig = serde_yaml::from_str(yaml)?;
        assert_eq!(config.directories.len(), 1);
        assert_eq!(config.directories[0].target, "$workspace/.cursor/plans");
        Ok(())
    }

    // -- SharedConfig serde --

    #[test]
    fn test_shared_config_files_only() -> anyhow::Result<()>
    {
        let yaml = r#"
files:
  - source: cmake-build.md
    target: '$instructions'
"#;
        let config: SharedConfig = serde_yaml::from_str(yaml)?;
        assert_eq!(config.files.len(), 1);
        assert!(config.skills.is_empty() == true);
        Ok(())
    }

    #[test]
    fn test_shared_config_with_skills() -> anyhow::Result<()>
    {
        let yaml = r#"
files:
  - source: cmake-build.md
    target: '$instructions'
skills:
  - name: cmake-skill
    source: 'https://github.com/user/cmake-skills/tree/main/cmake-skill'
"#;
        let config: SharedConfig = serde_yaml::from_str(yaml)?;
        assert_eq!(config.files.len(), 1);
        assert_eq!(config.skills.len(), 1);
        assert_eq!(config.skills[0].name, "cmake-skill");
        Ok(())
    }

    #[test]
    fn test_shared_config_empty_files() -> anyhow::Result<()>
    {
        let yaml = r#"
files: []
skills:
  - name: only-skill
    source: 'https://github.com/user/repo/tree/main/only-skill'
"#;
        let config: SharedConfig = serde_yaml::from_str(yaml)?;
        assert!(config.files.is_empty() == true);
        assert_eq!(config.skills.len(), 1);
        Ok(())
    }

    // -- resolve_language_skills --

    #[test]
    fn test_resolve_language_skills_own_only() -> anyhow::Result<()>
    {
        let mut config = minimal_config();
        config.languages.insert("rust".to_string(), LanguageConfig {
            includes: vec![],
            files:    vec![make_mapping("rust.md", "$instructions")],
            skills:   vec![SkillDefinition { name: "rust-analyzer".to_string(), source: "https://example.com/rust-analyzer".to_string() }]
        });

        let skills = resolve_language_skills("rust", &config)?;
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "rust-analyzer");
        Ok(())
    }

    #[test]
    fn test_resolve_language_skills_from_shared() -> anyhow::Result<()>
    {
        let mut config = minimal_config();
        config.shared.insert("cmake".to_string(), SharedConfig {
            files:  vec![make_mapping("cmake.md", "$instructions")],
            skills: vec![SkillDefinition { name: "cmake-skill".to_string(), source: "https://example.com/cmake-skill".to_string() }]
        });
        config.languages.insert("c".to_string(), make_lang(vec!["cmake".to_string()], vec![make_mapping("c.md", "$instructions")]));

        let skills = resolve_language_skills("c", &config)?;
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "cmake-skill");
        Ok(())
    }

    #[test]
    fn test_resolve_language_skills_shared_plus_own() -> anyhow::Result<()>
    {
        let mut config = minimal_config();
        config.shared.insert("cmake".to_string(), SharedConfig {
            files:  vec![make_mapping("cmake.md", "$instructions")],
            skills: vec![SkillDefinition { name: "cmake-skill".to_string(), source: "https://example.com/cmake-skill".to_string() }]
        });
        config.languages.insert("c".to_string(), LanguageConfig {
            includes: vec!["cmake".to_string()],
            files:    vec![make_mapping("c.md", "$instructions")],
            skills:   vec![SkillDefinition { name: "c-skill".to_string(), source: "https://example.com/c-skill".to_string() }]
        });

        let skills = resolve_language_skills("c", &config)?;
        assert_eq!(skills.len(), 2);
        assert_eq!(skills[0].name, "cmake-skill");
        assert_eq!(skills[1].name, "c-skill");
        Ok(())
    }

    #[test]
    fn test_resolve_language_skills_inherit_from_language() -> anyhow::Result<()>
    {
        let mut config = minimal_config();
        config.languages.insert("swift".to_string(), LanguageConfig {
            includes: vec![],
            files:    vec![make_mapping("swift.md", "$instructions")],
            skills:   vec![SkillDefinition { name: "swift-skill".to_string(), source: "https://example.com/swift-skill".to_string() }]
        });
        config.languages.insert("swiftui".to_string(), LanguageConfig {
            includes: vec!["swift".to_string()],
            files:    vec![make_mapping("swiftui.md", "$instructions")],
            skills:   vec![SkillDefinition { name: "swiftui-skill".to_string(), source: "https://example.com/swiftui-skill".to_string() }]
        });

        let skills = resolve_language_skills("swiftui", &config)?;
        assert_eq!(skills.len(), 2);
        assert_eq!(skills[0].name, "swift-skill");
        assert_eq!(skills[1].name, "swiftui-skill");
        Ok(())
    }

    #[test]
    fn test_resolve_language_skills_multilevel_language_inherit() -> anyhow::Result<()>
    {
        let mut config = minimal_config();
        config.languages.insert("base".to_string(), LanguageConfig {
            includes: vec![],
            files:    vec![],
            skills:   vec![SkillDefinition { name: "base-skill".to_string(), source: "https://example.com/base-skill".to_string() }]
        });
        config.languages.insert("mid".to_string(), LanguageConfig {
            includes: vec!["base".to_string()],
            files:    vec![],
            skills:   vec![SkillDefinition { name: "mid-skill".to_string(), source: "https://example.com/mid-skill".to_string() }]
        });
        config.languages.insert("top".to_string(), LanguageConfig {
            includes: vec!["mid".to_string()],
            files:    vec![],
            skills:   vec![SkillDefinition { name: "top-skill".to_string(), source: "https://example.com/top-skill".to_string() }]
        });

        let skills = resolve_language_skills("top", &config)?;
        assert_eq!(skills.len(), 3);
        assert_eq!(skills[0].name, "base-skill");
        assert_eq!(skills[1].name, "mid-skill");
        assert_eq!(skills[2].name, "top-skill");
        Ok(())
    }

    #[test]
    fn test_resolve_language_skills_cycle_detection()
    {
        let mut config = minimal_config();
        config.languages.insert("a".to_string(), LanguageConfig { includes: vec!["b".to_string()], files: vec![], skills: vec![] });
        config.languages.insert("b".to_string(), LanguageConfig { includes: vec!["a".to_string()], files: vec![], skills: vec![] });

        let err = resolve_language_skills("a", &config).unwrap_err();
        assert!(err.to_string().contains("Circular include detected in skills") == true);
    }

    #[test]
    fn test_resolve_language_skills_shared_no_skills() -> anyhow::Result<()>
    {
        let mut config = minimal_config();
        config.shared.insert("cmake".to_string(), make_shared(vec![make_mapping("cmake.md", "$instructions")]));
        config.languages.insert("c".to_string(), make_lang(vec!["cmake".to_string()], vec![make_mapping("c.md", "$instructions")]));

        let skills = resolve_language_skills("c", &config)?;
        assert!(skills.is_empty() == true);
        Ok(())
    }

    #[test]
    fn test_resolve_language_skills_not_found()
    {
        let config = minimal_config();
        let err = resolve_language_skills("nonexistent", &config).unwrap_err();
        assert!(err.to_string().contains("not found in templates.yml") == true);
    }
}
