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
    /// File was never tracked by regulator
    NotTracked,
    /// File exists and matches original SHA (user did not modify)
    Unmodified,
    /// File exists but SHA differs from original (user modified)
    Modified,
    /// File was tracked but no longer exists on disk
    Deleted
}

/// Tracks installed template files using SHA checksums
pub struct FileTracker
{
    metadata_path: PathBuf,
    metadata:      HashMap<String, FileMetadata>
}

impl FileTracker
{
    /// Resolves a file path to its absolute string representation
    ///
    /// For existing files, uses `fs::canonicalize`. For deleted files,
    /// attempts to resolve via the parent directory. Falls back to the
    /// path as-is if neither approach works.
    fn resolve_absolute_path(file_path: &Path) -> String
    {
        // Try direct canonicalize (works for existing files)
        if let Ok(canonical) = fs::canonicalize(file_path)
        {
            return canonical.to_string_lossy().to_string();
        }

        // File doesn't exist, try to construct absolute path from parent
        if let Some(parent) = file_path.parent() &&
            let Ok(parent_abs) = fs::canonicalize(parent) &&
            let Some(filename) = file_path.file_name()
        {
            return parent_abs.join(filename).to_string_lossy().to_string();
        }

        file_path.to_path_buf().to_string_lossy().to_string()
    }

    /// Create a new FileTracker and load existing metadata
    pub fn new(data_dir: &Path) -> anyhow::Result<Self>
    {
        let metadata_path = data_dir.join("installed_files.json");
        let metadata = if metadata_path.exists() == true
        {
            let contents = fs::read_to_string(&metadata_path)?;
            serde_json::from_str(&contents).unwrap_or_else(|_| HashMap::new())
        }
        else
        {
            HashMap::new()
        };

        Ok(Self { metadata_path, metadata })
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
    pub fn record_installation(&mut self, file_path: &Path, original_sha: String, template_version: u32, lang: Option<String>, category: String)
    {
        let now = chrono::Utc::now().to_rfc3339();
        let absolute_path = Self::resolve_absolute_path(file_path);

        self.metadata.insert(absolute_path, FileMetadata { original_sha, template_version, installed_date: now, lang, category });
    }

    /// Check the modification status of a file
    pub fn check_modification(&self, file_path: &Path) -> anyhow::Result<FileStatus>
    {
        let absolute_path = Self::resolve_absolute_path(file_path);

        // Check if file is tracked
        let metadata = match self.metadata.get(&absolute_path)
        {
            | Some(meta) => meta,
            | None => return Ok(FileStatus::NotTracked)
        };

        // Check if file still exists
        if file_path.exists() == false
        {
            return Ok(FileStatus::Deleted);
        }

        // Calculate current SHA and compare
        let current_sha = Self::calculate_sha256(file_path)?;
        if current_sha == metadata.original_sha
        {
            Ok(FileStatus::Unmodified)
        }
        else
        {
            Ok(FileStatus::Modified)
        }
    }

    /// Check if new template is different from original
    pub fn is_template_updated(&self, file_path: &Path, new_template_sha: &str) -> anyhow::Result<bool>
    {
        let absolute_path = Self::resolve_absolute_path(file_path);

        if let Some(metadata) = self.metadata.get(&absolute_path)
        {
            Ok(new_template_sha != metadata.original_sha)
        }
        else
        {
            // Not tracked, so consider it updated
            Ok(true)
        }
    }

    /// Remove a tracked file entry
    pub fn remove_entry(&mut self, file_path: &Path)
    {
        let absolute_path = Self::resolve_absolute_path(file_path);

        self.metadata.remove(&absolute_path);
    }

    /// Get metadata for a tracked file
    pub fn get_metadata(&self, file_path: &Path) -> Option<&FileMetadata>
    {
        let absolute_path = Self::resolve_absolute_path(file_path);

        self.metadata.get(&absolute_path)
    }

    /// Returns the installed language for files in the given workspace
    ///
    /// Used when re-initializing with only --agent to preserve the existing language
    /// (e.g. switching from Cursor to Claude without changing Rust setup).
    pub fn get_installed_language_for_workspace(&self, workspace: &Path) -> Option<String>
    {
        let workspace_canon = fs::canonicalize(workspace).ok().or_else(|| workspace.to_path_buf().canonicalize().ok())?;

        for (path_str, meta) in &self.metadata
        {
            if meta.lang.is_some() == true && Path::new(path_str).starts_with(&workspace_canon) == true
            {
                return meta.lang.clone();
            }
        }

        None
    }

    /// Returns all tracked files whose path falls under the given workspace
    ///
    /// # Arguments
    ///
    /// * `workspace` - Root directory of the workspace to query
    pub fn get_workspace_entries(&self, workspace: &Path) -> Vec<(PathBuf, &FileMetadata)>
    {
        let Some(workspace_canon) = fs::canonicalize(workspace).ok().or_else(|| workspace.to_path_buf().canonicalize().ok())
        else
        {
            return Vec::new();
        };

        self.metadata
            .iter()
            .filter(|(path_str, _)| Path::new(path_str).starts_with(&workspace_canon) == true)
            .map(|(path_str, meta)| (PathBuf::from(path_str), meta))
            .collect()
    }

    /// Returns tracked files under a workspace filtered by category
    ///
    /// # Arguments
    ///
    /// * `workspace` - Root directory of the workspace to query
    /// * `category` - Category to filter by (e.g. "skill", "agent", "language")
    pub fn get_workspace_entries_by_category(&self, workspace: &Path, category: &str) -> Vec<(PathBuf, &FileMetadata)>
    {
        let Some(workspace_canon) = fs::canonicalize(workspace).ok().or_else(|| workspace.to_path_buf().canonicalize().ok())
        else
        {
            return Vec::new();
        };

        self.metadata
            .iter()
            .filter(|(path_str, meta)| Path::new(path_str).starts_with(&workspace_canon) == true && meta.category == category)
            .map(|(path_str, meta)| (PathBuf::from(path_str), meta))
            .collect()
    }

    /// Save metadata to disk
    pub fn save(&self) -> anyhow::Result<()>
    {
        // Ensure parent directory exists
        if let Some(parent) = self.metadata_path.parent()
        {
            fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(&self.metadata)?;
        fs::write(&self.metadata_path, json)?;
        Ok(())
    }
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
        // SHA-256 of "Hello, World!"
        assert_eq!(sha, "dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f");

        Ok(())
    }

    #[test]
    fn test_file_tracking() -> anyhow::Result<()>
    {
        let temp_dir = TempDir::new()?;
        let data_dir = temp_dir.path().join("data");
        fs::create_dir_all(&data_dir)?;

        let mut tracker = FileTracker::new(&data_dir)?;

        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, b"Original content")?;

        let original_sha = FileTracker::calculate_sha256(&test_file)?;

        // Record installation
        tracker.record_installation(&test_file, original_sha.clone(), 1, Some("rust".to_string()), "language".to_string());

        // Check unmodified status
        let status = tracker.check_modification(&test_file)?;
        assert_eq!(status, FileStatus::Unmodified);

        // Modify file
        fs::write(&test_file, b"Modified content")?;
        let status = tracker.check_modification(&test_file)?;
        assert_eq!(status, FileStatus::Modified);

        // Delete file
        fs::remove_file(&test_file)?;
        let status = tracker.check_modification(&test_file)?;
        assert_eq!(status, FileStatus::Deleted);

        Ok(())
    }

