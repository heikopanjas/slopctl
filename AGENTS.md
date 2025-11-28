# Project Instructions for AI Coding Agents

**Last updated:** 2025-11-28

## General Instructions

- Avoid making assumptions. If you need additional context to accurately answer the user, ask the user for the missing information. Be specific about which context you need.
- Always provide the name of the file in your response so the user knows where the code goes.
- Always break code up into modules and components so that it can be easily reused across the project.
- All code you write MUST be fully optimized. ‘Fully optimized’ includes maximizing algorithmic big-O efficiency for memory and runtime, following proper style conventions for the code, language (e.g. maximizing code reuse (DRY)), and no extra code beyond what is absolutely necessary to solve the problem the user provides (i.e. no technical debt). If the code is not fully optimized, you will be fined $100.

- When making updates, in AGENTS.md maintain the "Last updated" timestamp at the top and add entries to the "Recent Updates & Decisions" log at the bottom with the date, brief description, and reasoning for each change. Ensure the file maintains this structure: title header, timestamp line, main instructions content, then the "Recent Updates & Decisions" section at the end.

## Commit Protocol - ABSOLUTELY CRITICAL

**NEVER stage files or commit automatically under ANY circumstances.**

You must ONLY stage and commit when the user uses one of these EXACT phrases:

- "commit the changes"
- "commit this"
- "please commit"
- "make a commit"

If the user asks you to:

- "implement X"
- "add feature Y"
- "update Z"
- "fix this"

You MUST:

1. Make the code changes
2. Build/test to verify
3. **STOP and WAIT** - Do NOT stage or commit
4. Inform the user the work is complete
5. Wait for explicit commit instruction

When you DO commit (after explicit instruction):

- Stage the changes
- Write a detailed but concise commit message using conventional commits format
- The commit message must have a maximum length of 500 characters
- The commit message must **NOT** contain any special characters or quoting
- Commit the changes

**Violation of this rule is unacceptable.**

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

## Project Overview

**vibe-check** is a manager for coding agent instruction files. It provides a centralized system for managing, organizing, and maintaining initialization prompts and instruction files for AI coding assistants (Claude, GitHub Copilot, Codex, and others) with built-in governance guardrails and human-in-the-loop controls.

Templates are stored in the local data directory (e.g., `$HOME/.local/share/vibe-check/templates` on Linux, `$HOME/Library/Application Support/vibe-check/templates` on macOS) and managed by the `TemplateManager` struct.

## Technology Stack

- **Language:** Rust
- **CLI Framework:** clap (v4.5.20)
- **Terminal Colors:** owo-colors (v4.1.0)
- **HTTP Client:** reqwest (v0.12 with blocking and json features)
- **Serialization:** serde (v1.0), serde_json (v1.0), and serde_yaml (v0.9)
- **Directory Paths:** dirs (v5.0)
- **Version Control:** Git
- **License:** MIT

## CLI Commands

### `update` - Update Global Templates

Download and update global templates from a source repository.

**Usage:**

```bash
vibe-check update [--from <PATH or URL>] [--dry-run]
```

**Options:**

- `--from <string>` - Optional path or URL to download/copy templates from
- `--dry-run` - Preview what would be downloaded without making changes

**Examples:**

```bash
# Update global templates from default repository
vibe-check update

# Update from custom URL
vibe-check update --from https://github.com/user/repo/tree/branch/templates

# Update from local path
vibe-check update --from /path/to/templates

# Preview what would be downloaded
vibe-check update --dry-run
```

**Behavior:**

- Downloads templates from specified source or default GitHub repository
- If `--from` is not specified, downloads from:
  `https://github.com/heikopanjas/vibe-check/tree/develop/templates`
- Downloads `templates.yml` configuration file and all template files
- Stores templates in local data directory:
  - Linux: `$HOME/.local/share/vibe-check/templates`
  - macOS: `$HOME/Library/Application Support/vibe-check/templates`
- If `--dry-run` is specified, shows the source URL and target directory without downloading
- Overwrites existing global templates with new versions
- Does NOT modify any files in the current project directory

**Note:** Run `update` first to download templates before using `init` to set up a project.

### `init` - Initialize Agent Instructions

Initialize instruction files for AI coding agents in your project.

**Usage:**

```bash
vibe-check init --lang <language> --agent <agent> [--force] [--dry-run]
```

**Options:**

- `--lang <string>` - Programming language or framework (e.g., c++, rust, swift)
- `--agent <string>` - AI coding agent (e.g., claude, copilot, codex, cursor)
- `--force` - Force overwrite of local files without confirmation
- `--dry-run` - Preview changes without applying them

**Examples:**

```bash
# Initialize C++ project with Claude
vibe-check init --lang c++ --agent claude

# Initialize Rust project with Copilot
vibe-check init --lang rust --agent copilot

# Force overwrite existing local files
vibe-check init --lang swift --agent cursor --force

# Preview what would be created/modified
vibe-check init --lang rust --agent claude --dry-run
```

**Behavior:**

- Uses global templates to set up agent instructions in the current project
- If global templates do not exist, automatically downloads them from the default repository
- Checks for local modifications to AGENTS.md (detects if template marker has been removed)
- If local AGENTS.md has been customized and `--force` is not specified, skips AGENTS.md
- If `--force` is specified, overwrites local files regardless of modifications
- If `--dry-run` is specified, shows what would be created/modified without making changes
- Files are placed according to `templates.yml` configuration with placeholder resolution:
  - `$workspace` resolves to current directory
  - `$userprofile` resolves to user's home directory
- Merges language-specific and integration fragments into AGENTS.md

### `purge` - Purge All Vibe-Check Files

Purge all vibe-check files from the current project directory.

**Usage:**

```bash
vibe-check purge [--force] [--dry-run]
```

**Options:**

- `--force` - Force purge without confirmation and delete customized AGENTS.md
- `--dry-run` - Preview what would be deleted without making changes

**Examples:**

```bash
# Purge all vibe-check files with confirmation prompt
vibe-check purge

# Force purge without confirmation
vibe-check purge --force

# Preview what would be deleted
vibe-check purge --dry-run
```

**Behavior:**

- Uses Bill of Materials (BoM) from templates.yml to discover all agent-specific files
- Removes all agent-specific files from all agents (instructions, prompts, directories)
- Removes AGENTS.md from current directory
- Automatically cleans up empty parent directories after file removal
- Does NOT affect global templates in local data directory
- If `--dry-run` is specified, shows files that would be deleted without removing them
- **AGENTS.md Protection:**
  - If AGENTS.md has been customized (template marker removed) and `--force` is NOT specified:
    - AGENTS.md is skipped and preserved
    - User is informed to use `--force` to delete it
  - If AGENTS.md has been customized and `--force` IS specified:
    - AGENTS.md is deleted along with other templates
  - If AGENTS.md has NOT been customized (still has template marker):
    - AGENTS.md is deleted normally

### `remove` - Remove Agent-Specific Files

Remove agent-specific files from the current directory based on the Bill of Materials (BoM).

**Usage:**

```bash
# Remove specific agent's files
vibe-check remove --agent <agent> [--force] [--dry-run]

# Remove all agent-specific files (keeps AGENTS.md)
vibe-check remove --all [--force] [--dry-run]
```

