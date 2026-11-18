use std::{
    cell::Cell,
    collections::{BTreeSet, HashMap},
    fs,
    io::Read,
    path::{Path, PathBuf}
};

use owo_colors::OwoColorize;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Legacy sentinel for language-agnostic ownership.
pub const LANG_NONE: &str = "none";

/// Legacy sentinel for agent-agnostic ownership.
pub const AGENT_ALL: &str = "all";

/// Metadata about an installed template file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata
{
    pub original_sha:     String,
    /// SHA-256 of the template content the installed file was based on, stored only when it
    /// differs from `original_sha` (a merged file); `None` means the file is a plain template copy
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template_sha:     Option<String>,
    pub template_version: u32,
    pub installed_date:   String,
    #[serde(default)]
    pub lang:             Vec<String>,
    #[serde(default)]
    pub agent:            Vec<String>,
    #[serde(default)]
    pub ref_count:        usize,
    pub category:         String,
    /// Id of the template system that provided the installed content; `None` for legacy entries,
    /// AGENTS.md (which merges every system) and overlay agent files
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin:           Option<String>
}

impl FileMetadata
{
    /// SHA-256 of the template content the installed file was based on
    pub fn template_sha(&self) -> &str
    {
        self.template_sha.as_deref().unwrap_or(&self.original_sha)
    }

    /// Keep owner arrays sorted, unique, and reflected in `ref_count`.
    fn normalize_ownership(&mut self)
    {
        Self::normalize_owner_list(&mut self.lang, LANG_NONE);
        Self::normalize_owner_list(&mut self.agent, AGENT_ALL);
        self.ref_count = self.lang.len() + self.agent.len();
    }

    /// Remove empty and legacy sentinel values, then sort and deduplicate.
    fn normalize_owner_list(owners: &mut Vec<String>, sentinel: &str)
    {
        let mut normalized = BTreeSet::new();
        for owner in owners.iter()
        {
            if owner.is_empty() == false && owner != sentinel
            {
                normalized.insert(owner.clone());
            }
        }
        *owners = normalized.into_iter().collect();
    }

    /// Returns true when this entry is owned by the given language.
    pub fn has_lang(&self, lang: &str) -> bool
    {
        self.lang.iter().any(|owner| owner == lang)
    }

    /// Returns true when this entry is owned by the given agent.
    pub fn has_agent(&self, agent: &str) -> bool
    {
        self.agent.iter().any(|owner| owner == agent)
    }

    /// Returns true if the provided scalar owner values would add a new owner.
    pub fn would_add_owners(&self, lang: &str, agent: &str) -> bool
    {
        (lang != LANG_NONE && self.has_lang(lang) == false) || (agent != AGENT_ALL && self.has_agent(agent) == false)
    }

    /// Returns true if any owner in the provided arrays would add a new owner.
    pub fn would_add_owner_lists(&self, langs: &[String], agents: &[String]) -> bool
    {
        langs.iter().any(|lang| lang != LANG_NONE && self.has_lang(lang) == false) || agents.iter().any(|agent| agent != AGENT_ALL && self.has_agent(agent) == false)
    }

    /// Returns true when no language or agent references this entry.
    pub fn is_unreferenced(&self) -> bool
    {
        self.ref_count == 0
    }

    /// Returns true for an entry an older slopctl registered from disk instead of installing
    ///
    /// Registration recorded existing files with `template_version` 0, no owners and no origin,
    /// which made user files look like pristine installs. Such entries are dropped on load, so
    /// those files count as the user's again (`docs/SPEC.md`, SPEC-11).
    fn is_registration_leftover(&self) -> bool
    {
        self.template_version == 0 && self.is_unreferenced() == true && self.origin.is_none() == true
    }

    /// Add a unique language owner, incrementing `ref_count` only on change.
    fn add_lang(&mut self, lang: &str)
    {
        if lang != LANG_NONE && self.has_lang(lang) == false
        {
            self.lang.push(lang.to_string());
            self.normalize_ownership();
        }
    }

    /// Add a unique agent owner, incrementing `ref_count` only on change.
    fn add_agent(&mut self, agent: &str)
    {
        if agent != AGENT_ALL && self.has_agent(agent) == false
        {
            self.agent.push(agent.to_string());
            self.normalize_ownership();
        }
    }

    /// Release a language owner, decrementing `ref_count` only on change.
    fn release_lang(&mut self, lang: &str) -> bool
    {
        let before = self.lang.len();
        self.lang.retain(|owner| owner != lang);
        let changed = before != self.lang.len();
        if changed == true
        {
            self.normalize_ownership();
        }
        changed
    }

    /// Release an agent owner, decrementing `ref_count` only on change.
    fn release_agent(&mut self, agent: &str) -> bool
    {
        let before = self.agent.len();
        self.agent.retain(|owner| owner != agent);
        let changed = before != self.agent.len();
        if changed == true
        {
            self.normalize_ownership();
        }
        changed
    }
}

/// Status of a tracked file
#[derive(Debug, PartialEq)]
pub enum FileStatus
{
    /// File was never tracked by slopctl
    NotTracked,
    /// File exists and matches original SHA (user did not modify)
    Unmodified,
    /// File exists but SHA differs from original (user modified)
    Modified,
    /// File exists and matches what slopctl wrote, but that content was merged and differs from its template
    Merged,
    /// File was tracked but no longer exists on disk
    Deleted
}

/// Name of the workspace-local slopctl directory
pub const SLOPCTL_DIR: &str = ".slopctl";

/// Name of the tracker YAML file inside the slopctl directory
const TRACKER_FILE: &str = "tracker.yml";

/// Extension appended to an unreadable tracker file before it is replaced
const TRACKER_BACKUP_EXTENSION: &str = "bak";

