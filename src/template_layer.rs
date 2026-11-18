//! Template layers: additional template systems stacked on top of the baseline catalog
//!
//! The baseline is the catalog fetched from `templates.uri`. Every layer configured under
//! `templates.layers` is a complete template system of its own (a `templates.yml`, its files and
//! skills, and optionally an `agent-defaults.yml`) cached in `layers/<id>/` next to the baseline
//! cache. [`apply_layers`] merges the layers into the baseline in configuration order, item by item,
//! with the later system winning:
//!
//! - `main` is replaced; `$instructions` fragments are keyed by their source path relative to their system, every other file by its target; keyed sections
//!   (`languages`, `shared`, `integration`, `agents`) merge field by field
//! - skills in the lists that share install directories (top-level, languages, shared) are settled globally by name: a layer defining skill `x` anywhere replaces `x`
//!   in every such list, so one install slot never has two sources; agent skill lists merge per list
//!
//! Layer sources are rebased to absolute paths inside the layer cache so every existing
//! `config_dir.join(source)` site resolves them unchanged; URL skills are rebased to the cached copy
//! recorded in the layer's [`SourceManifest`].

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf}
};

use serde::{Deserialize, Serialize};

use crate::{
    Result,
    agent_defaults::{self, AGENT_DEFAULTS_FILE, AgentCatalog},
    bom::{self, FileMapping, SkillDefinition, TemplateConfig},
    config::{ConfigScope, LayerSpec, is_valid_system_id},
    download_manager::UrlSkillNames,
    github,
    template_engine::load_template_config,
    utils::rebase_source
};

/// File name of the manifest every fetch writes into a template system's cache directory
pub const SOURCE_MANIFEST_FILE: &str = ".slopctl-source.yml";

/// Id of the baseline template system when its `templates.yml` declares none
pub const BASELINE_ID: &str = "baseline";

/// Directory next to the baseline cache that holds one subdirectory per layer
const LAYER_CACHE_DIR: &str = "layers";

/// Records where a cached template system was fetched from
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceManifest
{
    pub uri:        String,
    #[serde(rename = "ref", default, skip_serializing_if = "Option::is_none")]
    pub git_ref:    Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path:       Option<String>,
    /// Commit id for git sources
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commit:     Option<String>,
    /// RFC 3339 timestamp of the fetch
    pub fetched:    String,
    /// Skill names discovered per URL-based skill source, cached under `skills/<name>/`
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub url_skills: UrlSkillNames
}

impl SourceManifest
{
    /// Reads the manifest from a cache directory; `None` when the directory was never fetched
    ///
    /// # Errors
    ///
    /// Returns an error if the manifest exists but cannot be read or parsed
    pub fn read(dir: &Path) -> Result<Option<Self>>
    {
        let path = dir.join(SOURCE_MANIFEST_FILE);
        require!(path.is_file() == true, Ok(None));
        let manifest = serde_yaml::from_str(&fs::read_to_string(&path)?).map_err(|e| anyhow::anyhow!("Invalid {}: {}", path.display(), e))?;
        Ok(Some(manifest))
    }

    /// Writes the manifest into a cache directory
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be written
    pub fn write(&self, dir: &Path) -> Result<()>
    {
        fs::write(dir.join(SOURCE_MANIFEST_FILE), serde_yaml::to_string(self)?)?;
        Ok(())
    }

    /// Returns true when the cache was fetched from exactly the configured source
    pub fn matches(&self, spec: &LayerSpec) -> bool
    {
        self.uri == spec.uri && self.git_ref == spec.git_ref && self.path == spec.path
    }
}

/// Returns the cache directory of layer `id`, a sibling of the baseline cache `config_dir`
pub fn layer_cache_dir(config_dir: &Path, id: &str) -> PathBuf
{
    config_dir.parent().unwrap_or(config_dir).join(LAYER_CACHE_DIR).join(id)
}

/// One template system taking part in the effective catalog
#[derive(Debug, Clone)]
pub struct TemplateSystem
{
    pub id:       String,
    /// Config scope that configured the layer; `None` for the baseline
    pub scope:    Option<ConfigScope>,
    /// Cache directory holding the system's `templates.yml`
    pub dir:      PathBuf,
    pub manifest: Option<SourceManifest>
}

