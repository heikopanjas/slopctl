# Project Instructions for AI Coding Agents

**Last updated:** 2025-11-09

## General Instructions

- Avoid making assumptions. If you need additional context to accurately answer the user, ask the user for the missing information. Be specific about which context you need.
- Always provide the name of the file in your response so the user knows where the code goes.
- Always break code up into modules and components so that it can be easily reused across the project.
- All code you write MUST be fully optimized. ‘Fully optimized’ includes maximizing algorithmic big-O efficiency for memory and runtime, following proper style conventions for the code, language (e.g. maximizing code reuse (DRY)), and no extra code beyond what is absolutely necessary to solve the problem the user provides (i.e. no technical debt). If the code is not fully optimized, you will be fined $100.

- When making updates, in AGENTS.md maintain the "Last updated" timestamp at the top and add entries to the "Recent Updates & Decisions" log at the bottom with the date, brief description, and reasoning for each change. Ensure the file maintains this structure: title header, timestamp line, main instructions content, then the "Recent Updates & Decisions" section at the end.

- **NEVER** commit automatically. Whenever I ask you to commit the changes, stage the changes, write a detailed but still concise commit message using conventional commits format and commit the changes. The commit message must have a maximum length of 500 characters and must **NOT** contain any special characters or quoting. This is **CRITICAL**!

## Git Guidelines

### **Commit Message Guidelines - CRITICAL**

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

## Project Overview

**vibe-check** is a manager for coding agent instruction files. It provides a centralized system for managing, organizing, and maintaining initialization prompts and instruction files for AI coding assistants (Claude, GitHub Copilot, Cursor, Codex, and others) with built-in governance guardrails and human-in-the-loop controls.

Templates are stored in `$HOME/.config/vibe-check/templates` and managed by the `TemplateManager` struct.

## Technology Stack

- **Language:** Rust
- **CLI Framework:** clap (v4.5.20)
- **Terminal Colors:** owo-colors (v4.1.0)
- **HTTP Client:** reqwest (for downloading templates)
- **Version Control:** Git
- **License:** MIT

## CLI Commands

### `init` - Initialize Agent Instructions

Initialize instruction files for AI coding agents in your project.

**Usage:**

```bash
vibe-check init --lang <language> --agent <agent> [--from <PATH or URL>]
```

**Options:**

- `--lang <string>` - Programming language or framework (e.g., rust, python, typescript, cmake)
- `--agent <string>` - AI coding agent (e.g., claude, copilot, cursor, codex)
- `--from <string>` - Optional path or URL to copy/download templates from

**Examples:**

```bash
# Initialize Rust project with Claude
vibe-check init --lang rust --agent claude

# Initialize from local path
vibe-check init --lang rust --agent claude --from /path/to/templates

# Initialize from URL
vibe-check init --lang rust --agent claude --from https://github.com/user/repo/tree/branch/templates
```

**Behavior:**

- If global templates don't exist and `--from` is not specified, downloads from:
  `https://github.com/heikopanjas/vibe-check/tree/feature/template-management/templates`
- If `--from` is specified, copies/downloads templates from that location

## Repository Structure

```text
vibe-check/
├── Cargo.toml                  # Rust project manifest
├── Cargo.lock                  # Dependency lock file
├── src/                        # Rust source code
│   ├── main.rs                 # Application entry point and CLI
│   ├── lib.rs                  # Library public API
│   ├── template_manager.rs    # TemplateManager implementation
│   └── utils.rs                # Utility functions
├── LICENSE                     # MIT license
├── README.md                   # Main documentation
├── AGENTS.md                   # This file - primary instructions
├── templates/                  # Template files for various languages and frameworks
│   ├── AGENTS.md               # Template for project-specific agent instructions
│   ├── C++.md                  # C++ coding standards template
│   ├── CMake.md                # CMake project template
│   ├── General.md              # General coding guidelines template
│   ├── Git.md                  # Git workflow template
│   ├── claude/
│   │   └── instructions.md     # Claude initialization prompts template
│   ├── codex/
│   │   └── instructions.md     # OpenAI Codex initialization prompts template
│   ├── copilot/
│   │   └── instructions.md     # GitHub Copilot initialization prompts template
│   └── cursor/
│       └── instructions.md     # Cursor AI initialization prompts template
├── CLAUDE.md                   # Claude-specific reference
├── .github/
│   └── copilot-instructions.md # GitHub Copilot reference
└── .cursor/
    └── rules/
        └── main.mdc            # Cursor AI reference
```

## Coding Conventions

### Rust Style