/// Legacy tracker filename used in the global template directory
const LEGACY_TRACKER_FILE: &str = "installed_files.json";

/// Tracks installed template files using SHA checksums
///
/// Stores metadata in a workspace-local `.slopctl/tracker.yml` file.
/// All paths are stored relative to the workspace root.
pub struct FileTracker
{
    workspace:     PathBuf,
    metadata_path: PathBuf,
    metadata:      HashMap<String, FileMetadata>,
    /// The tracker file on disk could not be parsed; `save` backs it up before overwriting
    unparseable:   Cell<bool>
}

impl FileTracker
{
    /// Converts a file path to a relative, forward-slash-normalised key.
    ///
    /// Tries `Path::strip_prefix` first (works even when the file has been
    /// deleted).  Falls back to `fs::canonicalize` only when the raw prefix
    /// strip fails — e.g. when one side is a short 8.3 name and the other is
    /// long on Windows.  All backslashes are normalised to `/` so tracker keys
    /// are platform-independent.
    fn to_relative_key(&self, file_path: &Path) -> String
    {
        // Fast path: strip the workspace prefix directly (no I/O, works for deleted files)
        if let Ok(relative) = file_path.strip_prefix(&self.workspace)
        {
            return relative.to_string_lossy().replace('\\', "/");
        }

        // Slow path: canonicalize both sides to resolve symlinks, short names, etc.
        let absolute = fs::canonicalize(file_path).unwrap_or_else(|_| file_path.to_path_buf());
        let workspace_canon = fs::canonicalize(&self.workspace).unwrap_or_else(|_| self.workspace.clone());

        if let Ok(relative) = absolute.strip_prefix(&workspace_canon)
        {
            return relative.to_string_lossy().replace('\\', "/");
        }

        // Last resort: return the path as-is with normalised separators
        file_path.to_string_lossy().replace('\\', "/")
    }

    /// Create a new FileTracker for a workspace
    ///
    /// Loads existing tracker data from `.slopctl/tracker.yml` in the
    /// workspace root. Creates the `.slopctl/` directory if it does not exist.
    ///
    /// # Arguments
    ///
    /// * `workspace` - Absolute path to the workspace root directory
    ///
    /// # Errors
    ///
    /// Returns an error if the tracker file exists but cannot be read
    ///
    /// A tracker that exists but cannot be parsed (e.g. an obsolete format) starts empty with a
    /// warning; the next [`Self::save`] moves the old file to `tracker.yml.bak` instead of
    /// silently overwriting it.
    pub fn new(workspace: &Path) -> anyhow::Result<Self>
    {
        let slopctl_dir = workspace.join(SLOPCTL_DIR);
        let metadata_path = slopctl_dir.join(TRACKER_FILE);

        let mut unparseable = false;
        let mut metadata: HashMap<String, FileMetadata> = if metadata_path.exists() == true
        {
            let contents = fs::read_to_string(&metadata_path)?;
            match serde_yaml::from_str(&contents)
            {
                | Ok(metadata) => metadata,
                | Err(e) =>
                {
                    eprintln!(
                        "{} Ignoring unreadable tracker {} ({}); it will be backed up to {}.{} on the next write",
                        "!".yellow(),
                        metadata_path.display(),
                        e,
                        TRACKER_FILE,
                        TRACKER_BACKUP_EXTENSION
                    );
                    unparseable = true;
                    HashMap::new()
                }
            }
        }
        else
        {
            HashMap::new()
        };
        for meta in metadata.values_mut()
        {
            meta.normalize_ownership();
        }
        metadata.retain(|_, meta| meta.is_registration_leftover() == false);

        Ok(Self { workspace: workspace.to_path_buf(), metadata_path, metadata, unparseable: Cell::new(unparseable) })
    }

    /// Returns the workspace root this tracker is bound to
    pub fn workspace(&self) -> &Path
    {
        &self.workspace
    }

    /// Calculate SHA-256 checksum of a file
    pub fn calculate_sha256(file_path: &Path) -> anyhow::Result<String>
    {
        let mut file = fs::File::open(file_path)?;
        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 8192];

        loop
        {
            let bytes_read = file.read(&mut buffer)?;
            if bytes_read == 0
            {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }

        let hash = hasher.finalize();
        Ok(format!("{:x}", hash))
    }

    /// Calculate the SHA-256 checksum of in-memory content, formatted like `calculate_sha256`
    pub fn calculate_sha256_bytes(content: &[u8]) -> String
    {
        format!("{:x}", Sha256::digest(content))
    }

    /// Record a file installation with metadata
    ///
    /// The file path is stored relative to the workspace root.
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to the installed file (absolute or relative to workspace)
    /// * `original_sha` - SHA-256 of the file at install time
    /// * `template_version` - Template format version used
    /// * `lang` - Language owner or `LANG_NONE` when there is no language owner
    /// * `agent` - Agent owner or `AGENT_ALL` when there is no agent owner
    /// * `category` - Category tag (e.g. "main", "agent", "language", "skill")
    pub fn record_installation(&mut self, file_path: &Path, original_sha: String, template_version: u32, lang: String, agent: String, category: String)
    {
        self.record_installation_with_owners(file_path, original_sha, template_version, &[lang], &[agent], category);
    }

    /// Record a plain template copy with one or more language and agent owners.
    pub fn record_installation_with_owners(
        &mut self, file_path: &Path, original_sha: String, template_version: u32, langs: &[String], agents: &[String], category: String
    )
    {
        let template_sha = original_sha.clone();
        self.record_installation_with_template(file_path, original_sha, template_sha, template_version, langs, agents, category);
    }

