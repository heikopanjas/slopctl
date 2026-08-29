# Project Instructions for AI Coding Agents

**Last updated:** 2026-08-29 (v22.7.0)

<!-- {mission} -->

## Mission Statement

slopctl is a Rust CLI tool that manages coding agent instruction files (AGENTS.md, CLAUDE.md) across workspaces. It downloads, installs, updates, and synchronizes templates and Agent Skills for multiple AI coding assistants (Claude Code, Cursor, GitHub Copilot, Codex, Mistral Vibe, OpenCode) following the agents.md and agentskills.io community standards.

## Technology Stack

- **Language:** Rust (Edition 2024, nightly toolchain)
- **CLI Framework:** clap v4.5 (derive API) with clap_complete for shell completions
- **HTTP:** reqwest v0.12 (blocking, json) for GitHub API and template downloads; flate2 + tar for pure-Rust tarball extraction (cross-platform skill caching)
- **Serialization:** serde + serde_yaml for templates.yml, agent-defaults.yml, and file tracker, serde_json for legacy migration
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
- All code you write MUST be fully optimized. 'Fully optimized' includes maximizing algorithmic big-O efficiency for memory and runtime, following proper style conventions for the code, language (e.g. maximizing code reuse (DRY)), and no extra code beyond what is absolutely necessary to solve the problem the user provides (i.e. no technical debt). If the code is not fully optimized, you will be fined $100.

### Working Together

This file (`AGENTS.md`) is the primary instructions file for AI coding assistants working on this project. Agent-specific instruction files (such as `CLAUDE.md`, `.github/copilot-instructions.md`, `.cursorrules`) reference this document, maintaining a single source of truth.

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
- Log every change in the "Recent Updates & Decisions" log in `UPDATES.md` with:
  - Date (with time if multiple updates per day)
  - Brief description
  - Reasoning for the change
- New log entries go directly below the changelog marker in `UPDATES.md`, newest first; load the `recent-updates` skill for the full rules
- Preserve the AGENTS.md structure: title header → timestamp → main instructions

## Best Practices

### When Updating This Repository

1. **Maintain Consistency**: Keep code style consistent across the codebase
2. **Test First**: Write tests before implementing features when applicable
3. **Document Changes**: Update documentation when changing functionality
4. **Code Review**: Run `cargo fmt`, `cargo clippy`, and `cargo test` before committing
5. **Date Changes**: Update the "Last updated" timestamp in this file when making changes
6. **Log Updates**: Add entries to the "Recent Updates & Decisions" log in `UPDATES.md`

### Development Guidelines

- Use debug builds (`cargo build`) during development; reserve release builds for CI and deployment
- Branch model: `develop` is the active branch; `main` receives stable merges only
- Always use `--dry-run` to verify CLI behavior before writing destructive tests
- Keep `main.rs` thin (CLI parsing and dispatch only); business logic belongs in library modules
- One public struct or major component per source file; shared helpers go in `utils.rs`

### Toolchain Pinning

- `rust-toolchain.toml` at the repo root pins the exact nightly (`channel = "nightly-YYYY-MM-DD"`) plus its `rustfmt`/`clippy` components; `rustup` and `dtolnay/rust-toolchain` in CI both read this file automatically, so local dev and CI always resolve to the identical toolchain build
- CI workflows (`build.yml`, `release.yml`) must NOT pass an explicit `toolchain:` input to `dtolnay/rust-toolchain` — that would override the pinned file with a floating channel and reintroduce drift; only `components:` is passed explicitly
- Do not use an unpinned `nightly` channel anywhere (CI matrix, local `rustup default`, etc.); a floating nightly silently changes the shipped `rustfmt` version over time
- To bump the toolchain: update the `channel` date in `rust-toolchain.toml` and run `cargo fmt`/`cargo clippy` locally to confirm the new toolchain formats and lints cleanly before committing
- Do NOT reintroduce `.rustfmt.toml`'s `required_version` as a substitute for this: `.rustfmt.toml` is a shared template (see below) installed into arbitrary downstream workspaces that do not get this repo's `rust-toolchain.toml`, so a hardcoded version pin there breaks `cargo fmt` for any user on a different rustfmt build. The toolchain file is the only place version pinning belongs.

### File Tracker Ownership