/// An item of an earlier template system replaced by a later one
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Override
{
    /// Human-readable item, e.g. `skill 'review'` or `file '$workspace/.editorconfig' (languages.Rust++)`
    pub item:     String,
    /// Id of the system whose definition wins
    pub winner:   String,
    /// Id of the system whose definition was replaced
    pub shadowed: String
}

/// The template systems merged into an effective catalog, with provenance
#[derive(Debug, Default)]
pub struct TemplateSystems
{
    /// Baseline first, then layers in precedence order
    pub systems:      Vec<TemplateSystem>,
    pub overrides:    Vec<Override>,
    /// Ids of the systems defining each keyed entry, e.g. `"languages.swift" -> ["baseline", "private"]`
    pub contributors: BTreeMap<String, Vec<String>>
}

impl TemplateSystems
{
    /// Returns the id of the system whose cache contains `source`
    ///
    /// Layer sources are absolute paths inside their cache directory; everything else (relative
    /// baseline sources and files downloaded at install time) belongs to the baseline.
    pub fn origin_of(&self, source: &Path) -> Option<&str>
    {
        origin_in(&self.systems, source)
    }

    /// Returns true when a system with this id takes part in the effective catalog
    pub fn contains(&self, id: &str) -> bool
    {
        self.systems.iter().any(|system| system.id == id)
    }
}

/// Id of the system in `systems` (baseline first) whose cache contains `source`
fn origin_in<'a>(systems: &'a [TemplateSystem], source: &Path) -> Option<&'a str>
{
    let (baseline, layers) = systems.split_first()?;
    Some(layers.iter().find(|system| source.starts_with(&system.dir) == true).unwrap_or(baseline).id.as_str())
}

/// Merges the configured layers into the baseline catalogs
///
/// # Arguments
///
/// * `templates` / `agents` - Baseline catalogs, merged in place
/// * `config_dir` - Baseline cache directory (layer caches are its siblings)
/// * `layers` - Configured layers in precedence order
/// * `lenient` - Skip unusable layers with a warning instead of failing (remove, status)
///
/// # Errors
///
/// Returns an error if a layer is not fetched or out of sync with its configuration, its catalog is
/// invalid, its id or version does not match, or its sources escape its cache directory
pub fn apply_layers(
    templates: &mut TemplateConfig, agents: &mut AgentCatalog, config_dir: &Path, layers: &[(LayerSpec, ConfigScope)], lenient: bool
) -> Result<TemplateSystems>
{
    let baseline_id = templates.id.clone().unwrap_or_else(|| BASELINE_ID.to_string());
    require!(is_valid_system_id(&baseline_id) == true, Err(anyhow::anyhow!("Invalid template system id '{}' in baseline templates.yml", baseline_id)));

    let mut result = TemplateSystems {
        systems: vec![TemplateSystem {
            id:       baseline_id.clone(),
            scope:    None,
            dir:      config_dir.to_path_buf(),
            manifest: SourceManifest::read(config_dir).ok().flatten()
        }],
        ..Default::default()
    };
    record_contributors(&mut result.contributors, templates, &baseline_id);

    for (spec, scope) in layers
    {
        let dir = layer_cache_dir(config_dir, &spec.id);
        match load_layer(spec, &dir, templates.version)
        {
            | Ok((layer, layer_agents, manifest)) =>
            {
                require!(
                    result.contains(&spec.id) == false,
                    Err(anyhow::anyhow!("Template system id '{}' is used by more than one system; ids must be unique", spec.id))
                );
                result.systems.push(TemplateSystem { id: spec.id.clone(), scope: Some(*scope), dir, manifest: Some(manifest) });
                record_contributors(&mut result.contributors, &layer, &spec.id);

                let mut merger = Merger { systems: &result.systems, overrides: &mut result.overrides };
                merger.merge(templates, layer);
                if let Some(layer_agents) = layer_agents
                {
                    merger.merge_agent_defaults(agents, layer_agents);
                }
            }
            | Err(e) if lenient == true =>
            {
                eprintln!("! Skipping template layer '{}': {}", spec.id, e);
            }
            | Err(e) => return Err(e)
        }
    }

    Ok(result)
}