    /// Record an installation whose content may differ from its template (e.g. a merged file).
    ///
    /// # Arguments
    ///
    /// * `original_sha` - SHA-256 of the content slopctl wrote
    /// * `template_sha` - SHA-256 of the template content it was based on
    #[allow(clippy::too_many_arguments)]
    pub fn record_installation_with_template(
        &mut self, file_path: &Path, original_sha: String, template_sha: String, template_version: u32, langs: &[String], agents: &[String], category: String
    )
    {
        let now = chrono::Utc::now().to_rfc3339();
        let relative_key = self.to_relative_key(file_path);
        let template_sha = (template_sha != original_sha).then_some(template_sha);

        if let Some(metadata) = self.metadata.get_mut(&relative_key)
        {
            metadata.original_sha = original_sha;
            metadata.template_sha = template_sha;
            metadata.template_version = template_version;
            metadata.installed_date = now;
            metadata.category = category;
            for lang in langs
            {
                metadata.add_lang(lang);
            }
            for agent in agents
            {
                metadata.add_agent(agent);
            }
            metadata.normalize_ownership();
        }
        else
        {
            let mut metadata = FileMetadata {
                original_sha,
                template_sha,
                template_version,
                installed_date: now,
                lang: Vec::new(),
                agent: Vec::new(),
                ref_count: 0,
                category,
                origin: None
            };
            for lang in langs
            {
                metadata.add_lang(lang);
            }
            for agent in agents
            {
                metadata.add_agent(agent);
            }
            metadata.normalize_ownership();
            self.metadata.insert(relative_key, metadata);
        }
    }

    /// Check the modification status of a file
    pub fn check_modification(&self, file_path: &Path) -> anyhow::Result<FileStatus>
    {
        let relative_key = self.to_relative_key(file_path);

        let metadata = match self.metadata.get(&relative_key)
        {
            | Some(meta) => meta,
            | None => return Ok(FileStatus::NotTracked)
        };

        let absolute = self.workspace.join(&relative_key);
        if absolute.exists() == false
        {
            return Ok(FileStatus::Deleted);
        }

        let current_sha = Self::calculate_sha256(&absolute)?;
        if current_sha == metadata.original_sha
        {
            if metadata.template_sha.is_some() == true
            {
                Ok(FileStatus::Merged)
            }
            else
            {
                Ok(FileStatus::Unmodified)
            }
        }
        else
        {
            Ok(FileStatus::Modified)
        }
    }

    /// Remove a tracked file entry
    pub fn remove_entry(&mut self, file_path: &Path)
    {
        let relative_key = self.to_relative_key(file_path);

        self.metadata.remove(&relative_key);
    }

    /// Get metadata for a tracked file
    pub fn get_metadata(&self, file_path: &Path) -> Option<&FileMetadata>
    {
        let relative_key = self.to_relative_key(file_path);

        self.metadata.get(&relative_key)
    }

    /// Returns the installed languages for this workspace
    ///
    /// Scans tracked entries and returns every language owner in sorted order.
    pub fn get_installed_languages(&self) -> Vec<String>
    {
        let mut languages = BTreeSet::new();
        for meta in self.metadata.values()
        {
            for lang in &meta.lang
            {
                languages.insert(lang.clone());
            }
        }

        languages.into_iter().collect()
    }

    /// Returns one installed language for compatibility with older call sites.
    pub fn get_installed_language(&self) -> Option<String>
    {
        self.get_installed_languages().into_iter().next()
    }

    /// Returns the installed agents for this workspace
    ///
    /// Scans tracked entries and returns every agent owner in sorted order.
    /// Unlike marker-directory detection, this reflects only what slopctl installed.
    pub fn get_installed_agents(&self) -> Vec<String>
    {
        let mut agents = BTreeSet::new();
        for meta in self.metadata.values()
        {
            for agent in &meta.agent
            {
                agents.insert(agent.clone());
            }
        }

        agents.into_iter().collect()
    }

    /// Returns all tracked file entries
    ///
    /// Each entry is a `(PathBuf, &FileMetadata)` tuple where the path is
    /// relative to the workspace root.
    pub fn get_entries(&self) -> Vec<(PathBuf, &FileMetadata)>
    {
        self.metadata.iter().map(|(path_str, meta)| (PathBuf::from(path_str), meta)).collect()
    }

    /// Returns tracked file entries filtered by category
    ///
    /// # Arguments
    ///
    /// * `category` - Category to filter by (e.g. "skill", "agent", "language")
    pub fn get_entries_by_category(&self, category: &str) -> Vec<(PathBuf, &FileMetadata)>
    {
        self.metadata.iter().filter(|(_path_str, meta)| meta.category == category).map(|(path_str, meta)| (PathBuf::from(path_str), meta)).collect()
    }

    /// Release a language owner for all entries whose `lang` and `category` match
    ///
    /// Used after `remove --lang` to clear language ownership from files that are
    /// intentionally kept on disk (e.g. AGENTS.md, `category: "main"`), so that
    /// `get_installed_languages()` no longer reports the language as installed.
    ///
    /// # Arguments
    ///
    /// * `lang` - Language name to clear
    /// * `category` - Only entries with this category are updated
    pub fn clear_lang_for_category(&mut self, lang: &str, category: &str)
    {
        for meta in self.metadata.values_mut()
        {
            if meta.category == category
            {
                meta.release_lang(lang);
            }
        }
    }

    /// Release an agent owner for all entries whose category matches.
    pub fn clear_agent_for_category(&mut self, agent: &str, category: &str)
    {
        for meta in self.metadata.values_mut()
        {
            if meta.category == category
            {
                meta.release_agent(agent);
            }
        }
    }

    /// Release a language owner from every tracked entry.
    ///
    /// Removing a language must release its ownership tracker-wide, not only on
    /// files queued for deletion, so `get_installed_languages()` stays truthful
    /// even for entries whose files were deleted manually or live outside the
    /// removal sweep.
    pub fn clear_lang_owner(&mut self, lang: &str)
    {
        for meta in self.metadata.values_mut()
        {
            meta.release_lang(lang);
        }
    }

