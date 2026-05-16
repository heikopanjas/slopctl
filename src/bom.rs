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
#[derive(Debug, Default, Serialize, Deserialize)]
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
#[derive(Debug, Default, Serialize, Deserialize)]
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
#[derive(Debug, Default, Serialize, Deserialize)]
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
/// The skill name is derived from the last path component of `source` (which
/// must match the SKILL.md `name` frontmatter per the agentskills.io spec).
/// The optional `target` field overrides the default install directory:
/// - Absent: inferred from install context (agent/language routing)
/// - `"$workspace"`: agent's workspace skill dir, or `.agents/skills/` if no agent
/// - `"$userprofile"`: agent's userprofile skill dir, or `~/.agents/skills/` if no agent
/// - Any full path (e.g. `"$workspace/.agents/skills"`): resolved as-is
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDefinition
{
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>
}

impl SkillDefinition
{
    /// Derives the skill name from the last path component of `source`.
    ///
    /// For local paths returns the final path component. For GitHub URLs the last component is
    /// typically the repo name, but `discover_skills()` overrides this with
    /// the actual in-repo directory name (which matches the SKILL.md `name`
    /// frontmatter per the agentskills.io spec).
    pub fn derive_name(&self) -> &str
    {
        self.source.rsplit(['/', '\\']).next().filter(|s| s.is_empty() == false).unwrap_or(&self.source)
    }
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
    pub preamble:    Vec<FileMapping>,
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
    /// * `agent_name` - Name of the agent
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
            preamble:    vec![],
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
            .insert("Rust++".to_string(), make_lang(vec![], vec![make_mapping("rpp.md", "$instructions"), make_mapping("rpp.toml", "$workspace/.rpp.toml")]));

