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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub instructions: Vec<FileMapping>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prompts:      Vec<FileMapping>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skills:       Vec<FileMapping>
}

/// Language configuration with files and optional includes
///
/// Languages can include shared file groups or other languages via `includes`.
/// Resolution order: included files first (depth-first), then own `files`.
#[derive(Debug, Serialize, Deserialize)]
pub struct LanguageConfig
{
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub includes: Vec<String>,
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
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub agents:      HashMap<String, AgentConfig>,
    pub languages:   HashMap<String, LanguageConfig>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub shared:      HashMap<String, Vec<FileMapping>>,
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
        if entry.target.starts_with("$instructions") == true
        {
            continue;
        }
        if let Some(previous_source) = seen_targets.insert(&entry.target, &entry.source)
        {
            return Err(format!(
                "Duplicate target '{}' in language '{}': '{}' and '{}' both write to the same file",
                entry.target, lang, previous_source, entry.source
            )
            .into());
        }
    }

    Ok(files)
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

    for include_name in &lang_config.includes
    {
        // Check shared groups first
        if let Some(shared_files) = config.shared.get(include_name.as_str())
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

        for (agent_name, agent_config) in template_config.agents
        {
            let mut file_paths = Vec::new();

            for mapping in agent_config.instructions.iter().chain(&agent_config.prompts).chain(&agent_config.skills)
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
    use std::{error::Error, result::Result};

    use super::*;

    fn make_mapping(source: &str, target: &str) -> FileMapping
    {
        FileMapping { source: source.to_string(), target: target.to_string() }
    }

    fn minimal_config() -> TemplateConfig
    {
        TemplateConfig {
            version:     3,
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
    fn test_default_version_returns_3()
    {
        assert_eq!(default_version(), 3);
    }

    // -- TemplateConfig serde --

    #[test]
    fn test_template_config_version_defaults_to_3() -> Result<(), Box<dyn Error>>
    {
        let yaml = "languages: {}";
        let config: TemplateConfig = serde_yaml::from_str(yaml)?;
        assert_eq!(config.version, 3);
        Ok(())
    }

    #[test]
    fn test_template_config_explicit_version() -> Result<(), Box<dyn Error>>
    {
        let yaml = "version: 2\nlanguages: {}";
        let config: TemplateConfig = serde_yaml::from_str(yaml)?;
        assert_eq!(config.version, 2);
        Ok(())
    }

    #[test]
    fn test_template_config_optional_fields_absent() -> Result<(), Box<dyn Error>>
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
    fn test_language_config_files_defaults_empty() -> Result<(), Box<dyn Error>>
    {
        let yaml = "includes: [foo]";
        let config: LanguageConfig = serde_yaml::from_str(yaml)?;
        assert!(config.files.is_empty() == true);
        assert_eq!(config.includes.len(), 1);
        Ok(())
    }

    #[test]
    fn test_language_config_includes_absent() -> Result<(), Box<dyn Error>>
    {
        let yaml = "files:\n  - source: a.md\n    target: '$instructions'";
        let config: LanguageConfig = serde_yaml::from_str(yaml)?;
        assert!(config.includes.is_empty() == true);
        assert_eq!(config.files.len(), 1);
        Ok(())
    }

    // -- resolve_language_files: basic --

    #[test]
    fn test_resolve_simple_language_no_includes() -> Result<(), Box<dyn Error>>
    {
        let mut config = minimal_config();
        config.languages.insert("rust".to_string(), LanguageConfig {
            includes: vec![],
            files:    vec![make_mapping("rust.md", "$instructions"), make_mapping("rust.toml", "$workspace/.rustfmt.toml")]
        });

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
    fn test_resolve_includes_shared_group() -> Result<(), Box<dyn Error>>
    {
        let mut config = minimal_config();
        let mut shared = HashMap::new();
        shared.insert("cmake".to_string(), vec![make_mapping("cmake-build.md", "$instructions"), make_mapping("cmake.gitignore", "$workspace/.gitignore")]);
        config.shared = shared;

        config.languages.insert("c".to_string(), LanguageConfig { includes: vec!["cmake".to_string()], files: vec![make_mapping("c.md", "$instructions")] });

        let files = resolve_language_files("c", &config)?;
        assert_eq!(files.len(), 3);
        assert_eq!(files[0].source, "cmake-build.md");
        assert_eq!(files[1].source, "cmake.gitignore");
        assert_eq!(files[2].source, "c.md");
        Ok(())
    }

    // -- resolve_language_files: language includes --

    #[test]
    fn test_resolve_includes_another_language() -> Result<(), Box<dyn Error>>
    {
        let mut config = minimal_config();
        config.languages.insert("swift".to_string(), LanguageConfig {
            includes: vec![],
            files:    vec![make_mapping("swift.md", "$instructions"), make_mapping("swift.ini", "$workspace/.editorconfig")]
        });
        config
            .languages
            .insert("swiftui".to_string(), LanguageConfig { includes: vec!["swift".to_string()], files: vec![make_mapping("swiftui.md", "$instructions")] });

        let files = resolve_language_files("swiftui", &config)?;
        assert_eq!(files.len(), 3);
        assert_eq!(files[0].source, "swift.md");
        assert_eq!(files[1].source, "swift.ini");
        assert_eq!(files[2].source, "swiftui.md");
        Ok(())
    }

    // -- resolve_language_files: transitive includes --

    #[test]
    fn test_resolve_transitive_includes() -> Result<(), Box<dyn Error>>
    {
        let mut config = minimal_config();
        let mut shared = HashMap::new();
        shared.insert("base".to_string(), vec![make_mapping("base.gitignore", "$workspace/.gitignore")]);
        config.shared = shared;

        config.languages.insert("a".to_string(), LanguageConfig { includes: vec!["base".to_string()], files: vec![make_mapping("a.md", "$instructions")] });
        config.languages.insert("b".to_string(), LanguageConfig { includes: vec!["a".to_string()], files: vec![make_mapping("b.md", "$instructions")] });

        let files = resolve_language_files("b", &config)?;
        assert_eq!(files.len(), 3);
        assert_eq!(files[0].source, "base.gitignore");
        assert_eq!(files[1].source, "a.md");
        assert_eq!(files[2].source, "b.md");
        Ok(())
    }

    // -- resolve_language_files: mixed shared + language includes --

    #[test]
    fn test_resolve_mixed_shared_and_language_includes() -> Result<(), Box<dyn Error>>
    {
        let mut config = minimal_config();
        let mut shared = HashMap::new();
        shared.insert("cmake".to_string(), vec![make_mapping("cmake.md", "$instructions")]);
        config.shared = shared;

        config.languages.insert("c".to_string(), LanguageConfig { includes: vec![], files: vec![make_mapping("c.md", "$instructions")] });
        config.languages.insert("c-ext".to_string(), LanguageConfig {
            includes: vec!["cmake".to_string(), "c".to_string()],
            files:    vec![make_mapping("ext.md", "$instructions")]
        });

        let files = resolve_language_files("c-ext", &config)?;
        assert_eq!(files.len(), 3);
        assert_eq!(files[0].source, "cmake.md");
        assert_eq!(files[1].source, "c.md");
        assert_eq!(files[2].source, "ext.md");
        Ok(())
    }

    // -- resolve_language_files: include-only language (empty files) --

    #[test]
    fn test_resolve_include_only_language() -> Result<(), Box<dyn Error>>
    {
        let mut config = minimal_config();
        config.languages.insert("base".to_string(), LanguageConfig { includes: vec![], files: vec![make_mapping("base.md", "$instructions")] });
        config.languages.insert("alias".to_string(), LanguageConfig { includes: vec!["base".to_string()], files: vec![] });

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
        config.languages.insert("a".to_string(), LanguageConfig { includes: vec!["b".to_string()], files: vec![] });
        config.languages.insert("b".to_string(), LanguageConfig { includes: vec!["a".to_string()], files: vec![] });

        let err = resolve_language_files("a", &config).unwrap_err();
        assert!(err.to_string().contains("Circular include") == true);
    }

    #[test]
    fn test_resolve_include_not_found()
    {
        let mut config = minimal_config();
        config.languages.insert("lang".to_string(), LanguageConfig { includes: vec!["nonexistent".to_string()], files: vec![] });

        let err = resolve_language_files("lang", &config).unwrap_err();
        assert!(err.to_string().contains("not found in shared or languages") == true);
    }

    #[test]
    fn test_resolve_include_not_found_no_shared_section()
    {
        let mut config = minimal_config();
        config.shared = HashMap::new();
        config.languages.insert("lang".to_string(), LanguageConfig { includes: vec!["missing".to_string()], files: vec![] });

        let err = resolve_language_files("lang", &config).unwrap_err();
        assert!(err.to_string().contains("not found in shared or languages") == true);
    }

    // -- resolve_language_files: duplicate target detection --

    #[test]
    fn test_resolve_duplicate_disk_target_rejected()
    {
        let mut config = minimal_config();
        let mut shared = HashMap::new();
        shared.insert("group".to_string(), vec![make_mapping("a.ini", "$workspace/.editorconfig")]);
        config.shared = shared;

        config
            .languages
            .insert("lang".to_string(), LanguageConfig { includes: vec!["group".to_string()], files: vec![make_mapping("b.ini", "$workspace/.editorconfig")] });

        let err = resolve_language_files("lang", &config).unwrap_err();
        assert!(err.to_string().contains("Duplicate target") == true);
        assert!(err.to_string().contains(".editorconfig") == true);
    }

    #[test]
    fn test_resolve_multiple_instructions_targets_allowed() -> Result<(), Box<dyn Error>>
    {
        let mut config = minimal_config();
        config.languages.insert("rust".to_string(), LanguageConfig {
            includes: vec![],
            files:    vec![make_mapping("coding.md", "$instructions"), make_mapping("build.md", "$instructions"), make_mapping("extra.md", "$instructions")]
        });

        let files = resolve_language_files("rust", &config)?;
        assert_eq!(files.len(), 3);
        Ok(())
    }

    #[test]
    fn test_resolve_duplicate_instructions_from_include_allowed() -> Result<(), Box<dyn Error>>
    {
        let mut config = minimal_config();
        let mut shared = HashMap::new();
        shared.insert("group".to_string(), vec![make_mapping("shared.md", "$instructions")]);
        config.shared = shared;

        config.languages.insert("lang".to_string(), LanguageConfig { includes: vec!["group".to_string()], files: vec![make_mapping("own.md", "$instructions")] });

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
    fn test_bom_get_agent_files() -> Result<(), Box<dyn Error>>
    {
        let mut bom = BillOfMaterials::new();
        bom.agent_files.insert("cursor".to_string(), vec![PathBuf::from("./.cursorrules")]);

        assert!(bom.get_agent_files("cursor").is_some() == true);
        assert_eq!(bom.get_agent_files("cursor").ok_or("missing cursor agent files")?.len(), 1);
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
    fn test_resolve_workspace_path_workspace() -> Result<(), Box<dyn Error>>
    {
        let result = BillOfMaterials::resolve_workspace_path("$workspace/CLAUDE.md");
        assert_eq!(result.ok_or("expected workspace path")?, PathBuf::from("./CLAUDE.md"));
        Ok(())
    }

    #[test]
    fn test_resolve_workspace_path_no_placeholder() -> Result<(), Box<dyn Error>>
    {
        let result = BillOfMaterials::resolve_workspace_path("relative/path.md");
        assert_eq!(result.ok_or("expected relative path")?, PathBuf::from("relative/path.md"));
        Ok(())
    }

    // -- BillOfMaterials::from_config --

    #[test]
    fn test_bom_from_config_with_agents() -> Result<(), Box<dyn Error>>
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
        assert_eq!(bom.get_agent_files("claude").ok_or("missing claude agent files")?.len(), 2);
        // codex has only $userprofile paths, so all are skipped -> no entry
        assert!(bom.has_agent("codex") == false);
        Ok(())
    }

    #[test]
    fn test_bom_from_config_no_agents() -> Result<(), Box<dyn Error>>
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
    fn test_bom_from_config_agent_with_skills() -> Result<(), Box<dyn Error>>
    {
        let dir = tempfile::TempDir::new()?;
        let config_path = dir.path().join("templates.yml");

        let yaml = r#"
languages: {}
agents:
  cursor:
    skills:
      - source: skills/my-skill/SKILL.md
        target: '$workspace/.cursor/skills/my-skill/SKILL.md'
"#;
        fs::write(&config_path, yaml)?;

        let bom = BillOfMaterials::from_config(&config_path)?;
        assert!(bom.has_agent("cursor") == true);
        assert_eq!(bom.get_agent_files("cursor").ok_or("missing cursor agent files")?.len(), 1);
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
    fn test_full_template_config_parse() -> Result<(), Box<dyn Error>>
    {
        let yaml = r#"
version: 3
main:
  source: AGENTS.md
  target: '$workspace/AGENTS.md'
agents:
  claude:
    instructions:
      - source: claude/CLAUDE.md
        target: '$workspace/CLAUDE.md'
shared:
  cmake:
    - source: cmake-build.md
      target: '$instructions'
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
        assert_eq!(config.version, 3);
        assert!(config.main.is_some() == true);
        assert_eq!(config.main.as_ref().ok_or("missing main config")?.source, "AGENTS.md");
        assert!(config.agents.is_empty() == false);
        assert!(config.shared.is_empty() == false);
        assert_eq!(config.shared.get("cmake").ok_or("missing cmake group")?.len(), 1);
        assert_eq!(config.languages.len(), 2);
        assert!(config.languages.get("c").ok_or("missing c language")?.includes.is_empty() == false);
        assert!(config.languages.get("rust").ok_or("missing rust language")?.includes.is_empty() == true);
        assert!(config.integration.is_empty() == false);
        assert!(config.principles.is_empty() == false);
        assert!(config.mission.is_empty() == false);
        assert!(config.skills.is_empty() == false);
        assert_eq!(config.skills[0].name, "my-skill");
        Ok(())
    }
}
