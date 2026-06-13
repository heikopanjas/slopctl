---
name: rust-coding-conventions
description: Rust coding conventions covering error handling, naming, module organization, formatting, and testing. Load before writing, reviewing, or refactoring Rust code.
license: MIT
metadata:
  author: Heiko Panjas
  version: "1.0"
---

# Rust Coding Conventions

Read this skill before writing, reviewing, or refactoring Rust code in this project.
It covers error handling, naming, module organization, formatting, testing, and more.

---

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
- **DRY (Don't Repeat Yourself)**: Extract shared logic into functions, traits, or structs. When the same pattern appears in 2+ places, factor it out. Use parameter structs (e.g. `DownloadOptions`) to aggregate related arguments rather than passing many individual parameters. Prefer a single source of truth for data rather than duplicating paths in config and code.

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

- Define `require!` in `src/lib.rs` so future sessions can add or restore it:

  ```rust
  /// Early-return guard macro for precondition checks
  ///
  /// Returns the given expression when the condition is false.
  /// Works with any return type: `Result`, `Option`, or bare values.
  #[macro_export]
  macro_rules! require {
      ($cond:expr, $ret:expr) => {
          if ($cond) == false
          {
              return $ret;
          }
      };
  }
  ```

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
      return Err(anyhow!("Missing value"));
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