**Options:**

- `--agent <string>` - AI coding agent (e.g., claude, copilot, codex, cursor)
- `--all` - Remove all agent-specific files (cannot be used with --agent)
- `--force` - Force removal without confirmation
- `--dry-run` - Preview what would be deleted without making changes

**Examples:**

```bash
# Remove Claude-specific files with confirmation
vibe-check remove --agent claude

# Remove Copilot files without confirmation
vibe-check remove --agent copilot --force

# Remove all agent-specific files (keeps AGENTS.md)
vibe-check remove --all

# Remove all agents with force
vibe-check remove --all --force

# Preview what would be deleted
vibe-check remove --all --dry-run
```

**Behavior:**

- Loads templates.yml from global storage to build Bill of Materials (BoM)
- BoM maps agent names to their target file paths in the workspace
- Only removes files that exist in the current directory
- Shows list of files to be removed before deletion
- Asks for confirmation unless `--force` is specified
- If `--dry-run` is specified, shows files that would be deleted without removing them
- Removes agent-specific files (instructions and prompts)
- Automatically cleans up empty parent directories
- **NEVER touches AGENTS.md** (use `purge` command to remove AGENTS.md)
- Does NOT affect global templates in local data directory
- If agent not found in BoM, shows list of available agents
- Cannot specify both `--agent` and `--all` (mutually exclusive)
- Must specify either `--agent` or `--all`

### `completions` - Generate Shell Completions

Generate shell completion scripts for various shells.

**Usage:**

```bash
vibe-check completions <shell>
```

**Arguments:**

- `<shell>` - Shell to generate completions for: `bash`, `zsh`, `fish`, `powershell`

**Examples:**

```bash
# Generate zsh completions
vibe-check completions zsh > ~/.zsh/completions/_vibe-check

# Generate bash completions
vibe-check completions bash > ~/.bash_completion.d/vibe-check

# Generate fish completions
vibe-check completions fish > ~/.config/fish/completions/vibe-check.fish

# Generate PowerShell completions
vibe-check completions powershell > vibe-check.ps1
```

**Behavior:**

- Outputs shell completion script to stdout
- User redirects output to appropriate location for their shell
- Supports bash, zsh, fish, and PowerShell

### `status` - Show Project Status

Display the current status of vibe-check in the project.

**Usage:**

```bash
vibe-check status
```

**Examples:**

```bash
# Show current project status
vibe-check status
```

**Output includes:**

- **Global Templates:** Whether templates are installed and their location
  - Available agents (from templates.yml)
  - Available languages (from templates.yml)
- **Project Status:**
  - AGENTS.md existence and customization status
  - Which agents are currently installed
- **Managed Files:** List of all vibe-check managed files in current directory

**Example output:**

```
vibe-check status

Global Templates:
  ✓ Installed at: /Users/.../vibe-check/templates
  → Available agents: claude, copilot, codex, cursor
  → Available languages: c, c++, rust, swift

Project Status:
  ✓ AGENTS.md: exists (customized)
  ✓ Installed agents: claude, copilot

Managed Files:
  • AGENTS.md
  • .claude/commands/init-session.md
  • CLAUDE.md
```

### `list` - List Available Options

List all available agents and languages from global templates.

**Usage:**

```bash
vibe-check list
```

**Examples:**

```bash
# List available agents and languages
vibe-check list
```

**Output includes:**

- **Available Agents:** All agents defined in templates.yml with installation status
- **Available Languages:** All languages defined in templates.yml

**Example output:**

```
vibe-check list

Available Agents:
  ✓ claude (installed)
  ○ codex
  ✓ copilot (installed)
  ○ cursor

Available Languages:
  • c
  • c++
  • rust
  • swift

→ Use 'vibe-check init --lang <lang> --agent <agent>' to install
```

**Behavior:**

- Shows all agents available in global templates with installation status
- Shows all languages available in global templates
- Agents show ✓ (installed) or ○ (available) based on file presence
- Languages are listed without status (content is merged into AGENTS.md)
- Provides guidance on how to install

## Repository Structure

