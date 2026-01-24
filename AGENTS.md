# Project Instructions for AI Coding Agents

**Last updated:** 2026-01-24

<!-- {mission} -->

## Mission Statement

[Describe your project here - what it does, its purpose, and key features]

## Technology Stack

- **Language:** [e.g., Python, TypeScript, JavaScript]
- **Framework:** [e.g., React, Next.js, Django, FastAPI]
- **Version Control:** Git
- **Package Manager:** [e.g., npm, pip, poetry, yarn]
- **License:** [e.g., MIT, Apache 2.0]

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
4. **Code Review**: [Describe your code review process]
5. **Date Changes**: Update the "Last updated" timestamp in this file when making changes
6. **Log Updates**: Add entries to "Recent Updates & Decisions" section below

### Development Guidelines

[Add project-specific development guidelines]

- [Guideline 1]
- [Guideline 2]
- [Guideline 3]

### Security & Safety

- Never include API keys, tokens, or credentials in code
- Always require explicit human confirmation before commits
- Maintain conventional commit message standards
- Keep change history transparent through commit messages
- [Add project-specific security guidelines]

### Testing

[Describe your testing approach]

- Unit tests: [location and conventions]
- Integration tests: [location and conventions]
- Test coverage requirements: [if any]
- Testing framework: [e.g., Jest, pytest, JUnit]

### Documentation

[Describe your documentation requirements]

- Code comments: [when and how]
- API documentation: [format and location]
- README updates: [when required]
- Changelog: [if maintained]

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

**Error Handling:**

- Use `Result<T, E>` for all fallible operations
- Define a project-wide `Result<T>` type alias with unified error type:

  ```rust
  pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
  ```

- Use `?` operator for error propagation
- Avoid `.unwrap()` in library code; only use in application entry points after proper error handling
- Use `.ok_or_else()` or `.ok_or()` to convert `Option` to `Result` with meaningful error messages
- Provide context when returning errors: `Err(format!("Failed to download {}: {}", url, e).into())`
- Never panic in library code unless documenting preconditions with `#[panic]` doc comments

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

  pub use template_manager::TemplateManager;
  pub use utils::copy_dir_all;

  pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
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
  #[command(name = "vibe-check")]
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
  //! Template management functionality for vibe-check

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

**Note:** Version changes should be included in the commit with the actual code changes, not as a separate commit.

---

## Recent Updates & Decisions

### 2026-01-24

- Added `--mission` option to `init` command for custom mission statements
- Supports inline text or file input via `@filename` syntax (e.g., `--mission @mission.md`)
- Custom mission overrides default template mission statement in AGENTS.md
- Implemented in both v1 and v2 template engines
- Version bump: 6.1.1 → 6.2.0 (MINOR - new feature)

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
