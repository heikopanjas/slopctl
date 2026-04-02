//! Doctor command - checks for stale or broken managed files in the workspace

use std::{
    fs,
    path::{Path, PathBuf}
};

use owo_colors::OwoColorize;

use super::TemplateManager;
use crate::{
    Result,
    file_tracker::{FileStatus, FileTracker},
    template_engine
};

/// Classification of a detected workspace issue
#[derive(Debug)]
enum IssueKind
{
    /// File is tracked by FileTracker but no longer exists on disk
    MissingFile,
    /// File exists but still contains the template marker (merge never ran)
    UnmergedTemplate,
    /// File exists but was modified by the user since installation (informational)
    ModifiedFile
}

/// A single detected issue in the workspace
#[derive(Debug)]
struct DoctorIssue
{
    kind: IssueKind,
    path: PathBuf
}

impl TemplateManager
{
    /// Check workspace for stale or broken managed files
    ///
    /// Scans all FileTracker entries for the current workspace and reports three
    /// categories of issue: missing files (stale tracker entries), files with an
    /// unmerged template marker, and files modified since install (informational).
    ///
    /// # Arguments
    ///
    /// * `fix` - If true, automatically repair issues where safe to do so
    /// * `dry_run` - If true, show what would be done without applying any changes
    /// * `verbose` - If true, print every checked file and its check result
    ///
    /// # Errors
    ///
    /// Returns an error if the current directory or FileTracker cannot be read
    pub fn doctor(&self, fix: bool, dry_run: bool, verbose: bool) -> Result<()>
    {
        let workspace = std::env::current_dir()?;
        let mut tracker = FileTracker::new(&self.config_dir)?;

        let issues = Self::collect_issues(&tracker, &workspace, verbose)?;

        if issues.is_empty() == true
        {
            println!("{} No issues found", "✓".green());
            return Ok(());
        }

        // Display all issues grouped by kind
        println!("{}", "Issues found:".bold());
        println!();

        for issue in &issues
        {
            let rel = issue.path.strip_prefix(&workspace).unwrap_or(&issue.path);
            match issue.kind
            {
                | IssueKind::MissingFile =>
                {
                    println!("  {} {} {} {}", "✗".red(), "Missing:".red(), rel.display().to_string().red(), "(tracked but deleted)".dimmed());
                }
                | IssueKind::UnmergedTemplate =>
                {
                    println!("  {} {} {} {}", "✗".red(), "Unmerged:".red(), rel.display().to_string().red(), "(template marker still present)".dimmed());
                }
                | IssueKind::ModifiedFile =>
                {
                    println!("  {} {} {} {}", "!".yellow(), "Modified:".yellow(), rel.display().to_string().yellow(), "(changed since install)".dimmed());
                }
            }
        }

        let missing_count = issues.iter().filter(|i| matches!(i.kind, IssueKind::MissingFile)).count();
        let unmerged_count = issues.iter().filter(|i| matches!(i.kind, IssueKind::UnmergedTemplate)).count();
        let modified_count = issues.iter().filter(|i| matches!(i.kind, IssueKind::ModifiedFile)).count();

        println!();

        if missing_count > 0
        {
            println!(
                "  {} {} stale tracker entr{}",
                "✗".red(),
                missing_count,
                if missing_count == 1
                {
                    "y"
                }
                else
                {
                    "ies"
                }
            );
        }
        if unmerged_count > 0
        {
            println!(
                "  {} {} file{} with unmerged template marker",
                "✗".red(),
                unmerged_count,
                if unmerged_count == 1
                {
                    ""
                }
                else
                {
                    "s"
                }
            );
        }
        if modified_count > 0
        {
            println!(
                "  {} {} modified file{} (no automatic fix available)",
                "!".yellow(),
                modified_count,
                if modified_count == 1
                {
                    ""
                }
                else
                {
                    "s"
                }
            );
        }

        if fix == false
        {
            println!();
            println!("{} Run 'vibe-cop doctor --fix' to automatically fix issues", "→".blue());
            return Ok(());
        }

        // Apply fixes
        println!();
        if dry_run == true
        {
            println!("{}", "Dry run - no changes applied:".bold());
        }
        else
        {
            println!("{}", "Applying fixes:".bold());
        }
        println!();

        // Fix missing files: prune stale tracker entries
        let mut pruned = 0;
        for issue in &issues
        {
            if matches!(issue.kind, IssueKind::MissingFile) == false
            {
                continue;
            }

            let rel = issue.path.strip_prefix(&workspace).unwrap_or(&issue.path);
            if dry_run == true
            {
                println!("  {} Would prune stale tracker entry: {}", "→".blue(), rel.display().to_string().yellow());
            }
            else
            {
                tracker.remove_entry(&issue.path);
                println!("  {} Pruned stale tracker entry: {}", "✓".green(), rel.display().to_string().yellow());
                pruned += 1;
            }
        }

        if pruned > 0
        {
            tracker.save()?;
        }

        // Fix unmerged templates: strip the template marker from the file
        for issue in &issues
        {
            if matches!(issue.kind, IssueKind::UnmergedTemplate) == false
            {
                continue;
            }

            let rel = issue.path.strip_prefix(&workspace).unwrap_or(&issue.path);
            if dry_run == true
            {
                println!("  {} Would strip template marker from: {}", "→".blue(), rel.display().to_string().yellow());
            }
            else
            {
                Self::fix_unmerged_template(&issue.path)?;
                println!("  {} Stripped template marker from: {}", "✓".green(), rel.display().to_string().yellow());
            }
        }

        if modified_count > 0
        {
            println!(
                "  {} Skipped {} modified file{} - run 'vibe-cop install --force' to overwrite",
                "!".yellow(),
                modified_count,
                if modified_count == 1
                {
                    ""
                }
                else
                {
                    "s"
                }
            );
        }

        if dry_run == false && unmerged_count > 0
        {
            println!();
            println!("{} Run 'vibe-cop install' to re-merge language sections into AGENTS.md", "→".blue());
        }

        Ok(())
    }

