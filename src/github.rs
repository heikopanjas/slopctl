//! GitHub API integration for downloading files and listing directory contents
//!
//! Provides helpers for resolving GitHub URLs, expanding CLI shorthand notation,
//! listing repository directory contents via the GitHub Contents API, and
//! downloading individual files. Used during the `install` flow to fetch
//! remote sources on-the-fly.

use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf}
};

use owo_colors::OwoColorize;
use serde::Deserialize;

use crate::Result;

/// A single entry returned by the GitHub Contents API
#[derive(Debug, Deserialize)]
pub struct GitHubContentEntry
{
    pub name:         String,
    #[serde(rename = "type")]
    pub entry_type:   String,
    pub download_url: Option<String>,
    pub path:         String
}

/// Parsed components of a GitHub tree/blob URL
#[derive(Debug, Clone)]
pub struct GitHubUrl
{
    pub owner:  String,
    pub repo:   String,
    pub branch: String,
    pub path:   String
}

impl GitHubUrl
{
    /// Build the raw.githubusercontent.com URL for a specific file
    pub fn raw_file_url(&self, file_path: &str) -> String
    {
        if self.path.is_empty() == true
        {
            format!("https://raw.githubusercontent.com/{}/{}/{}/{}", self.owner, self.repo, self.branch, file_path)
        }
        else
        {
            format!("https://raw.githubusercontent.com/{}/{}/{}/{}/{}", self.owner, self.repo, self.branch, self.path, file_path)
        }
    }

    /// Build the GitHub Contents API URL for this path
    pub fn contents_api_url(&self) -> String
    {
        if self.path.is_empty() == true
        {
            format!("https://api.github.com/repos/{}/{}/contents?ref={}", self.owner, self.repo, self.branch)
        }
        else
        {
            format!("https://api.github.com/repos/{}/{}/contents/{}?ref={}", self.owner, self.repo, self.path, self.branch)
        }
    }

    /// Build a child URL by appending a subdirectory name to this path
    pub fn child(&self, name: &str) -> Self
    {
        let child_path = if self.path.is_empty() == true
        {
            name.to_string()
        }
        else
        {
            format!("{}/{}", self.path, name)
        };

        Self { owner: self.owner.clone(), repo: self.repo.clone(), branch: self.branch.clone(), path: child_path }
    }

    /// Derive a human-readable skill name from this URL
    ///
    /// Uses the last segment of `path` if non-empty, otherwise the repo name.
    pub fn skill_name(&self) -> String
    {
        if self.path.is_empty() == false
        {
            let trimmed = self.path.trim_end_matches('/');
            if let Some(last) = trimmed.rsplit('/').next() &&
                last.is_empty() == false
            {
                return last.to_string();
            }
        }

        self.repo.clone()
    }
}

/// Check if a string is a GitHub URL (full URL, not shorthand)
pub fn is_github_url(source: &str) -> bool
{
    source.starts_with("https://github.com/") || source.starts_with("http://github.com/")
}

/// Check if a source string is any URL (http/https)
pub fn is_url(source: &str) -> bool
{
    source.starts_with("http://") || source.starts_with("https://")
}