    #[test]
    fn test_get_installed_language_for_workspace() -> anyhow::Result<()>
    {
        let temp_dir = TempDir::new()?;
        let data_dir = temp_dir.path().join("data");
        fs::create_dir_all(&data_dir)?;

        let mut tracker = FileTracker::new(&data_dir)?;
        let project_file = temp_dir.path().join("project/AGENTS.md");
        fs::create_dir_all(project_file.parent().ok_or_else(|| anyhow::anyhow!("expected parent directory"))?)?;
        fs::write(&project_file, b"test")?;

        tracker.record_installation(&project_file, "sha123".to_string(), 1, Some("rust".to_string()), "main".to_string());

        let project_dir = temp_dir.path().join("project");
        let lang = tracker.get_installed_language_for_workspace(&project_dir);
        assert_eq!(lang, Some("rust".to_string()));

        let other_dir = temp_dir.path().join("other");
        let lang_other = tracker.get_installed_language_for_workspace(&other_dir);
        assert_eq!(lang_other, None);

        Ok(())
    }

    #[test]
    fn test_save_and_load() -> anyhow::Result<()>
    {
        let temp_dir = TempDir::new()?;
        let data_dir = temp_dir.path().join("data");
        fs::create_dir_all(&data_dir)?;

        // Create and save metadata
        {
            let mut tracker = FileTracker::new(&data_dir)?;
            let test_file = temp_dir.path().join("test.txt");
            fs::write(&test_file, b"Test")?;
            let sha = FileTracker::calculate_sha256(&test_file)?;
            tracker.record_installation(&test_file, sha, 1, None, "test".to_string());
            tracker.save()?;
        }

        // Load metadata
        {
            let tracker = FileTracker::new(&data_dir)?;
            assert_eq!(tracker.metadata.len(), 1);
        }

        Ok(())
    }

