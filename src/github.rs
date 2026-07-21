//! GitHub API integration for downloading files and listing directory contents
//!
//! Provides helpers for resolving full GitHub URLs, listing repository directory
//! contents via the GitHub Contents API, and downloading individual files. Used
//! during the `init` flow to fetch remote template sources on-the-fly.

use std::{
    cell::RefCell,
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    thread,
    time::{Duration, Instant}
};

use flate2::read::GzDecoder;
use owo_colors::OwoColorize;
use reqwest::{
    blocking::Client,
    header::{ACCEPT, USER_AGENT}
};
use serde::Deserialize;
use tar::Archive;

use crate::{Result, utils::copy_dir_all};

/// Test injection hooks for HTTP-bound GitHub calls
///
/// When set (typically by tests via [`set_test_hooks`]), the hooks intercept
/// [`list_directory_contents`] and [`download_file`] before any real network
/// I/O happens. In production, both hooks remain `None` and the cost is a
/// single thread-local read per call.
type ListContentsHook = Box<dyn Fn(&GitHubUrl) -> Result<Vec<GitHubContentEntry>>>;
type DownloadFileHook = Box<dyn Fn(&str) -> Result<Vec<u8>>>;
type TarballHook = Box<dyn Fn(&str, &str, &str) -> Result<Vec<u8>>>;

thread_local! {
    static LIST_CONTENTS_HOOK: RefCell<Option<ListContentsHook>> = const { RefCell::new(None) };
    static DOWNLOAD_FILE_HOOK: RefCell<Option<DownloadFileHook>> = const { RefCell::new(None) };
    static TARBALL_HOOK: RefCell<Option<TarballHook>> = const { RefCell::new(None) };
}

static GITHUB_CLIENT: OnceLock<Client> = OnceLock::new();
static RAW_DOWNLOAD_THROTTLE: Mutex<Option<Instant>> = Mutex::new(None);

const MAX_HTTP_RETRIES: u32 = 5;
const RAW_DOWNLOAD_MIN_GAP: Duration = Duration::from_millis(150);
const RATE_LIMIT_MESSAGE: &str = "GitHub rate limit exceeded; wait and retry 'slopctl templates --update'";

#[cfg(test)]
thread_local! {
    static MOCK_HTTP_STATUSES: RefCell<Vec<u16>> = const { RefCell::new(Vec::new()) };
}

#[cfg(test)]
fn push_mock_http_status(status: u16)
{
    MOCK_HTTP_STATUSES.with(|queue| queue.borrow_mut().push(status));
}

#[cfg(test)]
fn take_mock_http_status() -> Option<u16>
{
    MOCK_HTTP_STATUSES.with(|queue| {
        let mut statuses = queue.borrow_mut();
        if statuses.is_empty() == true
        {
            None
        }
        else
        {
            Some(statuses.remove(0))
        }
    })
}

#[cfg(test)]
fn clear_mock_http_statuses()
{
    MOCK_HTTP_STATUSES.with(|queue| queue.borrow_mut().clear());
}

fn github_client() -> &'static Client
{
    GITHUB_CLIENT.get_or_init(|| {
        Client::builder()
            .user_agent("slopctl")
            .connect_timeout(Duration::from_secs(30))
            .timeout(Duration::from_secs(300))
            .build()
            .expect("failed to build GitHub HTTP client")
    })
}

/// Returns seconds to wait from a GitHub `Retry-After` header, or a default backoff
fn retry_after_secs(response: &reqwest::blocking::Response) -> u64
{
    response.headers().get("retry-after").and_then(|value| value.to_str().ok()).and_then(|value| value.parse::<u64>().ok()).unwrap_or(2).min(60)
}