/// Parse a full GitHub URL into its components
///
/// Accepts URLs like:
/// - `https://github.com/owner/repo/tree/branch/path`
/// - `https://github.com/owner/repo/blob/branch/path`
/// - `https://github.com/owner/repo` (defaults to branch `main`, empty path)
///
/// # Arguments
///
/// * `url` - Full GitHub URL
///
/// # Returns
///
/// Parsed `GitHubUrl` or None if the URL is not a valid GitHub URL
pub fn parse_github_url(url: &str) -> Option<GitHubUrl>
{
    if is_github_url(url) == false
    {
        return None;
    }

    let parts: Vec<&str> = url.split('/').collect();
    let github_idx = parts.iter().position(|&p| p == "github.com")?;

    if parts.len() < github_idx + 3
    {
        return None;
    }

    let owner = parts[github_idx + 1].to_string();
    let repo = parts[github_idx + 2].to_string();

    // Bare repo URL: https://github.com/owner/repo
    if parts.len() <= github_idx + 3
    {
        return Some(GitHubUrl { owner, repo, branch: "main".to_string(), path: String::new() });
    }

    // URL with tree/blob: https://github.com/owner/repo/tree/branch/path
    if parts.len() >= github_idx + 5
    {
        let tree_or_blob = parts[github_idx + 3];
        if tree_or_blob == "tree" || tree_or_blob == "blob"
        {
            let branch = parts[github_idx + 4].to_string();
            let path = if parts.len() > github_idx + 5
            {
                parts[github_idx + 5..].join("/")
            }
            else
            {
                String::new()
            };
            return Some(GitHubUrl { owner, repo, branch, path });
        }
    }

    // Unrecognized structure, default to main
    Some(GitHubUrl { owner, repo, branch: "main".to_string(), path: String::new() })
}

/// Expand CLI shorthand to a full GitHub URL
///
/// Only used for `--skill` CLI arguments, never for templates.yml sources.
///
/// Shorthand formats:
/// - `user/repo` -> `https://github.com/user/repo/tree/main`
/// - `user/repo/sub/path` -> `https://github.com/user/repo/tree/main/sub/path`
///
/// If the input is already a full URL, it is returned as-is.
///
/// # Arguments
///
/// * `input` - CLI shorthand or full URL
pub fn expand_shorthand(input: &str) -> String
{
    if is_url(input) == true
    {
        return input.to_string();
    }

    let parts: Vec<&str> = input.split('/').collect();
    if parts.len() < 2
    {
        return input.to_string();
    }

    let owner = parts[0];
    let repo = parts[1];

    if parts.len() > 2
    {
        let sub_path = parts[2..].join("/");
        format!("https://github.com/{}/{}/tree/main/{}", owner, repo, sub_path)
    }
    else
    {
        format!("https://github.com/{}/{}/tree/main", owner, repo)
    }
}

/// List directory contents via the GitHub Contents API
///
/// Uses the unauthenticated GitHub API (60 requests/hour for public repos).
///
/// # Arguments
///
/// * `github_url` - Parsed GitHub URL pointing to a directory
///
/// # Errors
///
/// Returns an error if the API request fails or returns non-200
pub fn list_directory_contents(github_url: &GitHubUrl) -> Result<Vec<GitHubContentEntry>>
{
    let api_url = github_url.contents_api_url();

    let client = reqwest::blocking::Client::new();
    let response = client.get(&api_url).header("User-Agent", "slopctl").header("Accept", "application/vnd.github.v3+json").send()?;

    if response.status().is_success() == false
    {
        return Err(anyhow::anyhow!("GitHub API request failed: HTTP {} for {}", response.status(), api_url));
    }

    let entries: Vec<GitHubContentEntry> = response.json()?;
    Ok(entries)
}

/// Download a single file from a URL to a destination path
///
/// # Arguments
///
/// * `url` - URL to download from
/// * `dest_path` - Local file path to write to
///
/// # Errors
///
/// Returns an error if the download or file write fails
pub fn download_file(url: &str, dest_path: &Path) -> Result<()>
{
    let client = reqwest::blocking::Client::new();
    let response = client.get(url).header("User-Agent", "slopctl").send()?;

    if response.status().is_success() == false
    {
        return Err(anyhow::anyhow!("Failed to download {}: HTTP {}", url, response.status()));
    }

    let content = response.bytes()?;

    if let Some(parent) = dest_path.parent()
    {
        fs::create_dir_all(parent)?;
    }

    fs::write(dest_path, content)?;

    Ok(())
}