    #[test]
    fn test_get_workspace_entries_returns_all_categories() -> anyhow::Result<()>
    {
        let temp_dir = TempDir::new()?;
        let data_dir = temp_dir.path().join("data");
        let workspace = temp_dir.path().join("project");
        fs::create_dir_all(&data_dir)?;
        fs::create_dir_all(workspace.join(".cursor/skills/my-skill"))?;

        let mut tracker = FileTracker::new(&data_dir)?;

        let agent_file = workspace.join(".cursorrules");
        fs::write(&agent_file, b"agent")?;
        tracker.record_installation(&agent_file, "sha1".into(), 3, None, "agent".into());

        let skill_file = workspace.join(".cursor/skills/my-skill/SKILL.md");
        fs::write(&skill_file, b"skill")?;
        tracker.record_installation(&skill_file, "sha2".into(), 3, None, "skill".into());

        let lang_file = workspace.join("AGENTS.md");
        fs::write(&lang_file, b"main")?;
        tracker.record_installation(&lang_file, "sha3".into(), 3, Some("rust".into()), "main".into());

        let entries = tracker.get_workspace_entries(&workspace);
        assert_eq!(entries.len(), 3);

        Ok(())
    }

    #[test]
    fn test_get_workspace_entries_excludes_other_workspaces() -> anyhow::Result<()>
    {
        let temp_dir = TempDir::new()?;
        let data_dir = temp_dir.path().join("data");
        let project_a = temp_dir.path().join("project_a");
        let project_b = temp_dir.path().join("project_b");
        fs::create_dir_all(&data_dir)?;
        fs::create_dir_all(&project_a)?;
        fs::create_dir_all(&project_b)?;

        let mut tracker = FileTracker::new(&data_dir)?;

        let file_a = project_a.join("AGENTS.md");
        fs::write(&file_a, b"a")?;
        tracker.record_installation(&file_a, "sha_a".into(), 3, None, "main".into());

        let file_b = project_b.join("AGENTS.md");
        fs::write(&file_b, b"b")?;
        tracker.record_installation(&file_b, "sha_b".into(), 3, None, "main".into());

        let entries_a = tracker.get_workspace_entries(&project_a);
        assert_eq!(entries_a.len(), 1);

        let entries_b = tracker.get_workspace_entries(&project_b);
        assert_eq!(entries_b.len(), 1);

        Ok(())
    }

    #[test]
    fn test_get_workspace_entries_by_category_filters_correctly() -> anyhow::Result<()>
    {
        let temp_dir = TempDir::new()?;
        let data_dir = temp_dir.path().join("data");
        let workspace = temp_dir.path().join("project");
        fs::create_dir_all(&data_dir)?;
        fs::create_dir_all(workspace.join(".cursor/skills/foo"))?;

        let mut tracker = FileTracker::new(&data_dir)?;

        let agent_file = workspace.join(".cursorrules");
        fs::write(&agent_file, b"agent")?;
        tracker.record_installation(&agent_file, "sha1".into(), 3, None, "agent".into());

        let skill_file = workspace.join(".cursor/skills/foo/SKILL.md");
        fs::write(&skill_file, b"skill")?;
        tracker.record_installation(&skill_file, "sha2".into(), 3, None, "skill".into());

        let lang_file = workspace.join("AGENTS.md");
        fs::write(&lang_file, b"main")?;
        tracker.record_installation(&lang_file, "sha3".into(), 3, Some("rust".into()), "language".into());

        let skills = tracker.get_workspace_entries_by_category(&workspace, "skill");
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].1.category, "skill");

        let agents = tracker.get_workspace_entries_by_category(&workspace, "agent");
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].1.category, "agent");

        let none = tracker.get_workspace_entries_by_category(&workspace, "nonexistent");
        assert_eq!(none.len(), 0);

        Ok(())
    }

    #[test]
    fn test_get_workspace_entries_nonexistent_workspace()
    {
        let temp_dir = TempDir::new().expect("temp dir");
        let data_dir = temp_dir.path().join("data");
        fs::create_dir_all(&data_dir).expect("create data dir");

        let tracker = FileTracker::new(&data_dir).expect("tracker");

        let bogus = temp_dir.path().join("does_not_exist");
        let entries = tracker.get_workspace_entries(&bogus);
        assert!(entries.is_empty() == true);
    }
}
