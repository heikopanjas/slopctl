//! Utility functions for vibe-check

use std::{fs, path::Path};

use crate::Result;

/// Recursively copies all files and directories from source to destination
///
/// This function creates the destination directory if it doesn't exist and
/// copies all contents from the source directory, maintaining the directory
/// structure.
///
/// # Arguments
///
/// * `src` - Source directory path
/// * `dst` - Destination directory path
///
/// # Errors
///
/// Returns an error if:
/// - Directory creation fails
/// - Reading directory entries fails
/// - File or directory copy operations fail
///
/// # Examples
///
/// ```no_run
/// use std::path::Path;
///
/// use vibe_check::copy_dir_all;
///
/// let src = Path::new("/path/to/source");
/// let dst = Path::new("/path/to/dest");
/// copy_dir_all(src, dst).expect("Failed to copy directory");
/// ```
pub fn copy_dir_all(src: &Path, dst: &Path) -> Result<()>
{
    fs::create_dir_all(dst)?;

    for entry in fs::read_dir(src)?
    {
        let entry = entry?;
        let path = entry.path();
        let file_name = entry.file_name();
        let dst_path = dst.join(file_name);

        if path.is_dir()
        {
            copy_dir_all(&path, &dst_path)?;
        }
        else
        {
            fs::copy(&path, &dst_path)?;
        }
    }

    Ok(())
}

/// Copies a file from source to target, creating parent directories if needed
///
/// # Arguments
///
/// * `source` - Source file path
/// * `target` - Target file path
///
/// # Errors
///
/// Returns an error if directory creation or file copy fails
pub fn copy_file_with_mkdir(source: &Path, target: &Path) -> Result<()>
{
    if let Some(parent) = target.parent()
    {
        fs::create_dir_all(parent)?;
    }
    fs::copy(source, target)?;
    Ok(())
}

/// Removes a file and attempts to clean up empty parent directories
///
/// After removing the file, tries to remove up to 2 levels of parent
/// directories if they are empty. Errors during parent cleanup are ignored.
///
/// # Arguments
///
/// * `path` - Path to the file to remove
///
/// # Errors
///
/// Returns an error if file removal fails
pub fn remove_file_and_cleanup_parents(path: &Path) -> Result<()>
{
    fs::remove_file(path)?;

    // Try to remove empty parent directories (up to 2 levels)
    if let Some(parent) = path.parent()
    {
        let _ = fs::remove_dir(parent); // Ignore errors - directory might not be empty
        if let Some(grandparent) = parent.parent()
        {
            let _ = fs::remove_dir(grandparent);
        }
    }

    Ok(())
}