    /// Release an agent owner from every tracked entry.
    ///
    /// Removing an agent must release its ownership tracker-wide; native-only
    /// agents (e.g. Claude) also own shared cross-client copies that are never
    /// queued for deletion, and a leaked owner would make `get_installed_agents()`
    /// report the agent as still installed.
    pub fn clear_agent_owner(&mut self, agent: &str)
    {
        for meta in self.metadata.values_mut()
        {
            meta.release_agent(agent);
        }
    }

    /// Release a language owner from a tracked file.
    pub fn release_lang(&mut self, file_path: &Path, lang: &str) -> bool
    {
        let relative_key = self.to_relative_key(file_path);
        self.metadata.get_mut(&relative_key).map(|meta| meta.release_lang(lang)).unwrap_or(false)
    }

    /// Release an agent owner from a tracked file.
    pub fn release_agent(&mut self, file_path: &Path, agent: &str) -> bool
    {
        let relative_key = self.to_relative_key(file_path);
        self.metadata.get_mut(&relative_key).map(|meta| meta.release_agent(agent)).unwrap_or(false)
    }

    /// Returns true when a tracked file has no remaining owners.
    pub fn is_unreferenced(&self, file_path: &Path) -> bool
    {
        self.get_metadata(file_path).map(|meta| meta.is_unreferenced()).unwrap_or(true)
    }

    /// Records which template system provided a tracked file
    ///
    /// Does nothing for untracked files; call after recording the installation.
    pub fn set_origin(&mut self, file_path: &Path, origin: Option<&str>)
    {
        let relative_key = self.to_relative_key(file_path);
        if let Some(metadata) = self.metadata.get_mut(&relative_key)
        {
            metadata.origin = origin.map(str::to_string);
        }
    }

    /// Save metadata to disk
    ///
    /// Creates the `.slopctl/` directory if it does not exist.
    pub fn save(&self) -> anyhow::Result<()>
    {
        if let Some(parent) = self.metadata_path.parent()
        {
            fs::create_dir_all(parent)?;
        }

        if self.unparseable.replace(false) == true && self.metadata_path.exists() == true
        {
            let backup = self.metadata_path.with_extension(format!("yml.{}", TRACKER_BACKUP_EXTENSION));
            fs::rename(&self.metadata_path, &backup)?;
            println!("{} Backed up unreadable tracker to {}", "→".blue(), backup.display().to_string().yellow());
        }

        let yaml = serde_yaml::to_string(&self.metadata)?;
        fs::write(&self.metadata_path, yaml)?;
        Ok(())
    }

    /// Migrate entries from the legacy global tracker to this workspace-local tracker
    ///
    /// Reads the global `installed_files.json`, extracts entries whose
    /// `workspace` field matches this tracker's workspace root, converts
    /// their absolute paths to relative, and inserts them. The migrated
    /// entries are removed from the global file which is saved back.
    ///
    /// # Arguments
    ///
    /// * `global_tracker_path` - Path to the global `installed_files.json`
    ///
    /// # Returns
    ///
    /// The number of entries migrated, or 0 if the global file does not exist.
    pub fn migrate_from_global(&mut self, global_tracker_path: &Path) -> anyhow::Result<usize>
    {
        if global_tracker_path.exists() == false
        {
            return Ok(0);
        }

        let contents = fs::read_to_string(global_tracker_path)?;

        #[derive(Serialize, Deserialize)]
        struct LegacyMetadata
        {
            original_sha:     String,
            template_version: u32,
            installed_date:   String,
            lang:             Option<String>,
            category:         String,
            #[serde(default)]
            workspace:        Option<String>
        }

        let global_entries: HashMap<String, LegacyMetadata> = serde_json::from_str(&contents).unwrap_or_else(|_| HashMap::new());

        let workspace_canon = fs::canonicalize(&self.workspace).unwrap_or_else(|_| self.workspace.clone());
        let workspace_str = workspace_canon.to_string_lossy();

        let mut migrated_keys: Vec<String> = Vec::new();
        let mut count = 0usize;

        for (abs_path, legacy) in &global_entries
        {
            if legacy.workspace.as_deref() == Some(workspace_str.as_ref())
            {
                let abs = PathBuf::from(abs_path);
                let relative = if let Ok(rel) = abs.strip_prefix(&workspace_canon)
                {
                    rel.to_string_lossy().replace('\\', "/")
                }
                else
                {
                    continue;
                };

                let mut metadata = FileMetadata {
                    original_sha:     legacy.original_sha.clone(),
                    template_sha:     None,
                    template_version: legacy.template_version,
                    installed_date:   legacy.installed_date.clone(),
                    lang:             Vec::new(),
                    agent:            Vec::new(),
                    ref_count:        0,
                    category:         legacy.category.clone(),
                    origin:           None
                };
                if let Some(lang) = &legacy.lang
                {
                    metadata.add_lang(lang);
                }
                metadata.normalize_ownership();
                self.metadata.insert(relative, metadata);

                migrated_keys.push(abs_path.clone());
                count += 1;
            }
        }

        if count > 0
        {
            self.save()?;

            let remaining: HashMap<String, LegacyMetadata> = global_entries.into_iter().filter(|(k, _)| migrated_keys.contains(k) == false).collect();

            if remaining.is_empty() == true
            {
                let _ = fs::remove_file(global_tracker_path);
            }
            else
            {
                let pruned_json = serde_json::to_string_pretty(&remaining)?;
                fs::write(global_tracker_path, pruned_json)?;
            }
        }

        Ok(count)
    }
}