/// Loads, validates and rebases one fetched layer
fn load_layer(spec: &LayerSpec, dir: &Path, baseline_version: u32) -> Result<(TemplateConfig, Option<AgentCatalog>, SourceManifest)>
{
    let manifest =
        SourceManifest::read(dir)?.ok_or_else(|| anyhow::anyhow!("template layer '{}' has not been fetched yet; run 'slopctl templates --update'", spec.id))?;
    require!(
        manifest.matches(spec) == true,
        Err(anyhow::anyhow!("the configuration of template layer '{}' changed since it was fetched; run 'slopctl templates --update'", spec.id))
    );

    let mut config = load_template_config(dir).map_err(|e| anyhow::anyhow!("template layer '{}': {}", spec.id, e))?;
    match config.id.as_deref()
    {
        | Some(id) if id == spec.id =>
        {}
        | Some(id) => return Err(anyhow::anyhow!("template layer '{}' declares id '{}' in its templates.yml; the configured id must match", spec.id, id)),
        | None => return Err(anyhow::anyhow!("template layer '{}' must declare 'id: {}' in its templates.yml", spec.id, spec.id))
    }
    require!(
        config.version == baseline_version,
        Err(anyhow::anyhow!("template layer '{}' uses templates.yml version {}, the baseline uses {}", spec.id, config.version, baseline_version))
    );
    bom::require_unique_skill_sources(&config).map_err(|e| anyhow::anyhow!("template layer '{}': {}", spec.id, e))?;

    rebase_layer(&mut config, dir, &manifest).map_err(|e| anyhow::anyhow!("template layer '{}': {}", spec.id, e))?;
    let agents = if dir.join(AGENT_DEFAULTS_FILE).is_file() == true
    {
        Some(agent_defaults::load_agent_catalog_from_dir(dir).map_err(|e| anyhow::anyhow!("template layer '{}': {}", spec.id, e))?)
    }
    else
    {
        None
    };

    Ok((config, agents, manifest))
}

/// Rewrites every source of a layer catalog to an absolute path inside the layer cache
///
/// File sources must be relative paths. URL skill sources are replaced by their cached copies,
/// one definition per skill the URL yielded.
fn rebase_layer(config: &mut TemplateConfig, dir: &Path, manifest: &SourceManifest) -> Result<()>
{
    let mut mappings: Vec<&mut FileMapping> = Vec::new();
    mappings.extend(config.preamble.iter_mut().chain(config.principles.iter_mut()).chain(config.mission.iter_mut()));
    mappings.extend(config.languages.values_mut().flat_map(|lang| lang.files.iter_mut()));
    mappings.extend(config.shared.values_mut().flat_map(|group| group.files.iter_mut()));
    mappings.extend(config.integration.values_mut().flat_map(|group| group.files.iter_mut()));
    mappings.extend(config.agents.values_mut().flat_map(|agent| agent.instructions.iter_mut().chain(agent.prompts.iter_mut())));
    for mapping in mappings
    {
        mapping.source = rebase_file_source(&mapping.source, dir)?;
    }
    if let Some(main) = &mut config.main
    {
        main.source = rebase_file_source(&main.source, dir)?;
    }

    let mut skill_lists: Vec<&mut Vec<SkillDefinition>> = vec![&mut config.skills];
    skill_lists.extend(config.languages.values_mut().map(|lang| &mut lang.skills));
    skill_lists.extend(config.shared.values_mut().map(|group| &mut group.skills));
    skill_lists.extend(config.agents.values_mut().map(|agent| &mut agent.skills));
    for skills in skill_lists
    {
        let mut rebased = Vec::with_capacity(skills.len());
        for skill in skills.drain(..)
        {
            if github::is_url(&skill.source) == true
            {
                let names = manifest
                    .url_skills
                    .get(&skill.source)
                    .ok_or_else(|| anyhow::anyhow!("skill '{}' is not cached; run 'slopctl templates --update'", skill.source))?;
                for name in names
                {
                    rebased.push(SkillDefinition { source: rebase_source(&format!("skills/{}", name), dir)?, target: skill.target.clone() });
                }
            }
            else
            {
                rebased.push(SkillDefinition { source: rebase_source(&skill.source, dir)?, target: skill.target });
            }
        }
        *skills = rebased;
    }
    Ok(())
}

