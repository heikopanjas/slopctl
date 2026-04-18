# Project Instructions for AI Coding Agents

**Last updated:** 2026-04-18 (v15.4.2)

<!-- {mission} -->

## Mission Statement

slopctl is a Rust CLI tool that manages coding agent instruction files (AGENTS.md, CLAUDE.md, .cursorrules, CODEX.md) across workspaces. It downloads, installs, updates, and synchronizes templates and Agent Skills for multiple AI coding assistants (Claude Code, Cursor, GitHub Copilot, Codex) following the agents.md and agentskills.io community standards.

## Technology Stack

- **Language:** Rust (Edition 2024, nightly toolchain)
- **CLI Framework:** clap v4.5 (derive API) with clap_complete for shell completions
- **HTTP:** reqwest v0.12 (blocking, json) for GitHub API and template downloads
- **Serialization:** serde + serde_yaml for templates.yml, serde_json for file tracker
- **Version Control:** Git
- **Package Manager:** Cargo
- **CI/CD:** GitHub Actions (build.yml on develop, release.yml on main)
- **License:** MIT

## Session Protocol

When starting a new session, read this entire file and confirm you have
understood the project instructions before proceeding. Summarize the project
purpose and key conventions briefly. Do not make changes until you have
confirmed your understanding.

<!-- {principles} -->

## Primary Instructions

- Avoid making assumptions. If you need additional context to accurately answer the user, ask the user for the missing information. Be specific about which context you need.
- Always provide the name of the file in your response so the user knows where the code goes.
- Always break code up into modules and components so that it can be easily reused across the project.
- All code you write MUST be fully optimized. ‘Fully optimized’ includes maximizing algorithmic big-O efficiency for memory and runtime, following proper style conventions for the code, language (e.g. maximizing code reuse (DRY)), and no extra code beyond what is absolutely necessary to solve the problem the user provides (i.e. no technical debt). If the code is not fully optimized, you will be fined $100.

### Working Together

This file (`AGENTS.md`) is the primary instructions file for AI coding assistants working on this project. Agent-specific instruction files (such as `.github/copilot-instructions.md`, `CLAUDE.md`) reference this document, maintaining a single source of truth.

When initializing a session or analyzing the workspace, refer to instruction files in this order:

1. `AGENTS.md` (this file - primary instructions and single source of truth)
2. Agent-specific reference file (if present - points back to AGENTS.md)

### Update Protocol (CRITICAL)

**PROACTIVELY update this file (`AGENTS.md`) as we work together.** Whenever you make a decision, choose a technology, establish a convention, or define a standard, you MUST update AGENTS.md immediately in the same response.

**Update ONLY this file (`AGENTS.md`)** when coding standards, conventions, or project decisions evolve. Do not modify agent-specific reference files unless the reference mechanism itself needs changes.

**When to update** (do this automatically, without being asked):

- Technology choices (build tools, languages, frameworks)
- Directory structure decisions
- Coding conventions and style guidelines
- Architecture decisions
- Naming conventions
- Build/test/deployment procedures

**How to update AGENTS.md:**

- Maintain the "Last updated" timestamp at the top
- Add content to the relevant section (Project Overview, Coding Standards, etc.)
- Add entries to the "Recent Updates & Decisions" log at the bottom with:
  - Date (with time if multiple updates per day)
  - Brief description
  - Reasoning for the change
- Preserve this structure: title header → timestamp → main instructions → "Recent Updates & Decisions" section

## Best Practices

### When Updating This Repository

1. **Maintain Consistency**: Keep code style consistent across the codebase
2. **Test First**: Write tests before implementing features when applicable
3. **Document Changes**: Update documentation when changing functionality
4. **Code Review**: Run `cargo fmt`, `cargo clippy`, and `cargo test` before committing
5. **Date Changes**: Update the "Last updated" timestamp in this file when making changes
6. **Log Updates**: Add entries to "Recent Updates & Decisions" section below

### Development Guidelines

- Use debug builds (`cargo build`) during development; reserve release builds for CI and deployment
- Branch model: `develop` is the active branch; `main` receives stable merges only
- Always use `--dry-run` to verify CLI behavior before writing destructive tests
- Keep `main.rs` thin (CLI parsing and dispatch only); business logic belongs in library modules
- One public struct or major component per source file; shared helpers go in `utils.rs`

### Security & Safety

- Never include API keys, tokens, or credentials in code
- Always require explicit human confirmation before commits
- Maintain conventional commit message standards
- Keep change history transparent through commit messages
- GitHub tokens for API access are read from environment only, never stored in config files
- Template marker detection prevents accidental overwrites of user-customized files

### Testing

Unit tests are co-located with implementation in each source file under `#[cfg(test)] mod tests`.

- Unit tests: In-file `#[cfg(test)]` modules, named `test_<scenario>_<expected_outcome>`
- Integration tests: Manual CLI verification with `--dry-run` flag
- Test serialization: Tests that call `std::env::set_current_dir` share a `CWD_LOCK` mutex to prevent race conditions
- CI runs `cargo test --verbose` on Linux, macOS, and Windows (nightly toolchain)
- Testing framework: Built-in Rust test harness with `assert!`, `assert_eq!`, `assert_ne!`

### Documentation

- Code comments: `///` doc comments on all public APIs; `//` for non-obvious implementation details only
- API documentation: Generated via `cargo doc`; doc comment structure uses `# Arguments`, `# Errors`, `# Examples` sections
- README updates: Required when adding or changing CLI commands, flags, or user-visible behavior
- Changelog: Maintained in the "Recent Updates & Decisions" section at the bottom of this file

<!-- {languages} -->

## Rust Coding Conventions

**General Principles:**