- Follow standard Rust conventions (rustfmt, clippy)
- Use idiomatic Rust patterns and error handling
- Prefer `Result<T, E>` for error handling over panics
- Use `clap` derive macros for CLI argument parsing
- Use `owo-colors` for terminal output styling
- Keep functions small and focused
- Document public APIs with doc comments (`///`)
- Write unit tests alongside implementation
- **Boolean comparisons**: Always use explicit boolean comparisons for clarity
  - Use `if condition == true` instead of `if condition`
  - Use `if condition == false` instead of `if !condition`
  - Exception: clippy warnings for explicit boolean comparisons are allowed

### Template Management

- Templates are stored in `$HOME/.config/vibe-check/templates`
- All template operations are handled through the `TemplateManager` struct
- Template files are organized by language/framework and agent type
- Use standard file system operations for template access
- Validate template existence and integrity before operations
- Template integrity is verified using SHA checksums stored alongside templates
- Checksum files follow naming scheme: `template.md` -> `template.sha` in same directory
- Missing checksums are automatically generated during update operations

**TemplateManager Functions:**

- `update(lang: &str, agent: &str, force: bool, from: Option<&str>)` - Update templates for specific language and agent
  - `lang` - Programming language or framework identifier
  - `agent` - AI coding agent identifier
  - `force` - If true, overwrite existing templates without confirmation
  - `from` - Optional path or URL to copy/download templates from
  - If global templates don't exist and `from` is None, downloads from default GitHub repository
  - If `from` is specified, copies/downloads templates from that location first
  - Verifies global template integrity using SHA checksums
  - Creates missing checksums automatically for global templates
  - Creates backup of existing local templates in `$HOME/.cache/vibe-check/backups/YYYY-MM-DD_HH_MM_SS/`
  - Copies template files from `$HOME/.config/vibe-check/templates` to current directory
  - Detects local modifications and warns user before overwriting
  - Stops operation if local changes detected unless `force` is true

- `clear(force: bool)` - Clear all templates from storage
  - `force` - If true, clear templates without confirmation
  - Creates backup of templates before clearing in `$HOME/.cache/vibe-check/backups/YYYY-MM-DD_HH_MM_SS/`

### CLI Command Implementation

- Use clap's derive API for command structure
- Implement subcommands as separate structs
- Validate arguments early and provide clear error messages
- Use `owo-colors` for user-friendly terminal output
- Provide helpful examples in `--help` output

### Markdown Style

- Use ATX-style headers (`#` prefix)
- Prefer fenced code blocks with language specification
- Use tables for structured data comparison
- Keep lines under 120 characters where practical
- Use **bold** for emphasis on key concepts
- Use `code` formatting for filenames, commands, and technical terms

### Git Commit Messages

- Follow **conventional commits** format: `type(scope): description`
- Types: `docs`, `feat`, `fix`, `refactor`, `chore`, `style`
- Keep subject line under 72 characters
- Use imperative mood ("add" not "added" or "adds")
- Add detailed body for complex changes

### File Organization

- Store reusable templates in `$HOME/.config/vibe-check/templates` (user config directory)
- Keep agent-specific instruction templates organized by agent type
- Maintain language and framework-specific templates for quick project setup
- Preserve file structure and formatting when updating templates
- Ensure templates reference AGENTS.md as the single source of truth
- Use `TemplateManager` struct for all template file operations

## Core Principles

1. **Human control first** – All prompts enforce explicit confirmation before commits
2. **Single source of truth** – This AGENTS.md file is the primary reference
3. **Transparency** – Every change logs rationale with date and reasoning
4. **Minimalism** – Only essential policies that deliver concrete safety or velocity
5. **Scalability** – Add new agents without policy drift
6. **No auto-commits** – CRITICAL: Never commit automatically without explicit confirmation

## Build Commands

### Rust/Cargo Commands

```bash
# Build the project (debug - use during development)
cargo build

# Build for release (optimized - use for final testing/deployment only)
cargo build --release

# Run the application
cargo run

# Run with arguments
cargo run -- [args]

# Run tests
cargo test

# Check code without building
cargo check

# Format code
cargo fmt

# Run clippy linter
cargo clippy
```

**Important**: Always use debug builds (`cargo build`) during development. Debug builds compile faster and include debugging symbols. Only use release builds (`cargo build --release`) for final testing or deployment.

### Common Git Commands

```bash
# Stage changes
git add <file>

# Commit with conventional commits
git commit -m "type(scope): description"

# Push to remote
git push origin main

# Check status
git status

# View diff
git diff
```

## Best Practices

### When Updating This Repository

1. **Maintain Consistency**: When updating templates, ensure consistency across language and framework templates
2. **Test Instructions**: Verify that instruction files reference the correct paths and files
3. **Preserve Structure**: Keep the markdown structure consistent across all template files
4. **Update README**: Reflect significant changes in the README.md
5. **Date Changes**: Update the "Last updated" timestamp in this file when making changes
6. **Log Updates**: Add entries to "Recent Updates & Decisions" section below