/// Rebases a layer file source; URLs are only supported for skills
fn rebase_file_source(source: &str, dir: &Path) -> Result<String>
{
    require!(github::is_url(source) == false, Err(anyhow::anyhow!("file source '{}' must be a relative path; URL sources are only supported for skills", source)));
    rebase_source(source, dir)
}

/// Records `id` as a contributor of every keyed entry of `config`
fn record_contributors(contributors: &mut BTreeMap<String, Vec<String>>, config: &TemplateConfig, id: &str)
{
    let keys = config
        .languages
        .keys()
        .map(|name| format!("languages.{}", name))
        .chain(config.shared.keys().map(|name| format!("shared.{}", name)))
        .chain(config.integration.keys().map(|name| format!("integration.{}", name)))
        .chain(config.agents.keys().map(|name| format!("agents.{}", name)));
    for key in keys
    {
        let ids = contributors.entry(key).or_default();
        if ids.iter().any(|existing| existing == id) == false
        {
            ids.push(id.to_string());
        }
    }
}

/// Merges one layer into the effective catalog, recording overrides
struct Merger<'a>
{
    /// All systems merged so far, the layer being merged last
    systems:   &'a [TemplateSystem],
    overrides: &'a mut Vec<Override>
}

impl Merger<'_>
{
    /// Id of the layer being merged
    fn layer_id(&self) -> String
    {
        self.systems.last().map(|system| system.id.clone()).unwrap_or_default()
    }

    /// Id of the system a source string belongs to
    fn origin(&self, source: &str) -> String
    {
        origin_in(self.systems, Path::new(source)).unwrap_or(BASELINE_ID).to_string()
    }

    /// Source path relative to its system directory, with `/` separators (the fragment key)
    fn relative_source(&self, source: &str) -> String
    {
        let path = Path::new(source);
        self.systems
            .iter()
            .skip(1)
            .find_map(|system| path.strip_prefix(&system.dir).ok())
            .map_or_else(|| source.to_string(), |relative| relative.to_string_lossy().into_owned())
            .replace('\\', "/")
    }

    /// Records that `item`, defined by the system owning `shadowed_source`, was replaced
    fn record(&mut self, item: String, shadowed_source: &str)
    {
        let shadowed = self.origin(shadowed_source);
        let winner = self.layer_id();
        if self.overrides.iter().any(|existing| existing.item == item && existing.winner == winner) == false
        {
            self.overrides.push(Override { item, winner, shadowed });
        }
    }

