//! Thin wrapper around the user's `git` CLI for fetching template systems from git repositories
//!
//! slopctl never handles credentials itself: cloning through the installed `git` lets the user's
//! SSH keys and credential helpers authenticate private repositories on any host.

use std::{
    io,
    path::Path,
    process::{Command, Output}
};

use crate::Result;

/// Returns true when `uri` should be fetched with `git` rather than as a local path or GitHub tree URL
///
/// Matches scp-style (`git@host:org/repo.git`), `ssh://`, `git://` and `file://` URLs, and
/// `http(s)://` URLs ending in `.git`.
pub fn is_git_url(uri: &str) -> bool
{
    let is_http = uri.starts_with("https://") == true || uri.starts_with("http://") == true;
    uri.starts_with("git@") == true ||
        uri.starts_with("ssh://") == true ||
        uri.starts_with("git://") == true ||
        uri.starts_with("file://") == true ||
        (is_http == true && uri.trim_end_matches('/').ends_with(".git") == true)
}

/// Shallow-clones `uri` into `dest` (which must not exist or be empty)
///
/// # Arguments
///
/// * `uri` - Repository URL
/// * `git_ref` - Optional branch or tag; the remote default branch when `None`
/// * `dest` - Target directory
///
/// # Errors
///
/// Returns an error if `git` is not installed or the clone fails (including authentication)
pub fn clone_shallow(uri: &str, git_ref: Option<&str>, dest: &Path) -> Result<()>
{
    let mut command = Command::new("git");
    command.args(["clone", "--quiet", "--depth", "1"]);
    if let Some(git_ref) = git_ref
    {
        command.args(["--branch", git_ref]);
    }
    command.arg("--").arg(uri).arg(dest);
    run(command, &format!("clone {}", uri)).map(|_| ())
}

/// Returns the commit id checked out in `repo`
///
/// # Errors
///
/// Returns an error if `git` is not installed or `repo` is not a git repository
pub fn head_commit(repo: &Path) -> Result<String>
{
    let mut command = Command::new("git");
    command.arg("-C").arg(repo).args(["rev-parse", "HEAD"]);
    run(command, "rev-parse HEAD").map(|stdout| stdout.trim().to_string())
}

/// Returns the commit id `git_ref` (or the remote HEAD) points to, without cloning
///
/// # Errors
///
/// Returns an error if `git` is not installed, the remote is unreachable, or the ref does not exist
pub fn remote_commit(uri: &str, git_ref: Option<&str>) -> Result<String>
{
    let mut command = Command::new("git");
    command.args(["ls-remote", "--", uri, git_ref.unwrap_or("HEAD")]);
    let stdout = run(command, &format!("ls-remote {}", uri))?;
    stdout
        .lines()
        .find_map(|line| line.split_whitespace().next())
        .map(str::to_string)
        .ok_or_else(|| anyhow::anyhow!("Ref '{}' not found in {}", git_ref.unwrap_or("HEAD"), uri))
}

/// Runs a git command and returns its stdout, turning a missing binary or a failure into a clear error
fn run(mut command: Command, action: &str) -> Result<String>
{
    // Every path handed to git is absolute; pinning the working directory keeps git from failing
    // when the inherited one no longer exists (e.g. a deleted temporary workspace).
    command.current_dir(std::env::temp_dir());
    let output: Output = command.output().map_err(|e| match e.kind()
    {
        | io::ErrorKind::NotFound => anyhow::anyhow!("git is not installed or not on PATH; it is required for git template sources"),
        | _ => anyhow::anyhow!("Failed to run git {}: {}", action, e)
    })?;
    require!(output.status.success() == true, Err(anyhow::anyhow!("git {} failed: {}", action, String::from_utf8_lossy(&output.stderr).trim())));
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

#[cfg(test)]
pub(crate) mod tests
{
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    /// Creates a repository with one commit on branch `main` and returns its `file://` URL
    pub(crate) fn make_repo(dir: &Path, files: &[(&str, &str)]) -> String
    {
        for (path, content) in files
        {
            let file = dir.join(path);
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write(file, content).unwrap();
        }
        for args in [vec!["init", "--quiet", "--initial-branch", "main"], vec!["add", "."], vec![
            "-c", "user.name=slopctl", "-c", "user.email=slopctl@example.invalid", "commit", "--quiet", "-m", "init",
        ]]
        {
            let status = Command::new("git").current_dir(dir).args(args).status().unwrap();
            assert!(status.success() == true);
        }
        // file URLs need forward slashes and a leading `/` before a Windows drive letter
        let path = dir.to_string_lossy().replace('\\', "/");
        if path.starts_with('/') == true
        {
            format!("file://{}", path)
        }
        else
        {
            format!("file:///{}", path)
        }
    }

    #[test]
    fn test_is_git_url_variants()
    {
        assert!(is_git_url("git@github.com:org/repo.git") == true);
        assert!(is_git_url("ssh://git@example.com/org/repo") == true);
        assert!(is_git_url("file:///tmp/repo") == true);
        assert!(is_git_url("https://gitlab.example.com/org/repo.git") == true);
        assert!(is_git_url("https://github.com/org/repo/tree/main/templates") == false);
        assert!(is_git_url("/Users/me/templates") == false);
    }

    #[test]
    fn test_clone_shallow_and_commits_match()
    {
        let origin = TempDir::new().unwrap();
        let url = make_repo(origin.path(), &[("templates/templates.yml", "version: 5\n")]);
        let checkout = TempDir::new().unwrap();
        let dest = checkout.path().join("clone");

        clone_shallow(&url, Some("main"), &dest).unwrap();

        assert!(dest.join("templates/templates.yml").is_file() == true);
        assert_eq!(head_commit(&dest).unwrap(), remote_commit(&url, Some("main")).unwrap());
    }

    #[test]
    fn test_clone_shallow_unknown_ref_errors()
    {
        let origin = TempDir::new().unwrap();
        let url = make_repo(origin.path(), &[("templates.yml", "version: 5\n")]);
        let checkout = TempDir::new().unwrap();
        let err = clone_shallow(&url, Some("no-such-branch"), &checkout.path().join("clone")).unwrap_err().to_string();
        assert!(err.contains("git clone") == true);
    }
}
