use std::{
    collections::HashMap,
    fs,
    io::Read,
    path::{Path, PathBuf}
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Metadata about an installed template file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata
{
    pub original_sha:     String,
    pub template_version: u32,
    pub installed_date:   String,
    pub lang:             Option<String>,
    pub category:         String
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
    /// File was tracked but no longer exists on disk
    Deleted
}

/// Name of the workspace-local slopctl directory
pub const SLOPCTL_DIR: &str = ".slopctl";

/// Name of the tracker JSON file inside the slopctl directory
const TRACKER_FILE: &str = "tracker.json";

/// Legacy tracker filename used in the global template directory
const LEGACY_TRACKER_FILE: &str = "installed_files.json";

/// Tracks installed template files using SHA checksums
///
/// Stores metadata in a workspace-local `.slopctl/tracker.json` file.
/// All paths are stored relative to the workspace root.
pub struct FileTracker
{
    workspace:     PathBuf,
    metadata_path: PathBuf,
    metadata:      HashMap<String, FileMetadata>
}

impl FileTracker
{
    /// Converts a file path to a relative path string from the workspace root
    ///
    /// For absolute paths, strips the workspace prefix. For paths that are
    /// already relative, normalises separators to forward slashes.
    fn to_relative_key(&self, file_path: &Path) -> String
    {
        let absolute = if let Ok(canonical) = fs::canonicalize(file_path)
        {
            canonical
        }
        else if let Some(parent) = file_path.parent() &&
            let Ok(parent_abs) = fs::canonicalize(parent) &&
            let Some(filename) = file_path.file_name()
        {
            parent_abs.join(filename)
        }
        else
        {
            file_path.to_path_buf()
        };

        let workspace_canon = fs::canonicalize(&self.workspace).unwrap_or_else(|_| self.workspace.clone());

        if let Ok(relative) = absolute.strip_prefix(&workspace_canon)
        {
            let rel_str = relative.to_string_lossy();
            return rel_str.replace('\\', "/");
        }

        let lossy = file_path.to_string_lossy();
        lossy.replace('\\', "/")
    }

    /// Create a new FileTracker for a workspace
    ///
    /// Loads existing tracker data from `.slopctl/tracker.json` in the
    /// workspace root. Creates the `.slopctl/` directory if it does not exist.
    ///
    /// # Arguments
    ///
    /// * `workspace` - Absolute path to the workspace root directory
    ///
    /// # Errors
    ///
    /// Returns an error if the tracker file exists but cannot be read
    pub fn new(workspace: &Path) -> anyhow::Result<Self>
    {
        let slopctl_dir = workspace.join(SLOPCTL_DIR);
        let metadata_path = slopctl_dir.join(TRACKER_FILE);

        let metadata = if metadata_path.exists() == true
        {
            let contents = fs::read_to_string(&metadata_path)?;
            serde_json::from_str(&contents).unwrap_or_else(|_| HashMap::new())
        }
        else
        {
            HashMap::new()
        };

        Ok(Self { workspace: workspace.to_path_buf(), metadata_path, metadata })
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

    /// Record a file installation with metadata
    ///
    /// The file path is stored relative to the workspace root.
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to the installed file (absolute or relative to workspace)
    /// * `original_sha` - SHA-256 of the file at install time
    /// * `template_version` - Template format version used
    /// * `lang` - Language name if this file belongs to a language install
    /// * `category` - Category tag (e.g. "main", "agent", "language", "skill")
    pub fn record_installation(&mut self, file_path: &Path, original_sha: String, template_version: u32, lang: Option<String>, category: String)
    {
        let now = chrono::Utc::now().to_rfc3339();
        let relative_key = self.to_relative_key(file_path);

        self.metadata.insert(relative_key, FileMetadata { original_sha, template_version, installed_date: now, lang, category });
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
            Ok(FileStatus::Unmodified)
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

    /// Returns the installed language for this workspace
    ///
    /// Scans all tracked entries for one with a `lang` field set.
    pub fn get_installed_language(&self) -> Option<String>
    {
        for meta in self.metadata.values()
        {
            if meta.lang.is_some() == true
            {
                return meta.lang.clone();
            }
        }

        None
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

    /// Adopt existing slopctl-managed files that are not yet tracked
    ///
    /// Scans the workspace for agent instruction files, skills, and commands
    /// using the known agent conventions from `agent_defaults`. Any files
    /// found on disk that are not already in the tracker are adopted with
    /// their current SHA and a `template_version` of 0 (indicating adoption
    /// rather than a template install).
    ///
    /// # Returns
    ///
    /// The number of files adopted.
    pub fn adopt_untracked_files(&mut self, workspace: &Path) -> anyhow::Result<usize>
    {
        use crate::agent_defaults;

        let mut adopted = 0usize;
        let userprofile = dirs::home_dir().unwrap_or_default();

        // Adopt AGENTS.md (category "main")
        let agents_md = workspace.join("AGENTS.md");
        if agents_md.exists() == true
        {
            adopted += self.try_adopt(&agents_md, None, "main")?;
        }

        // Adopt agent instruction files (category "agent") for all known agents
        for agent_name in agent_defaults::known_agents()
        {
            if let Some(defaults) = agent_defaults::get_defaults(agent_name)
            {
                for instr in defaults.instruction_files
                {
                    if instr.placeholder == agent_defaults::PLACEHOLDER_WORKSPACE
                    {
                        let path = workspace.join(instr.path);
                        if path.exists() == true
                        {
                            adopted += self.try_adopt(&path, None, "agent")?;
                        }
                    }
                }
            }
        }

        // Adopt skills (category "skill") from all workspace-scoped skill directories
        for agent_name in agent_defaults::known_agents()
        {
            if let Some(defaults) = agent_defaults::get_defaults(agent_name) &&
                defaults.skill_dir.starts_with(agent_defaults::PLACEHOLDER_WORKSPACE) == true
            {
                let skill_dir = agent_defaults::resolve_placeholder_path(defaults.skill_dir, workspace, &userprofile);
                if skill_dir.exists() == true &&
                    let Ok(entries) = fs::read_dir(&skill_dir)
                {
                    for entry in entries.flatten()
                    {
                        if entry.path().is_dir() == true
                        {
                            let mut files = Vec::new();
                            crate::utils::collect_files_recursive(&entry.path(), &mut files)?;
                            for file in files
                            {
                                adopted += self.try_adopt(&file, None, "skill")?;
                            }
                        }
                    }
                }
            }
        }

        // Also scan the cross-client skill directory
        let cross_client = agent_defaults::resolve_placeholder_path(agent_defaults::CROSS_CLIENT_SKILL_DIR, workspace, &userprofile);
        if cross_client.exists() == true &&
            let Ok(entries) = fs::read_dir(&cross_client)
        {
            for entry in entries.flatten()
            {
                if entry.path().is_dir() == true
                {
                    let mut files = Vec::new();
                    crate::utils::collect_files_recursive(&entry.path(), &mut files)?;
                    for file in files
                    {
                        adopted += self.try_adopt(&file, None, "skill")?;
                    }
                }
            }
        }

        // Adopt commands/prompts from all workspace-scoped prompt directories
        for agent_name in agent_defaults::known_agents()
        {
            if let Some(defaults) = agent_defaults::get_defaults(agent_name) &&
                defaults.prompt_dir.starts_with(agent_defaults::PLACEHOLDER_WORKSPACE) == true
            {
                let prompt_dir = agent_defaults::resolve_placeholder_path(defaults.prompt_dir, workspace, &userprofile);
                if prompt_dir.exists() == true &&
                    let Ok(entries) = fs::read_dir(&prompt_dir)
                {
                    for entry in entries.flatten()
                    {
                        let path = entry.path();
                        if path.is_file() == true
                        {
                            adopted += self.try_adopt(&path, None, "command")?;
                        }
                    }
                }
            }
        }

        if adopted > 0
        {
            self.save()?;
        }

        Ok(adopted)
    }

    /// Try to adopt a single file if not already tracked
    ///
    /// Returns 1 if the file was adopted, 0 if it was already tracked.
    fn try_adopt(&mut self, file_path: &Path, lang: Option<String>, category: &str) -> anyhow::Result<usize>
    {
        let key = self.to_relative_key(file_path);
        if self.metadata.contains_key(&key) == true
        {
            return Ok(0);
        }

        let sha = Self::calculate_sha256(file_path)?;
        let now = chrono::Utc::now().to_rfc3339();

        self.metadata.insert(key, FileMetadata { original_sha: sha, template_version: 0, installed_date: now, lang, category: category.to_string() });

        Ok(1)
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

        let json = serde_json::to_string_pretty(&self.metadata)?;
        fs::write(&self.metadata_path, json)?;
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

                self.metadata.insert(relative, FileMetadata {
                    original_sha:     legacy.original_sha.clone(),
                    template_version: legacy.template_version,
                    installed_date:   legacy.installed_date.clone(),
                    lang:             legacy.lang.clone(),
                    category:         legacy.category.clone()
                });

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

        tracker.record_installation(&test_file, original_sha.clone(), 1, Some("rust".to_string()), "language".to_string());

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

        tracker.record_installation(&project_file, "sha123".to_string(), 1, Some("rust".to_string()), "main".to_string());
        let lang = tracker.get_installed_language();
        assert_eq!(lang, Some("rust".to_string()));

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
            tracker.record_installation(&test_file, sha, 1, None, "test".to_string());
            tracker.save()?;
        }

        {
            let tracker = FileTracker::new(workspace)?;
            assert_eq!(tracker.metadata.len(), 1);
        }

        Ok(())
    }

    #[test]
    fn test_get_entries_returns_all_categories() -> anyhow::Result<()>
    {
        let temp_dir = TempDir::new()?;
        let workspace = temp_dir.path();
        fs::create_dir_all(workspace.join(".cursor/skills/my-skill"))?;

        let mut tracker = FileTracker::new(workspace)?;

        let agent_file = workspace.join(".cursorrules");
        fs::write(&agent_file, b"agent")?;
        tracker.record_installation(&agent_file, "sha1".into(), 3, None, "agent".into());

        let skill_file = workspace.join(".cursor/skills/my-skill/SKILL.md");
        fs::write(&skill_file, b"skill")?;
        tracker.record_installation(&skill_file, "sha2".into(), 3, None, "skill".into());

        let lang_file = workspace.join("AGENTS.md");
        fs::write(&lang_file, b"main")?;
        tracker.record_installation(&lang_file, "sha3".into(), 3, Some("rust".into()), "main".into());

        let entries = tracker.get_entries();
        assert_eq!(entries.len(), 3);

        Ok(())
    }

    #[test]
    fn test_get_entries_by_category_filters_correctly() -> anyhow::Result<()>
    {
        let temp_dir = TempDir::new()?;
        let workspace = temp_dir.path();
        fs::create_dir_all(workspace.join(".cursor/skills/foo"))?;

        let mut tracker = FileTracker::new(workspace)?;

        let agent_file = workspace.join(".cursorrules");
        fs::write(&agent_file, b"agent")?;
        tracker.record_installation(&agent_file, "sha1".into(), 3, None, "agent".into());

        let skill_file = workspace.join(".cursor/skills/foo/SKILL.md");
        fs::write(&skill_file, b"skill")?;
        tracker.record_installation(&skill_file, "sha2".into(), 3, None, "skill".into());

        let lang_file = workspace.join("AGENTS.md");
        fs::write(&lang_file, b"main")?;
        tracker.record_installation(&lang_file, "sha3".into(), 3, Some("rust".into()), "language".into());

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
    fn test_relative_paths_stored() -> anyhow::Result<()>
    {
        let temp_dir = TempDir::new()?;
        let workspace = temp_dir.path();

        let mut tracker = FileTracker::new(workspace)?;

        let file = workspace.join("AGENTS.md");
        fs::write(&file, b"test")?;
        tracker.record_installation(&file, "sha1".into(), 5, None, "main".into());

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
        fs::create_dir_all(workspace.join(".cursor/skills/my-skill"))?;

        let mut tracker = FileTracker::new(workspace)?;

        let file = workspace.join(".cursor/skills/my-skill/SKILL.md");
        fs::write(&file, b"skill")?;
        tracker.record_installation(&file, "sha1".into(), 5, None, "skill".into());

        let entries = tracker.get_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].0, PathBuf::from(".cursor/skills/my-skill/SKILL.md"));

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
                "lang": "rust",
                "category": "main",
                "workspace": workspace_a_canon.to_string_lossy().to_string()
            },
            workspace_b_canon.join("AGENTS.md").to_string_lossy().to_string(): {
                "original_sha": "sha_b",
                "template_version": 5,
                "installed_date": "2026-01-01T00:00:00+00:00",
                "lang": "rust",
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

    #[test]
    fn test_adopt_untracked_files_discovers_agents_and_skills() -> anyhow::Result<()>
    {
        let temp_dir = TempDir::new()?;
        let workspace = temp_dir.path();

        fs::write(workspace.join("AGENTS.md"), b"# Instructions")?;
        fs::write(workspace.join("CLAUDE.md"), b"Read AGENTS.md")?;
        fs::create_dir_all(workspace.join(".claude/skills/git-workflow"))?;
        fs::write(workspace.join(".claude/skills/git-workflow/SKILL.md"), b"# Skill")?;
        fs::create_dir_all(workspace.join(".claude/commands"))?;
        fs::write(workspace.join(".claude/commands/init-session.md"), b"# Command")?;

        let mut tracker = FileTracker::new(workspace)?;
        assert_eq!(tracker.get_entries().len(), 0);

        let adopted = tracker.adopt_untracked_files(workspace)?;
        assert_eq!(adopted, 4);

        let entries = tracker.get_entries();
        assert_eq!(entries.len(), 4);

        let categories: Vec<&str> = entries.iter().map(|(_, m)| m.category.as_str()).collect();
        assert!(categories.contains(&"main"));
        assert!(categories.contains(&"agent"));
        assert!(categories.contains(&"skill"));
        assert!(categories.contains(&"command"));

        Ok(())
    }

    #[test]
    fn test_adopt_skips_already_tracked() -> anyhow::Result<()>
    {
        let temp_dir = TempDir::new()?;
        let workspace = temp_dir.path();

        fs::write(workspace.join("AGENTS.md"), b"# Instructions")?;

        let mut tracker = FileTracker::new(workspace)?;
        tracker.record_installation(&workspace.join("AGENTS.md"), "sha1".into(), 5, None, "main".into());
        assert_eq!(tracker.get_entries().len(), 1);

        let adopted = tracker.adopt_untracked_files(workspace)?;
        assert_eq!(adopted, 0);
        assert_eq!(tracker.get_entries().len(), 1);

        Ok(())
    }

    #[test]
    fn test_adopt_sets_template_version_zero() -> anyhow::Result<()>
    {
        let temp_dir = TempDir::new()?;
        let workspace = temp_dir.path();

        fs::write(workspace.join("AGENTS.md"), b"# Instructions")?;

        let mut tracker = FileTracker::new(workspace)?;
        tracker.adopt_untracked_files(workspace)?;

        let entries = tracker.get_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].1.template_version, 0);

        Ok(())
    }
}