    fn merge(&mut self, base: &mut TemplateConfig, layer: TemplateConfig)
    {
        if let Some(main) = layer.main
        {
            if let Some(previous) = &base.main &&
                previous.source != main.source
            {
                self.record("main template".to_string(), &previous.source.clone());
            }
            base.main = Some(main);
        }
        self.merge_files(&mut base.preamble, layer.preamble, "preamble");
        self.merge_files(&mut base.principles, layer.principles, "principles");
        self.merge_files(&mut base.mission, layer.mission, "mission");

        // Settle shared-list skills globally first: the layer's definition of a name replaces it in
        // every top-level, language and shared list of the base, keeping one source per install slot.
        let winners: BTreeMap<String, SkillDefinition> = layer
            .skills
            .iter()
            .chain(layer.languages.values().flat_map(|lang| &lang.skills))
            .chain(layer.shared.values().flat_map(|group| &group.skills))
            .map(|skill| (skill.derive_name().to_string(), skill.clone()))
            .collect();
        let mut base_lists: Vec<&mut Vec<SkillDefinition>> = vec![&mut base.skills];
        base_lists.extend(base.languages.values_mut().map(|lang| &mut lang.skills));
        base_lists.extend(base.shared.values_mut().map(|group| &mut group.skills));
        for list in base_lists
        {
            for skill in list.iter_mut()
            {
                if let Some(winner) = winners.get(skill.derive_name()) &&
                    winner.source != skill.source
                {
                    let item = format!("skill '{}'", skill.derive_name());
                    let previous = std::mem::replace(skill, winner.clone());
                    self.record(item, &previous.source);
                }
            }
        }

        self.merge_skills(&mut base.skills, layer.skills);
        for (name, lang) in layer.languages
        {
            let location = format!("languages.{}", name);
            match base.languages.get_mut(&name)
            {
                | Some(existing) =>
                {
                    for include in lang.includes
                    {
                        if existing.includes.contains(&include) == false
                        {
                            existing.includes.push(include);
                        }
                    }
                    self.merge_files(&mut existing.files, lang.files, &location);
                    self.merge_skills(&mut existing.skills, lang.skills);
                }
                | None =>
                {
                    base.languages.insert(name, lang);
                }
            }
        }
        for (name, group) in layer.shared
        {
            let location = format!("shared.{}", name);
            match base.shared.get_mut(&name)
            {
                | Some(existing) =>
                {
                    self.merge_files(&mut existing.files, group.files, &location);
                    self.merge_skills(&mut existing.skills, group.skills);
                }
                | None =>
                {
                    base.shared.insert(name, group);
                }
            }
        }
        for (name, group) in layer.integration
        {
            let location = format!("integration.{}", name);
            match base.integration.get_mut(&name)
            {
                | Some(existing) => self.merge_files(&mut existing.files, group.files, &location),
                | None =>
                {
                    base.integration.insert(name, group);
                }
            }
        }
        for (name, agent) in layer.agents
        {
            let location = format!("agents.{}", name);
            match base.agents.get_mut(&name)
            {
                | Some(existing) =>
                {
                    self.merge_files(&mut existing.instructions, agent.instructions, &location);
                    self.merge_files(&mut existing.prompts, agent.prompts, &location);
                    self.merge_agent_skills(&mut existing.skills, agent.skills, &location);
                    for directory in agent.directories
                    {
                        if existing.directories.iter().any(|known| known.target == directory.target) == false
                        {
                            existing.directories.push(directory);
                        }
                    }
                }
                | None =>
                {
                    base.agents.insert(name, agent);
                }
            }
        }
    }

    /// Merges file mappings: fragments by relative source, other files by target
    fn merge_files(&mut self, base: &mut Vec<FileMapping>, layer: Vec<FileMapping>, location: &str)
    {
        for mapping in layer
        {
            let key = self.file_key(&mapping);
            match base.iter().position(|existing| self.file_key(existing) == key)
            {
                | Some(position) =>
                {
                    let previous = std::mem::replace(&mut base[position], mapping);
                    if previous.source != base[position].source
                    {
                        let kind = if previous.target.starts_with("$instructions") == true
                        {
                            "fragment"
                        }
                        else
                        {
                            "file"
                        };
                        self.record(format!("{} '{}' ({})", kind, key, location), &previous.source);
                    }
                }
                | None => base.push(mapping)
            }
        }
    }

    /// Item key of a file mapping within its list
    fn file_key(&self, mapping: &FileMapping) -> String
    {
        if mapping.target.starts_with("$instructions") == true
        {
            self.relative_source(&mapping.source)
        }
        else
        {
            mapping.target.clone()
        }
    }

    /// Merges a shared-list skill list by name; overrides were already recorded by the global pass
    fn merge_skills(&mut self, base: &mut Vec<SkillDefinition>, layer: Vec<SkillDefinition>)
    {
        for skill in layer
        {
            match base.iter().position(|existing| existing.derive_name() == skill.derive_name())
            {
                | Some(position) => base[position] = skill,
                | None => base.push(skill)
            }
        }
    }