        let files = resolve_language_files("Rust++", &config)?;
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].source, "rpp.md");
        assert_eq!(files[1].source, "rpp.toml");
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
        shared.insert(
            "shared-build".to_string(),
            make_shared(vec![make_mapping("shared-build.md", "$instructions"), make_mapping("shared.gitignore", "$workspace/.gitignore")])
        );
        config.shared = shared;

        config.languages.insert("Rust++".to_string(), make_lang(vec!["shared-build".to_string()], vec![make_mapping("rpp.md", "$instructions")]));

        let files = resolve_language_files("Rust++", &config)?;
        assert_eq!(files.len(), 3);
        assert_eq!(files[0].source, "shared-build.md");
        assert_eq!(files[1].source, "shared.gitignore");
        assert_eq!(files[2].source, "rpp.md");
        Ok(())
    }

    // -- resolve_language_files: language includes --

    #[test]
    fn test_resolve_includes_another_language() -> anyhow::Result<()>
    {
        let mut config = minimal_config();
        config
            .languages
            .insert("Rust++".to_string(), make_lang(vec![], vec![make_mapping("rpp.md", "$instructions"), make_mapping("rpp.ini", "$workspace/.editorconfig")]));
        config.languages.insert("CppScript".to_string(), make_lang(vec!["Rust++".to_string()], vec![make_mapping("cppscript.md", "$instructions")]));

        let files = resolve_language_files("CppScript", &config)?;
        assert_eq!(files.len(), 3);
        assert_eq!(files[0].source, "rpp.md");
        assert_eq!(files[1].source, "rpp.ini");
        assert_eq!(files[2].source, "cppscript.md");
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
        shared.insert("shared-build".to_string(), make_shared(vec![make_mapping("shared-build.md", "$instructions")]));
        config.shared = shared;

        config.languages.insert("Rust++".to_string(), make_lang(vec![], vec![make_mapping("rpp.md", "$instructions")]));
        config
            .languages
            .insert("CppScript".to_string(), make_lang(vec!["shared-build".to_string(), "Rust++".to_string()], vec![make_mapping("extension.md", "$instructions")]));

        let files = resolve_language_files("CppScript", &config)?;
        assert_eq!(files.len(), 3);
        assert_eq!(files[0].source, "shared-build.md");
        assert_eq!(files[1].source, "rpp.md");
        assert_eq!(files[2].source, "extension.md");
        Ok(())
    }

    // -- resolve_language_files: include-only language (empty files) --

    #[test]
    fn test_resolve_include_only_language() -> anyhow::Result<()>
    {
        let mut config = minimal_config();
        config.languages.insert("Rust++".to_string(), make_lang(vec![], vec![make_mapping("rpp.md", "$instructions")]));
        config.languages.insert("CppScript".to_string(), make_lang(vec!["Rust++".to_string()], vec![]));

        let files = resolve_language_files("CppScript", &config)?;
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].source, "rpp.md");
        Ok(())
    }

    // -- resolve_language_files: error cases --

    #[test]
    fn test_resolve_circular_include()
    {
        let mut config = minimal_config();
        config.languages.insert("Rust++".to_string(), make_lang(vec!["CppScript".to_string()], vec![]));
        config.languages.insert("CppScript".to_string(), make_lang(vec!["Rust++".to_string()], vec![]));

        let err = resolve_language_files("Rust++", &config).unwrap_err();
        assert!(err.to_string().contains("Circular include") == true);
    }

    #[test]
    fn test_resolve_include_not_found()
    {
        let mut config = minimal_config();
        config.languages.insert("Rust++".to_string(), make_lang(vec!["nonexistent".to_string()], vec![]));

        let err = resolve_language_files("Rust++", &config).unwrap_err();
        assert!(err.to_string().contains("not found in shared or languages") == true);
    }

    #[test]
    fn test_resolve_include_not_found_no_shared_section()
    {
        let mut config = minimal_config();
        config.shared = HashMap::new();
        config.languages.insert("Rust++".to_string(), make_lang(vec!["missing".to_string()], vec![]));

        let err = resolve_language_files("Rust++", &config).unwrap_err();
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

        config.languages.insert("Rust++".to_string(), make_lang(vec!["group".to_string()], vec![make_mapping("b.ini", "$workspace/.editorconfig")]));

        let err = resolve_language_files("Rust++", &config).unwrap_err();
        assert!(err.to_string().contains("Duplicate target") == true);
        assert!(err.to_string().contains(".editorconfig") == true);
    }

    #[test]
    fn test_resolve_multiple_instructions_targets_allowed() -> anyhow::Result<()>
    {
        let mut config = minimal_config();
        config.languages.insert(
            "Rust++".to_string(),
            make_lang(vec![], vec![make_mapping("coding.md", "$instructions"), make_mapping("build.md", "$instructions"), make_mapping("extra.md", "$instructions")])
        );

        let files = resolve_language_files("Rust++", &config)?;
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

        config.languages.insert("Rust++".to_string(), make_lang(vec!["group".to_string()], vec![make_mapping("own.md", "$instructions")]));

        let files = resolve_language_files("Rust++", &config)?;
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
        bom.agent_files.insert("bogus".to_string(), vec![PathBuf::from("./.bogus/instructions.md")]);

        assert!(bom.has_agent("bogus") == true);
        assert!(bom.has_agent("fake") == false);
    }

    #[test]
    fn test_bom_get_agent_files() -> anyhow::Result<()>
    {
        let mut bom = BillOfMaterials::new();
        bom.agent_files.insert("bogus".to_string(), vec![PathBuf::from("./.bogus/instructions.md")]);

        assert!(bom.get_agent_files("bogus").is_some() == true);
        assert_eq!(bom.get_agent_files("bogus").ok_or_else(|| anyhow::anyhow!("missing bogus agent files"))?.len(), 1);
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
        assert!(BillOfMaterials::resolve_workspace_path("$userprofile/.bogus/prompts/init.md").is_none() == true);
    }

    #[test]
    fn test_resolve_workspace_path_instructions()
    {
        assert!(BillOfMaterials::resolve_workspace_path("$instructions").is_none() == true);
    }

    #[test]
    fn test_resolve_workspace_path_workspace() -> anyhow::Result<()>
    {
        let result = BillOfMaterials::resolve_workspace_path("$workspace/.bogus/instructions.md");
        assert_eq!(result.ok_or_else(|| anyhow::anyhow!("expected workspace path"))?, PathBuf::from("./.bogus/instructions.md"));
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
  bogus:
    instructions:
      - source: bogus/instructions.md
        target: '$workspace/.bogus/instructions.md'
    prompts:
      - source: bogus/commands/init.md
        target: '$workspace/.bogus/commands/init.md'
  fake:
    prompts:
      - source: fake/init.md
        target: '$userprofile/.fake/prompts/init.md'
"#;
        fs::write(&config_path, yaml)?;

        let bom = BillOfMaterials::from_config(&config_path)?;
        assert!(bom.has_agent("bogus") == true);
        assert_eq!(bom.get_agent_files("bogus").ok_or_else(|| anyhow::anyhow!("missing bogus agent files"))?.len(), 2);
        // fake has only $userprofile paths, so all are skipped -> no entry
        assert!(bom.has_agent("fake") == false);
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
  bogus:
    instructions:
      - source: bogus/instructions.md
        target: '$workspace/.bogus/instructions.md'
    skills:
      - name: create-rule
        source: 'https://github.com/user/bogus-skills/tree/main/create-rule'
"#;
        fs::write(&config_path, yaml)?;

        let bom = BillOfMaterials::from_config(&config_path)?;
        assert!(bom.has_agent("bogus") == true);
        // Skills are SkillDefinition (no target), so only instructions contribute to BoM
        assert_eq!(bom.get_agent_files("bogus").ok_or_else(|| anyhow::anyhow!("missing bogus agent files"))?.len(), 1);
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
  bogus:
    instructions:
      - source: bogus/instructions.md
        target: '$workspace/.bogus/instructions.md'
    skills:
      - source: 'https://github.com/user/bogus-skills/tree/main/skill-a'
    directories:
      - target: '$workspace/.bogus/plans'
shared:
  shared-build:
    files:
      - source: shared-build.md
        target: '$instructions'
    skills:
      - source: 'https://github.com/user/shared-skills/tree/main/shared-skill'
languages:
  CppScript:
    includes: [shared-build]
    files:
      - source: cppscript.md
        target: '$instructions'
  Rust++:
    files:
      - source: rpp.md
        target: '$instructions'
    skills:
      - source: 'https://github.com/user/rpp-skills/tree/main/rpp-analyzer'
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
  - source: 'https://github.com/user/repo/tree/main/skills/my-skill'
"#;
        let config: TemplateConfig = serde_yaml::from_str(yaml)?;
        assert_eq!(config.version, 5);
        assert!(config.main.is_some() == true);
        assert_eq!(config.main.as_ref().ok_or_else(|| anyhow::anyhow!("missing main config"))?.source, "AGENTS.md");
        assert!(config.agents.is_empty() == false);
        let bogus_config = config.agents.get("bogus").ok_or_else(|| anyhow::anyhow!("missing bogus config"))?;
        assert_eq!(bogus_config.skills.len(), 1);
        assert_eq!(bogus_config.directories.len(), 1);
        assert!(config.shared.is_empty() == false);
        let shared_build = config.shared.get("shared-build").ok_or_else(|| anyhow::anyhow!("missing shared group"))?;
        assert_eq!(shared_build.files.len(), 1);
        assert_eq!(shared_build.skills.len(), 1);
        assert_eq!(shared_build.skills[0].derive_name(), "shared-skill");
        assert_eq!(config.languages.len(), 2);
        assert!(config.languages.get("CppScript").ok_or_else(|| anyhow::anyhow!("missing CppScript language"))?.includes.is_empty() == false);
        assert!(config.languages.get("CppScript").ok_or_else(|| anyhow::anyhow!("missing CppScript language"))?.skills.is_empty() == true);
        assert!(config.languages.get("Rust++").ok_or_else(|| anyhow::anyhow!("missing Rust++ language"))?.includes.is_empty() == true);
        assert_eq!(config.languages.get("Rust++").ok_or_else(|| anyhow::anyhow!("missing Rust++ language"))?.skills.len(), 1);
        assert!(config.integration.is_empty() == false);
        assert!(config.principles.is_empty() == false);
        assert!(config.mission.is_empty() == false);
        assert!(config.skills.is_empty() == false);
        assert_eq!(config.skills[0].derive_name(), "my-skill");
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
  - source: rpp.md
    target: '$instructions'
skills:
  - source: 'https://github.com/user/rpp-skills/tree/main/rpp-analyzer'
"#;
        let config: LanguageConfig = serde_yaml::from_str(yaml)?;
        assert_eq!(config.skills.len(), 1);
        assert_eq!(config.skills[0].derive_name(), "rpp-analyzer");
        Ok(())
    }

    // -- AgentConfig skills serde --

    #[test]
    fn test_agent_config_skills_as_skill_definition() -> anyhow::Result<()>
    {
        let yaml = r#"
instructions:
  - source: bogus/instructions.md
    target: '$workspace/.bogus/instructions.md'
skills:
  - source: 'https://github.com/user/bogus-skills/tree/main/create-rule'
"#;
        let config: AgentConfig = serde_yaml::from_str(yaml)?;
        assert_eq!(config.skills.len(), 1);
        assert_eq!(config.skills[0].derive_name(), "create-rule");
        assert_eq!(config.instructions.len(), 1);
        Ok(())
    }

    // -- DirectoryEntry serde --

    #[test]
    fn test_directory_entry_basic() -> anyhow::Result<()>
    {
        let yaml = "target: '$workspace/.bogus/plans'";
        let entry: DirectoryEntry = serde_yaml::from_str(yaml)?;
        assert_eq!(entry.target, "$workspace/.bogus/plans");
        Ok(())
    }

    // -- AgentConfig directories serde --

    #[test]
    fn test_agent_config_directories_defaults_empty() -> anyhow::Result<()>
    {
        let yaml = "instructions:\n  - source: bogus/instructions.md\n    target: '$workspace/.bogus/instructions.md'";
        let config: AgentConfig = serde_yaml::from_str(yaml)?;
        assert!(config.directories.is_empty() == true);
        Ok(())
    }

    #[test]
    fn test_agent_config_with_directories() -> anyhow::Result<()>
    {
        let yaml = r#"
instructions:
  - source: bogus/instructions.md
    target: '$workspace/.bogus/instructions.md'
directories:
  - target: '$workspace/.bogus/plans'
"#;
        let config: AgentConfig = serde_yaml::from_str(yaml)?;
        assert_eq!(config.directories.len(), 1);
        assert_eq!(config.directories[0].target, "$workspace/.bogus/plans");
        Ok(())
    }

    // -- SharedConfig serde --

    #[test]
    fn test_shared_config_files_only() -> anyhow::Result<()>
    {
        let yaml = r#"
files:
  - source: shared-build.md
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
  - source: shared-build.md
    target: '$instructions'
skills:
  - source: 'https://github.com/user/shared-skills/tree/main/shared-skill'
"#;
        let config: SharedConfig = serde_yaml::from_str(yaml)?;
        assert_eq!(config.files.len(), 1);
        assert_eq!(config.skills.len(), 1);
        assert_eq!(config.skills[0].derive_name(), "shared-skill");
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
        config.languages.insert("Rust++".to_string(), LanguageConfig {
            includes: vec![],
            files:    vec![make_mapping("rpp.md", "$instructions")],
            skills:   vec![SkillDefinition { source: "https://example.com/rpp-skill".to_string(), target: None }]
        });

        let skills = resolve_language_skills("Rust++", &config)?;
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].derive_name(), "rpp-skill");
        Ok(())
    }

    #[test]
    fn test_resolve_language_skills_from_shared() -> anyhow::Result<()>
    {
        let mut config = minimal_config();
        config.shared.insert("shared-build".to_string(), SharedConfig {
            files:  vec![make_mapping("shared-build.md", "$instructions")],
            skills: vec![SkillDefinition { source: "https://example.com/shared-skill".to_string(), target: None }]
        });
        config.languages.insert("Rust++".to_string(), make_lang(vec!["shared-build".to_string()], vec![make_mapping("rpp.md", "$instructions")]));

        let skills = resolve_language_skills("Rust++", &config)?;
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].derive_name(), "shared-skill");
        Ok(())
    }

    #[test]
    fn test_resolve_language_skills_shared_plus_own() -> anyhow::Result<()>
    {
        let mut config = minimal_config();
        config.shared.insert("shared-build".to_string(), SharedConfig {
            files:  vec![make_mapping("shared-build.md", "$instructions")],
            skills: vec![SkillDefinition { source: "https://example.com/shared-skill".to_string(), target: None }]
        });
        config.languages.insert("Rust++".to_string(), LanguageConfig {
            includes: vec!["shared-build".to_string()],
            files:    vec![make_mapping("rpp.md", "$instructions")],
            skills:   vec![SkillDefinition { source: "https://example.com/rpp-skill".to_string(), target: None }]
        });

        let skills = resolve_language_skills("Rust++", &config)?;
        assert_eq!(skills.len(), 2);
        assert_eq!(skills[0].derive_name(), "shared-skill");
        assert_eq!(skills[1].derive_name(), "rpp-skill");
        Ok(())
    }

    #[test]
    fn test_resolve_language_skills_inherit_from_language() -> anyhow::Result<()>
    {
        let mut config = minimal_config();
        config.languages.insert("Rust++".to_string(), LanguageConfig {
            includes: vec![],
            files:    vec![make_mapping("rpp.md", "$instructions")],
            skills:   vec![SkillDefinition { source: "https://example.com/rpp-skill".to_string(), target: None }]
        });
        config.languages.insert("CppScript".to_string(), LanguageConfig {
            includes: vec!["Rust++".to_string()],
            files:    vec![make_mapping("cppscript.md", "$instructions")],
            skills:   vec![SkillDefinition { source: "https://example.com/cppscript-skill".to_string(), target: None }]
        });

        let skills = resolve_language_skills("CppScript", &config)?;
        assert_eq!(skills.len(), 2);
        assert_eq!(skills[0].derive_name(), "rpp-skill");
        assert_eq!(skills[1].derive_name(), "cppscript-skill");
        Ok(())
    }

    #[test]
    fn test_resolve_language_skills_language_inherit_preserves_order() -> anyhow::Result<()>
    {
        let mut config = minimal_config();
        config.languages.insert("Rust++".to_string(), LanguageConfig {
            includes: vec![],
            files:    vec![],
            skills:   vec![SkillDefinition { source: "https://example.com/rpp-skill".to_string(), target: None }]
        });
        config.languages.insert("CppScript".to_string(), LanguageConfig {
            includes: vec!["Rust++".to_string()],
            files:    vec![],
            skills:   vec![SkillDefinition { source: "https://example.com/cppscript-skill".to_string(), target: None }]
        });

        let skills = resolve_language_skills("CppScript", &config)?;
        assert_eq!(skills.len(), 2);
        assert_eq!(skills[0].derive_name(), "rpp-skill");
        assert_eq!(skills[1].derive_name(), "cppscript-skill");
        Ok(())
    }

    #[test]
    fn test_resolve_language_skills_cycle_detection()
    {
        let mut config = minimal_config();
        config.languages.insert("Rust++".to_string(), LanguageConfig { includes: vec!["CppScript".to_string()], files: vec![], skills: vec![] });
        config.languages.insert("CppScript".to_string(), LanguageConfig { includes: vec!["Rust++".to_string()], files: vec![], skills: vec![] });

        let err = resolve_language_skills("Rust++", &config).unwrap_err();
        assert!(err.to_string().contains("Circular include detected in skills") == true);
    }

    #[test]
    fn test_resolve_language_skills_shared_no_skills() -> anyhow::Result<()>
    {
        let mut config = minimal_config();
        config.shared.insert("shared-build".to_string(), make_shared(vec![make_mapping("shared-build.md", "$instructions")]));
        config.languages.insert("Rust++".to_string(), make_lang(vec!["shared-build".to_string()], vec![make_mapping("rpp.md", "$instructions")]));

        let skills = resolve_language_skills("Rust++", &config)?;
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