    /// Collect all issues from the FileTracker for the current workspace
    ///
    /// When `verbose` is true, prints each checked file and its check result
    /// as the scan progresses.
    fn collect_issues(tracker: &FileTracker, workspace: &Path, verbose: bool) -> Result<Vec<DoctorIssue>>
    {
        let mut issues: Vec<DoctorIssue> = Vec::new();
        let entries = tracker.get_workspace_entries(workspace);
        let has_entries = entries.is_empty() == false;

        if verbose == true && has_entries == true
        {
            println!("{}", "Checking workspace files:".bold());
            println!();
        }

        for (path, metadata) in entries
        {
            let rel = path.strip_prefix(workspace).unwrap_or(&path);
            match tracker.check_modification(&path)?
            {
                | FileStatus::Deleted =>
                {
                    if verbose == true
                    {
                        println!("  {} {} {}", "✗".red(), "Missing: ".red(), rel.display().to_string().red());
                    }
                    issues.push(DoctorIssue { kind: IssueKind::MissingFile, path });
                }
                | FileStatus::Modified =>
                {
                    if verbose == true
                    {
                        println!("  {} {} {}", "!".yellow(), "Modified:".yellow(), rel.display().to_string().yellow());
                    }
                    issues.push(DoctorIssue { kind: IssueKind::ModifiedFile, path });
                }
                | FileStatus::Unmodified =>
                {
                    // Only "main" category files carry the template marker; flag if still present
                    if metadata.category == "main" && template_engine::is_file_customized(&path)? == false
                    {
                        if verbose == true
                        {
                            println!("  {} {} {}", "✗".red(), "Unmerged:".red(), rel.display().to_string().red());
                        }
                        issues.push(DoctorIssue { kind: IssueKind::UnmergedTemplate, path });
                    }
                    else if verbose == true
                    {
                        println!("  {} {} {}", "✓".green(), "OK:      ".green(), rel.display().to_string().dimmed());
                    }
                }
                | FileStatus::NotTracked =>
                {
                    // Should not occur via get_workspace_entries; ignore defensively
                }
            }
        }

        if verbose == true && has_entries == true
        {
            println!();
        }

        Ok(issues)
    }