    /// Merges an agent's own skill list by name, recording replaced variants
    fn merge_agent_skills(&mut self, base: &mut Vec<SkillDefinition>, layer: Vec<SkillDefinition>, location: &str)
    {
        for skill in layer
        {
            match base.iter().position(|existing| existing.derive_name() == skill.derive_name())
            {
                | Some(position) =>
                {
                    let previous = std::mem::replace(&mut base[position], skill);
                    if previous.source != base[position].source
                    {
                        self.record(format!("skill '{}' ({})", previous.derive_name(), location), &previous.source);
                    }
                }
                | None => base.push(skill)
            }
        }
    }

    /// Merges a layer's `agent-defaults.yml` by agent name, later wins
    fn merge_agent_defaults(&mut self, base: &mut AgentCatalog, layer: AgentCatalog)
    {
        let winner = self.layer_id();
        for entry in layer.agents
        {
            match base.agents.iter().position(|existing| existing.name == entry.name)
            {
                | Some(position) =>
                {
                    let item = format!("agent defaults '{}'", entry.name);
                    if self.overrides.iter().any(|existing| existing.item == item && existing.winner == winner) == false
                    {
                        // Agent defaults carry no source path; the replaced entry is attributed to the
                        // system that introduced it last, which is the closest earlier contributor.
                        let shadowed = self.systems.iter().rev().nth(1).map(|system| system.id.clone()).unwrap_or_default();
                        self.overrides.push(Override { item, winner: winner.clone(), shadowed });
                    }
                    base.agents[position] = entry;
                }
                | None => base.agents.push(entry)
            }
        }
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    const BASELINE: &str = r#"version: 5
main:
  source: AGENTS.md
  target: '$workspace/AGENTS.md'
languages:
  Rust++:
    files:
      - source: rpp-hint.md
        target: '$instructions'
      - source: rpp.toml
        target: '$workspace/.rpp.toml'
    skills:
      - source: skills/review
  CppScript:
    skills:
      - source: skills/review
skills:
  - source: skills/review
agents:
  bogus:
    skills:
      - source: skills/init
"#;

    struct Fixture
    {
        root:       tempfile::TempDir,
        config_dir: PathBuf
    }

    impl Fixture
    {
        fn new() -> Self
        {
            let root = tempfile::TempDir::new().unwrap();
            let config_dir = root.path().join("templates");
            fs::create_dir_all(&config_dir).unwrap();
            fs::write(config_dir.join("templates.yml"), BASELINE).unwrap();
            Self { root, config_dir }
        }

        /// Writes a fetched layer cache with a matching manifest
        fn layer(&self, id: &str, templates: &str) -> LayerSpec
        {
            let dir = layer_cache_dir(&self.config_dir, id);
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("templates.yml"), templates).unwrap();
            let spec = LayerSpec { id: id.to_string(), uri: format!("/src/{}", id), ..Default::default() };
            SourceManifest { uri: spec.uri.clone(), fetched: "now".to_string(), ..Default::default() }.write(&dir).unwrap();
            spec
        }

        fn apply(&self, specs: &[LayerSpec], lenient: bool) -> Result<(TemplateConfig, AgentCatalog, TemplateSystems)>
        {
            let mut templates = load_template_config(&self.config_dir)?;
            let mut agents = AgentCatalog::default();
            let layers: Vec<(LayerSpec, ConfigScope)> = specs.iter().map(|spec| (spec.clone(), ConfigScope::Global)).collect();
            let systems = apply_layers(&mut templates, &mut agents, &self.config_dir, &layers, lenient)?;
            Ok((templates, agents, systems))
        }
    }

    fn sources(skills: &[SkillDefinition]) -> Vec<String>
    {
        skills.iter().map(|skill| skill.source.clone()).collect()
    }

    #[test]
    fn test_apply_layers_none_keeps_baseline()
    {
        let fixture = Fixture::new();
        let (templates, _, systems) = fixture.apply(&[], false).unwrap();
        assert_eq!(templates.languages.len(), 2);
        assert_eq!(systems.systems.len(), 1);
        assert_eq!(systems.systems[0].id, BASELINE_ID);
        assert!(systems.overrides.is_empty() == true);
    }