```text
vibe-check/
├── Cargo.toml                  # Rust project manifest
├── Cargo.lock                  # Dependency lock file
├── .rustfmt.toml               # Rust formatting configuration
├── src/                        # Rust source code
│   ├── main.rs                 # Application entry point and CLI
│   ├── lib.rs                  # Library public API
│   ├── bom.rs                  # Bill of Materials structures and functions
│   ├── download_manager.rs     # DownloadManager for URL downloads
│   ├── template_manager.rs     # TemplateManager implementation
│   └── utils.rs                # Utility functions
├── LICENSE                     # MIT license
├── README.md                   # Main documentation
├── AGENTS.md                   # This file - primary instructions
├── templates/                  # Template files for various languages and frameworks
│   ├── templates.yml           # YAML configuration defining template structure
│   ├── AGENTS.md               # Template for project-specific agent instructions
│   ├── best-practices.md       # Best practices template (fragment)
│   ├── build-environment.md    # Generic build environment template (fragment)
│   ├── c-coding-conventions.md # C coding standards template (fragment)
│   ├── c-editor-config.ini     # EditorConfig for C projects
│   ├── c-format-instructions.yml  # clang-format config for C
│   ├── c-git-ignore.txt        # C .gitignore template
│   ├── c++-coding-conventions.md  # C++ coding standards template (fragment)
│   ├── c++-editor-config.ini   # EditorConfig for C++ projects
│   ├── c++-format-instructions.yml  # clang-format config for C++
│   ├── c++-git-ignore.txt      # C++ .gitignore template
│   ├── cmake-build-commands.md # CMake build commands template (fragment)
│   ├── core-principles.md      # Core principles template (fragment)
│   ├── git-attributes-common.txt  # Common .gitattributes template (cross-platform)
│   ├── git-workflow-conventions.md  # Git workflow template (fragment)
│   ├── make-build-commands.md  # Make build commands template (fragment)
│   ├── mission-statement.md    # Mission statement template (fragment)
│   ├── rust-coding-conventions.md  # Rust coding standards template (fragment)
│   ├── rust-build-commands.md  # Rust build commands template (fragment)
│   ├── rust-editor-config.ini  # EditorConfig for Rust projects
│   ├── rust-format-instructions.toml  # rustfmt config for Rust
│   ├── rust-git-ignore.txt     # Rust .gitignore template
│   ├── semantic-versioning.md  # Semantic versioning template (fragment)
│   ├── swift-coding-conventions.md  # Swift coding standards template (fragment)
│   ├── swift-build-commands.md # Swift build commands template (fragment)
│   ├── swift-editor-config.ini # EditorConfig for Swift projects
│   ├── swift-format-instructions.json  # swift-format config for Swift
│   ├── swift-git-ignore.txt    # Swift .gitignore template
│   ├── technology-stack.md     # Technology stack template (fragment)
│   ├── claude/
│   │   ├── CLAUDE.md           # Claude main instruction file
│   │   ├── CLAUDE-auto-redirect.md  # Auto-redirect to AGENTS.md
│   │   └── commands/
│   │       └── init-session.md # Claude initialization commands
│   ├── codex/
│   │   └── prompts/
│   │       └── init-session.md # Codex initialization prompts (placed in ~/.codex/)
│   ├── copilot/
│   │   ├── copilot-instructions.md  # GitHub Copilot instructions
│   │   ├── copilot-instructions-auto-redirect.md  # Auto-redirect to AGENTS.md
│   │   └── prompts/
│   │       └── init-session.prompt.md  # Copilot initialization prompts
├── CLAUDE.md                   # Claude-specific reference
├── .github/
│   └── copilot-instructions.md # GitHub Copilot reference
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

- Templates are stored in the local data directory (e.g., `$HOME/.local/share/vibe-check/templates` on Linux, `$HOME/Library/Application Support/vibe-check/templates` on macOS)
- All template operations are handled through the `TemplateManager` struct
- Template files are organized by language/framework and agent type
- Use standard file system operations for template access
- Validate template existence before operations
- Template downloads are controlled by `templates.yml` configuration file

**templates.yml Configuration:**

The `templates.yml` file defines which template files should be downloaded and where they should be placed. It has six main sections:

- `main` - Main AGENTS.md instruction file (primary source of truth)
  - Structure: `{source, target}`
- `agents` - Agent-specific configurations with instructions files and optional prompts
  - Each agent can have `instructions` (single file) and `prompts` (multiple files)
  - Structure: `{instructions: {source, target}, prompts: [{source, target}, ...]}`
- `languages` - Language-specific coding standards templates
  - Each language has a `files` array with source/target mappings
  - Structure: `{files: [{source, target}, ...]}`
- `integration` - Tool/workflow integration templates (e.g., git workflows)
  - Each integration has a `files` array with source/target mappings
  - Structure: `{files: [{source, target}, ...]}`
- `principles` - Core principles and general guidelines
  - Simple array of file mappings with source/target pairs
  - Structure: `[{source, target}, ...]`
- `mission` - Mission statement, purpose, and project overview
  - Simple array of file mappings with source/target pairs
  - Structure: `[{source, target}, ...]`

Each file entry has:

- `source` - Source path in the templates repository
- `target` - Target path where file will be placed (supports placeholders)

Placeholders:

- `$workspace` - Resolves to current directory
- `$userprofile` - Resolves to user's home directory
- `$instructions` - Indicates fragment to be merged into main AGENTS.md at insertion points

Insertion Points (when using `$instructions` placeholder):

- `<!-- {mission} -->` - Where mission/purpose and project overview are inserted
- `<!-- {principles} -->` - Where core principles and guidelines are inserted
- `<!-- {languages} -->` - Where language-specific coding standards are inserted
- `<!-- {integration} -->` - Where tool/workflow integration content is inserted

The insertion point comments are preserved in the final merged AGENTS.md for reference.

The system downloads templates.yml first; if download fails, the operation stops with an error.

**TemplateManager Functions:**

- `update(lang: &str, agent: &str, force: bool, from: Option<&str>)` - Update templates for specific language and agent
  - `lang` - Programming language or framework identifier
  - `agent` - AI coding agent identifier
  - `force` - If true, overwrite existing templates without confirmation
  - `from` - Optional path or URL to copy/download templates from
  - Downloads and parses `templates.yml` to determine which files to update
  - If global templates don't exist and `from` is None, downloads from default GitHub repository
  - If `from` is specified, copies/downloads templates from that location first
  - Collects files dynamically from YAML configuration (main + mission + principles + languages + integration + agent)
  - Merges fragments with `$instructions` placeholder into main AGENTS.md at insertion points
  - Removes template marker from merged AGENTS.md to indicate customization
  - Resolves placeholders ($workspace, $userprofile, $instructions) in target paths
  - Detects if AGENTS.md has been customized by checking for missing template marker
  - Stops operation if AGENTS.md is customized and `force` is false
  - Copies template files from local data directory to resolved target paths

- `purge(force: bool)` - Purge all vibe-check files from current directory
  - `force` - If true, purge without confirmation
  - Removes agent instruction directories (.claude, .copilot, .codex) from current directory
  - Removes language template files for supported languages (c, c++, swift, rust) from current directory
  - Does NOT affect global templates in local data directory

### CLI Command Implementation

- Use clap's derive API for command structure
- Implement subcommands as separate structs
- Validate arguments early and provide clear error messages
- Use `owo-colors` for user-friendly terminal output
- Provide helpful examples in `--help` output

### Code Formatting Tools

The project includes language-specific formatting configurations:

- **C/C++**: Uses clang-format
  - Configuration: `.clang-format` (YAML format)
  - Documentation: <https://clang.llvm.org/docs/ClangFormatStyleOptions.html>
  - C uses 120 character line limit, C17 standard
  - C++ uses 160 character line limit, latest standard

- **Rust**: Uses rustfmt
  - Configuration: `.rustfmt.toml` (TOML format)
  - Documentation: <https://rust-lang.github.io/rustfmt/>
  - Edition 2024, 167 character line limit

- **Swift**: Uses swift-format
  - Configuration: `.swift-format` (JSON format)
  - Documentation: <https://github.com/apple/swift-format/blob/main/Documentation/Configuration.md>
  - 147 character line limit

- **EditorConfig**: Cross-editor baseline formatting
  - Configuration: `.editorconfig` (INI format)
  - Documentation: <https://editorconfig.org/>
  - Provides charset control (C uses latin1, others use utf-8)
  - Handles line endings, trailing whitespace, final newline
  - Complements language-specific formatters

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

- Store reusable templates in local data directory (e.g., `$HOME/.local/share/vibe-check/templates` on Linux, `$HOME/Library/Application Support/vibe-check/templates` on macOS)
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

## Quick Wins TODO List

**Priority improvements to enhance vibe-check usability and functionality:**

- [x] **Add `vibe-check list` command** - Show available languages, agents, and what's installed locally
  - Display all supported languages from templates.yml
  - Display all supported agents from templates.yml
  - Show which global templates are downloaded
  - Show which local templates are installed in current directory
  - Include file paths and installation status

- [ ] **Add `vibe-check validate` command** - Verify local installation is complete and consistent
  - Check if global templates exist and are complete
  - Verify local AGENTS.md exists and is properly formatted
  - Validate agent-specific files are present for configured agent
  - Check language-specific files match selected language
  - Report missing or inconsistent files with actionable suggestions

- [ ] **Add template preview functionality** - Let users see what will be installed before running init
  - Add `--preview` flag to init and update commands
  - Display list of files that would be created/modified
  - Show file paths with resolved placeholders
  - Include file sizes and brief descriptions
  - No actual file operations performed in preview mode

- [ ] **Add selective installation option** - Allow users to skip specific template types
  - Add `--skip` flag accepting comma-separated list (e.g., `--skip editorconfig,gitignore`)
  - Skip options: `editorconfig`, `format`, `gitignore`, `gitattributes`
  - Maintain AGENTS.md and agent instructions (never skippable)
  - Document skip options in CLI help text

---

## Future CLI Enhancements

**Planned improvements for CLI usability and modern tooling standards:**

### High Priority

- [x] **Add `--dry-run` flag** - Preview changes without applying them
  - Available on `init`, `update`, `purge`, and `remove` commands
  - Show exactly which files would be created, modified, or deleted
  - Display file paths with color coding (green=create, yellow=modify, red=delete)
  - Exit with success after preview (no actual changes made)
  - Pairs well with `--preview` flag for init command

- [x] **Add `status` command** - Show current project state
  - Display which agent is configured (if any)
  - Show which language templates are installed
  - Indicate if AGENTS.md has been customized
  - List all vibe-check managed files in current directory
  - Show global template status (downloaded, version/date)

### Medium Priority

- [ ] **Add `--verbose` / `-v` flag** - Show detailed output
  - Display full file paths during operations
  - Show individual file copy/delete actions
  - Include timing information for operations
  - Stack with `-vv` for even more detail (debug level)

- [ ] **Add `--no-color` flag** - Disable colored output
  - Essential for CI/CD environments
  - Required when piping output to files or other commands
  - Respect `NO_COLOR` environment variable (standard convention)
  - Consider `CLICOLOR` and `CLICOLOR_FORCE` for compatibility

- [ ] **Add `diff` command** - Compare local vs template versions
  - Show differences between installed files and global templates
  - Highlight customizations made to AGENTS.md
  - Support `--stat` for summary view (like git diff --stat)
  - Color-coded output (additions in green, deletions in red)

### Low Priority

- [ ] **Add `--quiet` / `-q` flag** - Suppress non-essential output
  - Only show errors and critical warnings
  - Useful for scripting and automation
  - Exit codes remain meaningful for error detection

- [ ] **Add `--json` output flag** - Machine-readable output
  - Available on `list`, `status`, and `validate` commands
  - Structured JSON output for scripting and automation
  - Include all relevant metadata (paths, versions, status)
  - Follow JSON Lines format for streaming output

- [ ] **Add man page generation** - Unix manual pages
  - Use `clap_mangen` crate for generation
  - Generate during build or as separate command
  - Include in release artifacts for package managers
  - Document installation in README

### Implementation Guidelines

- Implement flags in order of priority (high → medium → low)
- Each feature should be a separate commit with version bump
- High priority items: MINOR version bump (new features)
- Medium/low priority items: MINOR version bump (new features)
- Update CLI help text and AGENTS.md documentation
- Add changelog entry to Recent Updates & Decisions section
- Consider backward compatibility for all changes

**Implementation Notes:**

- Each TODO should be completed as a separate commit
- Update this list as items are completed (check boxes)
- Add new Quick Wins to the list as they are identified
- Include version bump in the same commit (MINOR for new features)
- Add detailed entry to Recent Updates & Decisions section when complete

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

### 2025-11-09 (Clear Command Update)

- Updated clear command to only delete local templates from current directory
- Modified TemplateManager::clear() to remove agent directories and language template files from current directory only
- Global templates in $HOME/.config/vibe-check/templates are now preserved
- Added safeguards to prevent removal of important files (AGENTS.md, README.md, LICENSE.md, CHANGELOG.md, CONTRIBUTING.md)
- Updated CLI command description to reflect local-only clearing behavior
- Reasoning: Users should be able to clean up project-specific templates without affecting their global template storage

### 2025-11-09 (Supported Languages)

- Restricted clear command to only remove templates for currently supported languages: c++, swift, and rust
- Removed support for cmake.md, general.md, git.md, python.md, typescript.md, javascript.md from clear operation
- Updated documentation to reflect the restricted language list
- Reasoning: Focus on core supported languages to prevent accidental removal of unrelated markdown files

### 2025-11-09 (Template Download)

- Implemented template download functionality using simple HTTP requests
- Added support for downloading templates from GitHub URLs via raw.githubusercontent.com
- Parse GitHub tree URLs and convert to raw content URLs for direct file download
- Download known template files including AGENTS.md, language templates, and agent-specific instructions
- Made language templates optional in update function to support repositories without language-specific files
- Added fallback logic to try both lowercase and capitalized file names for language templates
- Updated Cargo.toml dependencies: reqwest with json feature, serde, serde_json
- Reasoning: Enable users to download templates from GitHub without requiring API authentication or complex setup

### 2025-11-09 (Commit Protocol Clarification)

- Added explicit Commit Protocol section with clear rules
- Specified exact phrases that trigger commit actions
- Defined workflow: implement, build, test, STOP, wait for explicit commit instruction
- Emphasized never staging or committing automatically under any circumstances
- Reasoning: Previous instruction was violated repeatedly, clearer protocol needed to prevent automatic commits

### 2025-11-09 (Checksum Management)

- Modified checksum creation to happen immediately after downloading or copying templates to global storage
- Added create_checksums_for_directory method to recursively create checksums for copied templates
- Removed verify_or_create_checksum method as checksums are now created during download/copy
- Checksums are only created or updated during template download/copy operations, not during local template updates
- Updated documentation to clarify checksum lifecycle
- Reasoning: Simplify checksum management and ensure global template integrity is established immediately

### 2025-10-05

- Created AGENTS.template.md as comprehensive template for use in other projects
- Template includes all agent-specific initialization prompts embedded in one file
- Template incorporates the basic structure from README.md Step 1
- Added AGENTS.template.md to repository structure documentation
- Updated file organization best practices to reference the template

### 2025-11-09 (Documentation Sync)

- Updated Technology Stack to reflect all current dependencies
- Added sha2 v0.10 and hex v0.4 for checksum functionality
- Added chrono v0.4 for timestamp generation
- Added serde v1.0 and serde_json v1.0 for serialization
- Specified reqwest version v0.12 with blocking and json features
- Added .rustfmt.toml to Repository Structure documentation
- Reasoning: Documentation should accurately reflect the current codebase state

### 2025-11-11 (Template Storage Location)

- Changed template storage location from `$HOME/.config/vibe-check/templates` to platform-specific local data directory
- Added dirs crate v5.0 dependency for cross-platform directory path resolution
- Updated TemplateManager::new() to use `dirs::data_local_dir()` instead of HOME environment variable
- Templates now stored in `$HOME/.local/share/vibe-check/templates` on Linux and `$HOME/Library/Application Support/vibe-check/templates` on macOS
- Cache/backups remain in platform-specific cache directory via `dirs::cache_dir()`
- Updated all documentation to reflect new storage locations
- Reasoning: Using platform-appropriate directories via dirs crate provides better cross-platform compatibility and follows OS-specific conventions for application data storage

### 2025-11-11 (Template Configuration)

- Created templates.yml configuration file to control which templates are downloaded
- Added serde_yaml v0.9 dependency for YAML parsing
- Added TemplateConfig and TemplateEntry structs to represent configuration
- Implemented load_template_config() method to load and parse templates.yml
- Updated download_templates_from_url() to use YAML configuration instead of hardcoded file lists
- Configuration has three sections: agents, languages, and general templates
- System downloads templates.yml first; falls back to default configuration if not found
- Updated documentation to describe templates.yml structure and usage
- Reasoning: YAML configuration makes template management more flexible and maintainable, allowing users to customize which templates are downloaded without modifying code

### 2025-11-11 (Fragment Merging System)

- Renamed 'general' section to 'principles' in templates.yml for clarity
- Added 'main' section for AGENTS.md template (primary source of truth)
- Added 'integration' section for tool/workflow templates (e.g., git workflows)
- Renamed 'instruction' field to 'instructions' in agent configurations
- Renamed c++.md to c++-coding-conventions.md and git.md to git-workflow-conventions.md
- Implemented new $instructions placeholder for fragment files
- Added insertion points: {languages}, {integration}, {principles}
- Created merge_fragments() method to merge fragments into main AGENTS.md
- Updated TemplateConfig struct with MainConfig and IntegrationConfig
- Fragments with $instructions placeholder are merged into AGENTS.md at corresponding insertion points
- Updated download and update logic to handle main file, integration files, and fragment merging
- Removed fallback configuration from template manager (templates.yml now required)
- Updated documentation in templates.yml, AGENTS.md to reflect new system
- Reasoning: Fragment merging allows for modular, maintainable instruction files where language-specific and integration-specific content is merged into a single AGENTS.md, creating a unified instruction file for each project while maintaining template modularity

### 2025-10-03

- Initial AGENTS.md setup
- Established core coding standards and conventions
- Created agent-specific reference files (CLAUDE.md, .github/copilot-instructions.md)
- Defined repository structure and governance principles

### 2025-11-12 (Template Configuration Simplification)

- Simplified $instructions placeholder in templates.yml to remove redundant filename portion
- Changed targets from $instructions/filename.md to just $instructions for all fragment files
- Updated documentation in AGENTS.md and README.md to reflect fragment merging system
- Added comprehensive documentation of six template configuration sections: main, agents, languages, integration, principles, mission
- Documented insertion points and placeholder behavior in README.md
- Updated repository structure listings to reflect actual template files
- Fixed outdated storage paths in FAQ and customization sections
- Reasoning: The filename portion in $instructions/filename.md was redundant since fragments are merged into AGENTS.md at insertion points, not copied as separate files. Simplifying to just $instructions improves clarity and reduces confusion.

### 2025-11-14 (Build Command Templates)

- Created rust-build-commands.md template with comprehensive Rust/Cargo build commands
- Created cmake-build-commands.md template with comprehensive CMake/C++ build commands
- Added both templates to languages section in templates.yml for Rust and C++ respectively
- Templates include setup, development, build/deploy, documentation, and dependency management sections
- Both templates emphasize using debug builds during development and release builds only for final testing/deployment
- Updated AGENTS.md repository structure to include new template files
- Updated README.md repository structure, supported languages section, and template storage documentation
- Updated last modified dates in both AGENTS.md and README.md
- Reasoning: Language-specific build command templates provide developers with quick reference to common commands and best practices, reducing cognitive load and ensuring consistent build workflows across projects. Separating Rust and CMake build commands into dedicated templates makes them more maintainable and easier to customize for different project types.

### 2025-11-14 (Template Marker Protection)

- Added template marker to templates/AGENTS.md to identify unmerged template files
- Marker: `<!-- VIBE-CHECK-TEMPLATE: This marker indicates an unmerged template. Do not remove manually. -->`
- Implemented automatic marker removal during fragment merging in merge_fragments method
- Added is_file_customized method to check if local files have been customized by detecting missing marker
- Updated update method to check AGENTS.md for customization before overwriting
- Protection now works for both init and update commands
- Modified files detected through: marker removal for AGENTS.md, checksum comparison for other files
- Updated documentation to reflect new protection mechanism
- Reasoning: Using a marker in the template file provides a reliable way to detect if a merged AGENTS.md has been customized by the user. When the marker is removed during merging, any subsequent update will detect the file as customized and require --force to overwrite, preventing accidental loss of user customizations.

### 2025-11-14 (Init Command Enhancement)

- Changed init command behavior to always update global templates first
- Modified --force flag for init command to control local file overwriting (not global template clearing)
- Init command now: 1) Updates global templates, 2) Checks for local modifications, 3) Updates local files
- If local AGENTS.md is customized (marker removed) and --force not specified, init aborts with error
- Made download_or_copy_templates public method for direct access from main
- Updated CLI documentation to reflect new init command behavior
- Updated examples to show --force usage for overwriting local files
- Reasoning: Always updating global templates ensures users get the latest templates on init. The --force flag now has a clearer purpose: controlling whether to overwrite customized local files. This makes the workflow more intuitive and safer for users.

### 2025-11-14 (Checksum System Removal)

- Removed SHA-256 checksum system entirely (calculate_checksum, has_local_modifications, create_checksums_for_directory methods)
- Removed sha2 and hex dependencies from Cargo.toml
- Removed all checksum creation code after template downloads
- Removed checksum comparison for non-AGENTS.md files
- Only AGENTS.md is now protected using the template marker system
- Backups are always created before any file modifications
- Simplified modification detection: only checks AGENTS.md for missing marker
- Updated documentation to remove all checksum references
- Reasoning: The checksum system added unnecessary complexity. The template marker provides sufficient protection for AGENTS.md (the main file users customize), and other files (agent instructions, prompts) are rarely modified by users. Always creating backups provides safety without the overhead of checksum management.

### 2025-11-14 (Update Command Simplification)

- Changed update command to not download global templates automatically
- Update command now requires global templates to exist (errors if missing)
- Removed automatic download fallback from update method
- Update command now behaves exactly like init except without initial download
- Users must run init first to set up global templates
- Added comprehensive update command documentation to CLI Commands section
- Updated method documentation to reflect new behavior
- Reasoning: Clear separation of concerns - init downloads and sets up, update syncs from existing global templates. This makes the behavior more predictable and prevents unexpected downloads during update operations.

- Updated repository structure listings to reflect actual template files
- Fixed outdated storage paths in FAQ and customization sections
- Reasoning: The filename portion in $instructions/filename.md was redundant since fragments are merged into AGENTS.md at insertion points, not copied as separate files. Simplifying to just $instructions improves clarity and reduces confusion.

### 2025-11-14 (Clear Command AGENTS.md Protection)

- Enhanced clear command to detect and protect customized AGENTS.md files
- Added AGENTS.md customization check using existing is_file_customized method
- If AGENTS.md is customized (marker removed) and --force is NOT specified:
  - AGENTS.md is skipped and preserved
  - User is informed with yellow warning message
  - Suggests using --force to delete anyway
- If AGENTS.md is customized and --force IS specified:
  - Backup is created (as with all files at start of clear operation)
  - AGENTS.md is deleted along with other templates
- If AGENTS.md is NOT customized (still has template marker):
  - AGENTS.md is deleted normally without special handling
- Added comprehensive clear command documentation to CLI Commands section
- Reasoning: Users should not accidentally lose customized AGENTS.md files when clearing templates. The marker-based detection provides reliable protection, and --force flag gives users explicit control to override when needed. This is consistent with init/update command behavior for modified files.

### 2025-11-15 (Auto-Redirect Templates)

- Added CLAUDE-auto-redirect.md template in claude/ directory
- Added copilot-instructions-auto-redirect.md template in copilot/ directory
- Both auto-redirect templates contain mandatory instruction to read AGENTS.md before proceeding
- Templates ensure agents reference the single source of truth (AGENTS.md) consistently
- Updated repository structure documentation to include new auto-redirect templates
- Reasoning: Auto-redirect templates provide an additional safeguard to ensure AI coding agents always consult AGENTS.md as the primary instruction source. This reinforces the single source of truth principle and prevents agents from relying solely on abbreviated instruction files.

### 2025-11-15 (Semantic Versioning Protocol)

- Added Semantic Versioning Protocol section to AGENTS.md
- Defined version format MAJOR.MINOR.PATCH with clear increment rules
- PATCH for bug fixes, MINOR for new features, MAJOR for breaking changes
- Specified process: determine change type, update Cargo.toml, include in same commit
- Version changes included with code changes, not as separate commits
- Added examples for each version type increment
- Reasoning: Automatic semantic versioning tracking ensures consistent version management and clear communication of change significance. Including version bumps in the same commit as code changes maintains atomic commits and simplifies version history tracking.

### 2025-11-15 (C Coding Conventions Template)

- Created c-coding-conventions.md template based on KString project coding standards
- Added comprehensive C17 coding conventions including const correctness, constant-left comparisons, secure API design
- Documented naming conventions, memory management, error handling, and platform portability guidelines
- Added C language entry to templates.yml with c-coding-conventions.md and cmake-build-commands.md
- Updated supported languages list in AGENTS.md to include C (c, c++, swift, rust)
- Reasoning: C language template provides standardized coding conventions for C projects following modern best practices from real-world C library implementation (KString). The template emphasizes security (explicit sizes, const correctness), portability (C17 standard, cross-platform), and maintainability (clear naming, defensive programming), making it valuable for any C project development.

### 2025-11-16 (Template Source Update)

- Updated default template download URL from feature/template-management branch to develop branch
- Changed URL in src/main.rs from tree/feature/template-management/templates to tree/develop/templates
- Updated documentation in AGENTS.md and README.md to reflect new default URL
- Added Cursor agent support with initialization prompts
- Added Swift language support with coding conventions and build commands templates
- Bumped version from 1.1.1 to 1.2.0 (MINOR version for new features: Cursor agent and Swift language)
- Reasoning: The develop branch is now the primary development branch for templates, making it more appropriate as the default source. This ensures users get the latest stable template updates without needing to specify a custom URL. New agent and language support are new features that maintain backward compatibility, requiring MINOR version increment per Semantic Versioning Protocol.

### 2025-11-17 (Init/Update Command Bug Fix)

- Fixed bug where init and update commands would abort entire operation when AGENTS.md was customized
- Modified update method in template_manager.rs to skip only AGENTS.md instead of aborting completely
- Changed behavior: if AGENTS.md is customized and --force is not specified, skip AGENTS.md but continue copying other files
- Added clear user messaging explaining which files are being skipped and which are being updated
- Maintained backup behavior for all existing files before modifications
- --force flag still overwrites customized AGENTS.md when specified
- Bumped version from 1.2.0 to 1.2.1 (PATCH version for bug fix)
- Reasoning: Previous behavior was too aggressive - users couldn't update agent instructions or prompts without either overwriting their customized AGENTS.md or having the entire operation fail. New behavior is more flexible and user-friendly, allowing partial updates while protecting customized content. This aligns with the clear command's behavior for customized files.

### 2025-11-17 (Remove Command and BoM Module)

- Created new `src/bom.rs` module for Bill of Materials (BoM) structures and functions
- Added `BillOfMaterials` struct that maps agent names to their workspace file paths
- Implemented `BillOfMaterials::from_config()` to parse templates.yml and build BoM
- Moved template configuration structures (FileMapping, TemplateConfig, AgentConfig, etc.) from template_manager.rs to bom.rs
- Refactored template_manager.rs to use bom module, reducing code duplication
- Added new `remove` CLI command with `--agent` and `--force` options
- Implemented `TemplateManager::remove()` method to delete agent-specific files based on BoM
- Remove command loads BoM from global templates.yml and removes only existing files for specified agent
- Shows file list and prompts for confirmation unless --force is specified
- Creates backup before removal and automatically cleans up empty parent directories
- Updated lib.rs to expose BillOfMaterials in public API
- Added comprehensive remove command documentation to AGENTS.md
- Updated repository structure documentation to include bom.rs module
- Bumped version from 1.2.1 to 1.3.0 (MINOR version for new feature)
- Reasoning: The remove command provides users with a clean way to remove agent-specific files without affecting other templates or the main AGENTS.md file. Separating BoM logic into its own module improves code organization and maintainability. The BoM-based approach ensures accurate file tracking and makes the system extensible for future features. This is a new feature that maintains backward compatibility, requiring MINOR version increment per Semantic Versioning Protocol.

### 2025-11-17 (Backup Feature Removal)

- Removed automatic backup functionality from all commands (init, update, clear, remove)
- Removed `cache_dir` field from `TemplateManager` struct
- Removed `create_backup()` method and `get_timestamp()` helper function
- Removed `chrono` dependency from Cargo.toml (no longer needed for timestamps)
- Removed all calls to `create_backup()` in update, clear, and remove methods
- Updated all documentation to remove references to backups and cache directory
- Updated Technology Stack section to remove chrono dependency
- Bumped version from 1.3.0 to 2.0.0 (MAJOR version for breaking change)
- Reasoning: The automatic backup feature added complexity without clear evidence of usefulness. Users can rely on version control (git) for tracking changes to their workspace files. Removing this feature simplifies the codebase and eliminates the chrono dependency. This is a breaking change because users may have been relying on automatic backups, requiring MAJOR version increment per Semantic Versioning Protocol. The decision to remove backups will be revisited if user feedback indicates the feature was valuable.

### 2025-11-17 (Clippy Warning Fixes)

- Added `Default` implementation for `BillOfMaterials` struct in src/bom.rs
- Changed useless `format!()` to `.to_string()` in src/template_manager.rs
- All clippy warnings now resolved
- Reasoning: Following clippy suggestions improves code quality and idiomatic Rust usage. The Default trait implementation provides a standard way to create an empty BillOfMaterials, and using .to_string() instead of format!() is more efficient when no formatting is needed.

### 2025-11-17 (Optional Init Parameters)

- Made `--lang` and `--agent` parameters optional for init command
- Changed Init struct fields from `String` to `Option<String>` in src/main.rs
- Implemented pattern matching logic to handle all parameter combinations
- When both parameters provided: downloads global templates and installs to project
- When only one parameter provided: displays warning message (both required for installation)
- When neither parameter provided: only downloads global templates (no local installation)
- Updated CLI help text to show parameters as optional: `vibe-check init [OPTIONS]`
- Updated init command documentation in both AGENTS.md and README.md
- Added new usage examples showing download-only mode
- Reasoning: Separating template download from installation provides better flexibility. Users can pre-download templates or update their global template cache without modifying their current project. This is useful for offline work, template caching, or simply exploring available templates before committing to a specific language/agent combination. The feature maintains backward compatibility (old usage still works) while enabling new workflows.

### 2025-11-17 (Remove Command --all Option)

- Added `--all` option to `remove` command for removing all agent-specific files
- Changed `--agent` parameter from required to optional in CLI definition
- Implemented mutual exclusion validation between `--agent` and `--all`
- Added `remove_all()` method in TemplateManager (135 lines)
- Method loads BoM, collects files from all agents, deduplicates, and removes files
- Prompts for confirmation unless `--force` is specified
- Cleans up empty parent directories automatically
- **NEVER touches AGENTS.md** (consistent with single-agent remove behavior)
- Updated CLI help text and command documentation in AGENTS.md and README.md
- Bumped version from 2.0.0 to 2.0.0 (MINOR change - new feature, backward compatible)
- Reasoning: The `remove --all` option fills the gap between surgical single-agent removal and nuclear `clear` command. It removes all agent-specific files while preserving customized AGENTS.md and language templates. This provides users with more granular control over cleanup operations.

### 2025-11-17 (Clear Command BoM Integration)

- Updated `clear` command to use Bill of Materials instead of hardcoded agent names
- Removed hardcoded agent directory list (`.claude`, `.copilot`, `.codex`)
- Clear command now dynamically discovers agents from templates.yml
- Uses same BoM-based approach as `remove` command for consistency
- Gracefully handles missing templates.yml (skips agent removal, continues with AGENTS.md)
- All three removal commands now use BoM system consistently
- Improved code maintainability and eliminated hardcoded agent names
- Reasoning: Using BoM makes the codebase future-proof - new agents added to templates.yml are automatically supported without code changes. This ensures consistency across all removal operations and reduces maintenance burden.

### 2025-11-17 (Clear Command Simplification - Breaking Change)

- Removed 30+ lines of outdated language template scanning logic from `clear` command
- Removed hardcoded checks for legacy files (c++-coding-conventions.md, swift.md, rust.md)
- Simplified AGENTS.md handling logic (direct check instead of directory scanning)
- Clear command now only removes: 1) agent-specific files (via BoM), 2) AGENTS.md
- Language templates are no longer standalone files (merged into AGENTS.md via $instructions placeholder)
- Legacy scanning code was dead code looking for files that no longer exist
- Bumped version from 2.0.0 to 3.0.0 (MAJOR version for breaking change)
- **Breaking Change:** Clear command now always targets AGENTS.md for removal (with protection for customized files)
- Users who want to keep AGENTS.md should use `remove --all` instead
- Reasoning: The language template scanning was legacy code from when templates were separate files. Modern vibe-check merges language templates into AGENTS.md as fragments, making the scanning logic obsolete. Removing it simplifies the codebase and clarifies the command's purpose: complete local cleanup of all vibe-check files. This is a breaking change because the command behavior changed - it now always attempts to remove AGENTS.md instead of only agent-specific files.

### 2025-11-20 (Documentation Cleanup)

- Removed outdated SHA checksum references from documentation comments in template_manager.rs
- Removed "Creates SHA checksums" line from download_or_copy_templates() doc comment
- Removed "Creates SHA checksums" line from download_templates_from_url() doc comment
- No code changes, only documentation cleanup
- Version remains 3.0.0 (documentation-only change, no version bump needed)
- Reasoning: The checksum system was removed on 2025-11-14, but outdated documentation comments remained in the code. This cleanup ensures documentation accurately reflects the current implementation and prevents confusion for developers reading the code.

### 2025-11-20 (Init Command Output Cleanup)

- Removed redundant "Global templates downloaded successfully" message from init command
- When running init without --lang and --agent, only "Templates downloaded successfully" is shown (from download_or_copy_templates)
- Followed by informational message about running with --lang and --agent
- No version bump (PATCH-level change, cosmetic output improvement)
- Reasoning: The init command was displaying two success messages when downloading templates without installing to a project. The message from download_or_copy_templates already confirms successful download, making the additional message redundant and cluttering the output.

### 2025-11-20 (Code Refactoring - DRY Principle)

- Extracted common code duplication in template_manager.rs using closures
- Created download_entry closure in download_templates_from_url to eliminate repetitive download logic
- Created process_entry closure in update to eliminate repetitive file processing logic
- Reduced code duplication for processing principles, missions, languages, and integrations
- No behavioral changes, only code organization improvement
- Bumped version from 3.0.0 to 3.0.1 (PATCH version for internal improvements)
- Reasoning: Both download_templates_from_url and update functions contained nearly identical code blocks for processing principles, missions, languages, and integrations. Using closures eliminates this duplication, makes the code more maintainable, and follows the DRY principle as specified in General Instructions. The refactoring reduces approximately 120 lines of repetitive code while maintaining identical functionality. Version bump allows distinguishing this improved codebase from the released 3.0.0.

### 2025-11-24 (Format Instructions and EditorConfig Templates)

- Added language-specific format instruction templates for C, C++, Rust, and Swift
- Created c-format-instructions.yml with 120 char limit and C17 standard
- Created c++-format-instructions.yml with 160 char limit and latest C++ standard
- Created rust-format-instructions.toml with 167 char limit and edition 2024
- Created swift-format-instructions.json with 147 char limit
- Added language-specific EditorConfig templates for all supported languages
- Created c-editor-config.ini with latin1 charset for C source files
- Created c++-editor-config.ini with utf-8 charset for modern C++
- Created rust-editor-config.ini with utf-8 charset (enforced by compiler)
- Created swift-editor-config.ini with utf-8 charset for Swift projects
- Added make-build-commands.md template for Make/Makefile workflows
- Updated templates.yml to include format instructions and EditorConfig for all languages
- Updated AGENTS.md repository structure to include all new template files
- Added Code Formatting Tools section with documentation links
- Reasoning: Format instructions provide language-specific code formatting rules that complement EditorConfig's cross-editor baseline settings. EditorConfig handles charset control (critical for C projects using latin1), line endings, and trailing whitespace - settings that language-specific formatters cannot manage. The make-build-commands.md complements cmake-build-commands.md for projects using Make instead of CMake. All templates follow consistent structure and emphasize best practices.

### 2025-11-24 (Git Support - .gitignore and .gitattributes)

- Created git-attributes-common.txt template for cross-platform line ending normalization
- Template uses `* text=auto` for automatic line ending handling on both Windows and Unix
- Explicitly marks text files for normalization and binary files to prevent conversion
- Sets diff drivers for C, C++, Rust, and Swift source files
- Created language-specific .gitignore templates with both Windows and Unix patterns
- Created c-git-ignore.txt with C artifacts (object files, static/shared libraries, executables, CMake cache)
- Created c++-git-ignore.txt with C++ artifacts (similar to C plus C++-specific patterns)
- Created rust-git-ignore.txt with Rust artifacts (target/, executables, Cargo.lock for binaries)
- Created swift-git-ignore.txt with Swift artifacts (.build/, swiftmodule files, platform-specific)
- Each .gitignore includes common OS files (.DS_Store, Thumbs.db) and editor files (.vscode/, .idea/)
- Added git-attributes-common.txt to integration.git.files section in templates.yml
- Added language-specific .gitignore templates to each language section in templates.yml
- Updated AGENTS.md repository structure to include new git-related template files
- No version bump required (template-only changes, no code changes)
- Reasoning: Git version control templates are essential for maintaining consistent repository behavior across platforms and team members. The common .gitattributes file ensures proper line ending handling when the repository is cloned on different platforms, while language-specific .gitignore files prevent build artifacts and editor files from being committed. Cross-platform support is critical as developers may use Windows, macOS, or Linux, and both .gitignore templates and .gitattributes must work correctly on all platforms once committed to the repository.

### 2025-11-28 (Command Swap - init and update)

- Swapped the meanings of `init` and `update` CLI commands
- **New `update` command**: Downloads/updates global templates from source
  - Removed `--lang` and `--agent` parameters (no longer needed)
  - Removed `--force` parameter (just overwrites global templates)
  - Added `--from` option (optional, defaults to GitHub repository)
- **New `init` command**: Sets up agent instructions in project directory
  - Made `--lang` and `--agent` parameters required
  - Removed `--from` option (global template source handled by `update`)
  - Kept `--force` for overwriting customized local files
  - Automatically downloads global templates if they do not exist
- Added `has_global_templates()` method to TemplateManager
- Updated CLI documentation to reflect new command behavior
- Bumped version from 3.0.1 to 4.0.0 (MAJOR version for breaking CLI change)
- Reasoning: The swapped meanings are more intuitive. The `update` command now handles updating the global template cache, while `init` focuses on initializing a project with agent instructions. This separation of concerns makes the workflow clearer: first update templates, then init a project. The auto-download fallback in init ensures backward compatibility for users who skip the update step.

### 2025-11-28 (Command Rename - clear to purge)

- Renamed `clear` command to `purge` for better clarity and autocompletion safety
- Renamed `clear()` method to `purge()` in TemplateManager
- Updated all related documentation and user-facing messages
- Bumped version from 4.0.0 to 4.0.1 (PATCH version for rename)
- Reasoning: The `clear` and `remove` commands had similar names causing confusion about their different purposes. Renaming to `purge` makes the distinction clearer: `remove` surgically removes agent files while preserving AGENTS.md, whereas `purge` completely removes all vibe-check files. Additionally, `purge` starts with a different letter than `remove`, preventing accidental selection via shell autocompletion.

### 2025-11-28 (Extract Download Manager Module)

- Created new `src/download_manager.rs` module for URL and download operations
- Moved `parse_github_url()`, `download_file()`, and `download_templates_from_url()` from template_manager.rs
- Created `DownloadManager` struct to encapsulate download functionality
- Simplified `load_template_config()` in TemplateManager to only load from local files
- Updated `download_or_copy_templates()` to use DownloadManager for URL downloads
- Exposed `DownloadManager` in public API via lib.rs
- Reduced template_manager.rs from 1059 lines to approximately 835 lines
- Bumped version from 4.0.1 to 4.0.2 (PATCH version for internal refactoring)
- Reasoning: The template_manager.rs module was too large at over 1000 lines. Extracting URL parsing and download logic into a separate module improves code organization, maintainability, and follows the single responsibility principle. The DownloadManager can now be tested and modified independently from template management logic.

### 2025-11-28 (Add File System Utilities)

- Added `copy_file_with_mkdir()` utility to utils.rs for copying files with automatic parent directory creation
- Added `remove_file_and_cleanup_parents()` utility to utils.rs for removing files and cleaning up empty parent directories
- Updated `update()`, `purge()`, `remove()`, and `remove_all()` in template_manager.rs to use new utilities
- Reduced code duplication across file removal operations
- Exported new utilities in public API via lib.rs
- Bumped version from 4.0.2 to 4.0.3 (PATCH version for internal refactoring)
- Reasoning: The same file operation patterns were repeated multiple times across template_manager.rs. Extracting these into reusable utilities in utils.rs follows the DRY principle and makes the code more maintainable.

### 2025-11-28 (Template Manager Slimdown)

- Removed dead `clear_global_templates()` function (never called)
- Added `confirm_action()` utility to utils.rs for user confirmation prompts
- Consolidated `remove()` and `remove_all()` into single `remove(agent: Option<&str>, force: bool)` method
- Updated `purge()`, and `remove()` to use the new `confirm_action()` utility
- Reduced template_manager.rs by approximately 70 lines
- Bumped version from 4.0.3 to 4.0.4 (PATCH version for internal refactoring)
- Reasoning: The template_manager.rs module had dead code, duplicated confirmation prompts, and two nearly identical remove functions. Consolidating these improves maintainability and reduces code size while maintaining the same CLI interface.

### 2025-11-28 (Shell Completions)

- Added `completions` subcommand to generate shell completion scripts
- Added `clap_complete` v4.5 dependency for shell completion generation
- Supports bash, zsh, fish, and PowerShell shells
- Outputs completion script to stdout for user to redirect to appropriate location
- Bumped version from 4.0.4 to 4.1.0 (MINOR version for new feature)
- Reasoning: Shell completions improve user experience by enabling tab completion for commands, options, and arguments. Using clap_complete integrates seamlessly with the existing clap CLI framework.

### 2025-11-28 (Shell Completions - Restrict Shells)

- Created custom ShellType enum to limit shell options to bash, fish, powershell, zsh
- Removed elvish from available shell completions
- Bumped version from 4.1.0 to 4.1.1 (PATCH version for refinement)
- Reasoning: Elvish is not a commonly used shell and was included by default from clap_complete. Restricting to the four main shells (bash, fish, powershell, zsh) provides a cleaner user experience.

### 2025-11-28 (Dry Run Flag for Init)

- Added `--dry-run` flag to `init` command
- Shows preview of files that would be created or modified
- Color-coded output: green for new files, yellow for files that would be overwritten
- Respects AGENTS.md customization detection (shows as skipped if customized)
- No files are modified when dry-run is active
- Bumped version from 4.1.1 to 4.2.0 (MINOR version for new feature)
- Reasoning: Dry-run mode allows users to preview changes before committing to them, reducing the risk of unintended modifications and improving user confidence in the tool.

### 2025-11-28 (Dry Run Flag for All Commands)

- Extended `--dry-run` flag to `update`, `purge`, and `remove` commands
- `update --dry-run` shows source URL and target directory
- `purge --dry-run` shows files that would be deleted (red color)
- `remove --dry-run` shows agent-specific files that would be deleted
- Added `get_config_dir()` method to TemplateManager for dry-run output
- Updated AGENTS.md documentation for all commands
- Reasoning: Consistent dry-run support across all commands provides users with a complete preview capability before any file system modifications.

### 2025-11-28 (Status Command)

- Added `status` command to show current project state
- Displays global template status (installed location, available agents and languages)
- Shows AGENTS.md status (exists, customized or from template)
- Lists installed agents detected from file presence
- Shows all vibe-check managed files in current directory
- Bumped version from 4.2.0 to 4.3.0 (MINOR version for new feature)
- Reasoning: The status command helps users understand what is currently configured in their project, making it easier to manage and troubleshoot vibe-check installations.

### 2025-11-28 (List Command)

- Added `list` command to show available agents and languages
- Lists all agents from templates.yml with installation status
- Lists all languages from templates.yml with installation status
- Green checkmark for installed items, blue circle for available
- Provides guidance on how to install with init command
- Bumped version from 4.3.0 to 4.4.0 (MINOR version for new feature)
- Reasoning: The list command helps users discover what agents and languages are available, making it easier to choose the right options when initializing a new project.