/// Returns the path to the legacy global tracker file
///
/// Used during migration to locate the old `installed_files.json` that
/// lives alongside `templates.yml` in the global template directory.
pub fn legacy_tracker_path(global_template_dir: &Path) -> PathBuf
{
    global_template_dir.join(LEGACY_TRACKER_FILE)
}

#[cfg(test)]
mod tests
{
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn test_calculate_sha256() -> anyhow::Result<()>
    {
        let temp_dir = TempDir::new()?;
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, b"Hello, World!")?;

        let sha = FileTracker::calculate_sha256(&test_file)?;
        assert_eq!(sha, "dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f");
        assert_eq!(FileTracker::calculate_sha256_bytes(b"Hello, World!"), sha);

        Ok(())
    }

    #[test]
    fn test_record_installation_with_template_reports_merged() -> anyhow::Result<()>
    {
        let temp_dir = TempDir::new()?;
        let workspace = temp_dir.path();
        let mut tracker = FileTracker::new(workspace)?;

        let merged_file = workspace.join("merged.md");
        fs::write(&merged_file, b"template plus user lines")?;
        let installed_sha = FileTracker::calculate_sha256(&merged_file)?;
        let template_sha = FileTracker::calculate_sha256_bytes(b"template");

        tracker.record_installation_with_template(&merged_file, installed_sha.clone(), template_sha.clone(), 5, &[], &[], "language".into());
        assert_eq!(tracker.check_modification(&merged_file)?, FileStatus::Merged);
        assert_eq!(tracker.get_metadata(&merged_file).map(|metadata| metadata.template_sha().to_string()), Some(template_sha));

        fs::write(&merged_file, b"edited after the merge")?;
        assert_eq!(tracker.check_modification(&merged_file)?, FileStatus::Modified, "an edit after a merge is a modification");

        // A plain copy over a merged file clears the template SHA again.
        tracker.record_installation_with_owners(&merged_file, installed_sha.clone(), 5, &[], &[], "language".into());
        let metadata = tracker.get_metadata(&merged_file).expect("entry must exist");
        assert!(metadata.template_sha.is_none() == true);
        assert_eq!(metadata.template_sha(), installed_sha);

        Ok(())
    }

    #[test]
    fn test_load_drops_registration_leftovers() -> anyhow::Result<()>
    {
        let temp_dir = TempDir::new()?;
        let workspace = temp_dir.path();
        let mut tracker = FileTracker::new(workspace)?;

        // Older versions registered files from disk with version 0, no owners and no origin.
        tracker.record_installation(&workspace.join("registered.md"), "sha1".into(), 0, LANG_NONE.into(), AGENT_ALL.into(), "command".into());
        tracker.record_installation(&workspace.join("UPDATES.md"), "sha2".into(), 5, LANG_NONE.into(), AGENT_ALL.into(), "integration".into());
        tracker.record_installation(&workspace.join("lang.toml"), "sha3".into(), 0, "Rust++".into(), AGENT_ALL.into(), "language".into());
        tracker.save()?;

        let reloaded = FileTracker::new(workspace)?;
        assert!(reloaded.get_metadata(&workspace.join("registered.md")).is_none() == true, "a registration leftover must be dropped");
        assert!(reloaded.get_metadata(&workspace.join("UPDATES.md")).is_some() == true, "an installed file without owners must be kept");
        assert!(reloaded.get_metadata(&workspace.join("lang.toml")).is_some() == true, "an owned entry must be kept");

        Ok(())
    }

    #[test]
    fn test_plain_copy_omits_template_sha_in_tracker_file() -> anyhow::Result<()>
    {
        let temp_dir = TempDir::new()?;
        let workspace = temp_dir.path();
        let mut tracker = FileTracker::new(workspace)?;

        tracker.record_installation_with_template(&workspace.join("plain.md"), "sha1".into(), "sha1".into(), 5, &[], &[], "language".into());
        tracker.record_installation_with_template(&workspace.join("merged.md"), "sha2".into(), "sha3".into(), 5, &[], &[], "language".into());
        tracker.save()?;

        let saved = fs::read_to_string(workspace.join(SLOPCTL_DIR).join(TRACKER_FILE))?;
        assert_eq!(saved.matches("template_sha").count(), 1, "only the merged entry stores a template SHA:\n{}", saved);

        let reloaded = FileTracker::new(workspace)?;
        assert_eq!(reloaded.get_metadata(&workspace.join("merged.md")).map(|metadata| metadata.template_sha().to_string()), Some("sha3".to_string()));

        Ok(())
    }

    #[test]
    fn test_file_tracking() -> anyhow::Result<()>
    {
        let temp_dir = TempDir::new()?;
        let workspace = temp_dir.path();

        let mut tracker = FileTracker::new(workspace)?;

        let test_file = workspace.join("test.txt");
        fs::write(&test_file, b"Original content")?;

        let original_sha = FileTracker::calculate_sha256(&test_file)?;

        tracker.record_installation(&test_file, original_sha.clone(), 1, "Rust++".into(), AGENT_ALL.into(), "language".into());

        let status = tracker.check_modification(&test_file)?;
        assert_eq!(status, FileStatus::Unmodified);

        fs::write(&test_file, b"Modified content")?;
        let status = tracker.check_modification(&test_file)?;
        assert_eq!(status, FileStatus::Modified);

        fs::remove_file(&test_file)?;
        let status = tracker.check_modification(&test_file)?;
        assert_eq!(status, FileStatus::Deleted);

        Ok(())
    }

    #[test]
    fn test_get_installed_language() -> anyhow::Result<()>
    {
        let temp_dir = TempDir::new()?;
        let workspace = temp_dir.path();

        let mut tracker = FileTracker::new(workspace)?;
        let project_file = workspace.join("AGENTS.md");
        fs::write(&project_file, b"test")?;

        tracker.record_installation(&project_file, "sha123".into(), 1, "Rust++".into(), AGENT_ALL.into(), "main".into());
        let lang = tracker.get_installed_language();
        assert_eq!(lang, Some("Rust++".to_string()));

        Ok(())
    }

    #[test]
    fn test_record_installation_merges_unique_owners_and_ref_count() -> anyhow::Result<()>
    {
        let temp_dir = TempDir::new()?;
        let workspace = temp_dir.path();
        let shared_file = workspace.join(".gitignore");
        fs::write(&shared_file, b"target/\n")?;

        let mut tracker = FileTracker::new(workspace)?;
        tracker.record_installation(&shared_file, "sha1".into(), 5, "Rust++".into(), AGENT_ALL.into(), "language".into());
        tracker.record_installation(&shared_file, "sha1".into(), 5, "CppScript".into(), AGENT_ALL.into(), "language".into());
        tracker.record_installation(&shared_file, "sha1".into(), 5, "Rust++".into(), AGENT_ALL.into(), "language".into());

        let metadata = tracker.get_metadata(&shared_file).ok_or_else(|| anyhow::anyhow!("missing metadata"))?;
        assert_eq!(metadata.lang, vec!["CppScript".to_string(), "Rust++".to_string()]);
        assert!(metadata.agent.is_empty() == true);
        assert_eq!(metadata.ref_count, 2);
        Ok(())
    }

    #[test]
    fn test_release_owner_decrements_ref_count_and_keeps_other_owner() -> anyhow::Result<()>
    {
        let temp_dir = TempDir::new()?;
        let workspace = temp_dir.path();
        let shared_file = workspace.join(".agents/skills/git-workflow/SKILL.md");
        fs::create_dir_all(shared_file.parent().ok_or_else(|| anyhow::anyhow!("missing parent"))?)?;
        fs::write(&shared_file, b"# skill\n")?;

        let mut tracker = FileTracker::new(workspace)?;
        tracker.record_installation(&shared_file, "sha1".into(), 5, "Rust++".into(), "fake".into(), "skill".into());

        assert!(tracker.release_agent(&shared_file, "fake") == true);
        let metadata = tracker.get_metadata(&shared_file).ok_or_else(|| anyhow::anyhow!("missing metadata"))?;
        assert_eq!(metadata.lang, vec!["Rust++".to_string()]);
        assert!(metadata.agent.is_empty() == true);
        assert_eq!(metadata.ref_count, 1);
        assert!(metadata.is_unreferenced() == false);

        assert!(tracker.release_lang(&shared_file, "Rust++") == true);
        let metadata = tracker.get_metadata(&shared_file).ok_or_else(|| anyhow::anyhow!("missing metadata"))?;
        assert!(metadata.lang.is_empty() == true);
        assert!(metadata.agent.is_empty() == true);
        assert_eq!(metadata.ref_count, 0);
        assert!(metadata.is_unreferenced() == true);
        Ok(())
    }

    #[test]
    fn test_scalar_tracker_yaml_loads_empty() -> anyhow::Result<()>
    {
        let temp_dir = TempDir::new()?;
        let workspace = temp_dir.path();
        let slopctl_dir = workspace.join(SLOPCTL_DIR);
        fs::create_dir_all(&slopctl_dir)?;
        fs::write(
            slopctl_dir.join(TRACKER_FILE),
            "AGENTS.md:\n  original_sha: sha1\n  template_version: 5\n  installed_date: 2026-01-01T00:00:00+00:00\n  lang: Rust++\n  agent: all\n  category: main\n"
        )?;

        let tracker = FileTracker::new(workspace)?;

        assert!(tracker.get_entries().is_empty() == true);
        Ok(())
    }

    #[test]
    fn test_unparseable_tracker_backed_up_on_save() -> anyhow::Result<()>
    {
        let temp_dir = TempDir::new()?;
        let workspace = temp_dir.path();
        let slopctl_dir = workspace.join(SLOPCTL_DIR);
        fs::create_dir_all(&slopctl_dir)?;
        let corrupt = "AGENTS.md: [not, a, mapping\n";
        fs::write(slopctl_dir.join(TRACKER_FILE), corrupt)?;

        let tracker = FileTracker::new(workspace)?;
        assert_eq!(fs::read_to_string(slopctl_dir.join(TRACKER_FILE))?, corrupt, "loading must not touch the file");

        tracker.save()?;
        tracker.save()?;

        assert_eq!(fs::read_to_string(slopctl_dir.join("tracker.yml.bak"))?, corrupt);
        assert!(FileTracker::new(workspace)?.get_entries().is_empty() == true);
        assert_ne!(fs::read_to_string(slopctl_dir.join(TRACKER_FILE))?, corrupt);
        Ok(())
    }

    #[test]
    fn test_save_and_load() -> anyhow::Result<()>
    {
        let temp_dir = TempDir::new()?;
        let workspace = temp_dir.path();

        {
            let mut tracker = FileTracker::new(workspace)?;
            let test_file = workspace.join("test.txt");
            fs::write(&test_file, b"Test")?;
            let sha = FileTracker::calculate_sha256(&test_file)?;
            tracker.record_installation(&test_file, sha, 1, LANG_NONE.into(), AGENT_ALL.into(), "test".into());
            tracker.save()?;
        }

        {
            let tracker = FileTracker::new(workspace)?;
            assert_eq!(tracker.metadata.len(), 1);
            let metadata = tracker.get_metadata(&workspace.join("test.txt")).ok_or_else(|| anyhow::anyhow!("missing metadata"))?;
            assert!(metadata.lang.is_empty() == true);
            assert!(metadata.agent.is_empty() == true);
            assert_eq!(metadata.ref_count, 0);
        }

        Ok(())
    }

    #[test]
    fn test_get_entries_returns_all_categories() -> anyhow::Result<()>
    {
        let temp_dir = TempDir::new()?;
        let workspace = temp_dir.path();
        fs::create_dir_all(workspace.join(".bogus/skills/my-skill"))?;

        let mut tracker = FileTracker::new(workspace)?;

        let agent_file = workspace.join(".bogus/instructions.md");
        fs::create_dir_all(agent_file.parent().ok_or_else(|| anyhow::anyhow!("missing parent"))?)?;
        fs::write(&agent_file, b"agent")?;
        tracker.record_installation(&agent_file, "sha1".into(), 3, LANG_NONE.into(), "bogus".into(), "agent".into());

        let skill_file = workspace.join(".bogus/skills/my-skill/SKILL.md");
        fs::write(&skill_file, b"skill")?;
        tracker.record_installation(&skill_file, "sha2".into(), 3, LANG_NONE.into(), "bogus".into(), "skill".into());

        let lang_file = workspace.join("AGENTS.md");
        fs::write(&lang_file, b"main")?;
        tracker.record_installation(&lang_file, "sha3".into(), 3, "Rust++".into(), AGENT_ALL.into(), "main".into());

        let entries = tracker.get_entries();
        assert_eq!(entries.len(), 3);

        Ok(())
    }

    #[test]
    fn test_get_entries_by_category_filters_correctly() -> anyhow::Result<()>
    {
        let temp_dir = TempDir::new()?;
        let workspace = temp_dir.path();
        fs::create_dir_all(workspace.join(".bogus/skills/foo"))?;

        let mut tracker = FileTracker::new(workspace)?;

        let agent_file = workspace.join(".bogus/instructions.md");
        fs::create_dir_all(agent_file.parent().ok_or_else(|| anyhow::anyhow!("missing parent"))?)?;
        fs::write(&agent_file, b"agent")?;
        tracker.record_installation(&agent_file, "sha1".into(), 3, LANG_NONE.into(), "bogus".into(), "agent".into());

        let skill_file = workspace.join(".bogus/skills/foo/SKILL.md");
        fs::write(&skill_file, b"skill")?;
        tracker.record_installation(&skill_file, "sha2".into(), 3, LANG_NONE.into(), "bogus".into(), "skill".into());

        let lang_file = workspace.join("AGENTS.md");
        fs::write(&lang_file, b"main")?;
        tracker.record_installation(&lang_file, "sha3".into(), 3, "Rust++".into(), AGENT_ALL.into(), "language".into());

        let skills = tracker.get_entries_by_category("skill");
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].1.category, "skill");

        let agents = tracker.get_entries_by_category("agent");
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].1.category, "agent");

        let none = tracker.get_entries_by_category("nonexistent");
        assert_eq!(none.len(), 0);

        Ok(())
    }

    #[test]
    fn test_clear_agent_owner_releases_across_categories() -> anyhow::Result<()>
    {
        let temp_dir = TempDir::new()?;
        let workspace = temp_dir.path();
        let mut tracker = FileTracker::new(workspace)?;

        let agent_file = workspace.join("a.md");
        fs::write(&agent_file, b"a")?;
        tracker.record_installation(&agent_file, "sha1".into(), 5, LANG_NONE.into(), "bogus".into(), "agent".into());

        let skill_file = workspace.join("s.md");
        fs::write(&skill_file, b"s")?;
        tracker.record_installation(&skill_file, "sha2".into(), 5, LANG_NONE.into(), "bogus".into(), "skill".into());
        tracker.record_installation(&skill_file, "sha2".into(), 5, LANG_NONE.into(), "fake".into(), "skill".into());

        tracker.clear_agent_owner("bogus");

        assert!(tracker.get_installed_agents().iter().any(|agent| agent == "bogus") == false, "owner must be released in every category");
        let meta = tracker.get_metadata(&skill_file).expect("entry must remain");
        assert!(meta.has_agent("fake") == true, "other owners must be preserved");
        assert_eq!(meta.ref_count, 1);

        Ok(())
    }

    #[test]
    fn test_clear_lang_owner_releases_across_categories() -> anyhow::Result<()>
    {
        let temp_dir = TempDir::new()?;
        let workspace = temp_dir.path();
        let mut tracker = FileTracker::new(workspace)?;

        let lang_file = workspace.join("l.md");
        fs::write(&lang_file, b"l")?;
        tracker.record_installation(&lang_file, "sha1".into(), 5, "Rust++".into(), AGENT_ALL.into(), "language".into());

        let skill_file = workspace.join("s.md");
        fs::write(&skill_file, b"s")?;
        tracker.record_installation(&skill_file, "sha2".into(), 5, "Rust++".into(), AGENT_ALL.into(), "skill".into());

        tracker.clear_lang_owner("Rust++");

        assert!(tracker.get_installed_languages().is_empty() == true, "language owner must be released in every category");

        Ok(())
    }

    #[test]
    fn test_get_installed_agents_returns_unique_sorted_owners() -> anyhow::Result<()>
    {
        let temp_dir = TempDir::new()?;
        let workspace = temp_dir.path();
        let mut tracker = FileTracker::new(workspace)?;

        let file_a = workspace.join("a.md");
        fs::write(&file_a, b"a")?;
        tracker.record_installation(&file_a, "sha1".into(), 5, LANG_NONE.into(), "fake".into(), "agent".into());

        let file_b = workspace.join("b.md");
        fs::write(&file_b, b"b")?;
        tracker.record_installation(&file_b, "sha2".into(), 5, LANG_NONE.into(), "bogus".into(), "agent".into());

        let file_c = workspace.join("c.md");
        fs::write(&file_c, b"c")?;
        tracker.record_installation(&file_c, "sha3".into(), 5, "Rust++".into(), "bogus".into(), "language".into());

        // Sentinel-owned files contribute no agent owner.
        let file_d = workspace.join("d.md");
        fs::write(&file_d, b"d")?;
        tracker.record_installation(&file_d, "sha4".into(), 5, LANG_NONE.into(), AGENT_ALL.into(), "integration".into());

        assert_eq!(tracker.get_installed_agents(), vec!["bogus".to_string(), "fake".to_string()]);

        Ok(())
    }

    #[test]
    fn test_relative_paths_stored() -> anyhow::Result<()>
    {
        let temp_dir = TempDir::new()?;
        let workspace = temp_dir.path();

        let mut tracker = FileTracker::new(workspace)?;

        let file = workspace.join("AGENTS.md");
        fs::write(&file, b"test")?;
        tracker.record_installation(&file, "sha1".into(), 5, LANG_NONE.into(), AGENT_ALL.into(), "main".into());

        let entries = tracker.get_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].0, PathBuf::from("AGENTS.md"));

        Ok(())
    }

    #[test]
    fn test_nested_relative_paths() -> anyhow::Result<()>
    {
        let temp_dir = TempDir::new()?;
        let workspace = temp_dir.path();
        fs::create_dir_all(workspace.join(".bogus/skills/my-skill"))?;

        let mut tracker = FileTracker::new(workspace)?;

        let file = workspace.join(".bogus/skills/my-skill/SKILL.md");
        fs::write(&file, b"skill")?;
        tracker.record_installation(&file, "sha1".into(), 5, LANG_NONE.into(), "bogus".into(), "skill".into());

        let entries = tracker.get_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].0, PathBuf::from(".bogus/skills/my-skill/SKILL.md"));

        Ok(())
    }

    #[test]
    fn test_migrate_from_global() -> anyhow::Result<()>
    {
        let temp_dir = TempDir::new()?;
        let workspace_a = temp_dir.path().join("project_a");
        let workspace_b = temp_dir.path().join("project_b");
        let global_dir = temp_dir.path().join("global");
        fs::create_dir_all(&workspace_a)?;
        fs::create_dir_all(&workspace_b)?;
        fs::create_dir_all(&global_dir)?;

        let workspace_a_canon = fs::canonicalize(&workspace_a)?;
        let workspace_b_canon = fs::canonicalize(&workspace_b)?;

        let agents_a = workspace_a.join("AGENTS.md");
        fs::write(&agents_a, b"project a")?;

        let agents_b = workspace_b.join("AGENTS.md");
        fs::write(&agents_b, b"project b")?;

        let global_tracker = global_dir.join(LEGACY_TRACKER_FILE);
        let global_data = serde_json::json!({
            workspace_a_canon.join("AGENTS.md").to_string_lossy().to_string(): {
                "original_sha": "sha_a",
                "template_version": 5,
                "installed_date": "2026-01-01T00:00:00+00:00",
                "lang": "Rust++",
                "category": "main",
                "workspace": workspace_a_canon.to_string_lossy().to_string()
            },
            workspace_b_canon.join("AGENTS.md").to_string_lossy().to_string(): {
                "original_sha": "sha_b",
                "template_version": 5,
                "installed_date": "2026-01-01T00:00:00+00:00",
                "lang": "Rust++",
                "category": "main",
                "workspace": workspace_b_canon.to_string_lossy().to_string()
            }
        });
        fs::write(&global_tracker, serde_json::to_string_pretty(&global_data)?)?;

        let mut tracker_a = FileTracker::new(&workspace_a)?;
        let migrated = tracker_a.migrate_from_global(&global_tracker)?;
        assert_eq!(migrated, 1);

        let entries = tracker_a.get_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].0, PathBuf::from("AGENTS.md"));
        assert_eq!(entries[0].1.original_sha, "sha_a");

        assert!(global_tracker.exists() == true);
        let remaining: HashMap<String, serde_json::Value> = serde_json::from_str(&fs::read_to_string(&global_tracker)?)?;
        assert_eq!(remaining.len(), 1);
        assert!(remaining.keys().next().ok_or_else(|| anyhow::anyhow!("expected key"))?.contains("project_b") == true);

        Ok(())
    }

    #[test]
    fn test_migrate_from_global_removes_empty_file() -> anyhow::Result<()>
    {
        let temp_dir = TempDir::new()?;
        let workspace = temp_dir.path().join("project");
        let global_dir = temp_dir.path().join("global");
        fs::create_dir_all(&workspace)?;
        fs::create_dir_all(&global_dir)?;

        let workspace_canon = fs::canonicalize(&workspace)?;

        let agents = workspace.join("AGENTS.md");
        fs::write(&agents, b"test")?;

        let global_tracker = global_dir.join(LEGACY_TRACKER_FILE);
        let global_data = serde_json::json!({
            workspace_canon.join("AGENTS.md").to_string_lossy().to_string(): {
                "original_sha": "sha1",
                "template_version": 5,
                "installed_date": "2026-01-01T00:00:00+00:00",
                "category": "main",
                "workspace": workspace_canon.to_string_lossy().to_string()
            }
        });
        fs::write(&global_tracker, serde_json::to_string_pretty(&global_data)?)?;

        let mut tracker = FileTracker::new(&workspace)?;
        let migrated = tracker.migrate_from_global(&global_tracker)?;
        assert_eq!(migrated, 1);

        assert!(global_tracker.exists() == false);

        Ok(())
    }

    #[test]
    fn test_migrate_from_global_nonexistent() -> anyhow::Result<()>
    {
        let temp_dir = TempDir::new()?;
        let workspace = temp_dir.path();

        let mut tracker = FileTracker::new(workspace)?;
        let count = tracker.migrate_from_global(&PathBuf::from("/nonexistent/tracker.json"))?;
        assert_eq!(count, 0);

        Ok(())
    }
}