    #[test]
    fn test_apply_layers_skill_override_applies_to_every_shared_list()
    {
        let fixture = Fixture::new();
        let spec = fixture.layer("fake-layer", "version: 5\nid: fake-layer\nlanguages:\n  Rust++:\n    skills:\n      - source: skills/review\n");
        let (templates, _, systems) = fixture.apply(&[spec], false).unwrap();

        let layer_review = rebase_source("skills/review", &layer_cache_dir(&fixture.config_dir, "fake-layer")).unwrap();
        assert_eq!(sources(&templates.skills), vec![layer_review.clone()]);
        assert_eq!(sources(&templates.languages["Rust++"].skills), vec![layer_review.clone()]);
        assert_eq!(sources(&templates.languages["CppScript"].skills), vec![layer_review]);
        assert_eq!(sources(&templates.agents["bogus"].skills), vec!["skills/init".to_string()], "agent lists are not rewritten");
        assert_eq!(systems.overrides, vec![Override {
            item:     "skill 'review'".to_string(),
            winner:   "fake-layer".to_string(),
            shadowed: BASELINE_ID.to_string()
        }]);
        bom::require_unique_skill_sources(&templates).unwrap();
    }

    #[test]
    fn test_apply_layers_adds_skill_and_language_without_dropping_baseline()
    {
        let fixture = Fixture::new();
        let spec = fixture.layer(
            "fake-layer",
            "version: 5\nid: fake-layer\nlanguages:\n  Rust++:\n    skills:\n      - source: skills/fake-review\n  Mojo++:\n    files:\n      - source: m.md\n        target: \
             '$instructions'\n"
        );
        let (templates, _, systems) = fixture.apply(&[spec], false).unwrap();

        let names: Vec<&str> = templates.languages["Rust++"].skills.iter().map(SkillDefinition::derive_name).collect();
        assert_eq!(names, vec!["review", "fake-review"]);
        assert!(templates.languages.contains_key("Mojo++") == true);
        assert_eq!(systems.contributors["languages.Rust++"], vec![BASELINE_ID.to_string(), "fake-layer".to_string()]);
        assert!(systems.overrides.is_empty() == true);
    }

    #[test]
    fn test_apply_layers_fragments_append_or_replace_by_relative_source()
    {
        let fixture = Fixture::new();
        let spec =
            fixture.layer(
                "fake-layer",
                "version: 5\nid: fake-layer\nlanguages:\n  Rust++:\n    files:\n      - source: rpp-hint.md\n        target: '$instructions'\n      - source: \
                 extra.md\n        target: '$instructions'\n      - source: other.toml\n        target: '$workspace/.rpp.toml'\n"
            );
        let (templates, _, systems) = fixture.apply(&[spec], false).unwrap();

        let dir = layer_cache_dir(&fixture.config_dir, "fake-layer");
        let files: Vec<(String, String)> = templates.languages["Rust++"].files.iter().map(|m| (m.source.clone(), m.target.clone())).collect();
        assert_eq!(files, vec![
            (rebase_source("rpp-hint.md", &dir).unwrap(), "$instructions".to_string()),
            (rebase_source("other.toml", &dir).unwrap(), "$workspace/.rpp.toml".to_string()),
            (rebase_source("extra.md", &dir).unwrap(), "$instructions".to_string()),
        ]);
        let items: Vec<&str> = systems.overrides.iter().map(|o| o.item.as_str()).collect();
        assert_eq!(items, vec!["fragment 'rpp-hint.md' (languages.Rust++)", "file '$workspace/.rpp.toml' (languages.Rust++)"]);
    }

    #[test]
    fn test_apply_layers_later_layer_wins_and_attributes_shadowed_layer()
    {
        let fixture = Fixture::new();
        let first = fixture.layer("bogus-layer", "version: 5\nid: bogus-layer\nskills:\n  - source: skills/review\n");
        let second = fixture.layer("fake-layer", "version: 5\nid: fake-layer\nskills:\n  - source: skills/review\n");
        let (templates, _, systems) = fixture.apply(&[first, second], false).unwrap();

        assert!(templates.skills[0].source.contains("fake-layer") == true);
        assert_eq!(systems.overrides.last().unwrap().shadowed, "bogus-layer");
        assert_eq!(systems.origin_of(Path::new(&templates.skills[0].source)), Some("fake-layer"));
        assert_eq!(systems.origin_of(Path::new("rpp.toml")), Some(BASELINE_ID));
    }