    /// Strip the template marker comment from a file without altering other content
    ///
    /// After stripping, the file is considered "customized" by `is_file_customized`,
    /// so future `install` runs will prompt before overwriting it.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or written
    fn fix_unmerged_template(path: &Path) -> Result<()>
    {
        let content = fs::read_to_string(path)?;
        let fixed: String = content.lines().filter(|line| *line != template_engine::TEMPLATE_MARKER).collect::<Vec<_>>().join("\n");
        fs::write(path, fixed + "\n")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests
{
    use std::fs;

    use super::*;

    fn make_temp_dir() -> tempfile::TempDir
    {
        tempfile::tempdir().expect("failed to create temp dir")
    }

    #[test]
    fn test_fix_unmerged_template_strips_marker()
    {
        let dir = make_temp_dir();
        let file = dir.path().join("AGENTS.md");
        let content = format!("# Header\n{}\nsome content\n", template_engine::TEMPLATE_MARKER);
        fs::write(&file, &content).unwrap();

        TemplateManager::fix_unmerged_template(&file).unwrap();

        let result = fs::read_to_string(&file).unwrap();
        assert!(result.contains(template_engine::TEMPLATE_MARKER) == false);
        assert!(result.contains("# Header") == true);
        assert!(result.contains("some content") == true);
    }

    #[test]
    fn test_fix_unmerged_template_preserves_content_without_marker()
    {
        let dir = make_temp_dir();
        let file = dir.path().join("AGENTS.md");
        let content = "# My custom content\n\nSome instructions here.\n";
        fs::write(&file, content).unwrap();

        TemplateManager::fix_unmerged_template(&file).unwrap();

        let result = fs::read_to_string(&file).unwrap();
        assert!(result.contains("# My custom content") == true);
        assert!(result.contains("Some instructions here.") == true);
    }

    #[test]
    fn test_collect_issues_empty_tracker()
    {
        let dir = make_temp_dir();
        // Empty tracker (no installed_files.json) → no issues
        let tracker = FileTracker::new(dir.path()).unwrap();
        let issues = TemplateManager::collect_issues(&tracker, dir.path(), false).unwrap();
        assert!(issues.is_empty() == true);
    }

    #[test]
    fn test_doctor_no_issues_prints_ok()
    {
        let dir = make_temp_dir();
        let manager = TemplateManager { config_dir: dir.path().to_path_buf() };
        // Should succeed with no issues when tracker is empty
        // (doctor uses current_dir, not dir.path(), so we just test it doesn't error)
        let result = manager.doctor(false, false, false);
        assert!(result.is_ok() == true);
    }

    #[test]
    fn test_issue_kind_missing_file()
    {
        let dir = make_temp_dir();
        let data_dir = dir.path().join("data");
        fs::create_dir_all(&data_dir).unwrap();

        let workspace = dir.path().join("workspace");
        fs::create_dir_all(&workspace).unwrap();

        let target = workspace.join("AGENTS.md");

        // Create a tracker entry for a file that doesn't exist on disk
        let mut tracker = FileTracker::new(&data_dir).unwrap();
        tracker.record_installation(&target, "abc123".to_string(), 4, None, "main".to_string());
        tracker.save().unwrap();

        // Reload and check
        let tracker2 = FileTracker::new(&data_dir).unwrap();
        let issues = TemplateManager::collect_issues(&tracker2, &workspace, false).unwrap();
        assert!(issues.len() == 1);
        assert!(matches!(issues[0].kind, IssueKind::MissingFile) == true);
    }

    #[test]
    fn test_issue_kind_unmerged_template()
    {
        let dir = make_temp_dir();
        let data_dir = dir.path().join("data");
        fs::create_dir_all(&data_dir).unwrap();

        let workspace = dir.path().join("workspace");
        fs::create_dir_all(&workspace).unwrap();

        let target = workspace.join("AGENTS.md");
        let content = format!("# Header\n{}\n", template_engine::TEMPLATE_MARKER);
        fs::write(&target, &content).unwrap();

        let sha = FileTracker::calculate_sha256(&target).unwrap();
        let mut tracker = FileTracker::new(&data_dir).unwrap();
        tracker.record_installation(&target, sha, 4, None, "main".to_string());
        tracker.save().unwrap();

        let tracker2 = FileTracker::new(&data_dir).unwrap();
        let issues = TemplateManager::collect_issues(&tracker2, &workspace, false).unwrap();
        assert!(issues.len() == 1);
        assert!(matches!(issues[0].kind, IssueKind::UnmergedTemplate) == true);
    }
}
