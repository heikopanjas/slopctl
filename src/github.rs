//! GitHub API integration for downloading files and listing directory contents
//!
//! Provides helpers for resolving GitHub URLs, expanding CLI shorthand notation,
//! listing repository directory contents via the GitHub Contents API, and
//! downloading individual files. Used during the `install` flow to fetch
//! remote sources on-the-fly.

use std::{fs, path::Path};

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
    let response = client.get(&api_url).header("User-Agent", "vibe-check").header("Accept", "application/vnd.github.v3+json").send()?;

    if response.status().is_success() == false
    {
        return Err(format!("GitHub API request failed: HTTP {} for {}", response.status(), api_url).into());
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
    let response = client.get(url).header("User-Agent", "vibe-check").send()?;

    if response.status().is_success() == false
    {
        return Err(format!("Failed to download {}: HTTP {}", url, response.status()).into());
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

#[cfg(test)]
mod tests
{
    use std::{error::Error, result::Result};

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
    fn test_parse_github_url_full() -> Result<(), Box<dyn Error>>
    {
        let parsed = parse_github_url("https://github.com/user/repo/tree/main/path/to/dir").ok_or("expected parsed URL")?;
        assert_eq!(parsed.owner, "user");
        assert_eq!(parsed.repo, "repo");
        assert_eq!(parsed.branch, "main");
        assert_eq!(parsed.path, "path/to/dir");
        Ok(())
    }

    #[test]
    fn test_parse_github_url_bare_repo() -> Result<(), Box<dyn Error>>
    {
        let parsed = parse_github_url("https://github.com/user/repo").ok_or("expected parsed URL")?;
        assert_eq!(parsed.owner, "user");
        assert_eq!(parsed.repo, "repo");
        assert_eq!(parsed.branch, "main");
        assert_eq!(parsed.path, "");
        Ok(())
    }

    #[test]
    fn test_parse_github_url_blob() -> Result<(), Box<dyn Error>>
    {
        let parsed = parse_github_url("https://github.com/user/repo/blob/develop/src/file.rs").ok_or("expected parsed URL")?;
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
}
