# vibe-check

**A manager for coding agent instruction files** – A Rust CLI tool that provides a centralized system for managing, organizing, and maintaining initialization prompts and instruction files for AI coding assistants (Claude, GitHub Copilot, Codex, Cursor, and others) with built-in governance guardrails and human-in-the-loop controls.

![MIT License](https://img.shields.io/badge/-MIT%20License-000000?style=flat-square&logo=opensource&logoColor=white)
![CLI](https://img.shields.io/badge/-CLI-000000?style=flat-square&logo=zsh&logoColor=white)
![Rust](https://img.shields.io/badge/-Rust-000000?style=flat-square&logo=rust&logoColor=white)
![Claude](https://img.shields.io/badge/-Claude-000000?style=flat-square&logo=anthropic&logoColor=white)
![GitHub Copilot](https://img.shields.io/badge/-GitHub%20Copilot-000000?style=flat-square&logo=github&logoColor=white)
![Codex](https://img.shields.io/badge/-Codex-000000?style=flat-square&logo=openai&logoColor=white)

## Overview

vibe-check is a command-line tool that helps you:

- **Manage templates globally** – Store templates in platform-specific directories (e.g., `~/Library/Application Support/vibe-check/templates` on macOS)
- **Configure via YAML** – Define template structure and file mappings in `templates.yml`
- **Initialize projects quickly** – Set up agent instructions with a single command
- **Keep templates synchronized** – Update global templates from remote sources
- **Enforce governance** – Built-in guardrails for no auto-commits and human confirmation
- **Support multiple agents** – Works with Claude, Copilot, Codex, Cursor, and more
- **Flexible file placement** – Use placeholders (`$workspace`, `$userprofile`) for custom locations
- **Template versioning** – Support for multiple template format versions

## Repository Structure

```text
vibe-check/
├── Cargo.toml                  # Rust project manifest
├── Cargo.lock                  # Dependency lock file
├── build.rs                    # Build script for man page generation
├── .rustfmt.toml               # Rust formatting configuration
├── src/                        # Rust source code
│   ├── main.rs                 # Application entry point and CLI
│   ├── lib.rs                  # Library public API
│   ├── bom.rs                  # Bill of Materials structures and functions
│   ├── download_manager.rs     # DownloadManager for URL downloads
│   ├── template_engine_v1.rs   # Template engine for version 1 templates
│   ├── template_manager.rs     # TemplateManager implementation
│   └── utils.rs                # Utility functions
├── LICENSE                     # MIT license
├── README.md                   # You are here
├── AGENTS.md                   # Primary project instructions
├── templates/                  # Template files for various languages and frameworks
│   ├── templates.yml           # Template configuration (defines structure and mappings)
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
│   │       └── init-session.md # Claude session initialization prompt
│   ├── codex/
│   │   └── prompts/
│   │       └── init-session.md # Codex session initialization prompt
│   ├── copilot/
│   │   ├── copilot-instructions.md # Copilot main instruction file
│   │   ├── copilot-instructions-auto-redirect.md  # Auto-redirect to AGENTS.md
│   │   └── prompts/
│   │       └── init-session.prompt.md # Copilot session prompt
│   └── cursor/
│       └── commands/
│           └── init-session.md # Cursor session initialization prompt
├── CLAUDE.md                   # Claude-specific reference
└── .github/
    └── copilot-instructions.md # GitHub Copilot reference
```

## Philosophy

1. **Human control first** – All prompts enforce explicit confirmation before commits
2. **Single source of truth** – Centralized `AGENTS.md` file for project instructions
3. **Transparency** – Every change logs rationale with date and reasoning
4. **Minimalism** – Only essential policies that deliver concrete safety or velocity
5. **Scalability** – Add new agents without policy drift

## Installation

### From Source

```bash
git clone https://github.com/heikopanjas/vibe-check.git
cd vibe-check
cargo build --release
sudo cp target/release/vibe-check /usr/local/bin/
```

### Using Cargo

```bash
cargo install --path .
```

## Quick Start

### 1. Download global templates

```bash
vibe-check update
```

### 2. Initialize a C++ project with Claude

```bash
cd your-project
vibe-check init --lang c++ --agent claude
```

This will:

1. Copy general templates (AGENTS.md) to your project
2. Merge language-specific fragments into AGENTS.md
3. Copy Claude instruction file (CLAUDE.md) to your project root
4. Create `.claude/commands/` directory with session initialization prompt

### Initialize from a custom template source

```bash
# From a local path
vibe-check update --from /path/to/templates

# From a GitHub URL
vibe-check update --from https://github.com/user/repo/tree/branch/templates

# Then initialize the project
vibe-check init --lang c++ --agent claude
```

**Note:** The custom source must include a `templates.yml` file that defines the template structure.

## Complete Walkthrough: C++ Project with Claude

This walkthrough demonstrates setting up a new C++ project with Claude AI assistant using vibe-check.

### Step 1: Create Your Project Directory

```bash
mkdir my-cpp-project
cd my-cpp-project
```

### Step 2: Initialize with vibe-check

```bash
vibe-check init --lang c++ --agent claude
```

**What happens:**

1. **Downloads templates** (first run only):
   - Fetches `templates.yml` from GitHub
   - Downloads all template files to platform-specific directory (e.g., `~/Library/Application Support/vibe-check/templates/` on macOS)

2. **Processes configuration**:
   - Parses `templates.yml` to determine which files to install
   - Identifies fragments marked with `$instructions` placeholder

3. **Creates main AGENTS.md**:
   - Downloads main AGENTS.md template
   - Merges fragments at insertion points:
     - **Mission section**: mission-statement.md, technology-stack.md
     - **Principles section**: core-principles.md, best-practices.md
     - **Languages section**: c++-coding-conventions.md, cmake-build-commands.md (C++ specific)
     - **Integration section**: git-workflow-conventions.md, semantic-versioning.md
   - Saves complete merged file to `./AGENTS.md`

4. **Installs Claude files**:
   - Copies `CLAUDE.md` to project root
   - Creates `.claude/commands/` directory
   - Copies `init-session.md` prompt to `.claude/commands/`

### Step 3: Verify Installation

```bash
ls -la
```

**Expected structure:**

```text
my-cpp-project/
├── AGENTS.md                          # Main instruction file (merged)
├── CLAUDE.md                          # Claude-specific reference
├── .clang-format                      # C++ formatting configuration
├── .editorconfig                      # Editor configuration
├── .gitignore                         # Git ignore file
├── .gitattributes                     # Git attributes file
└── .claude/
    └── commands/
        └── init-session.md            # Claude initialization prompt
```

### Step 4: Start Claude Session

Open Claude and run the initialization prompt:

Simply type `/init-session` in Claude to execute the custom prompt. By placing `init-session.md` in `.claude/commands/`, vibe-check makes it available as a custom command that Claude automatically recognizes.

Alternatively, in Claude's project settings, reference `AGENTS.md` as your project instructions.

### Step 5: Verify Claude Understands Instructions

Ask Claude to confirm:

```text
Please confirm you've read AGENTS.md and understand the project instructions.
```

Claude should acknowledge the:

- Commit protocol (no auto-commits)
- C++ coding conventions
- Git workflow conventions
- Build environment requirements

### Step 6: Start Coding

Now you can work with Claude following the established guidelines:

```text
Claude, help me create a CMakeLists.txt for this C++ project with library and test targets.
```

Claude will follow the conventions in AGENTS.md, including:

- Using proper C++ style
- Following conventional commits
- Waiting for explicit commit confirmation
- Documenting decisions

### Step 7: Update Templates Later (Optional)

If templates are updated upstream:

```bash
# Update global templates
vibe-check update

# Then reinitialize the project (will skip customized AGENTS.md unless --force)
vibe-check init --lang c++ --agent claude
```

vibe-check will:

- Check if AGENTS.md has been customized (template marker removed)
- Skip customized AGENTS.md unless `--force` is used

### Step 8: Working with Multiple Agents

You can initialize multiple agents for the same project:

```bash
# Add GitHub Copilot
vibe-check init --lang c++ --agent copilot
```

**Updated structure:**

```text
my-cpp-project/
├── AGENTS.md
├── CLAUDE.md
├── .claude/
│   └── commands/
│       └── init-session.md
└── .github/
    ├── copilot-instructions.md
    └── prompts/
        └── init-session.prompt.md
```

### Common Scenarios

**Scenario: Modified AGENTS.md locally**

```bash
$ vibe-check init --lang c++ --agent claude
! Local AGENTS.md has been customized and will be skipped
→ Other files will still be updated
→ Use --force to overwrite AGENTS.md
```

**Solution:** Review changes, commit them, then use `--force`:

```bash
git diff AGENTS.md              # Review changes
git add AGENTS.md
git commit -m "docs: customize project instructions"
vibe-check init --lang c++ --agent claude --force
```

**Scenario: Clean up project templates**

```bash
# Remove all agent files and AGENTS.md
vibe-check purge

# Removes: .claude/, .github/, .cursor/, AGENTS.md (unless customized without --force)
# Preserves: README.md, LICENSE, source code, and customized AGENTS.md (unless --force)
```

**Scenario: Remove only agent-specific files (keep AGENTS.md)**

```bash
# Remove all agent files but keep AGENTS.md
vibe-check remove --all

# Or remove only one agent
vibe-check remove --agent claude
```

**Scenario: Use custom templates**

```bash
# Your team maintains custom templates
vibe-check update --from https://github.com/yourteam/templates/tree/main/templates

# Then initialize
vibe-check init --lang c++ --agent claude
```

### Tips for Success

1. **Initialize early**: Run `vibe-check init` at project start before adding code
2. **Commit instructions**: Add AGENTS.md and agent files to version control
3. **Team consistency**: All team members should use same template source
4. **Customize carefully**: Modify AGENTS.md as needed, but track changes in git
5. **Update periodically**: Check for template updates monthly or quarterly
6. **Use force sparingly**: Only use `--force` when you understand what you're overwriting
7. **Use version control**: Git is your primary safety net for tracking changes
8. **Preview first**: Use `--dry-run` to preview changes before applying them

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

- `--lang <string>` - Programming language or framework (e.g., c++, rust, swift, c)
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

### `status` - Show Project Status

Display the current status of vibe-check in the project.

**Usage:**

```bash
vibe-check status
```

**Output includes:**

- **Global Templates:** Whether templates are installed and their location
  - Template version
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
  → Template version: 1
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

## Core Governance Principles

All templates in this repository enforce these critical rules:

- **Never auto-commit** – Explicit human request required before any commit
- **Conventional commits** – Standardized commit message format (max 500 chars)
- **Change logging** – Maintain "Recent Updates & Decisions" log with timestamps
- **Single source of truth** – Update only `AGENTS.md`, not reference files
- **Structured updates** – Preserve file structure: header → timestamp → content → log
- **No secrets** – Never add credentials, API keys, or sensitive data

## Supported Agents

| Agent | Status | Configuration in templates.yml | Notes |
|-------|--------|-------------------------------|-------|
| Claude | Active | `agents.claude` | Main instruction: CLAUDE.md, Prompts: .claude/commands/ |
| GitHub Copilot | Active | `agents.copilot` | Main instruction: .github/copilot-instructions.md, Prompts: .github/prompts/ |
| Codex | Active | `agents.codex` | Prompts: $userprofile/.codex/prompts/ |
| Cursor | Active | `agents.cursor` | Prompts: .cursor/commands/ |

## Supported Languages

Currently configured in `templates.yml`:

- **C** - C programming language (fragments: `c-coding-conventions.md` and `cmake-build-commands.md` merged into AGENTS.md)
- **C++** - C++ programming language (fragments: `c++-coding-conventions.md` and `cmake-build-commands.md` merged into AGENTS.md)
- **Rust** - Rust programming language (fragments: `rust-coding-conventions.md` and `rust-build-commands.md` merged into AGENTS.md)
- **Swift** - Swift programming language (fragments: `swift-coding-conventions.md` and `swift-build-commands.md` merged into AGENTS.md)

Additional language templates can be added to `templates.yml` configuration. Language-specific content is stored as fragments in the global templates directory and merged into AGENTS.md during init.

## How It Works

### Template Storage

Templates are stored in platform-specific directories:

- **macOS**: `~/Library/Application Support/vibe-check/templates/`
- **Linux**: `~/.local/share/vibe-check/templates/`
- **Windows**: `%LOCALAPPDATA%\vibe-check\templates\`

Templates include:

- **templates.yml**: Configuration file defining structure and file mappings (with version field)
- **Main template**: AGENTS.md (primary instruction file)
- **Language fragments**: Language-specific coding standards and build commands - merged into AGENTS.md
- **Integration fragments**: Tool/workflow templates (e.g., git-workflow-conventions.md) - merged into AGENTS.md
- **Principle fragments**: Core principles and best practices - merged into AGENTS.md
- **Mission fragments**: Mission statement, technology stack - merged into AGENTS.md
- **Agent templates**: Agent-specific instruction files and prompts (copied to project directories)
- **Config files**: EditorConfig, format configurations, .gitignore, .gitattributes

### Template Configuration (templates.yml)

The `templates.yml` file defines the template structure with a version field and six main sections:

1. **version**: Template format version (currently 1, defaults to 1 if missing)
2. **main**: Main AGENTS.md instruction file (primary source of truth)
3. **agents**: Agent-specific files with `instructions` (main file) and `prompts` (custom commands)
4. **languages**: Language-specific coding standards fragments (merged into AGENTS.md)
5. **integration**: Tool/workflow integration fragments (merged into AGENTS.md, e.g., git workflows)
6. **principles**: Core principles and general guidelines fragments (merged into AGENTS.md)
7. **mission**: Mission statement, purpose, and project overview fragments (merged into AGENTS.md)

Each file entry specifies:

- `source`: Path in the template repository
- `target`: Destination path using placeholders

**Placeholders:**

- `$workspace` - Resolves to current directory
- `$userprofile` - Resolves to user's home directory
- `$instructions` - Indicates fragment to be merged into main AGENTS.md at insertion points

**Fragment Merging:**

Templates using `$instructions` as the target are merged into the main AGENTS.md file at specific insertion points:

- `<!-- {mission} -->` - Where mission/purpose and project overview are inserted
- `<!-- {principles} -->` - Where core principles and guidelines are inserted
- `<!-- {languages} -->` - Where language-specific coding standards are inserted
- `<!-- {integration} -->` - Where tool/workflow integration content is inserted

Example structure:

```yaml
version: 1

main:
    source: AGENTS.md
    target: '$workspace/AGENTS.md'

agents:
    claude:
        instructions:
            - source: claude/CLAUDE.md
              target: '$workspace/CLAUDE.md'
        prompts:
            - source: claude/commands/init-session.md
              target: '$workspace/.claude/commands/init-session.md'

languages:
    c++:
        files:
            - source: c++-coding-conventions.md
              target: '$instructions'
            - source: cmake-build-commands.md
              target: '$instructions'
            - source: c++-format-instructions.yml
              target: '$workspace/.clang-format'
            - source: c++-editor-config.ini
              target: '$workspace/.editorconfig'
            - source: c++-git-ignore.txt
              target: '$workspace/.gitignore'

principles:
    - source: core-principles.md
      target: '$instructions'
```

### Template Versioning

Templates include a version field to support future format changes:

- **Version 1** (current): The default format described above
- Missing version field defaults to 1 for backward compatibility
- Future versions will be handled by separate template engines

The `status` command shows the template version currently installed.

### Template Management

1. **First run**: `update` downloads `templates.yml` and all specified files from GitHub
2. **Local storage**: Templates are cached in platform-specific directory
3. **Protection**: Template marker in AGENTS.md detects customization and prevents accidental overwrites
4. **Updates**: Detect AGENTS.md customization and warn before overwriting
5. **Placeholders**: `$workspace` and `$userprofile` resolve to appropriate paths

### Project Initialization

When you run `vibe-check init --lang c++ --agent claude`:

1. Checks if global templates exist (downloads if needed)
2. Loads `templates.yml` configuration and detects version
3. Uses appropriate template engine for the version
4. Downloads main AGENTS.md template
5. Downloads and merges fragments (mission, principles, language, integration) into AGENTS.md at insertion points
6. Copies language config files (.clang-format, .editorconfig, .gitignore, .gitattributes)
7. Copies Claude instruction file (CLAUDE.md) to project root
8. Creates `.claude/commands/` directory with prompts
9. You're ready to start coding with proper agent instructions

The resulting AGENTS.md contains the complete merged content with all relevant sections for your project.

### Modification Detection

vibe-check detects if you've customized AGENTS.md by checking for the template marker:

```bash
$ vibe-check init --lang c++ --agent claude
! Local AGENTS.md has been customized and will be skipped
→ Other files will still be updated
→ Use --force to overwrite AGENTS.md
```

The template marker is automatically removed when fragments are merged into AGENTS.md during initialization. This marks the file as customized and prevents accidental overwrites. Use `--force` to override and update anyway.

## Customization

### Using Custom Templates

You can use your own template repository:

```bash
# From a local path
vibe-check update --from /path/to/your/templates

# From a GitHub repository
vibe-check update --from https://github.com/yourname/your-templates/tree/main/templates

# Then initialize your project
vibe-check init --lang c++ --agent claude
```

**Note:** Your custom template repository must include a `templates.yml` file that defines the template structure and file mappings.

### Modifying Global Templates

1. Navigate to platform-specific template directory:
   - macOS: `~/Library/Application Support/vibe-check/templates/`
   - Linux: `~/.local/share/vibe-check/templates/`
   - Windows: `%LOCALAPPDATA%\vibe-check\templates\`
2. Edit the templates as needed
3. Run `vibe-check init` to apply changes to your projects

### Creating New Templates

To add a new language or agent template:

1. Fork this repository
2. Add your template to the `templates/` directory
3. For languages: Create coding conventions and build commands markdown files
4. For agents: Create `agent-name/` directory with instructions and prompts
5. Update `templates.yml` with the new entries
6. Submit a pull request

## Technology Stack

- **Language:** Rust (Edition 2024)
- **CLI Framework:** clap v4.5.20
- **Shell Completions:** clap_complete v4.5
- **Terminal Colors:** owo-colors v4.1.0
- **HTTP Client:** reqwest v0.12 (blocking, json)
- **Serialization:** serde v1.0, serde_yaml v0.9
- **Directory Paths:** dirs v5.0
- **Man Pages:** clap_mangen v0.2 (build dependency)

## FAQ

**Where are templates stored?**

- Global templates (macOS): `~/Library/Application Support/vibe-check/templates/`
- Global templates (Linux): `~/.local/share/vibe-check/templates/`
- Global templates (Windows): `%LOCALAPPDATA%\vibe-check\templates\`

**What happens if I modify AGENTS.md?**
vibe-check detects customization via template marker removal and skips AGENTS.md when updating. Use `--force` to override.

**Can I use my own template repository?**
Yes! Use the `--from` option with the `update` command to specify a local path or GitHub URL.

**Why AGENTS.md as single source of truth?**
Centralized updates prevent drift and make it easier to maintain consistency across sessions.

**Can I use this in commercial projects?**
Yes! MIT license allows commercial use. Attribution appreciated but not required.

**How do I update templates?**
Run `vibe-check update` to download the latest global templates, then `vibe-check init` to apply to your project.

**How do I remove local templates?**
Run `vibe-check purge` to remove all agent files and AGENTS.md, or `vibe-check remove --all` to keep AGENTS.md.

**How do I preview changes before applying?**
Use the `--dry-run` flag on any command: `vibe-check init --lang rust --agent claude --dry-run`

**What template version is supported?**
Currently version 1. Run `vibe-check status` to see the installed template version.

## License

MIT License - See [LICENSE](LICENSE) for details.

## Building from Source

```bash
# Clone the repository
git clone https://github.com/heikopanjas/vibe-check.git
cd vibe-check

# Build in debug mode (for development)
cargo build

# Run tests
cargo test

# Run the application
cargo run -- init --lang rust --agent claude

# Build in release mode (optimized, generates man pages)
cargo build --release

# Format code
cargo fmt

# Run linter
cargo clippy
```

---

Last updated: November 28, 2025