/// Performs a GET with retry on HTTP 429/503
fn get_bytes_with_retry(url: &str, accept: Option<&str>) -> Result<Vec<u8>>
{
    for attempt in 0..MAX_HTTP_RETRIES
    {
        #[cfg(test)]
        if let Some(status) = take_mock_http_status()
        {
            if (status == 429 || status == 503) && attempt + 1 < MAX_HTTP_RETRIES
            {
                thread::sleep(Duration::from_millis(1));
                continue;
            }

            if (200..300).contains(&status) == true
            {
                return Ok(b"mock-http-success".to_vec());
            }

            if status == 429 || status == 503
            {
                return Err(anyhow::anyhow!("{} (HTTP {})", RATE_LIMIT_MESSAGE, status));
            }

            return Err(anyhow::anyhow!("GitHub request failed: HTTP {} for {}", status, url));
        }

        let mut request = github_client().get(url).header(USER_AGENT, "slopctl");
        if let Some(value) = accept
        {
            request = request.header(ACCEPT, value);
        }

        let response = request.send()?;
        let status = response.status();

        if (status.as_u16() == 429 || status.as_u16() == 503) && attempt + 1 < MAX_HTTP_RETRIES
        {
            thread::sleep(Duration::from_secs(retry_after_secs(&response)));
            continue;
        }

        if status.is_success() == false
        {
            if status.as_u16() == 429 || status.as_u16() == 503
            {
                return Err(anyhow::anyhow!("{} (HTTP {})", RATE_LIMIT_MESSAGE, status));
            }
            return Err(anyhow::anyhow!("GitHub request failed: HTTP {} for {}", status, url));
        }

        return Ok(response.bytes()?.to_vec());
    }

    Err(anyhow::anyhow!("{}", RATE_LIMIT_MESSAGE))
}

/// Enforces a minimum gap between raw file downloads to reduce burst 429s
fn throttle_raw_download()
{
    let mut guard = RAW_DOWNLOAD_THROTTLE.lock().expect("raw download throttle mutex poisoned");
    if let Some(last) = *guard
    {
        let elapsed = last.elapsed();
        if elapsed < RAW_DOWNLOAD_MIN_GAP
        {
            thread::sleep(RAW_DOWNLOAD_MIN_GAP - elapsed);
        }
    }
    *guard = Some(Instant::now());
}

/// Repository identity for tarball deduplication within a single command run
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RepoTarballKey
{
    pub owner:  String,
    pub repo:   String,
    pub branch: String
}

impl RepoTarballKey
{
    /// Builds a key from a parsed GitHub URL
    pub fn from_github_url(url: &GitHubUrl) -> Self
    {
        Self { owner: url.owner.clone(), repo: url.repo.clone(), branch: url.branch.clone() }
    }
}

/// Deduplicates tarball downloads and extraction within one templates/init run
pub struct RepoTarballCache
{
    roots: std::collections::HashMap<RepoTarballKey, (tempfile::TempDir, PathBuf)>
}

impl RepoTarballCache
{
    /// Creates an empty tarball cache
    pub fn new() -> Self
    {
        Self { roots: std::collections::HashMap::new() }
    }

    /// Returns the extracted repository root directory, downloading once per key
    ///
    /// # Errors
    ///
    /// Returns an error if tarball download or extraction fails
    pub fn repo_root(&mut self, key: &RepoTarballKey) -> Result<&Path>
    {
        if self.roots.contains_key(key) == false
        {
            let temp_dir = tempfile::TempDir::new()?;
            let bytes = download_repo_tarball_bytes(&key.owner, &key.repo, &key.branch)?;
            let root = extract_tarball_gz(&bytes, temp_dir.path())?;
            self.roots.insert(key.clone(), (temp_dir, root));
        }

        Ok(&self.roots.get(key).expect("repo tarball cache entry").1)
    }
}

impl Default for RepoTarballCache
{
    fn default() -> Self
    {
        Self::new()
    }
}

/// Builds the GitHub tarball API URL for a repository ref
pub fn repo_tarball_url(owner: &str, repo: &str, branch: &str) -> String
{
    format!("https://api.github.com/repos/{}/{}/tarball/{}", owner, repo, branch)
}

/// Downloads a repository tarball as raw bytes (one REST call per repo)
///
/// # Errors
///
/// Returns an error if the download fails or rate limits are exhausted
pub fn download_repo_tarball_bytes(owner: &str, repo: &str, branch: &str) -> Result<Vec<u8>>
{
    if let Some(result) = TARBALL_HOOK.with(|hook| hook.borrow().as_ref().map(|fetch| fetch(owner, repo, branch)))
    {
        return result;
    }

    let url = repo_tarball_url(owner, repo, branch);
    get_bytes_with_retry(&url, Some("application/vnd.github+json"))
}

