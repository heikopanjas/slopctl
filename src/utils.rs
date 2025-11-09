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
