//! Utility functions for vibe-check

use std::{
    fs,
    io::{self, Write},
    path::Path,
    process::Command
};

use owo_colors::OwoColorize;

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

/// Prompts the user for confirmation and returns true if they confirm
///
/// Displays the prompt and waits for user input. Returns true only if
/// the user enters 'y' or 'Y'.
///
/// # Arguments
///
/// * `prompt` - The confirmation prompt to display
///
/// # Errors
///
/// Returns an error if reading from stdin fails
pub fn confirm_action(prompt: &str) -> Result<bool>
{
    print!("{}", prompt);
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    Ok(input.trim().eq_ignore_ascii_case("y"))
}

/// Response from interactive file modification prompt
#[derive(Debug, PartialEq)]
pub enum FileActionResponse
{
    Skip,
    Overwrite,
    Quit
}

/// Prompts user for action when a modified file is detected
///
/// Shows the file path and SHA checksums, then presents options to:
/// - Skip (keep local version)
/// - Overwrite (use new template)
/// - Show diff
/// - Quit operation
///
/// # Arguments
///
/// * `file_path` - Path to the modified file
/// * `original_sha` - SHA checksum when file was originally installed
/// * `current_sha` - Current SHA checksum of the file
/// * `template_path` - Path to the new template file (for diff)
///
/// # Returns
///
/// Returns the user's choice or an error if input fails
///
/// # Errors
///
/// Returns an error if reading from stdin fails or showing diff fails
pub fn prompt_file_modification(file_path: &Path, original_sha: &str, current_sha: &str, template_path: &Path) -> Result<FileActionResponse>
{
    loop
    {
        println!();
        println!("{} {}", "File has been modified:".yellow(), file_path.display().yellow().bold());
        println!();
        println!("  Original SHA: {}", original_sha.dimmed());
        println!("  Current SHA:  {}", current_sha.dimmed());
        println!();
        println!("Options:");
        println!("  [{}] Skip (keep your version)", "s".green().bold());
        println!("  [{}] Overwrite (use new template)", "o".red().bold());
        println!("  [{}] Show diff", "d".blue().bold());
        println!("  [{}] Quit operation", "q".yellow().bold());
        println!();
        print!("{} ", "Choice:".cyan());
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let choice = input.trim().to_lowercase();

        match choice.as_str()
        {
            | "s" | "skip" => return Ok(FileActionResponse::Skip),
            | "o" | "overwrite" => return Ok(FileActionResponse::Overwrite),
            | "q" | "quit" => return Ok(FileActionResponse::Quit),
            | "d" | "diff" =>
            {
                show_diff(file_path, template_path)?;
            }
            | _ =>
            {
                println!("{} Invalid choice. Please enter s, o, d, or q.", "!".red());
            }
        }
    }
}

/// Shows a diff between two files using external diff command
///
/// Attempts to use `diff -u` for unified diff output. If diff command
/// is not available, shows a simple notification.
///
/// # Arguments
///
/// * `file_a` - First file path (current version)
/// * `file_b` - Second file path (new template)
///
/// # Errors
///
/// Returns an error if diff command execution fails
fn show_diff(file_a: &Path, file_b: &Path) -> Result<()>
{
    println!();
    println!("{}", "═".repeat(80).dimmed());

    // Try to use external diff command
    let result = Command::new("diff").arg("-u").arg("--color=auto").arg(file_a).arg(file_b).status();

    match result
    {
        | Ok(status) =>
        {
            // diff returns 0 if files are identical, 1 if different, 2 on error
            if status.code() == Some(2)
            {
                println!("{} Error running diff command", "!".red());
                show_simple_diff(file_a, file_b)?;
            }
        }
        | Err(_) =>
        {
            // diff command not available, show simple comparison
            println!("{} diff command not available, showing file sizes:", "!".yellow());
            show_simple_diff(file_a, file_b)?;
        }
    }

    println!("{}", "═".repeat(80).dimmed());
    println!();

    Ok(())
}

/// Shows a simple file comparison when diff command is not available
///
/// # Arguments
///
/// * `file_a` - First file path
/// * `file_b` - Second file path
///
/// # Errors
///
/// Returns an error if file metadata cannot be read
fn show_simple_diff(file_a: &Path, file_b: &Path) -> Result<()>
{
    let meta_a = fs::metadata(file_a)?;
    let meta_b = fs::metadata(file_b)?;

    println!("  Current file:  {} ({} bytes)", file_a.display(), meta_a.len());
    println!("  Template file: {} ({} bytes)", file_b.display(), meta_b.len());

    // Show first few lines of each file
    let content_a = fs::read_to_string(file_a).unwrap_or_else(|_| String::from("<binary file>"));
    let content_b = fs::read_to_string(file_b).unwrap_or_else(|_| String::from("<binary file>"));

    let lines_a: Vec<&str> = content_a.lines().take(5).collect();
    let lines_b: Vec<&str> = content_b.lines().take(5).collect();

    println!();
    println!("  Current file (first 5 lines):");
    for line in lines_a
    {
        println!("    {}", line.dimmed());
    }

    println!();
    println!("  Template file (first 5 lines):");
    for line in lines_b
    {
        println!("    {}", line.dimmed());
    }

    Ok(())
}