- Follow standard Rust conventions (use `rustfmt` and `clippy`)
- Use idiomatic Rust patterns throughout
- Prefer `Result<T, E>` for error handling over panics
- Apply RAII principles through Rust's ownership system
- Use const-correctness via immutable references (`&`)
- Write self-documenting code with clear naming and structure
- Leverage the type system for compile-time safety
- Keep functions focused and modular
- **DRY (Don't Repeat Yourself)**: Extract shared logic into functions, traits, or structs. When the same pattern appears in 2+ places, factor it out. Use parameter structs (e.g. `UpdateOptions`) to aggregate related arguments rather than passing many individual parameters. Prefer a single source of truth for data (e.g. `agent_defaults.rs` for agent path conventions rather than duplicating paths in config and code).

**Error Handling:**

- Use `Result<T, E>` for all fallible operations
- Use `anyhow` crate for error handling; re-export from `lib.rs`:

  ```rust
  pub use anyhow::Result;
  ```

- Use `anyhow!()` macro for constructing errors:

  ```rust
  Err(anyhow!("Config file not found"))
  Err(anyhow!("Failed to download {}: {}", url, e))
  ```

- Use `?` operator for error propagation
- Avoid `.unwrap()` in library code; only use in application entry points after proper error handling
- Use `.ok_or_else()` or `.ok_or()` to convert `Option` to `Result` with meaningful error messages
- Never panic in library code unless documenting preconditions with `#[panic]` doc comments
- Use the `require!` macro for precondition checks with early return:

  ```rust
  require!(config_file.exists() == true, Err(anyhow!("Config not found")));
  require!(name.is_empty() == false, None);
  require!(count > 0, Ok(()));
  ```

  - Syntax: `require!(condition, return_expression)`
  - Returns the expression when the condition is **false**
  - Works with any return type: `Result`, `Option`, or bare values
  - Use `require!` only for precondition checks at the **top of a function** (before any real work), mimicking design-by-contract
  - Do NOT use `require!` for conditional logic deep inside function bodies; those should remain as regular `if` blocks

**Comparison and Conditional Expressions:**

- Always use explicit boolean comparisons for clarity and consistency
- Use `== true` and `== false` instead of bare conditionals or negation
- Examples:
  - ✅ Correct: `if condition == true`, `if value == false`
  - ❌ Incorrect: `if condition`, `if !value`
- Exception: Direct variable tests in control flow are allowed when clearly intentional
- Apply to all boolean comparisons including `Option` and `Result` checks
- Use explicit comparisons with `None`: `if option_value.is_none() == true` or `if option_value == None`
- Allow clippy warnings for explicit boolean comparisons with project-level configuration

**Loop Flow Control:**

- Avoid `if condition { continue; }` guards at the top of loop bodies; they add visual noise especially with `AlwaysNextLine` brace style
- Instead, combine guard conditions with the subsequent logic using `&&`, `if/else if/else` chains, or let-chains
- Examples:
  - ❌ Incorrect:

    ```rust
    for entry in &files
    {
        if entry.is_skippable() == true
        {
            continue;
        }
        if let Some(value) = entry.process()
        {
            handle(value);
        }
    }
    ```

  - ✅ Correct:

    ```rust
    for entry in &files
    {
        if entry.is_skippable() == false &&
            let Some(value) = entry.process()
        {
            handle(value);
        }
    }
    ```

- For multi-branch dispatch, use `if/else if/else` instead of `continue` to skip to the next branch
- Exception: `continue` inside `match` error arms (log-and-skip) is acceptable since it serves as early return from an error handler, not a guard

**Module Organization:**

- Use module structure to organize code by functionality
- One public struct or major component per file
- Related utility functions in dedicated `utils.rs`
- Module declaration order in `lib.rs`:
  1. Private module declarations (`mod`)
  2. Public re-exports (`pub use`)
  3. Type aliases
- Example:

  ```rust
  mod template_manager;
  mod utils;

  pub use anyhow::Result;
  pub use template_manager::TemplateManager;
  pub use utils::copy_dir_all;
  ```

**Functions and Methods:**

- Document all public APIs with doc comments (`///`)
- Use doc comment structure:
  - Brief one-line description (no explicit `# Description` header)
  - Longer explanation if needed (separated by blank line)
  - `# Arguments` section for parameters
  - `# Returns` section for return values (when non-obvious)
  - `# Errors` section for fallible functions
  - `# Examples` section when helpful
  - `# Panics` section if function can panic
- Example:

  ```rust
  /// Creates a new TemplateManager instance
  ///
  /// Initializes paths to local data and cache directories using the `dirs` crate.
  /// Templates are stored in the local data directory and backups in the cache directory.
  ///
  /// # Errors
  ///
  /// Returns an error if the local data directory cannot be determined
  pub fn new() -> Result<Self>
  ```

- Pass by reference (`&`) for complex types, by value for `Copy` types
- Use immutable references (`&`) unless mutation is required (`&mut`)
- Keep function signatures on one line when under max width (167 chars)
- Private helper functions should have single-line doc comments when logic is non-trivial

**Structs and Types:**

- Use clear, descriptive names for all types
- Define fields in logical grouping order
- Document struct purpose and usage with doc comments
- Example:

  ```rust
  /// Manages template files for coding agent instructions
  ///
  /// The `TemplateManager` handles all operations related to template storage,
  /// verification, backup, and synchronization. Templates are stored in the
  /// local data directory and backed up to the cache directory before modifications.
  pub struct TemplateManager
  {
      config_dir: PathBuf,
      cache_dir:  PathBuf
  }
  ```

- Use `#[derive]` for common traits when appropriate
- Implement `Default` for structs with sensible defaults
- Group related structs together in the same file when tightly coupled
- Never wrap collection types in `Option`; use empty collections instead:
  - ❌ `Option<Vec<T>>`, `Option<HashMap<K,V>>` — creates redundant states (`None` vs empty)
  - ✅ `Vec<T>`, `HashMap<K,V>` — empty collection represents absence
  - For serde: use `#[serde(default, skip_serializing_if = "Vec::is_empty")]` or `"HashMap::is_empty"`
  - `Option` is appropriate for non-collection types where the default/zero value differs from absence (e.g., `Option<Config>`)
- When exposing an internal `Vec<T>` via a getter, return `&[T]` (slice) not `&Vec<T>`

**Naming Conventions:**

- Types (structs, enums, traits): Upper PascalCase (e.g., `TemplateManager`, `FileMapping`, `Result`)
- Functions/methods: snake_case (e.g., `download_file`, `create_backup`, `load_template_config`)
- Variables and function parameters: snake_case (e.g., `config_dir`, `source_path`, `file_name`)
- Constants: UPPER_SNAKE_CASE (e.g., `MAX_WIDTH`, `DEFAULT_TIMEOUT`)
- Type parameters: Single uppercase letter or PascalCase (e.g., `T`, `E`, `Error`)
- Lifetimes: Short lowercase names (e.g., `'a`, `'static`)
- Module names: snake_case (e.g., `template_manager`, `utils`)

**Enums and Pattern Matching:**

- Use descriptive variant names in PascalCase
- Derive common traits when appropriate
- Use `#[derive(Debug)]` for all types when possible for better error messages
- Use exhaustive pattern matching; avoid `_ =>` catch-alls when possible
- Use `if let` for single-pattern matching
- Use `match` for multiple patterns or when you need exhaustiveness checking
- Use `let...else` for early returns with single pattern:

  ```rust
  let Some(value) = option else {
      return Err("Missing value".into());
  };
  ```

**CLI Design with clap:**

- Use clap's derive API for argument parsing
- Define main CLI struct with `#[derive(Parser)]`
- Use `#[derive(Subcommand)]` for command structure
- Add helpful descriptions with `#[command]` attributes
- Example:

  ```rust
  #[derive(Parser)]
  #[command(name = "my-app")]
  #[command(about = "A manager for coding agent instruction files", long_about = None)]
  struct Cli
  {
      #[command(subcommand)]
      command: Commands
  }
  ```

- Use clear, descriptive field names that match CLI conventions
- Provide defaults with `#[arg(default_value = "...")]`
- Add documentation comments to show in `--help` output

**Formatting Configuration (.rustfmt.toml):**

- Use project-specific rustfmt configuration for consistency
- Key formatting rules:
  - `max_width = 167` - Allow longer lines for readability
  - `brace_style = "AlwaysNextLine"` - Opening braces on new lines
  - `control_brace_style = "AlwaysNextLine"` - Consistent brace placement
  - `trailing_comma = "Never"` - No trailing commas
  - `edition = "2024"` - Use latest Rust edition
  - `tab_spaces = 4` - Standard indentation
  - `imports_granularity = "Crate"` - Group imports by crate
  - `group_imports = "StdExternalCrate"` - Organize imports logically
- Run `cargo fmt` before committing code
- Configure editor to format on save

**Imports and Dependencies:**

- Group imports in order:
  1. Standard library (`std::`)
  2. External crates (alphabetically)
  3. Project modules (`crate::`)
- Use explicit imports over glob imports
- Example:

  ```rust
  use std::{
      fs,
      io::{self, Write},
      path::{Path, PathBuf}
  };

  use chrono::{DateTime, Utc};
  use owo_colors::OwoColorize;
  use serde::{Deserialize, Serialize};

  use crate::{Result, utils::copy_dir_all};
  ```

- Re-export commonly used items from `lib.rs` for convenience

**Conditional Compilation and Features:**

- Use feature flags for optional functionality
- Document feature requirements in doc comments
- Use `#[cfg(feature = "...")]` for conditional code
- Specify features in `Cargo.toml` dependencies when needed:

  ```toml
  reqwest = { version = "0.12", features = ["blocking", "json"] }
  ```

**Testing:**

- Write unit tests alongside implementation in the same file
- Use `#[cfg(test)]` module for tests
- Name test functions descriptively: `test_<scenario>_<expected_outcome>`
- Use `assert!`, `assert_eq!`, `assert_ne!` macros
- Test both success and error cases
- Example:

  ```rust
  #[cfg(test)]
  mod tests
  {
      use super::*;

      #[test]
      fn test_parse_github_url_valid()
      {
          // Test implementation
      }
  }
  ```

**Comments and Documentation:**

- Use `///` for public API documentation (appears in generated docs)
- Use `//!` for module-level documentation at file top
- Use `//` for implementation comments and explanations
- Document the "why" not the "what" in implementation comments
- Keep comments up-to-date with code changes
- Use full sentences with proper punctuation in doc comments
- Example:

  ```rust
  //! Template management functionality for my-app

  /// Creates a timestamped backup of a directory
  ///
  /// Backups are stored in the cache directory with timestamp: `backups/YYYY-MM-DD_HH_MM_SS/`
  fn create_backup(&self, source_dir: &Path) -> Result<()>
  {
      // Skip backup if source doesn't exist
      if source_dir.exists() == false
      {
          return Ok(());
      }
      // ... rest of implementation
  }
  ```

**Linting Configuration:**

- Allow specific clippy lints when project style differs from defaults
- Configure in `Cargo.toml`:

  ```toml
  [lints.clippy]
  bool_comparison = "allow"
  ```

- Can also use module-level attributes:

  ```rust
  #![allow(clippy::bool_comparison)]
  ```

- Document reasoning for lint exceptions

**File Organization:**

- Entry point: `src/main.rs` (minimal, delegates to library)
- Library API: `src/lib.rs` (public interface)
- Implementation: Feature modules in `src/`
- Keep `main.rs` focused on CLI handling and error reporting
- Put business logic in library modules for reusability
- Example structure:

  ```text
  src/
  ├── main.rs              # CLI entry point
  ├── lib.rs               # Public API
  ├── template_manager.rs  # Core functionality
  └── utils.rs             # Shared utilities
  ```

**Best Practices:**

- Use `std::env::current_dir()` over hardcoding paths
- Use `Path` and `PathBuf` for filesystem paths
- Use `Path::starts_with()` for path prefix/subpath checks; avoid string-based path comparison (e.g. `path.starts_with("foo/")`) to ensure cross-platform behavior (Windows uses `\`, Unix uses `/`)
- When resolving placeholders in paths (e.g. `$workspace/AGENTS.md`), use `Path::join()` with the suffix instead of string replace; string replace can produce mixed separators on Windows
- Leverage `std::io::Write` trait for flushing output buffers
- Use `owo-colors` or similar crate for terminal output styling
- Use platform-appropriate paths via `dirs` crate (prefer over `$HOME` env var)
- Implement `flush()` when printing without newline for immediate output:

  ```rust
  print!("{} Processing... ", "→".blue());
  io::stdout().flush()?;
  ```

- Use early returns to reduce nesting depth
- Prefer iterators and functional patterns over loops when clear

**Error Messages:**

- Use colored output for user-facing messages (owo-colors)
- Format: `"{} {}", symbol.color(), message.color()`
- Symbols: `✓` (success/green), `✗` (error/red), `→` (info/blue), `!` (warning/yellow), `?` (prompt/yellow)
- Provide actionable error messages
- Include file paths and operation details in errors
- Example:

  ```rust
  println!("{} Creating backup in {}", "→".blue(), backup_dir.display().to_string().yellow());
  eprintln!("{} Failed to download {}: {}", "✗".red(), url, error.to_string().red());
  ```

**Version and Edition:**

- Use Rust 2024 edition for latest language features
- Specify in `Cargo.toml`:

  ```toml
  [package]
  edition = "2024"
  ```

- Keep dependencies up-to-date but specify versions explicitly
- Use semantic versioning in package version

**Code Review Checklist:**

- [ ] All public APIs have doc comments
- [ ] Error handling uses `Result` consistently
- [ ] No `.unwrap()` calls in library code
- [ ] Explicit boolean comparisons used throughout
- [ ] Code formatted with `cargo fmt`
- [ ] No clippy warnings (or explicitly allowed with reasoning)
- [ ] Tests pass with `cargo test`
- [ ] Code builds in both debug and release modes
- [ ] Imports organized and minimal
- [ ] Functions are focused and modular

## Build Commands

### Setup

```bash
# Install Rust toolchain (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Update Rust to latest stable version
rustup update

# Install additional components (optional)
rustup component add rustfmt clippy
```

### Development

```bash
# Build the project (debug - use during development)
cargo build

# Run the application
cargo run

# Run with arguments
cargo run -- [args]

# Check code without building (faster than build)
cargo check

# Run tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_name

# Format code
cargo fmt

# Run clippy linter
cargo clippy

# Run clippy with all warnings
cargo clippy -- -W clippy::all
```

### Build & Deploy

```bash
# Build for release (optimized - use for final testing/deployment only)
cargo build --release

# Run release build
cargo run --release

# Build with verbose output
cargo build --verbose

# Clean build artifacts
cargo clean
```

### Documentation

```bash
# Generate and open project documentation
cargo doc --open

# Generate documentation for dependencies too
cargo doc --no-deps --open
```

### Dependency Management

```bash
# Update dependencies to latest compatible versions
cargo update

# Add a new dependency
cargo add <crate_name>

# Check for outdated dependencies (requires cargo-outdated)
cargo outdated

# Audit dependencies for security vulnerabilities (requires cargo-audit)
cargo audit
```

**Important**: Always use debug builds (`cargo build`) during development. Debug builds compile faster and include debugging symbols. Only use release builds (`cargo build --release`) for final testing or deployment.

<!-- {integration} -->

## Commit Protocol (CRITICAL)

- **NEVER commit automatically** - always wait for explicit confirmation

Whenever asked to commit changes:

- Stage the changes
- Write a detailed but concise commit message using conventional commits format
- Commit the changes
- Load the `git-workflow` skill for the full message format, character limits, and examples before committing

This is **CRITICAL**!

## **Commit Message Guidelines - CRITICAL**

Follow these rules to prevent VSCode terminal crashes and ensure clean git history:

**Message Format (Conventional Commits):**

```text
<type>(<scope>): <subject>

<body>

<footer>
```

**Character Limits:**

- **Subject line**: Maximum 50 characters (strict limit)
- **Body lines**: Wrap at 72 characters per line
- **Total message**: Keep under 500 characters total
- **Blank line**: Always add blank line between subject and body

**Subject Line Rules:**

- Use conventional commit types: `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`, `build`, `ci`, `perf`
- Scope is optional but recommended: `feat(api):`, `fix(build):`, `docs(readme):`
- Use imperative mood: "add feature" not "added feature"
- No period at end of subject line
- Keep concise and descriptive

**Body Rules (if needed):**

- Add blank line after subject before body
- Wrap each line at 72 characters maximum
- Explain what and why, not how
- Use bullet points (`-`) for multiple items with lowercase text after bullet
- Keep it concise

**Special Character Safety:**

- Avoid nested quotes or complex quoting
- Avoid special shell characters: `$`, `` ` ``, `!`, `\`, `|`, `&`, `;`
- Use simple punctuation only
- No emoji or unicode characters

**Best Practices:**

- **Break up large commits**: Split into smaller, focused commits with shorter messages
- **One concern per commit**: Each commit should address one specific change
- **Test before committing**: Ensure code builds and works
- **Reference issues**: Use `#123` format in footer if applicable

**Examples:**

Good:

```text
feat(api): add KStringTrim function

- add trimming function to remove whitespace from
  both ends of string
- supports all encodings
```

Good (short):

```text
fix(build): correct static library output name
```

Bad (too long):

```text
feat(api): add a new comprehensive string trimming function that handles all edge cases including UTF-8, UTF-16LE, UTF-16BE, and ANSI encodings with proper boundary checking and memory management
```

Bad (special characters):

```text
fix: update `KString` with "nested 'quotes'" & $special chars!
```

## Windows / PowerShell Guidelines

The development environment uses **PowerShell on Windows**. All shell commands executed by AI agents must use PowerShell-compatible syntax.

**Shell Syntax:**

- **Never use bash-specific constructs**: heredocs (`<<'EOF'`), `$(command)` substitution, `&&` chaining (PowerShell 7+ supports `&&` but avoid for safety)
- **Use PowerShell here-strings** for multi-line text:

  ```powershell
  @"
  multi-line
  string
  "@
  ```

- **Use multiple `-m` flags** for multi-line git commit messages:

  ```powershell
  git commit -m "subject line" -m "- body point one" -m "- body point two"
  ```

- **Use semicolons** (`;`) to chain commands, not `&&`
- **Escape rules differ**: PowerShell uses backtick (`` ` ``) as escape character, not backslash

**Path Handling:**

- Windows uses backslash (`\`) as path separator; forward slash (`/`) works in most contexts but not all
- Absolute paths require a drive letter (`C:\path`); a bare `/path` is relative to the current drive root, not an absolute path
- Use `Path::join()` and `Path::is_absolute()` in Rust code; never assume `/` prefixed paths are absolute
- In tests, use `#[cfg(windows)]` / `#[cfg(not(windows))]` when asserting platform-specific path behavior

**Line Endings:**

- Repository uses `.gitattributes` to enforce LF for Rust source files (`*.rs`)
- Be aware of CRLF vs LF differences when comparing file content or hashes

## Semantic Versioning Protocol

**AUTOMATICALLY track version changes using semantic versioning (SemVer) in Cargo.toml.**

The current version is defined in `Cargo.toml` under `[package]` section as `version = "X.Y.Z"`.

### Version Format: MAJOR.MINOR.PATCH

**When to increment:**

1. **PATCH version** (X.Y.Z → X.Y.Z+1)
   - Bug fixes and minor corrections
   - Performance improvements without API changes
   - Documentation updates
   - Internal refactoring that doesn't affect public API
   - Example: `1.0.0` → `1.0.1`

2. **MINOR version** (X.Y.Z → X.Y+1.0)
   - New features added
   - New CLI commands or options
   - New functionality that maintains backward compatibility
   - Example: `1.0.1` → `1.1.0`

3. **MAJOR version** (X.Y.Z → X+1.0.0)
   - Breaking changes to public API
   - Removal of features or commands
   - Changes that require user action or code updates
   - Incompatible CLI changes
   - Example: `1.1.0` → `2.0.0`

### Process

After making ANY code changes:

1. Determine the type of change (fix, feature, or breaking change)
2. Update the version in `Cargo.toml` accordingly
3. Include the version change in the same commit as the code change
4. Mention version bump in commit message footer if significant
5. Load the `semantic-versioning` skill for the full PATCH/MINOR/MAJOR decision rules

**Note:** Version changes should be included in the commit with the actual code changes, not as a separate commit.

---

<!-- {changelog} -->

## Recent Updates & Decisions

### 2026-04-18 (v15.4.2, protect userprofile skill directories from filesystem scans)

- Fixed `remove`, `purge`, and `list` commands scanning userprofile-based skill directories (e.g. codex `~/.codex/skills`), which picked up agent-internal files (`.system/`) and skills belonging to other workspaces
- Added `get_workspace_skill_search_dirs()` to `agent_defaults.rs`: returns only `$workspace`-prefixed skill directories, excluding `$userprofile`-based ones (currently codex)
- `remove --agent`, `remove --all`, `purge`, and `list` now use the workspace-only function for filesystem scanning; FileTracker continues to cover userprofile skills that slopctl installed
- `remove --agent` additionally checks the `$workspace` prefix before scanning an agent's skill dir
- `remove --skill` still uses `get_all_skill_search_dirs` since it targets a specific skill name (safe)
- Added 3 new tests: `get_workspace_skill_search_dirs_excludes_codex`, `remove_agent_codex_skips_userprofile_skill_scan`, `purge_skips_userprofile_skill_dir_scan`
- Version bump: 15.4.1 to 15.4.2 (PATCH - bug fix)

### 2026-04-18 (v15.4.1, fix remove command not discovering untracked agent skills)

- Fixed `remove --agent` and `remove --all` not discovering untracked/manually placed skill files in agent skill directories
- Root cause: both code paths only consulted the FileTracker for skill files, missing any skills not recorded in the tracker (e.g. manually placed or installed by other tools)
- `remove --agent <name>` now scans the agent's skill directory on the filesystem (e.g. `.cursor/skills/`) via `collect_files_recursive`, matching the approach used by `purge`
- `remove --all` now scans all agent skill directories and the cross-client `.agents/skills/` directory via `get_all_skill_search_dirs`, same as `purge`
- FileTracker sweep is retained as a supplement to catch tracked skill files outside standard directory trees
- `purge` command was already correct (filesystem scan was present since v12.3.2)
- Added 2 new tests: `test_remove_agent_discovers_untracked_skill_files`, `test_remove_all_discovers_untracked_skill_files`
- Version bump: 15.4.0 to 15.4.1 (PATCH - bug fix)

### 2026-04-18 (v15.4.0, merge command optimization: streaming, changelog marker, partial recovery)

- Added `<!-- {changelog} -->` marker to AGENTS.md template and user file to split template-managed content from user-owned changelog
- `classify_files()` now splits at the changelog marker: only the template half is compared, so changelog-only diffs are classified as `Unchanged` (no LLM call)
- When the template half differs, only it is sent to the LLM; the user's changelog is re-attached verbatim after merge, preventing truncation
- Replaced blocking `chat()` with streaming `chat_stream()` on `LlmClient`: tokens arrive incrementally via SSE
- Per-provider SSE parsing: OpenAI/Mistral (`data: {...}` lines with `stream_options.include_usage`), Anthropic (`content_block_delta`/`message_delta` events), Ollama (newline-delimited JSON with `done: true`)
- `chat()` reimplemented as a thin wrapper around `chat_stream()` with a no-op callback (DRY)
- Added `max_tokens: 32768` to all providers (was missing for OpenAI/Mistral, was 16384 for Anthropic)
- Live progress display during merge: character count and elapsed time updated on each streaming chunk
- Partial file recovery: `.partial` sidecar written during streaming; on success it is deleted, on error or truncation it is preserved for user inspection
- Truncation detection: if `stop_reason` indicates `max_tokens`/`length`, the `.partial` file is kept and the target is not overwritten
- Hoisted `LlmClient` construction out of the per-file loop; single client reuses TCP connection pool across all diverged files
- Added 7 new tests: `partial_path`, `split_at_changelog` (present/absent), `reassemble` (with/without changelog), `classify_files_changelog_only_diff_is_unchanged`, `classify_files_template_half_differs_is_diverged`
- Version bump: 15.3.0 to 15.4.0 (MINOR - streaming LLM, changelog marker optimization, partial recovery)

### 2026-04-18 (v15.3.0, merge command redesign: DRY shared pipeline)

- Redesigned merge command to follow the same file-resolution pipeline as init (DRY)
- Extracted `resolve_all_files()` from `TemplateEngine::update()` into a reusable public method; both init and merge now call it
- Added `ResolvedFiles` struct grouping `TemplateContext`, files-to-copy, and directories-to-create
- Added `build_target_content_map()` on `TemplateEngine` that resolves all files and reads sources into `HashMap<PathBuf, String>`
- Moved `generate_fresh_main()` from merge.rs into template_engine.rs as an associated function (pure template logic)
- Refactored `merge_fragments()` to delegate content generation to `generate_fresh_main()`, eliminating duplicate fragment-merging code
- Added `normalize_path()` as a public function in template_engine.rs
- Merge now classifies files into three categories: New (write directly), Unchanged (skip), Diverged (LLM merge)
- New files are created without LLM involvement; only diverged files are sent to the AI
- LLM provider resolution is deferred until diverged files are actually found (no API key needed for new-only merges)
- Deleted ~400 lines of duplicated code from merge.rs: build_target_source_map, find_merge_candidates, generate_fresh_main, insert_source_content, insert_skill_sources, insert_skill_dir_recursive, resolve_target, normalize_path, sha256_string
- Added `categorize_path()` helper for FileTracker category detection during merge
- Added 8 new tests: classify_files (new, unchanged, diverged, mixed, sorted), categorize_path (main, skill, integration, agent, language), plural helper
- All written files (New + Diverged) are now recorded in FileTracker during merge
- Version bump: 15.2.0 to 15.3.0 (MINOR - behavioral redesign of merge command)

### 2026-04-18 (v15.2.0, init/merge redesign: AI-free init, AI-powered merge)

- Removed --smart flag from init command: init is now pure template installation with no AI involvement; users resolve conflicts manually
- Removed --provider and --model CLI flags from merge command: provider/model are resolved only from config (merge.provider, merge.model) and environment API keys (ANTHROPIC_API_KEY, OPENAI_API_KEY, MISTRAL_API_KEY)
- Added --lang, --agent, --mission, --skill options to merge command (mirror init's options): these override the auto-detected installed language, agent, and allow specifying a custom mission or extra skills when generating the fresh template for AI comparison
- Introduced MergeOptions<'a> struct in src/template_manager/merge.rs grouping lang/agent/mission/skills; re-exported via template_manager::MergeOptions and slopctl::MergeOptions
- generate_fresh_main now accepts a mission override that takes precedence over template-defined mission fragments
- build_target_source_map now accepts MergeOptions; lang override falls back to tracker.get_installed_language_for_workspace(); agent override falls back to agent_defaults::detect_all_installed_agents(); extra --skill sources are added to the target→content map (URL-based sources skipped)
- Removed generate_smart_mission from smart.rs (no longer needed); smart.rs now only contains smart_doctor + parse_smart_issues + SmartIssueKind/SmartIssue
- resolve_provider_and_model simplified to take no arguments (priority: config merge.provider > env auto-detect > error)
- collect_workspace_context and its four tests removed (no production callers after generate_smart_mission removal)
- Updated doctor.rs to call smart_doctor() without arguments
- Version bump: 15.1.0 to 15.2.0 (MINOR - behavioural change to merge, CLI flag removals from init and merge, new CLI flags on merge)

### 2026-04-18 (v15.1.0, smart features for init and doctor)

- Added --smart flag to init command: generates mission statement from workspace context using LLM
- Added --smart flag to doctor command: AI-assisted linting of AGENTS.md for contradictions, stale references, and unclear instructions
- New src/template_manager/smart.rs: collect_workspace_context, generate_smart_mission, smart_doctor, parse_smart_issues
- collect_workspace_context gathers directory listing, README.md (2000 chars), and project manifest (500 chars)
- Smart mission generation uses LLM to produce a 2-4 sentence mission paragraph; skipped in dry-run mode
- Smart doctor sends AGENTS.md to LLM and parses JSON array of issues with three kinds: contradiction, stale_reference, unclear_instruction
- JSON parsing is tolerant of surrounding prose in LLM responses
- init --smart and init --mission are mutually exclusive (enforced by clap conflicts_with)
- Provider/model resolution reuses merge command priority chain: CLI > config merge.provider/merge.model > env auto-detect
- resolve_provider_and_model changed to pub(super) for access from smart.rs sibling module
- doctor() restructured to remove early returns; smart analysis always runs at the end when --smart is set
- Added 8 new tests in smart.rs covering context collection, JSON parsing, edge cases
- Version bump: 15.0.0 to 15.1.0 (MINOR - new CLI flags)

### 2026-04-18 (v15.0.0, rebrand to slopctl)

- MAJOR version bump: 14.0.0 to 15.0.0 (breaking: binary, config paths, data paths all renamed)
- Renamed tool from slopcop to slopctl across entire codebase
- Binary name: slopcop to slopctl
- Config path: ~/.config/slopcop/ to ~/.config/slopctl/
- Data path: ~/.local/share/slopcop/templates to ~/.local/share/slopctl/templates
- Template marker: SLOPCOP-TEMPLATE to SLOPCTL-TEMPLATE
- User-Agent header: slopcop to slopctl
- Default template source URL updated to heikopanjas/slopctl (pending GitHub repo rename)
- Updated all CLI help text, error messages, and user-facing strings
- Updated CI workflows (build.yml, release.yml) artifact names
- Updated README.md, ROADMAP.md, and templates/v5/AGENTS.md with new tool name

### 2026-04-18 (v14.0.0, rebrand to slopcop)

- MAJOR version bump: 13.3.0 to 14.0.0 (breaking: binary, config paths, data paths all renamed)
- Renamed tool from vibe-cop to slopcop across entire codebase
- Binary name: vibe-cop to slopcop
- Config path: ~/.config/vibe-cop/ to ~/.config/slopcop/
- Data path: ~/.local/share/vibe-cop/templates to ~/.local/share/slopcop/templates
- Template marker: VIBE-COP-TEMPLATE to SLOPCOP-TEMPLATE
- User-Agent header: vibe-cop to slopcop
- Default template source URL updated to heikopanjas/slopcop (pending GitHub repo rename)
- Updated all CLI help text, error messages, and user-facing strings
- Updated CI workflows (build.yml, release.yml) artifact names
- Updated README.md, ROADMAP.md, and templates/v5/AGENTS.md with new tool name

### 2026-04-10 (v13.3.0, merge --verbose token usage)

- Added `--verbose` (`-v`) flag to the `merge` command for token usage reporting
- New `ChatResponse` struct in `llm.rs` returning content, input/output token counts, and stop reason
- Updated `chat()`, `chat_openai_compatible()`, `chat_anthropic()` to return `ChatResponse` instead of `String`
- Token extraction: Anthropic `usage.input_tokens`/`usage.output_tokens`/`stop_reason`; OpenAI/Mistral `usage.prompt_tokens`/`usage.completion_tokens`/`choices[0].finish_reason`; Ollama `prompt_eval_count`/`eval_count`/`done_reason`
- Merge accumulates token counts across all files; prints summary with `--verbose`
- Detects `max_tokens`/`length` stop reasons and warns about truncated files
- Added `accumulate_usage()` and `format_number()` helpers with 5 new tests
- Version bump: 13.2.3 to 13.3.0 (MINOR - new CLI flag)

### 2026-04-10 (v13.2.3, merge writes to original by default)

- Changed `merge` default behavior: merged content now replaces the original file directly
- Previously, `merge` always wrote `.merged` sidecar files requiring manual review
- Added `--preview` flag to opt into the old sidecar behavior when manual review is desired
- Added HTTP timeouts to the LLM client: 30s connect, 5min read (prevents OS-level TCP timeout killing large requests)
- Updated README.md merge command documentation with examples and new `--preview`/`--list-models` flags
- Version bump: 13.2.2 to 13.2.3 (PATCH - behavioral fix)

### 2026-04-10 (v13.2.2, fix merge command ignoring skills)

- Fixed `merge` command not detecting changes in skill files (e.g. git-workflow)
- Root cause: `find_merge_candidates()` explicitly skipped `category == "skill"` entries, and `build_target_source_map()` never included skill files
- Removed skill exclusion from `find_merge_candidates()`; skills are now treated like any other tracked file
- Added skill file mapping to `build_target_source_map()`: walks local skill source directories for top-level, agent, and language skills and maps each file to its installed workspace target
- Uses `detect_all_installed_agents()` to cover multi-agent workspaces; top-level skills mapped to each agent's skill dir, falling back to cross-client dir when no agents detected
- Added `insert_skill_sources()` helper: resolves skill base dir from placeholder, iterates skill definitions, skips URL-based sources (not cached locally)
- Added `insert_skill_dir_recursive()` helper: recursively reads skill source directories and inserts file content into the target-source map
- Added 4 new tests: recursive dir mapping, subdirectory handling, URL skip, local skill inclusion
- Fixed merge ignoring untracked files (e.g. AGENTS.md customized before tracking, or skipped by init); merge now also checks files from the target-source map that exist on disk but are not in the FileTracker
- For untracked files, merge compares current file SHA against template SHA directly (no original_sha needed)
- Known limitation: ad-hoc CLI skills (`--skill user/repo`) have no template source in templates.yml, so they cannot be merge candidates
- Version bump: 13.2.1 to 13.2.2 (PATCH - bug fix)

### 2026-04-10 (v13.2.1, config CLI option rename)

- Replaced positional `<key> <value>` set syntax with `--add <key> <value>` (`-a`) flag
- Renamed `--unset` (`-u`) to `--remove` (`-r`) for consistency
- Updated `handle_config` function, CLI struct, dispatch, and inline help text
- Updated README.md config command documentation and examples
- Version bump: 13.2.0 to 13.2.1 (PATCH - CLI option rename)

### 2026-04-10 (v13.2.0, merge --list-models)

- Added `--list-models` (`-L`) flag to the `merge` command for querying available models from the selected LLM provider
- Added `Provider::models_endpoint()` returning the model listing URL per provider
  - OpenAI: `GET /v1/models` (Bearer auth, `data[].id`)
  - Anthropic: `GET /v1/models` (`x-api-key` + `anthropic-version` headers, `data[].id`)
  - Mistral: `GET /v1/models` (Bearer auth, `data[].id`)
  - Ollama: `GET /api/tags` (no auth, `models[].name`)
- Added `LlmClient::list_models()` with three parsing paths: OpenAI-compatible, Anthropic, and Ollama
- Added `TemplateManager::list_models()` encapsulating provider resolution and API call
- Output lists models alphabetically; the active default model is marked with `(default)`
- Added `test_provider_models_endpoint` unit test
- Version bump: 13.1.0 to 13.2.0 (MINOR - new CLI flag)

### 2026-04-10 (v13.1.0, AI-assisted merge command)

- Implemented `merge` command: LLM-assisted merge of customized workspace files with updated templates
- New `src/llm.rs`: LLM provider abstraction supporting OpenAI, Anthropic, Ollama, and Mistral
- Auto-detection of LLM provider from environment API key variables
  - Priority: CLI `--provider` > config `merge.provider` > env auto-detect > error
  - Checks `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, `MISTRAL_API_KEY` in order
  - Ollama not auto-detected (requires no key, would always match)
  - `Provider` enum with `from_name()`, `default_model()`, `endpoint()`, API key env var lookup
  - `LlmClient` struct with `chat()` method; OpenAI-compatible path for OpenAI/Ollama/Mistral, dedicated Anthropic Messages API path
  - API keys read from environment variables (`OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, `MISTRAL_API_KEY`); Ollama requires no key
  - Temperature fixed at 0.0 for deterministic merge output
- New `src/template_manager/merge.rs`: merge command logic
  - Finds merge candidates: tracked files that are both user-modified AND have updated template sources
  - Generates fresh AGENTS.md by re-merging base template with all fragments (principles, mission, languages, integration)
  - Builds target-to-source map from templates.yml for non-main files (agent, language, integration entries)
  - Writes `.merged` sidecar files; skips if sidecar already exists
  - Dry-run mode shows candidates without calling the LLM
- Extended `Config` with `merge.provider` and `merge.model` keys
  - New `MergeConfig` struct in `config.rs` with `provider` and `model` fields
  - Priority: CLI `--provider`/`--model` > config values > error (provider required)
- Added `resolve_target()` public method on `TemplateEngine` (exposes placeholder resolution for merge module)
- Wired merge dispatch in `main.rs`: passes CLI args to `TemplateManager::merge()`
- Added 21 new tests across `llm.rs`, `merge.rs`, and `config.rs`
- Version bump: 13.0.0 to 13.1.0 (MINOR - new feature)

### 2026-04-10 (v13.0.0, Codex modernization, CLI rename, merge skeleton)

- MAJOR version bump: 12.4.0 to 13.0.0 (breaking CLI rename, template changes)
- Renamed `install` command to `init` across CLI, source, README, and user-facing strings
- Removed stale Codex template files: `codex/CODEX.md` and `codex/prompts/init-session.md`
- Codex reads AGENTS.md natively; redirect file and prompt are no longer needed
- Codex agent entry in templates.yml is now empty (`codex: {}`)
- Added Session Protocol section to AGENTS.md template for agents that read it directly
- Added `merge` command skeleton with `--provider`, `--model`, `--dry-run` flags (not yet implemented)
- Updated README.md: migration guide v12-to-v13, removed Codex directory from repo tree, added merge command docs
- Updated ROADMAP.md with completed items and future considerations

### 2026-04-10 (v12.4.0, CLI rework: templates and status commands)

- Replaced `update` command with `templates` command that consolidates global template management
- `templates --update` downloads/updates global templates (replaces `slopctl update`)
- `templates --list` shows available agents, languages, and skills (replaces `slopctl list --global`)
- Both flags can be combined: `templates --update --list` updates then shows the catalog
- Running `slopctl templates` with neither flag prints an error with usage examples
- `--from` and `--dry-run` now require `--update` (enforced by clap)
- Renamed `list` command back to `status` (workspace-only; `--global` moved to `templates --list`)
- Made `list_global()` public on `TemplateManager` for use by the templates command path
- Updated all user-facing strings referencing `slopctl update` and `slopctl list` across codebase
- Updated README.md CLI docs and examples
- Version bump: 12.3.4 to 12.4.0 (MINOR - new commands, removed old commands)

### 2026-04-10 (docs update, CI badge and AGENTS.md placeholders)

- Added CI status badge to README.md linking to the Build and Test workflow on develop branch
- Updated README.md footer version from v12.2.0 to v12.3.4
- Filled in all placeholder sections in the project AGENTS.md with actual project information:
  - Mission Statement: describes slopctl purpose and supported agents
  - Technology Stack: Rust 2024, clap, reqwest, serde, GitHub Actions CI/CD, MIT license
  - Development Guidelines: debug builds, branch model, dry-run testing, module organization
  - Code Review: cargo fmt, clippy, and test before committing
  - Security: environment-only token access, template marker protection
  - Testing: in-file unit tests, CWD_LOCK mutex, cross-platform CI
  - Documentation: doc comments, cargo doc, README update policy, changelog location

### 2026-04-09 (v12.3.4, fix skill directories not downloaded during update)

- Fixed `download_templates_from_url` not downloading skill directories from the remote repository
- Root cause: download loops processed `files` entries for all sections but never iterated over `skills` entries
- Local-path skills (e.g. `skills/rust-coding-conventions`) were missing from the global template cache after `update`, causing "Skill source not found" errors at install time
- Added `collect_local_skill_sources()`: gathers deduplicated local-path skill sources from all config sections (top-level, agents, languages, shared); skips URL-based skills (fetched at install time)
- Added `download_skill_directory()`: uses GitHub Contents API to download a skill directory tree into the global cache, preserving directory structure
- Added `download_skill_entries()`: recursive helper for downloading nested skill directory contents
- Added `Default` derive to `AgentConfig`, `LanguageConfig`, `SharedConfig` for test ergonomics
- Added 5 unit tests: empty config, top-level skills, URL filtering, deduplication, all-sections collection
- Version bump: 12.3.3 to 12.3.4 (PATCH - bug fix)

### 2026-04-09 (v12.3.3, clippy collapsible-if fix)

- Collapsed nested `if let` into a single let-chain in `template_engine.rs` update() agent config block
- Resolves clippy `collapsible_if` warning using idiomatic Rust 2024 let-chain syntax
- Version bump: 12.3.2 to 12.3.3 (PATCH - lint fix)

### 2026-04-07 (v12.3.2, fix purge path dedup and skill discovery)

- Fixed `purge` and `remove --all` attempting to delete the same file twice when it appeared in both BoM (relative path) and FileTracker (absolute path)
- Root cause: BoM returns relative paths (e.g. `./.cursorrules`), FileTracker returns absolute paths; `sort()` + `dedup()` could not reconcile the two formats
- BoM-sourced paths are now canonicalized to absolute paths before collection in both `purge.rs` and `remove.rs` (`--agent` and `--all` branches)
- Added filesystem skill directory scanning to `purge` via `get_all_skill_search_dirs` + `collect_files_recursive`, matching the behavior already present in `list` (v12.2.1) and `remove --skill` (v11.6.0)
- Previously, manually placed or untracked skills survived `purge`; they are now discovered and removed
- Added 3 new purge tests: empty workspace dry-run, BoM/tracker deduplication, untracked skill discovery
- Consolidated CWD_LOCK mutex from per-module statics in purge and remove tests to a single shared static in `template_manager/mod.rs`, preventing test race conditions on `set_current_dir`
- Version bump: 12.3.1 to 12.3.2 (PATCH - bug fix)

### 2026-04-06 (v12.3.1, remove falls back to FileTracker)

- Made `remove --agent` and `remove --lang` fall back to FileTracker when the entry no longer exists in templates.yml
- Previously, removing an agent/language that was deleted from templates.yml after installation returned a hard error, orphaning installed files
- `remove --agent`: tries BoM first; if agent not found, queries FileTracker for category "agent" entries filtered by `path_belongs_to_agent`
- `remove --lang`: tries templates.yml first; if language not found, queries FileTracker for entries where `meta.lang` matches, excluding "main" and "skill" categories
- `remove --all`: now supplements BoM agent files with FileTracker-tracked agent entries, catching agents no longer in BoM
- Removed hard `require!(config_file.exists())` guards; BoM loading is now optional/graceful
- Renamed test `test_remove_lang_unknown_errors` to `test_remove_lang_unknown_no_error` (no longer an error)
- Added 3 new tests: agent fallback to tracker, lang fallback to tracker, lang fallback excludes main/skill
- Added CWD_LOCK mutex for test serialization (process-global `set_current_dir` safety)
- Version bump: 12.3.0 to 12.3.1 (PATCH - bug fix)

### 2026-04-06 (v12.3.0, validate agent and language before install)

- Added early validation in `TemplateEngine::update()` that rejects unknown agent/language names before any work begins
- Previously, unknown agent printed a soft warning and continued the install (main template, principles, integration still installed)
- Unknown language was caught by `resolve_language_files` but only after principles/mission had already been processed
- Both now fail immediately after loading templates.yml, listing available names in the error message
- Removed the unreachable soft-warning `else` branch for unknown agents
- Error format matches existing `remove --lang` pattern with newline-separated available list
- Added 4 new tests: unknown agent rejected, unknown language rejected, known agent accepted, known language accepted
- Version bump: 12.2.1 to 12.3.0 (MINOR - new validation behavior)

### 2026-04-06 (v12.2.1, filesystem skill detection in list)

- Enhanced `list` (workspace mode) to discover installed skills by scanning agent skill directories on disk
- Previously relied solely on FileTracker, missing manually placed or externally installed skills
- Uses `agent_defaults::get_all_skill_search_dirs` to find all skill directories for installed agents + cross-client dir
- Enumerates subdirectories in each skill dir; merges with FileTracker entries as fallback
- Added "No skills installed" message when no skills found (consistent with agents/language sections)
- Fixed `continue` guard in skill detection loop to use combined conditions per coding conventions
- Version bump: 12.2.0 to 12.2.1 (PATCH - improved skill detection)

### 2026-04-06 (v12.2.0, agent directories)

- Added `DirectoryEntry` struct with `target` field in `bom.rs`
- Added `directories: Vec<DirectoryEntry>` to `AgentConfig` for agent-declared workspace directories
- Agent directories created with `create_dir_all` during install
- Dry-run output now shows directories that would be created
- Updated `templates/v5/templates.yml`: cursor agent gets `directories` with `.cursor/plans`
- Added 3 new tests: DirectoryEntry serde, AgentConfig directories defaults and parsing
- Version bump: 12.1.1 to 12.2.0 (MINOR - new feature)

### 2026-04-06 (v12.1.1, fix workspace scoping in FileTracker)

- Fixed `list`, `purge`, `remove`, and `doctor` commands matching files from other workspaces when run from a parent directory (e.g. home)
- Root cause: `get_workspace_entries` used `Path::starts_with` prefix matching, so `/Users/me` matched entries from `/Users/me/projects/app/`
- Added `workspace: Option<String>` field to `FileMetadata` to record the exact workspace root at install time
- Changed `record_installation()` to accept a `workspace: &Path` parameter and store its canonicalized path
- Changed `get_workspace_entries()`, `get_workspace_entries_by_category()`, and `get_installed_language_for_workspace()` to exact-match on `meta.workspace` instead of `starts_with`
- Legacy entries without workspace field (`None`) are silently skipped; they get a workspace assigned on next `install` or `update`
- `workspace` field uses `serde(default, skip_serializing_if)` for backwards-compatible JSON serialization
- Added 2 new tests: parent dir exclusion and legacy entry handling
- Version bump: 12.1.0 to 12.1.1 (PATCH - bug fix)

### 2026-04-06 (v12.1.0, merge list and status commands)

- Removed `status` command; `list` now serves both roles
- `list` (default): shows workspace state (global templates, AGENTS.md, installed agents/language/skills, managed files with `-v`)
- `list --global` / `list -g`: shows available template catalog (agents, languages, skills from templates.yml)
- Added `--global` (`-g`) and `--verbose` (`-v`) flags to `List` command
- Enhanced `--global` view to show language/shared-group skill names inline under each language
- Deleted `src/template_manager/status.rs`; moved logic into `list.rs` split across `list_workspace()` and `list_global()` helpers
- Updated README.md references from `slopctl status` to `slopctl list`
- Version bump: 12.0.0 to 12.1.0 (MINOR - merged commands, new --global flag)

### 2026-04-06 (v12.0.0, template version V5)

- MAJOR version bump: 11.11.0 to 12.0.0 (template format version change)
- Bumped template version from 4 to 5
- Renamed `templates/v4` directory to `templates/v5`
- Updated default source URL, migration error message, and accepted version range (2..=5)
- Updated `default_version()` return value and all related tests
- Updated all V4/v4 references in README.md to V5/v5
- No migration path from V4 provided

### 2026-04-06 (v11.11.0, short CLI options)

- Added short option aliases to all CLI flags across every command
- install: `-l` (lang), `-a` (agent), `-m` (mission), `-s` (skill), `-f` (force), `-n` (dry-run)
- update: `-f` (from), `-n` (dry-run)
- purge: `-f` (force), `-n` (dry-run)
- remove: `-a` (agent), `-l` (lang), `-s` (skill), `-f` (force), `-n` (dry-run); `--all` stays long-only (dangerous operation)
- doctor: `-f` (fix), `-n` (dry-run), `-v` (verbose)
- status: `-v` (verbose)
- config: `-l` (list), `-u` (unset)
- Version bump: 11.10.0 to 11.11.0 (MINOR - new CLI ergonomics)

### 2026-04-06 (v11.10.0, status --verbose flag)

- Managed file list in `status` command is now hidden by default; shown only with `--verbose`
- Added `--verbose` flag to `Status` variant in `main.rs`
- Changed `status()` signature to accept `verbose: bool`
- Moved managed file collection and display into a `verbose`-gated block
- Separated agent detection from file collection to avoid dead writes when non-verbose
- Version bump: 11.9.0 to 11.10.0 (MINOR - new CLI option)

### 2026-04-03 (templates-to-skills migration)

- Converted all coding-convention files to skills to reduce AGENTS.md context pressure
- Created `templates/v4/skills/` with 10 skill directories: `rust-coding-conventions`, `c-coding-conventions`, `c++-coding-conventions`, `swift-coding-conventions`, `rust-build-commands`, `cmake-build-commands`, `swift-build-commands`, `make-build-commands`, `git-workflow`, `semantic-versioning`
- Removed all `$instructions` entries for coding-conventions and build-commands from `templates.yml`; replaced with `skills:` entries on their respective languages and shared groups
- For `git-workflow` and `semantic-versioning` (Tier 3): kept slim anchor fragments (`git-workflow-summary.md`, `semantic-versioning-summary.md`) as `$instructions`; added full-detail skills to top-level `skills:` section
- Added `make-build-commands` skill to top-level `skills:` section (was not previously in templates.yml)
- AGENTS.md template now receives no language convention blocks; agents load conventions on demand via the skill system
- Context window savings: ~81 KB from coding conventions alone removed from AGENTS.md per install

### 2026-04-02 (v11.9.0, language-to-language skill propagation)

- Changed `resolve_language_skills` to propagate skills from included *languages* in addition to shared groups
- Extracted `resolve_language_skills_inner` recursive helper with cycle detection (mirrors `resolve_language_files_inner`)
- A language that `includes` another language now inherits its skills depth-first; own skills always come last
- Updated test `test_resolve_language_skills_no_inherit_from_language` → `test_resolve_language_skills_inherit_from_language` to assert propagation
- Added `test_resolve_language_skills_multilevel_language_inherit` (3-level chain) and `test_resolve_language_skills_cycle_detection`
- Updated README.md and AGENTS.md to reflect new behaviour
- Version bump: 11.8.0 to 11.9.0 (MINOR - new skill propagation feature)

### 2026-04-02 (v11.8.0, remove --lang flag)

- Added `--lang <name>` option to the `remove` subcommand
- Resolves the language's full file list via `bom::resolve_language_files` (honours `includes` chains)
- Skips `$instructions` fragments (merged into AGENTS.md) and `$userprofile` paths (user-global)
- Validates that the given language name exists in templates.yml; reports available languages on error
- `--all` is now mutually exclusive with both `--agent` and `--lang`
- Made `BillOfMaterials::resolve_workspace_path` public for reuse in remove.rs
- Added 4 new tests: instructions/userprofile skipping, workspace path resolution, unknown lang error
- Version bump: 11.7.0 to 11.8.0 (MINOR - new flag)

### 2026-04-02 (v11.7.0, doctor command)

- Added `doctor` subcommand to check workspace for stale or broken managed files
- Detects three issue categories: missing files (tracker entry exists, file deleted), unmerged templates (main file still has TEMPLATE_MARKER), and modified files (SHA changed since install, informational)
- Added `--fix` flag: prunes stale FileTracker entries for missing files and strips TEMPLATE_MARKER from unmerged files
- Added `--dry-run` flag: shows what would be fixed without applying changes; works independently of `--fix`
- Created `src/template_manager/doctor.rs` with `DoctorIssue`, `IssueKind`, `collect_issues`, `fix_unmerged_template`, and `doctor()` on `TemplateManager`
- Added `Doctor { fix, dry_run }` variant to `Commands` enum in `main.rs` with dispatch arm
- Added 5 unit tests covering: marker stripping, content preservation, empty tracker, missing file detection, unmerged template detection
- Version bump: 11.6.1 to 11.7.0 (MINOR - new command)

### 2026-04-02 (v11.6.1, fix phantom skills in status)

- Fixed `status` reporting phantom skill names (e.g. `.agents`) from stale FileTracker entries whose files no longer exist on disk
- `status` now filters both skill name extraction and managed file listing by `path.exists()`
- Fixed `remove --skill <name>` to silently prune stale tracker entries (tracked but missing from disk) for the given skill name; previously stale entries were never cleaned up because `path.exists()` was false
- Stale entries are cleaned whether or not real files are also being removed: when there are no real files to delete, the stale entries are pruned directly; when there ARE real files, stale entries are pruned alongside them
- Version bump: 11.6.0 to 11.6.1 (PATCH - bug fixes)

### 2026-04-02 (v11.6.0, multi-agent skill remove)

- Changed `remove --skill <name>` to scan the filesystem in every installed agent's skill directory AND the cross-client `.agents/skills/` directory
- Previously only the FileTracker was consulted, missing untracked or manually placed skill files
- Added `resolve_placeholder_path()` free function to `agent_defaults.rs` (extracts logic from the private `TemplateEngine::resolve_placeholder` method)
- Added `get_all_skill_search_dirs(workspace, userprofile) -> Vec<PathBuf>` to `agent_defaults.rs`: returns agent skill dirs for all detected agents + cross-client dir, deduplicated
- Added `collect_files_recursive(dir, files)` to `utils.rs` and exported from `lib.rs`
- Updated `remove.rs` skill block: for each skill name, scans `<search_dir>/<skill_name>/` on disk in all candidate dirs; falls back to FileTracker sweep for any tracked files outside standard dirs; deduplicates before removal
- Added 7 new tests: `resolve_placeholder_path` (workspace, userprofile, literal), `get_all_skill_search_dirs` (no agents, with agent), `collect_files_recursive` (flat, nested)
- Version bump: 11.5.0 to 11.6.0 (MINOR - new multi-agent skill removal)

### 2026-04-02 (v11.5.0, multi-agent skill install)

- Changed `install --skill` (without `--agent`) to detect all installed agents in the workspace
- Skills are now installed into each detected agent's skill directory (e.g. `.cursor/skills/`, `.claude/skills/`)
- Falls back to cross-client `.agents/skills/` only when no agents are detected
- Added `detect_all_installed_agents()` to `agent_defaults.rs`; returns all agents whose instruction files exist (vs. `detect_installed_agent` which returns only the first)
- Updated `install_skills_only()` in `template_engine.rs` to iterate over all detected agent skill dirs
- Updated skill-only log message in `main.rs` from "to cross-client directory" to generic "Installing skills"
- Added 3 new tests: `test_detect_all_installed_agents_none`, `_single`, `_multiple`
- Version bump: 11.4.0 to 11.5.0 (MINOR - new multi-agent skill install behavior)

### 2026-03-26 (v11.4.0, shared group skills propagation)

- Introduced `SharedConfig` struct in `bom.rs` with `files` and `skills` fields
- Changed `TemplateConfig.shared` from `HashMap<String, Vec<FileMapping>>` to `HashMap<String, SharedConfig>`
- Skills defined on shared groups propagate to including languages via `includes`
- Added `resolve_language_skills()` function: collects language own skills + shared group skills
- Skills from included *languages* are still NOT propagated (only shared groups)
- Updated `template_engine.rs` to use `resolve_language_skills` for language skill install
- Updated `download_manager.rs` shared download loop for new `SharedConfig` format
- Updated `list.rs` to use `resolve_language_skills` for accurate skill counts
- Updated `templates/v4/templates.yml` shared section to use `files:` key
- Added `make_shared()` test helper in `bom.rs`
- Added tests: `SharedConfig` serde, `resolve_language_skills` (own, shared, combined, no-inherit, not-found)
- Version bump: 11.3.0 to 11.4.0 (MINOR - shared skills feature)

### 2026-03-26 (v11.3.0, V4 template format with agent/language skills)

- Bumped template version from 3 to 4 (V4 format)
- Renamed `templates/v3` directory to `templates/v4`; updated default source URL
- Added `skills: Vec<SkillDefinition>` to `LanguageConfig` for language-associated skills
- Changed `AgentConfig.skills` from `Vec<FileMapping>` to `Vec<SkillDefinition>` (name+source instead of source+target)
- Agent skills install to agent-specific directory (e.g. `.cursor/skills/`)
- Language skills install to cross-client `.agents/skills/` directory; NOT inherited via `includes`
- Top-level and ad-hoc skills continue using existing logic (agent dir if agent specified, cross-client otherwise)
- Removed agent skills from `BillOfMaterials` file chain (skills tracked via `FileTracker` instead)
- Removed agent skills from `download_templates_from_url` (skills resolved at install time, not during update)
- Updated `list` command to show language skill counts alongside agent skill counts
- Updated version match in `update.rs` to accept `2..=4`
- Added `make_lang()` test helper in `bom.rs` for concise `LanguageConfig` construction
- Added tests for `LanguageConfig` skills serde, `AgentConfig` skills as `SkillDefinition`, and full V4 round-trip
- Updated all version references from 3 to 4 across tests and source
- Version bump: 11.2.0 to 11.3.0 (MINOR - V4 template format)

### 2026-03-26 (v11.2.0, independent skill installation)

- Made `--skill` independent from `--lang` and `--agent` in the `install` command
- `--skill` alone now installs skills without requiring global templates, AGENTS.md, or an agent
- Skills installed without `--agent` go to the cross-client `$workspace/.agents/skills/` directory per the agentskills.io specification
- Skills installed with `--agent` continue using the agent-specific path (e.g. `.cursor/skills/`)
- When `--skill` is used with `--lang` (no agent), skills also use the cross-client path
- Added `CROSS_CLIENT_SKILL_DIR` constant in `agent_defaults.rs`
- Added `install_skills_only()` method to `TemplateEngine` for standalone skill installation
- Added `install_skills()` method to `TemplateManager` for skill-only routing
- Refactored `install_skills()` to accept `skill_base_dir` directly instead of deriving from agent name (DRY)
- Refactored `copy_files_with_tracking()` to accept `template_version: u32` instead of `&TemplateContext`
- Extracted `resolve_adhoc_skills()` helper to eliminate duplication between `update()` and `install_skills_only()`
- Updated `main.rs` to route skill-only mode directly, skipping template download
- Added 8 new tests covering cross-client dir, resolve_adhoc_skills, and install_skills
- Version bump: 11.1.1 to 11.2.0 (MINOR - independent skill install feature)

### 2026-03-21 (Windows/PowerShell guidelines)

- Added "Windows / PowerShell Guidelines" section to AGENTS.md
- Covers shell syntax (no bash heredocs, use PowerShell here-strings or multi -m flags)
- Covers path handling (drive letters required for absolute, cfg-gated test assertions)
- Covers line ending awareness (CRLF vs LF, .gitattributes)
- Prevents agents from attempting bash-only syntax in PowerShell terminals

### 2026-03-21 (v11.1.1, fix Windows CI test failure)

- Fixed `test_resolve_local_skill_path_absolute` failing on Windows CI
- Unix-style `/opt/skills/my-skill` is not absolute on Windows (no drive letter), causing path resolution to prepend cwd
- Test now uses `#[cfg(windows)]` / `#[cfg(not(windows))]` with platform-appropriate absolute paths
- Version bump: 11.1.0 to 11.1.1 (PATCH - test fix)

### 2026-03-21 (v11.1.0, local ad-hoc skill installation)

- Added support for installing skills from local filesystem paths via `--skill`
- Previously `--skill` only accepted GitHub URLs and `user/repo` shorthand
- Now also accepts absolute paths (`/path/to/skill`), relative paths (`./skill`, `../skill`), and home-relative paths (`~/skills/my-skill`)
- Added `is_local_path()` to detect filesystem path syntax before GitHub shorthand expansion
- Added `resolve_local_skill_path()` to expand `~`, resolve relative paths against cwd
- In `install_skills()`, absolute source paths are used directly instead of joining with config_dir
- Updated CLI help text and `UpdateOptions` doc comment
- Added 10 unit tests covering path detection, resolution, and edge cases
- Version bump: 11.0.2 to 11.1.0 (MINOR - new feature)

### 2026-03-20 (v11.0.2, cross-section duplicate target detection)

- Added `validate_no_duplicate_targets()` in `template_engine.rs` to catch cross-section conflicts
- Previously, if two sections (e.g. language + integration) targeted the same workspace file, the last one silently overwrote the first
- Now returns a clear error naming both sources and the conflicting target path
- Extracted as standalone public function for testability
- Added 4 unit tests covering empty, unique, duplicate, and same-source-different-target cases
- Version bump: 11.0.1 to 11.0.2 (PATCH - defensive validation)

### 2026-03-20 (v11.0.1, fix shared template download)

- Fixed `download_templates_from_url` in `download_manager.rs` skipping `shared` section of templates.yml
- Shared file groups (e.g. cmake files used by C and C++ via `includes`) were never downloaded from remote
- This caused `resolve_language_files` to silently skip shared files during install (source not found on disk)
- Added download loop for `config.shared` values before the language download loop
- Version bump: 11.0.0 to 11.0.1 (PATCH - bug fix)

### 2026-03-19 (v11.0.0, rebrand to slopctl)

- MAJOR version bump: 10.0.0 to 11.0.0 (breaking: binary, config paths, data paths all renamed)
- Renamed tool from regulator to slopctl across entire codebase
- Binary name: `regulator` to `slopctl`
- Config path: `~/.config/regulator/` to `~/.config/slopctl/`
- Data path: `~/.local/share/regulator/` to `~/.local/share/slopctl/`
- Template marker: `REGULATOR-TEMPLATE` to `VIBE-COP-TEMPLATE`
- User-Agent header: `regulator` to `slopctl`
- Updated all CLI help text, error messages, and user-facing strings
- Updated CI workflows (build.yml, release.yml) artifact names
- Updated README.md with new tool name
- Man page renamed to slopctl.1

### 2026-03-13 (v10.0.0, rebrand to regulator)

- MAJOR version bump: 9.1.0 to 10.0.0 (breaking: binary, config paths, data paths all renamed)
- Renamed tool from vibe-check to regulator across entire codebase
- Binary name: `vibe-check` to `regulator`
- Config path: `~/.config/vibe-check/` to `~/.config/regulator/`
- Data path: `~/.local/share/vibe-check/` to `~/.local/share/regulator/`
- Template marker: `VIBE-CHECK-TEMPLATE` to `REGULATOR-TEMPLATE`
- User-Agent header: `vibe-check` to `regulator`
- Updated all CLI help text, error messages, and user-facing strings
- Updated CI workflows (build.yml, release.yml) artifact names
- Updated README.md with new tool name
- GitHub repo URL unchanged (rename pending); TODO markers left at URL references
- Template examples in rust-coding-conventions.md made generic (my-app)
- Man page renamed to regulator.1

### 2026-03-07 (v9.1.0, skill-aware subcommands)

- Upgraded status, purge, remove, and list commands to handle all skill sources correctly
- Previously only BoM-defined agent skills (templates.yml) were visible; top-level and ad-hoc skills were missed
- Added `FileTracker::get_workspace_entries()` and `get_workspace_entries_by_category()` query methods
- **status**: uses FileTracker to show all installed skills grouped by name (replaces SKILL.md path heuristic)
- **purge**: merges FileTracker entries into file collection so ad-hoc and top-level skills are also purged
- **remove**: added `--skill <name>` repeatable flag for targeted skill removal
- **remove**: `--agent` now also removes ad-hoc skill files under that agent's skill directory
- **remove**: `--all` now removes all tracked skill files in the workspace
- **list**: shows ad-hoc installed skills from FileTracker marked as "(ad-hoc)"
- Extracted shared `extract_skill_name_from_path()` helper in `template_manager/mod.rs` (DRY)
- Added `path_belongs_to_agent()` helper in `remove.rs` for agent-specific skill matching
- Added 13 new tests covering FileTracker queries, skill name extraction, and agent path matching
- Version bump: 9.0.4 to 9.1.0 (MINOR - new CLI flag, new FileTracker API)

### 2026-03-07 (v9.0.4, reduce GitHub API calls in skill install)

- Eliminated redundant `list_directory_contents` API calls during GitHub skill installation
- `discover_skills()` now carries pre-fetched directory entries in `DiscoveredSkill.entries`
- Added `download_directory_from_entries()` to accept pre-fetched entries, skipping re-listing
- Extracted shared `download_entries()` helper used by both download functions (DRY)
- `install_skills()` passes discovery entries to download phase instead of re-fetching
- Saves N GitHub API calls per install (one per discovered skill), reducing rate-limit pressure
- Version bump: 9.0.3 to 9.0.4 (PATCH - performance fix)

### 2026-03-07 (v9.0.3, fix GitHub skill installation)

- Fixed GitHub skill installation failing for repos without standardized directory structure
- Added `discover_skills()` to `github.rs`: recursively scans for SKILL.md to find skills in repos
- Added `download_directory_recursive()` to `github.rs`: downloads all files including subdirectories
- Added `GitHubUrl::child()` and `GitHubUrl::skill_name()` helper methods
- Fixed `skill_name_from_url` bug: bare `user/repo` shorthand returned branch name instead of repo name
- Reworked `install_skills` in `template_engine.rs` to use discovery + recursive download
- Local skill sources now also copied recursively (supports `scripts/`, `references/`, `assets/` subdirs)
- Added `collect_local_skill_files()` helper for recursive local skill collection
- Added tests for `skill_name`, `child`, and `skill_name_from_url` bare-repo cases
- Version bump: 9.0.2 to 9.0.3 (PATCH - bug fix)

### 2026-02-25 (anyhow migration)

- Migrated error handling from custom `Result<T>` type alias (`Box<dyn Error>`) to `anyhow` crate
- `lib.rs` now re-exports `pub use anyhow::Result;` instead of defining type alias
- Replaced all `Err(format!(...).into())` with `Err(anyhow!(...))`
- Replaced all `Err("literal".into())` with `Err(anyhow!(...))`
- Updated `.ok_or()` patterns for anyhow compatibility
- Updated `file_tracker.rs` signatures from `Box<dyn Error>` to anyhow
- Updated all test return types to `anyhow::Result`
- Fixed config test race condition with static Mutex
- Retrofitted `require!` macro at function-top preconditions across codebase
- Updated AGENTS.md error handling section to reflect anyhow patterns

### 2026-02-24 (require! macro)

- Added `require!(condition, return_expr)` precondition macro in `lib.rs`
- Returns expression when condition is false; works with `Result`, `Option`, and bare values
- Added unit tests for all three return type variants
- Documented convention in AGENTS.md error handling section

### 2026-02-24 (v9.0.2, coding conventions)

- Added "Loop Flow Control" coding convention: avoid `continue` guards, prefer combined conditions
- Added "No Option-wrapped collections" convention: use `Vec<T>` / `HashMap<K,V>` with `serde(default)` instead of `Option<Vec<T>>` / `Option<HashMap<K,V>>`
- Added convention: expose internal `Vec<T>` as `&[T]` slice, not `&Vec<T>`
- Replaced all `.unwrap()` with `?` operator across tests and library code
- Converted all `Option<Vec<T>>` and `Option<HashMap<K,V>>` struct fields to plain collections
- Changed `get_agent_files` return type from `Option<&Vec<PathBuf>>` to `Option<&[PathBuf]>`
- Eliminated all `continue` guard patterns in loop bodies
- Simplified consumers: removed `if let Some(...)` wrappers, used `.chain()` and `if/else if/else`
- Applied `cargo fmt` formatting fixes
- Version bump: 9.0.1 to 9.0.2 (PATCH - conventions and internal refactor)

### 2026-02-24 (v9.0.1)

- Decoupled `--lang` and `--agent` CLI flags: each now operates independently
- Removed `--no-lang` flag (redundant: omitting `--lang` has the same effect)
- Changed `UpdateOptions.lang` from `&str` to `Option<&str>`; removed `no_lang: bool` field
- Removed auto-resolution of language when only `--agent` is specified
- `--agent cursor` alone now installs only agent files; `--lang rust` alone installs only language files
- Simplified `template_manager/update.rs`: removed lang resolution block, pass options through
- Simplified CLI message branching in `main.rs`
- Version bump: 9.0.0 to 9.0.1 (PATCH - behavioral refinement)

### 2026-02-24 (v9.0.0, post-release)

- Added duplicate disk-file target validation in `resolve_language_files()`
- Two entries targeting the same workspace file now produce a clear error
- Entries targeting `$instructions` (AGENTS.md fragments) are exempt since multiple fragments are expected

### 2026-02-24 (v9.0.0)

- MAJOR version bump: 8.0.0 to 9.0.0 (V1 removed, V3 template format)
- Removed V1 template engine and all V1 templates (deprecated since v7.0.0)
- Deleted `src/template_engine_v1.rs` and entire `templates/v1/` directory (39 files)
- Introduced V3 template format: `shared` file groups and `includes` directive on languages
- New `shared` section in templates.yml for reusable file groups (e.g. cmake files shared by C and C++)
- New `includes` field on `LanguageConfig`: compose shared groups or extend other languages
- Recursive include resolution with cycle detection via `resolve_language_files()` in `bom.rs`
- Merged `template_engine_v2.rs` into `template_engine.rs`: dissolved `TemplateEngine` trait into struct
- Single `TemplateEngine` struct replaces the old trait + `TemplateEngineV2` struct
- Default template version changed from 2 to 3
- V2 templates remain backward-compatible (V3 is a superset)
- Updated `list` command to show includes annotations on composed languages
- DRY: cmake-build-commands.md and cmake-git-ignore.txt now defined once in shared section

### 2026-02-24 (v8.0.0)

- MAJOR version bump: 7.0.0 to 8.0.0 (breaking CLI change)
- Renamed `init` command to `install` for clearer semantics
- Added `--skill` repeatable CLI flag for ad-hoc GitHub skill installation
- Supports `user/repo` shorthand and full GitHub URLs
- Added GitHub URL support in templates.yml `source` fields (full URLs only, no shorthand)
- New `src/agent_defaults.rs`: single source of truth for agent paths (instructions, prompts, skills)
- New `src/github.rs`: GitHub API integration (Contents API, URL parsing, shorthand expansion)
- New top-level `skills` section in templates.yml for agent-agnostic skills
- Added `SkillDefinition` struct to `bom.rs` and `skills` field to `TemplateConfig`
- Skills downloaded on-the-fly during `install` (no local cache)
- Automatic agent detection via `detect_installed_agent()` when `--agent` not specified
- Moved `tempfile` from dev-dependencies to runtime dependency
- Updated all user-facing messages from `init` to `install`
- DRY refactoring: eliminated duplicate `download_file` and `parse_github_url` from `download_manager.rs` (now uses `github.rs`)
- DRY refactoring: collapsed repeated agent instructions/prompts/skills resolve-and-copy pattern into single loop
- Removed dead code: `github::download_directory()` (install_skills handles it directly)
- Added `skills` field to `UpdateOptions` and refactored all `update()` methods to accept `&UpdateOptions` instead of 7-8 individual parameters
- Removed `#[allow(clippy::too_many_arguments)]` suppressions
- Added DRY principle to Rust coding conventions in AGENTS.md

### 2026-02-17 (evening, v7.0.0)

- MAJOR version bump: 6.6.0 to 7.0.0
- Switched default template version from 1 to 2 (agents.md standard)
- V1 deprecation warning updated: no longer references v7.0.0, says future release
- This was planned since v6.2.0 (2026-01-24)

### 2026-02-17 (evening)

- Added Agent Skills infrastructure to V2 template engine (agentskills.io spec)
- New `skills` field in `AgentConfig` alongside `instructions` and `prompts`
- Skills are agent-specific: defined per-agent in templates.yml, installed to agent-specific locations
- Download, install, track, list, and status commands all support skills
- Skill files tracked with "skill" category in file tracker
- Infrastructure only: no pre-built skills ship yet; users can add their own
- Version bump: 6.5.7 to 6.6.0 (MINOR - new feature)

### 2026-02-17

- Added CLAUDE.md instruction file to V2 templates (references AGENTS.md)
- V2 engine now processes agent `instructions` alongside `prompts`
- Updated V2 templates.yml with claude instructions section
- Fixed misleading comments claiming V2 has no agents section
- Version bump: 6.5.5 to 6.5.6 (PATCH - bug fix, missing CLAUDE.md in V2)
- Added instruction files for all V2 agents: copilot (.github/copilot-instructions.md), cursor (.cursorrules), codex (CODEX.md)
- All instruction files redirect to AGENTS.md as single source of truth
- Version bump: 6.5.6 to 6.5.7 (PATCH - add remaining agent instruction files)

### 2026-02-16

- Introduced `UpdateOptions` struct in `template_engine.rs` to aggregate CLI parameters
- Introduced `TemplateContext` struct to aggregate source, target, and fragments
- Reduced `handle_main_template` from 11 to 6 params, `merge_fragments` from 6 to 3 params
- `copy_files_with_tracking` reduced from 8 to 5 params, `show_dry_run_files` uses both new structs
- v1/v2 engines construct both structs in `update()` and pass through
- Renamed `config_version` to `template_version` and moved into `TemplateContext`
- `handle_main_template` reduced from 6 to 5 params (version now in context)
- `copy_files_with_tracking` param renamed from `config_version` to `template_version`
- Made `TemplateContext` non-optional: fail early if `config.main` missing
- `show_dry_run_files` takes `&TemplateContext` instead of `Option<&TemplateContext>`
- `copy_files_with_tracking` takes `&TemplateContext` instead of `template_version: u32`
- Removed dead `files_to_copy.is_empty() && main_template.is_none()` early return
- Split `template_manager.rs` (713 lines) into directory module with per-command files
- `src/template_manager/mod.rs` holds struct, constructor, and helpers (~100 lines)
- Commands extracted to `update.rs`, `purge.rs`, `remove.rs`, `status.rs`, `list.rs`
- Changed `config_dir` visibility to `pub(crate)` for submodule access
- Version bump: 6.5.0 to 6.5.5 (PATCH - internal refactor, no public API change)
- Fixed `get_installed_language_for_workspace` failing on Windows CI (path separator mismatch)

### 2026-02-15 (evening)

- Major DRY refactoring across the codebase to eliminate 11 code duplication violations
- Introduced `TemplateEngine` trait in new `template_engine.rs` with shared default implementations
- Extracted `load_template_config` and `is_file_customized` as free functions (were triplicated across v1, v2, template_manager)
- Moved `resolve_placeholder`, `merge_fragments`, `show_dry_run_files`, `handle_main_template`, `copy_files_with_tracking`, `show_skipped_files_summary` into trait default methods
- Slimmed `template_engine_v1.rs` and `template_engine_v2.rs` to orchestration-only (removed ~300 duplicate lines)
- Extracted `DEFAULT_SOURCE_URL` const, `resolve_source()`, and `download_with_fallback()` helpers in `main.rs`
- Extracted `resolve_absolute_path()` helper in `file_tracker.rs` (was repeated 5 times)
- Reused `download_entry` closure in `download_manager.rs` for agent files
- Removed redundant `get_template_version()` from `template_manager.rs`
- Version bump: 6.4.2 to 6.5.0 (MINOR - new TemplateEngine trait, internal refactor)
- Fixed `resolve_placeholder` producing mixed path separators on Windows; use `Path::join()` instead of string replace
- Use `Path::starts_with()` instead of string-based prefix check for cross-platform correctness
- Added Rust coding convention: use `Path::starts_with()` for path comparison, not string prefix
- Version bump: 6.4.0 → 6.4.2 (PATCH - bug fixes)

### 2026-02-15

- Added `--no-lang` option to skip language-specific setup (AGENTS.md + agent prompts only, no coding-conventions)
- Use for language-independent setup: `slopctl init --no-lang` or `--no-lang --agent cursor`
- Mutually exclusive with `--lang`; valid with `--agent` for agent prompts without language fragments
- Made `--lang` and `--agent` optional; user must specify at least one of --lang, --agent, or --no-lang
- When only `--agent` specified: prefers existing installation language (e.g. switch Cursor to Claude, keep Rust)
- Fallback: first available language from templates.yml (fresh init with agent only)
- Version bump: 6.3.0 → 6.4.0 (MINOR - new features)
- V1 templates still require both (error if only one specified)
- Version bump: 6.2.0 → 6.3.0 (MINOR - new CLI behavior)

### 2026-01-24

- Added `--mission` option to `init` command for custom mission statements
- Supports inline text or file input via `@filename` syntax (e.g., `--mission @mission.md`)
- Custom mission overrides default template mission statement in AGENTS.md
- Implemented in both v1 and v2 template engines
- Version bump: 6.1.1 → 6.2.0 (MINOR - new feature)
- **Done in v7.0.0:** Switched default template version from 1 to 2 (see `default_version()` in `src/bom.rs`)

### 2025-12-28

- Fixed Swift format template JSON formatting and typos
- Corrected indentation issues on `respectsExistingLineBreaks` and `ValidateDocumentationComments` properties
- Fixed typo: removed erroneous colon from `NeverUseImplicitlyUnwrappedOptionals` property name
- Applied fixes to both v1 and v2 template versions
- Version bump: 6.1.0 → 6.1.1 (PATCH - bug fix)

### 2025-12-23

- Fixed gitattributes line ending conflict with Rust formatting
- Enforced LF line endings for Rust source files (*.rs) in .gitattributes to match rustfmt configuration (newline_style = "Unix")
- Updated both v1 and v2 template versions to prevent future conflicts

### 2025-10-05

- Initial AGENTS.md setup
- Established core coding standards and conventions
- Created agent-specific reference files
- Defined repository structure and governance principles