/// Download a single file from a GitHub URL
///
/// Resolves the GitHub URL to a raw download URL and fetches the file.
///
/// # Arguments
///
/// * `github_url` - Parsed GitHub URL pointing to a file
/// * `dest_path` - Local file path to write to
///
/// # Errors
///
/// Returns an error if the download fails
pub fn download_github_file(github_url: &GitHubUrl, dest_path: &Path) -> Result<()>
{
    let raw_url = format!("https://raw.githubusercontent.com/{}/{}/{}/{}", github_url.owner, github_url.repo, github_url.branch, github_url.path);

    download_file(&raw_url, dest_path)
}

/// Recursively download all files from a GitHub directory
///
/// Lists directory contents via the Contents API, downloads files, and
/// recurses into subdirectories. Returns `(temp_path, relative_path)` pairs
/// where `relative_path` preserves the directory structure under the root.
///
/// # Arguments
///
/// * `github_url` - Parsed GitHub URL pointing to a directory
/// * `temp_dir` - Local temp directory for downloaded files
/// * `prefix` - Flat prefix for temp file names (avoids collisions)
/// * `rel_base` - Relative path prefix for preserving directory structure
///
/// # Errors
///
/// Returns an error if directory listing fails (individual file errors are logged and skipped)
pub fn download_directory_recursive(github_url: &GitHubUrl, temp_dir: &Path, prefix: &str, rel_base: &str) -> Result<Vec<(PathBuf, String)>>
{
    let entries = list_directory_contents(github_url)?;
    download_entries(entries, github_url, temp_dir, prefix, rel_base)
}

/// Download files from pre-fetched GitHub directory entries
///
/// Same as [`download_directory_recursive`] but accepts already-fetched entries
/// for the top-level directory, avoiding a redundant API call when the listing
/// was obtained during a prior discovery phase.
///
/// Subdirectories are still fetched recursively via the Contents API.
///
/// # Arguments
///
/// * `entries` - Pre-fetched directory entries from a prior `list_directory_contents` call
/// * `github_url` - Parsed GitHub URL for the directory (used for subdirectory recursion)
/// * `temp_dir` - Local temp directory for downloaded files
/// * `prefix` - Flat prefix for temp file names (avoids collisions)
/// * `rel_base` - Relative path prefix for preserving directory structure
///
/// # Errors
///
/// Returns an error if a subdirectory listing fails (individual file errors are logged and skipped)
pub fn download_directory_from_entries(
    entries: Vec<GitHubContentEntry>, github_url: &GitHubUrl, temp_dir: &Path, prefix: &str, rel_base: &str
) -> Result<Vec<(PathBuf, String)>>
{
    download_entries(entries, github_url, temp_dir, prefix, rel_base)
}

/// Process directory entries: download files and recurse into subdirectories
fn download_entries(entries: Vec<GitHubContentEntry>, github_url: &GitHubUrl, temp_dir: &Path, prefix: &str, rel_base: &str) -> Result<Vec<(PathBuf, String)>>
{
    let mut downloaded = Vec::new();

    for entry in &entries
    {
        let rel_path = if rel_base.is_empty() == true
        {
            entry.name.clone()
        }
        else
        {
            format!("{}/{}", rel_base, entry.name)
        };

        if entry.entry_type == "file" &&
            let Some(ref dl_url) = entry.download_url
        {
            let safe_name = rel_path.replace('/', "_");
            let temp_path = temp_dir.join(format!("{}_{}", prefix, safe_name));

            print!("  {} Downloading {}... ", "→".blue(), rel_path.yellow());
            io::stdout().flush()?;

            match download_file(dl_url, &temp_path)
            {
                | Ok(_) =>
                {
                    println!("{}", "✓".green());
                    downloaded.push((temp_path, rel_path));
                }
                | Err(e) =>
                {
                    println!("{} ({})", "✗".red(), e);
                }
            }
        }
        else if entry.entry_type == "dir"
        {
            let child_url = github_url.child(&entry.name);
            match download_directory_recursive(&child_url, temp_dir, prefix, &rel_path)
            {
                | Ok(sub_files) => downloaded.extend(sub_files),
                | Err(e) =>
                {
                    println!("  {} Skipping subdirectory {}: {}", "!".yellow(), entry.name.yellow(), e);
                }
            }
        }
    }

    Ok(downloaded)
}