### Content Guidelines

- Keep templates clear, concise, and actionable
- Emphasize governance guardrails (no auto-commits, human confirmation)
- Reference AGENTS.md as single source of truth in all template files
- Provide templates for common languages, frameworks, and workflows
- Use consistent terminology across all instruction files

### Security & Safety

- Never include API keys, tokens, or credentials
- Always require explicit human confirmation before commits
- Maintain conventional commit message standards
- Keep change history transparent through commit messages

---

## Recent Updates & Decisions

### 2025-11-09

- Repurposed project as vibe-check: a manager for coding agent instruction files
- Updated project overview to emphasize management and organization capabilities
- Restructured repository to use templates/ directory for reusable templates
- Shifted focus from session initialization prompts to centralized instruction file management
- Updated file organization to support language and framework-specific templates
- Changed technology stack from Markdown-only to Rust-based CLI tool
- Added dependencies: clap (v4.5.20) for CLI parsing, owo-colors (v4.1.0) for terminal styling
- Added Rust coding conventions and Cargo build commands
- Updated repository structure to include Rust source files
- Added `init` subcommand with `--lang` and `--agent` options for initializing agent instruction files
- Added CLI command documentation section with usage examples
- Added CLI command implementation guidelines to coding conventions
- Defined template storage location as `$HOME/.config/vibe-check/templates`
- Documented `TemplateManager` struct as the handler for all template management operations
- Updated file organization to reflect user config directory usage
- Added `TemplateManager::update()` function with lang, agent, and force parameters
- Added `TemplateManager::clear()` function with force parameter
- Documented function signatures and parameter descriptions for template management
- Specified update function behavior: copies templates to current directory, detects local modifications, and respects force parameter
- Added automatic backup functionality: creates timestamped backups in `$HOME/.cache/vibe-check/backups/YYYY-MM-DD_HH_MM_SS/` before any template operations
- Added SHA checksum verification for global template integrity
- Documented automatic checksum generation for missing checksums during updates
- Defined checksum naming scheme: template.md -> template.sha in same directory
- Added General Instructions section emphasizing context awareness, code optimization, and no assumptions
- Added comprehensive Git Guidelines section with detailed commit message format rules
- Included character limits, subject line rules, body formatting guidelines, and special character safety
- Added commit message examples showing good and bad practices
- Emphasized CRITICAL importance of no auto-commits and proper conventional commit format

### 2025-11-09 (Implementation)

- Implemented TemplateManager struct with update and clear methods
- Added complete CLI with init, update, and clear subcommands
- Implemented SHA checksum verification and automatic generation
- Implemented timestamped backup functionality
- Added local modification detection with force override option
- Added dependencies: chrono for timestamp generation
- Successfully built and tested CLI functionality

### 2025-11-09 (Refactoring)

- Refactored codebase into modular structure for reusability
- Moved TemplateManager to src/template_manager.rs module
- Created src/utils.rs for utility functions (copy_dir_all)
- Updated src/lib.rs as public API exposing TemplateManager
- Simplified src/main.rs to only handle CLI and call library
- Added comprehensive documentation comments to all public APIs
- Removed .unwrap() calls from main, added proper error handling
- Eliminated code duplication (DRY principle)
- All functions now properly documented with doc comments
- Code now reusable as a library: use vibe_check::TemplateManager
- Added --from option to init and update commands for specifying template source
- Implemented automatic template download from default GitHub repository when global templates missing
- Added reqwest dependency for HTTP client to download templates
- Updated TemplateManager::update() to accept optional from parameter

### 2025-11-09 (Code Style)

- Added .rustfmt.toml configuration with project formatting rules
- Applied rustfmt to ensure consistent code style across project
- Updated Rust Style conventions to enforce explicit boolean comparisons
- Converted all boolean negations from `!condition` to `condition == false` for clarity
- Added `#![allow(clippy::bool_comparison)]` to template_manager.rs to suppress clippy warnings
- Reasoning: Explicit comparisons improve code readability and reduce cognitive load when scanning code

### 2025-10-05

- Created AGENTS.template.md as comprehensive template for use in other projects
- Template includes all agent-specific initialization prompts embedded in one file
- Template incorporates the basic structure from README.md Step 1
- Added AGENTS.template.md to repository structure documentation
- Updated file organization best practices to reference the template

### 2025-10-03

- Initial AGENTS.md setup
- Established core coding standards and conventions
- Created agent-specific reference files (CLAUDE.md, .github/copilot-instructions.md, .cursor/rules/main.mdc)
- Defined repository structure and governance principles