/// Extracts a `.tar.gz` archive into `dest`, returning the GitHub root directory path
///
/// GitHub tarballs contain a single top-level directory (`owner-repo-<sha>/`). Symlinks and
/// special entries are skipped for cross-platform safety.
///
/// # Errors
///
/// Returns an error if the archive is invalid or paths are unsafe
pub fn extract_tarball_gz(archive: &[u8], dest: &Path) -> Result<PathBuf>
{
    fs::create_dir_all(dest)?;

    let decoder = GzDecoder::new(archive);
    let mut archive = Archive::new(decoder);

    for entry in archive.entries()?
    {
        let mut entry = entry?;
        let entry_type = entry.header().entry_type();

        if entry_type.is_file() == false && entry_type.is_dir() == false
        {
            continue;
        }

        let path = entry.path()?.into_owned();
        if path.components().any(|component| component == std::path::Component::ParentDir) == true
        {
            continue;
        }

        entry.unpack_in(dest)?;
    }

    single_subdirectory(dest)
}

/// Finds the sole top-level directory GitHub adds to repository tarballs
fn single_subdirectory(dest: &Path) -> Result<PathBuf>
{
    let mut children = fs::read_dir(dest)?.filter_map(|entry| entry.ok()).filter(|entry| entry.path().is_dir() == true).map(|entry| entry.path()).collect::<Vec<_>>();

    if children.len() == 1
    {
        return Ok(children.remove(0));
    }

    Err(anyhow::anyhow!("Expected one root directory in GitHub tarball, found {}", children.len()))
}

/// Joins a template-relative source path with an optional repository subpath prefix
pub fn repo_relative_template_path(parsed: &GitHubUrl, source: &str) -> PathBuf
{
    if parsed.path.is_empty() == true
    {
        PathBuf::from(source)
    }
    else
    {
        Path::new(&parsed.path).join(source)
    }
}

/// Discovers skills by scanning a local directory tree for `SKILL.md` files
///
/// If `search_root` itself contains `SKILL.md`, it is treated as a single skill named from
/// the directory. Otherwise subdirectories containing `SKILL.md` are returned.
pub fn discover_skills_in_dir(search_root: &Path) -> Vec<(String, PathBuf)>
{
    if search_root.is_dir() == false
    {
        return Vec::new();
    }

    if search_root.join("SKILL.md").is_file() == true
    {
        let name = search_root.file_name().and_then(|value| value.to_str()).unwrap_or("skill").to_string();
        return vec![(name, search_root.to_path_buf())];
    }

    let mut found = Vec::new();
    discover_skills_in_dir_recursive(search_root, &mut found);
    found.sort_by(|left, right| left.0.cmp(&right.0));
    found
}

fn discover_skills_in_dir_recursive(dir: &Path, found: &mut Vec<(String, PathBuf)>)
{
    let Ok(entries) = fs::read_dir(dir)
    else
    {
        return;
    };

    for entry in entries.flatten()
    {
        let path = entry.path();
        if path.is_dir() == false
        {
            continue;
        }

        if path.join("SKILL.md").is_file() == true
        {
            if let Some(name) = path.file_name().and_then(|value| value.to_str())
            {
                found.push((name.to_string(), path));
            }
        }
        else
        {
            discover_skills_in_dir_recursive(&path, found);
        }
    }
}

/// Copies an extracted skill directory tree into a destination directory
///
/// # Errors
///
/// Returns an error if copying fails
pub fn copy_skill_tree(source: &Path, dest: &Path) -> Result<()>
{
    if dest.exists() == true
    {
        fs::remove_dir_all(dest)?;
    }
    fs::create_dir_all(dest.parent().unwrap_or(dest))?;
    copy_dir_all(source, dest)?;
    Ok(())
}

/// Extracts a repository tarball into `staging` and returns the repo root path
///
/// # Errors
///
/// Returns an error if download or extraction fails
pub fn fetch_repo_extracted_into(owner: &str, repo: &str, branch: &str, staging: &Path) -> Result<PathBuf>
{
    let bytes = download_repo_tarball_bytes(owner, repo, branch)?;
    extract_tarball_gz(&bytes, staging)
}

/// RAII guard that clears the test hooks when dropped
///
/// Ensures hooks installed by one test do not leak into another even if the
/// test panics or returns early.
pub struct TestHookGuard
{
    _private: ()
}