    #[test]
    fn test_apply_layers_url_skill_expands_to_cached_names()
    {
        let fixture = Fixture::new();
        let url = "https://github.com/bogus/skills/tree/main";
        let spec = fixture.layer("fake-layer", &format!("version: 5\nid: fake-layer\nskills:\n  - source: '{}'\n", url));
        let dir = layer_cache_dir(&fixture.config_dir, "fake-layer");
        let mut manifest = SourceManifest::read(&dir).unwrap().unwrap();
        manifest.url_skills.insert(url.to_string(), vec!["alpha".to_string(), "beta".to_string()]);
        manifest.write(&dir).unwrap();

        let (templates, ..) = fixture.apply(&[spec], false).unwrap();
        let names: Vec<&str> = templates.skills.iter().map(SkillDefinition::derive_name).collect();
        assert_eq!(names, vec!["review", "alpha", "beta"]);
    }

    #[test]
    fn test_apply_layers_validation_errors()
    {
        let fixture = Fixture::new();
        let missing_id = fixture.layer("fake-layer", "version: 5\n");
        assert!(fixture.apply(&[missing_id], false).unwrap_err().to_string().contains("must declare 'id: fake-layer'") == true);

        let wrong_version = fixture.layer("fake-layer", "version: 4\nid: fake-layer\n");
        assert!(fixture.apply(&[wrong_version], false).unwrap_err().to_string().contains("version 4") == true);

        let url_file = fixture
            .layer("fake-layer", "version: 5\nid: fake-layer\nmission:\n  - source: 'https://github.com/bogus/x/tree/main/m.md'\n    target: '$instructions'\n");
        assert!(fixture.apply(&[url_file], false).unwrap_err().to_string().contains("only supported for skills") == true);

        let escaping = fixture.layer("fake-layer", "version: 5\nid: fake-layer\nmission:\n  - source: '../secret.md'\n    target: '$instructions'\n");
        assert!(fixture.apply(&[escaping], false).is_err() == true);
    }

    #[test]
    fn test_apply_layers_unfetched_or_changed_layer_strict_errors_lenient_skips()
    {
        let fixture = Fixture::new();
        let unfetched = LayerSpec { id: "fake-layer".to_string(), uri: "/src/fake-layer".to_string(), ..Default::default() };
        assert!(fixture.apply(std::slice::from_ref(&unfetched), false).unwrap_err().to_string().contains("not been fetched") == true);
        assert_eq!(fixture.apply(&[unfetched], true).unwrap().2.systems.len(), 1);

        let mut changed = fixture.layer("fake-layer", "version: 5\nid: fake-layer\n");
        changed.git_ref = Some("main".to_string());
        assert!(fixture.apply(&[changed], false).unwrap_err().to_string().contains("changed since it was fetched") == true);
        drop(fixture.root);
    }

    #[test]
    fn test_apply_layers_merges_agent_defaults_by_name()
    {
        let fixture = Fixture::new();
        let spec = fixture.layer("fake-layer", "version: 5\nid: fake-layer\nagents:\n  fake: {}\n");
        fs::write(
            layer_cache_dir(&fixture.config_dir, "fake-layer").join(AGENT_DEFAULTS_FILE),
            "version: 1\nagents:\n  - name: fake\n    markers: [.fake]\n    prompt_dir: $workspace/.fake/prompts\n    skill_dir: $workspace/.fake/skills\n    \
             reads_cross_client_skills: true\n"
        )
        .unwrap();

        let (templates, agents, _) = fixture.apply(&[spec], false).unwrap();
        assert!(templates.agents.contains_key("fake") == true);
        assert_eq!(agents.agents.iter().map(|a| a.name.as_str()).collect::<Vec<_>>(), vec!["fake"]);
    }
}