/// A discovered skill: its name, GitHub URL, and pre-fetched directory entries
///
/// Carries the directory listing obtained during discovery so that the
/// subsequent download phase can reuse it instead of making a redundant
/// GitHub API call.
pub struct DiscoveredSkill
{
    pub name:    String,
    pub url:     GitHubUrl,
    pub entries: Vec<GitHubContentEntry>
}

/// Discover skills by recursively scanning a GitHub directory for SKILL.md
///
/// If the directory itself contains a SKILL.md, it is treated as a single skill.
/// Otherwise, subdirectories are scanned recursively for SKILL.md files.
///
/// # Arguments
///
/// * `github_url` - Parsed GitHub URL pointing to a directory
///
/// # Errors
///
/// Returns an error if the top-level directory listing fails
pub fn discover_skills(github_url: &GitHubUrl) -> Result<Vec<DiscoveredSkill>>
{
    let entries = list_directory_contents(github_url)?;

    let has_skill_md = entries.iter().any(|e| e.entry_type == "file" && e.name == "SKILL.md");

    if has_skill_md == true
    {
        return Ok(vec![DiscoveredSkill { name: github_url.skill_name(), url: github_url.clone(), entries }]);
    }

    let mut found = Vec::new();
    for entry in &entries
    {
        if entry.entry_type == "dir"
        {
            let child_url = github_url.child(&entry.name);
            match discover_skills(&child_url)
            {
                | Ok(sub_skills) => found.extend(sub_skills),
                | Err(e) =>
                {
                    println!("  {} Skipping {}: {}", "!".yellow(), entry.name.yellow(), e);
                }
            }
        }
    }

    Ok(found)
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn test_is_github_url()
    {
        assert!(is_github_url("https://github.com/user/repo") == true);
        assert!(is_github_url("http://github.com/user/repo") == true);
        assert!(is_github_url("https://gitlab.com/user/repo") == false);
        assert!(is_github_url("user/repo") == false);
        assert!(is_github_url("local-path/file.md") == false);
    }

    #[test]
    fn test_is_url()
    {
        assert!(is_url("https://example.com") == true);
        assert!(is_url("http://example.com") == true);
        assert!(is_url("local-path") == false);
        assert!(is_url("user/repo") == false);
    }

    #[test]
    fn test_parse_github_url_full() -> anyhow::Result<()>
    {
        let parsed = parse_github_url("https://github.com/user/repo/tree/main/path/to/dir").ok_or_else(|| anyhow::anyhow!("expected parsed URL"))?;
        assert_eq!(parsed.owner, "user");
        assert_eq!(parsed.repo, "repo");
        assert_eq!(parsed.branch, "main");
        assert_eq!(parsed.path, "path/to/dir");
        Ok(())
    }

    #[test]
    fn test_parse_github_url_bare_repo() -> anyhow::Result<()>
    {
        let parsed = parse_github_url("https://github.com/user/repo").ok_or_else(|| anyhow::anyhow!("expected parsed URL"))?;
        assert_eq!(parsed.owner, "user");
        assert_eq!(parsed.repo, "repo");
        assert_eq!(parsed.branch, "main");
        assert_eq!(parsed.path, "");
        Ok(())
    }

    #[test]
    fn test_parse_github_url_blob() -> anyhow::Result<()>
    {
        let parsed = parse_github_url("https://github.com/user/repo/blob/develop/src/file.rs").ok_or_else(|| anyhow::anyhow!("expected parsed URL"))?;
        assert_eq!(parsed.owner, "user");
        assert_eq!(parsed.repo, "repo");
        assert_eq!(parsed.branch, "develop");
        assert_eq!(parsed.path, "src/file.rs");
        Ok(())
    }

    #[test]
    fn test_parse_github_url_invalid()
    {
        assert!(parse_github_url("https://gitlab.com/user/repo").is_none());
        assert!(parse_github_url("not-a-url").is_none());
    }

    #[test]
    fn test_expand_shorthand_user_repo()
    {
        assert_eq!(expand_shorthand("user/repo"), "https://github.com/user/repo/tree/main");
    }

    #[test]
    fn test_expand_shorthand_with_path()
    {
        assert_eq!(expand_shorthand("user/repo/skills/create-rule"), "https://github.com/user/repo/tree/main/skills/create-rule");
    }

    #[test]
    fn test_expand_shorthand_full_url_passthrough()
    {
        let url = "https://github.com/user/repo/tree/develop/path";
        assert_eq!(expand_shorthand(url), url);
    }

    #[test]
    fn test_expand_shorthand_single_segment()
    {
        assert_eq!(expand_shorthand("just-a-name"), "just-a-name");
    }

    #[test]
    fn test_github_url_raw_file_url()
    {
        let url = GitHubUrl { owner: "user".into(), repo: "repo".into(), branch: "main".into(), path: "skills/my-skill".into() };
        assert_eq!(url.raw_file_url("SKILL.md"), "https://raw.githubusercontent.com/user/repo/main/skills/my-skill/SKILL.md");
    }

    #[test]
    fn test_github_url_contents_api_url()
    {
        let url = GitHubUrl { owner: "user".into(), repo: "repo".into(), branch: "main".into(), path: "skills/my-skill".into() };
        assert_eq!(url.contents_api_url(), "https://api.github.com/repos/user/repo/contents/skills/my-skill?ref=main");
    }

    #[test]
    fn test_github_url_contents_api_url_empty_path()
    {
        let url = GitHubUrl { owner: "user".into(), repo: "repo".into(), branch: "main".into(), path: String::new() };
        assert_eq!(url.contents_api_url(), "https://api.github.com/repos/user/repo/contents?ref=main");
    }

    // -- skill_name --

    #[test]
    fn test_skill_name_with_path()
    {
        let url = GitHubUrl { owner: "user".into(), repo: "repo".into(), branch: "main".into(), path: "skills/my-skill".into() };
        assert_eq!(url.skill_name(), "my-skill");
    }

    #[test]
    fn test_skill_name_empty_path_uses_repo()
    {
        let url = GitHubUrl { owner: "twostraws".into(), repo: "swiftui-agent-skill".into(), branch: "main".into(), path: String::new() };
        assert_eq!(url.skill_name(), "swiftui-agent-skill");
    }

    #[test]
    fn test_skill_name_single_path_segment()
    {
        let url = GitHubUrl { owner: "user".into(), repo: "repo".into(), branch: "main".into(), path: "swiftui-pro".into() };
        assert_eq!(url.skill_name(), "swiftui-pro");
    }

    #[test]
    fn test_skill_name_trailing_slash()
    {
        let url = GitHubUrl { owner: "user".into(), repo: "repo".into(), branch: "main".into(), path: "skills/my-skill/".into() };
        assert_eq!(url.skill_name(), "my-skill");
    }

    // -- child --

    #[test]
    fn test_child_empty_path()
    {
        let parent = GitHubUrl { owner: "user".into(), repo: "repo".into(), branch: "main".into(), path: String::new() };
        let child = parent.child("subdir");
        assert_eq!(child.owner, "user");
        assert_eq!(child.repo, "repo");
        assert_eq!(child.branch, "main");
        assert_eq!(child.path, "subdir");
    }

    #[test]
    fn test_child_with_existing_path()
    {
        let parent = GitHubUrl { owner: "user".into(), repo: "repo".into(), branch: "main".into(), path: "skills".into() };
        let child = parent.child("my-skill");
        assert_eq!(child.path, "skills/my-skill");
    }
}