- Tracker entries use `lang: Vec<String>` and `agent: Vec<String>` owner arrays, plus a stored `ref_count`
- `ref_count` must always equal `lang.len() + agent.len()`; add/remove operations increment or decrement only when a unique owner is added or released
- Legacy scalar tracker values (`lang: none`, `agent: all`) are obsolete; old `.slopctl/tracker.yml` files are not migrated and workspaces should recreate tracker state fresh
- Shared ownership only applies to identical files, such as shared `.editorconfig`, other formatter config files, and skill files; if the incoming template SHA differs from the tracked SHA, `init` must fail preflight and point users to merge instead of incrementing `ref_count`
- `AGENTS.md` is a special main file: ownership can be tracked, but normal `remove --lang` and `remove --agent` must not delete it
- Files carrying the changelog marker (`<!-- {changelog} -->`, e.g. `UPDATES.md`) are protected like `AGENTS.md`: `init` and `update` (full or `--file`) never write them, under any flag, and `update --file` on one is a hard error pointing to `merge`; `merge` is the only command that may refresh the template half above the marker, and `remove`/`remove --purge` preserve the whole file (purge overrides with `--force`). Protection is keyed on `template_engine::is_changelog_protected`/`file_contains_changelog_marker` finding the marker as its own trimmed line on the target, never on `FileStatus` — keying on `Modified` alone misses a file whose tracker SHA was re-recorded by `merge`, and matching the marker as a raw substring false-positives on docs that mention it as example text (e.g. the `recent-updates` skill)
- Tracker categories (`main`/`agent`/`language`/`skill`/`integration`) are recorded from the templates.yml section a file resolves from, carried on `ResolvedFile.category`; they must never be derived from path substrings
- Init never hard-fails on a tracked modified file that adds no new owners: it plans `SkipModified` and keeps the local version; `merge` is the update path, `--force` overwrites. Owner-expanding conflicts and untracked collisions remain hard preflight errors
- Command verb separation: `init` installs something new (a language or agent) and rejects a no-op re-init when everything requested is already tracker-installed (bypass with `--force`); bare `slopctl update` refreshes the whole workspace from the local cache (selectors narrow it); `merge` reconciles customized files. "Already installed" is determined from FileTracker owners (`get_installed_languages`/`get_installed_agents`), never from marker directories, because agents create their own markers
- Removing an agent or language releases its ownership across ALL tracker entries (`clear_agent_owner`/`clear_lang_owner`), not only on deleted files; native-only agents also own shared cross-client copies that stay on disk
- Location-based removal checks (`path_belongs_to_agent`) match by `Path::starts_with` against the agent's catalog directories (markers, skill_dir, prompt_dir); never by substring or path-component name matching
- `update` never creates agent-category files for agents that were not slopctl-installed; marker presence only drives skill distribution. `merge` without `--agent` unions the resolved content across all detected agents

### Security & Safety

- Never include API keys, tokens, or credentials in code
- Always require explicit human confirmation before commits
- Maintain conventional commit message standards
- Keep change history transparent through commit messages
- GitHub API access is unauthenticated (no tokens or credentials); unauthenticated limits apply (~60 REST requests/hour per IP, plus raw.githubusercontent.com throttling)
- Template marker detection prevents accidental overwrites of user-customized files

### Testing

Unit tests are co-located with implementation in each source file under `#[cfg(test)] mod tests`.

- Unit tests: In-file `#[cfg(test)]` modules, named `test_<scenario>_<expected_outcome>`
- Integration tests: Manual CLI verification with `--dry-run` flag
- Test serialization: Tests that call `std::env::set_current_dir` share a `CWD_LOCK` mutex to prevent race conditions
- CI runs `cargo test --verbose` on Linux, macOS, and Windows (nightly toolchain)
- Testing framework: Built-in Rust test harness with `assert!`, `assert_eq!`, `assert_ne!`
- Rust source tests must not use real-world coding-agent or programming-language fixture names. Use artificial agents `bogus` and `fake`; use artificial languages `Rust++` and `CppScript`. Real supported names belong only in shipped template/catalog data, not source fixtures.

### Documentation

- Code comments: `///` doc comments on all public APIs; `//` for non-obvious implementation details only
- API documentation: Generated via `cargo doc`; doc comment structure uses `# Arguments`, `# Errors`, `# Examples` sections
- README updates: Required when adding or changing CLI commands, flags, or user-visible behavior
- Changelog: Maintained in `UPDATES.md` at the repository root (append-only; see the `recent-updates` skill)

<!-- {languages} -->

## Rust Coding Standards

Load the `rust-coding-conventions` skill before writing, reviewing, or refactoring Rust code.
Load the `rust-build-commands` skill when building or running the project.

### Rust Coding Conventions

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

- Prefer `Option<T>` over sentinel enum variants. Do not add `Invalid`, `Unknown`, or `None` variants to an enum solely to avoid wrapping it in `Option`. `Option<T>` is niche-optimized (zero runtime cost for most enums) and forces callers to handle absence at compile time, whereas sentinel variants move that guarantee to a runtime convention and pollute every match site with a defensive arm.
  - ❌ Incorrect: `enum Color { Invalid, Red, Green, Blue }` returned from a parser
  - ✅ Correct: `enum Color { Red, Green, Blue }` with `Option<Color>` at the boundary
  - Exception: when "unknown" is a meaningful domain state — e.g. forward-compatible protocol parsing where unrecognized variants must round-trip — model it explicitly (`HttpVersion::Unknown(String)`). This is "modeling the domain accurately," not "avoiding `Option`."

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

