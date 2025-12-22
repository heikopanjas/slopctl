use std::{
    collections::HashMap,
    error::Error,
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
    /// File was never tracked by vibe-check
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
    /// Create a new FileTracker and load existing metadata
    pub fn new(data_dir: &Path) -> Result<Self, Box<dyn Error>>
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
    pub fn calculate_sha256(file_path: &Path) -> Result<String, Box<dyn Error>>
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
        let absolute_path = fs::canonicalize(file_path).unwrap_or_else(|_| file_path.to_path_buf()).to_string_lossy().to_string();

        self.metadata.insert(absolute_path, FileMetadata { original_sha, template_version, installed_date: now, lang, category });
    }

    /// Check the modification status of a file
    pub fn check_modification(&self, file_path: &Path) -> Result<FileStatus, Box<dyn Error>>
    {
        // Try to get the absolute path - for deleted files, we need to construct it manually
        let absolute_path = if file_path.exists() == true
        {
            fs::canonicalize(file_path).unwrap_or_else(|_| file_path.to_path_buf()).to_string_lossy().to_string()
        }
        else
        {
            // File doesn't exist, try to construct absolute path from parent
            if let Some(parent) = file_path.parent()
            {
                if let Ok(parent_abs) = fs::canonicalize(parent)
                {
                    if let Some(filename) = file_path.file_name()
                    {
                        parent_abs.join(filename).to_string_lossy().to_string()
                    }
                    else
                    {
                        file_path.to_path_buf().to_string_lossy().to_string()
                    }
                }
                else
                {
                    file_path.to_path_buf().to_string_lossy().to_string()
                }
            }
            else
            {
                file_path.to_path_buf().to_string_lossy().to_string()
            }
        };

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
    pub fn is_template_updated(&self, file_path: &Path, new_template_sha: &str) -> Result<bool, Box<dyn Error>>
    {
        let absolute_path = fs::canonicalize(file_path).unwrap_or_else(|_| file_path.to_path_buf()).to_string_lossy().to_string();

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
        let absolute_path = fs::canonicalize(file_path).unwrap_or_else(|_| file_path.to_path_buf()).to_string_lossy().to_string();

        self.metadata.remove(&absolute_path);
    }

    /// Get metadata for a tracked file
    pub fn get_metadata(&self, file_path: &Path) -> Option<&FileMetadata>
    {
        let absolute_path = fs::canonicalize(file_path).unwrap_or_else(|_| file_path.to_path_buf()).to_string_lossy().to_string();

        self.metadata.get(&absolute_path)
    }

    /// Save metadata to disk
    pub fn save(&self) -> Result<(), Box<dyn Error>>
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
    fn test_calculate_sha256() -> Result<(), Box<dyn Error>>
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
    fn test_file_tracking() -> Result<(), Box<dyn Error>>
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
    fn test_save_and_load() -> Result<(), Box<dyn Error>>
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
}