impl Drop for TestHookGuard
{
    fn drop(&mut self)
    {
        LIST_CONTENTS_HOOK.with(|h| *h.borrow_mut() = None);
        DOWNLOAD_FILE_HOOK.with(|h| *h.borrow_mut() = None);
        TARBALL_HOOK.with(|h| *h.borrow_mut() = None);
    }
}

/// Install test hooks for `list_directory_contents` and `download_file`
///
/// The hooks are thread-local; each test runs on its own thread under
/// `cargo test`, so installations do not collide between tests. The returned
/// guard clears the hooks on drop, so callers must keep it alive for the
/// duration of the test.
///
/// # Arguments
///
/// * `list` - Replacement for [`list_directory_contents`]
/// * `download` - Replacement for [`download_file`] (and therefore [`download_github_file`])
pub fn set_test_hooks(list: ListContentsHook, download: DownloadFileHook) -> TestHookGuard
{
    LIST_CONTENTS_HOOK.with(|h| *h.borrow_mut() = Some(list));
    DOWNLOAD_FILE_HOOK.with(|h| *h.borrow_mut() = Some(download));
    TestHookGuard { _private: () }
}

/// Install a test hook for [`download_repo_tarball_bytes`]
pub fn set_tarball_test_hook(fetch: TarballHook) -> TestHookGuard
{
    TARBALL_HOOK.with(|h| *h.borrow_mut() = Some(fetch));
    TestHookGuard { _private: () }
}

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
    if let Some(result) = LIST_CONTENTS_HOOK.with(|h| h.borrow().as_ref().map(|hook| hook(github_url)))
    {
        return result;
    }

    let api_url = github_url.contents_api_url();
    let bytes = get_bytes_with_retry(&api_url, Some("application/vnd.github.v3+json"))?;
    let entries: Vec<GitHubContentEntry> = serde_json::from_slice(&bytes)?;
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
    let content = if let Some(result) = DOWNLOAD_FILE_HOOK.with(|h| h.borrow().as_ref().map(|hook| hook(url)))
    {
        result?
    }
    else
    {
        throttle_raw_download();
        get_bytes_with_retry(url, None)?
    };

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

    #[test]
    fn test_send_with_retry_succeeds_after_429() -> anyhow::Result<()>
    {
        clear_mock_http_statuses();
        push_mock_http_status(429);
        push_mock_http_status(200);

        let bytes = get_bytes_with_retry("https://example.com/mock", None)?;
        assert_eq!(bytes, b"mock-http-success");
        Ok(())
    }

    #[test]
    fn test_extract_tarball_gz_strips_github_root() -> anyhow::Result<()>
    {
        let tarball = build_test_github_tarball("fake-skill", b"# Fake skill\n");
        let dest = tempfile::TempDir::new()?;
        let root = extract_tarball_gz(&tarball, dest.path())?;

        assert!(root.ends_with("owner-repo-deadbeef") == true);
        assert!(root.join("fake-skill/SKILL.md").is_file() == true);
        Ok(())
    }

    #[test]
    fn test_discover_skills_in_dir_finds_skill_md() -> anyhow::Result<()>
    {
        let dir = tempfile::TempDir::new()?;
        let skill_dir = dir.path().join("skills").join("fake-skill");
        fs::create_dir_all(&skill_dir)?;
        fs::write(skill_dir.join("SKILL.md"), "# Fake skill\n")?;

        let found = discover_skills_in_dir(&dir.path().join("skills"));
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].0, "fake-skill");
        assert!(found[0].1.ends_with("fake-skill") == true);
        Ok(())
    }

    /// Builds a GitHub-style tarball with a single root directory prefix
    fn build_test_github_tarball(skill_name: &str, skill_content: &[u8]) -> Vec<u8>
    {
        use flate2::{Compression, write::GzEncoder};

        let gz = GzEncoder::new(Vec::new(), Compression::default());
        let mut builder = tar::Builder::new(gz);
        let path = format!("owner-repo-deadbeef/{}/SKILL.md", skill_name);
        let mut header = tar::Header::new_gnu();
        header.set_size(skill_content.len() as u64);
        header.set_mode(0o644);
        header.set_entry_type(tar::EntryType::Regular);
        header.set_cksum();
        builder.append_data(&mut header, &path, skill_content).expect("append skill to tarball");
        builder.finish().expect("finish tarball");
        builder.into_inner().expect("unwrap gzip encoder").finish().expect("finish gzip")
    }
}