- This repo's root `.rustfmt.toml` is dogfooded from `templates/v5/rust-format-instructions.toml`, the same file slopctl installs as `.rustfmt.toml` into every downstream workspace that adds Rust language support. The two must stay byte-identical; editing one without the other desyncs this workspace's tracked SHA from the template SHA, which trips the shared-ownership preflight check and blocks `slopctl update`/`init` on this repo's own `rust` language files
- Do NOT add `required_version` (or other CI-environment-specific settings) to either copy — a hardcoded version pin breaks `cargo fmt` for any downstream user whose local rustfmt doesn't match exactly, since they don't get this repo's `rust-toolchain.toml`. Version consistency for this repo's own CI is handled entirely by the toolchain pin (see "Toolchain Pinning" above)
- Key formatting rules:
  - `max_width = 167` - Allow longer lines for readability
  - `brace_style = "AlwaysNextLine"` - Opening braces on new line
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
- Prefer crate-level attributes when project style intentionally differs from clippy defaults:

  ```rust
  #![allow(clippy::bool_comparison)]
  ```

- Avoid package-level `[lints.clippy]` in `Cargo.toml` for now because the editor TOML schema flags it even though Cargo accepts it
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

## Semantic Versioning Protocol

**AUTOMATICALLY track version changes using semantic versioning (SemVer) in Cargo.toml.**

Automatically bump the project version after every code change and include it in the same commit. Load the `semantic-versioning` skill for the full PATCH/MINOR/MAJOR decision rules.

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

## Commit Protocol (CRITICAL)

- **NEVER commit automatically** — always wait for explicit user confirmation
- Stage changes, write a conventional commits message (max 50-char subject, 72-char body lines), then commit
- Load the `git-workflow` skill for the full message format, character limits, and examples before committing
- **No co-authorship by coding agents**: never add `Co-Authored-By` trailers, `Generated with` footers, or any other attribution naming an AI coding agent to commit messages

Whenever asked to commit changes:

- Stage the changes
- Write a detailed but concise commit message using conventional commits format
- Commit the changes

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
- **Body lines**: Wrap at 72 characters maximum
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
- Use bullet points (`-`) for all body items with lowercase text after bullet
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

## Shell & Platform Guidelines

Development happens on **macOS, Linux, and Windows**. AI agents must detect the current platform and use the shell native to it:

- **macOS / Linux:** zsh or bash (POSIX syntax)
- **Windows:** PowerShell

Never emit syntax for the wrong shell; the sections below define the rules per platform.

### macOS / Linux Shell Syntax (zsh / bash)

- Standard POSIX constructs are available: `&&` chaining, `$(command)` substitution, heredocs
- For multi-line git commit bodies, put the subject in the first `-m` and the entire body in a second `-m` using ANSI-C quoting (`$'- bullet one\n- bullet two'`), or write the message to a file and use `git commit -F <file>`
- Escape character is backslash (`\`); prefer single quotes for literal strings
- Prefer platform-appropriate paths via the `dirs` crate in Rust code rather than assuming `$HOME` layout

### Windows Shell Syntax (PowerShell)

- **Never use bash-specific constructs**: heredocs (`<<'EOF'`), `$(command)` substitution, `&&` chaining (PowerShell 7+ supports `&&` but avoid for safety)
- **Use PowerShell here-strings** for multi-line text:

  ```powershell
  @"
  multi-line
  string
  "@
  ```

- **Single `-m` flag for the body of git commits**: each `-m` creates a separate paragraph with a blank line between it and the next, which breaks bullet lists. Put the subject in the first `-m` and the entire body (with embedded newlines) in a second `-m` using a PowerShell here-string, or write the message to a file and use `git commit -F <file>`:

  ```powershell
  git commit -m "subject line" -m @"
  - body point one
  - body point two
  "@
  ```

- **Use semicolons** (`;`) to chain commands, not `&&`
- **Escape rules differ**: PowerShell uses backtick (`` ` ``) as escape character, not backslash

### Cross-Platform Path Handling

- Windows uses backslash (`\`) as path separator; forward slash (`/`) works in most contexts but not all
- Absolute paths require a drive letter (`C:\path`); a bare `/path` is relative to the current drive root, not an absolute path
- Use `Path::join()` and `Path::is_absolute()` in Rust code; never assume `/` prefixed paths are absolute
- In tests, use `#[cfg(windows)]` / `#[cfg(not(windows))]` when asserting platform-specific path behavior

### Line Endings

- Repository uses `.gitattributes` to enforce LF for Rust source files (`*.rs`)
- Be aware of CRLF vs LF differences when comparing file content or hashes
