# vibe-check

**A manager for coding agent instruction files** – A Rust CLI tool that provides a centralized system for managing, organizing, and maintaining initialization prompts and instruction files for AI coding assistants. Supports the [agents.md community standard](https://agents.md) where a single AGENTS.md file works across all agents (Claude, Cursor, Copilot, Aider, Jules, Factory, and others) with built-in governance guardrails and human-in-the-loop controls.

![MIT License](https://img.shields.io/badge/-MIT%20License-000000?style=flat-square&logo=opensource&logoColor=white)
![CLI](https://img.shields.io/badge/-CLI-000000?style=flat-square&logo=zsh&logoColor=white)
![Rust](https://img.shields.io/badge/-Rust-000000?style=flat-square&logo=rust&logoColor=white)
![Claude](https://img.shields.io/badge/-Claude-000000?style=flat-square&logo=anthropic&logoColor=white)
![GitHub Copilot](https://img.shields.io/badge/-GitHub%20Copilot-000000?style=flat-square&logo=github&logoColor=white)
![Codex](https://img.shields.io/badge/-Codex-000000?style=flat-square&logo=openai&logoColor=white)
![Cursor](https://img.shields.io/badge/-Cursor-000000?style=flat-square&logo=visualstudiocode&logoColor=white)

## Overview

vibe-check is a command-line tool that helps you:

- **Manage templates globally** – Store templates in platform-specific directories (e.g., `~/Library/Application Support/vibe-check/templates` on macOS)
- **Configure via YAML** – Define template structure and file mappings in `templates.yml`
- **Initialize projects quickly** – Set up agent instructions with a single command
- **agents.md standard** – V2 templates follow the [agents.md](https://agents.md) community standard (single AGENTS.md for all agents)
- **Keep templates synchronized** – Update global templates from remote sources
- **Enforce governance** – Built-in guardrails for no auto-commits and human confirmation
- **Support multiple agents** – Compatible with Claude, Cursor, Copilot, Aider, Jules, Factory, and more
- **Flexible file placement** – Use placeholders (`$workspace`, `$userprofile`) for custom locations
- **Template versioning** – V1 (agent-specific files) and V2 (agents.md standard) formats supported

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
│   ├── config.rs               # Configuration management
│   ├── download_manager.rs     # DownloadManager for URL downloads
│   ├── template_engine_v1.rs   # Template engine for version 1 templates
│   ├── template_engine_v2.rs   # Template engine for version 2 templates (agents.md standard)
│   ├── template_manager.rs     # TemplateManager implementation
│   └── utils.rs                # Utility functions
├── LICENSE                     # MIT license
├── README.md                   # You are here
├── AGENTS.md                   # Primary project instructions
├── templates/                  # Template files organized by version
│   ├── v1/                     # Version 1 templates (agent-specific files)
│   │   ├── templates.yml       # V1 template configuration (version: 1)
│   │   ├── AGENTS.md           # Main instruction template
│   │   ├── claude/             # Claude-specific files
│   │   │   ├── CLAUDE.md
│   │   │   └── commands/
│   │   │       └── init-session.md
│   │   ├── copilot/            # GitHub Copilot files
│   │   ├── codex/              # Codex files
│   │   ├── cursor/             # Cursor files
│   │   └── ...                 # Language templates (coding conventions, build commands, etc.)
│   └── v2/                     # Version 2 templates (agents.md standard)
│       ├── templates.yml       # V2 template configuration (version: 2, no agents section)
│       ├── AGENTS.md           # Single instruction file for all agents
│       └── ...                 # Language templates only (no agent directories)
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

## Template Versions

vibe-check supports two template formats through its versioning system:

### Version 2 (Default in v6.0.0+) - agents.md Standard

**Philosophy**: One AGENTS.md file that works across all agents.

- Follows the [agents.md](https://agents.md) community standard
- Single AGENTS.md file compatible with Claude, Cursor, Copilot, Aider, Jules, Factory, and more
- No agent-specific instruction files (CLAUDE.md, copilot-instructions.md, etc.)
- Simpler initialization: `vibe-check init --lang rust` or `vibe-check init --no-lang` for language-independent setup
- Optional `--lang` and `--agent` (specify at least one; `--agent` alone preserves existing language when switching)
- URL: `https://github.com/heikopanjas/vibe-check/tree/develop/templates/v2`

**Usage:**
```bash
# V2 is the default in v6.0.0+
vibe-check update                    # Downloads v2 templates
vibe-check init --lang rust          # With language conventions
vibe-check init --no-lang            # Language-independent (AGENTS.md only)
vibe-check init --agent cursor       # Switch agent, keep existing language
```

### Version 1 (Default in v5.x) - Agent-Specific Files

**Philosophy**: Separate instruction files per agent.

- Separate instruction files per agent (CLAUDE.md, copilot-instructions.md, etc.)
- Agent-specific prompt directories (.claude/commands/, .github/prompts/, etc.)
- Requires `--agent` flag: `vibe-check init --lang rust --agent claude`
- URL: `https://github.com/heikopanjas/vibe-check/tree/develop/templates/v1`

**Usage:**
```bash
# Explicitly configure v1 (if needed in v6.0.0+)
vibe-check config source.url https://github.com/heikopanjas/vibe-check/tree/develop/templates/v1
vibe-check update
vibe-check init --lang rust --agent claude
```

### Migration from v5 to v6

**Upgrading from v5.x to v6.0.0:**

1. **If you want v2 (agents.md standard - recommended):**
   ```bash
   # No action needed! v6 defaults to v2
   vibe-check update                    # Gets v2 templates
   vibe-check purge --force             # Clean v1 files
   vibe-check init --lang rust          # Initialize with v2
   ```

2. **If you want to stay on v1:**
   ```bash
   # Configure v1 before updating
   vibe-check config source.url https://github.com/heikopanjas/vibe-check/tree/develop/templates/v1
   vibe-check update                    # Gets v1 templates
   vibe-check init --lang rust --agent claude
   ```

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

### Version 6.0.0+ (V2 - agents.md Standard - Default)

```bash
# 1. Download global templates (v2 by default)
vibe-check update

# 2. Initialize your project (choose one style)
cd your-project
vibe-check init --lang rust         # With Rust conventions and config files
vibe-check init --no-lang          # Language-independent (AGENTS.md + integration only)
vibe-check init --agent cursor     # Agent prompts only (preserves existing language)
```

With `--lang rust` this will:

1. Copy main AGENTS.md template to your project
2. Merge language-specific fragments (Rust conventions, build commands) into AGENTS.md
3. Copy language config files (.rustfmt.toml, .editorconfig, .gitignore, .gitattributes)
4. **Single AGENTS.md works with all agents** (Claude, Cursor, Copilot, Aider, Jules, Factory, etc.)

With `--no-lang` you get AGENTS.md with mission, principles, and integration (e.g. git) only—no language-specific files.

### Version 5.x (V1 - Agent-Specific Files)

If you're using v5.x or explicitly configured v1 templates:

```bash
# 1. Download global templates
vibe-check update

# 2. Initialize a C++ project with Claude
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

## Complete Walkthrough: Rust Project (V6.0.0+ with V2 Templates)

This walkthrough demonstrates setting up a new Rust project using vibe-check with v2 templates (agents.md standard).

### Step 1: Create Your Project Directory

```bash
mkdir my-rust-project
cd my-rust-project
```

### Step 2: Initialize with vibe-check

```bash
vibe-check init --lang rust
```

**What happens:**

1. **Downloads templates** (first run only):
   - Fetches `templates.yml` from GitHub (v2 format)
   - Downloads all template files to platform-specific directory (e.g., `~/Library/Application Support/vibe-check/templates/` on macOS)

2. **Processes configuration**:
   - Detects template version 2 (agents.md standard)
   - Identifies fragments marked with `$instructions` placeholder

3. **Creates main AGENTS.md**:
   - Downloads main AGENTS.md template
   - Merges fragments at insertion points:
     - **Mission section**: mission-statement.md, technology-stack.md
     - **Principles section**: core-principles.md, best-practices.md
     - **Languages section**: rust-coding-conventions.md, rust-build-commands.md (Rust specific)
     - **Integration section**: git-workflow-conventions.md, semantic-versioning.md
   - Saves complete merged file to `./AGENTS.md`

4. **Installs language config files**:
   - Copies `.rustfmt.toml` for Rust formatting
   - Copies `.editorconfig` for editor configuration
   - Copies `.gitignore` for Rust artifacts
   - Copies `.gitattributes` for cross-platform compatibility

### Step 3: Verify Installation

```bash
ls -la
```

**Expected structure:**

```text
my-rust-project/
├── AGENTS.md                          # Single instruction file (works with all agents)
├── .rustfmt.toml                      # Rust formatting configuration
├── .editorconfig                      # Editor configuration
├── .gitignore                         # Git ignore file
└── .gitattributes                     # Git attributes file
```

### Step 4: Start Coding with Any Agent

**With Claude/Cursor:**

Open your agent and reference `AGENTS.md` in project settings. The single AGENTS.md works automatically.

**With Aider:**

```bash
aider --read AGENTS.md
```

**With GitHub Copilot:**

Copilot automatically reads AGENTS.md from your workspace.

### Step 5: Verify Agent Understands Instructions

Ask your agent to confirm:

```text
Please confirm you've read AGENTS.md and understand the project instructions.
```

The agent should acknowledge the:

- Commit protocol (no auto-commits)
- Rust coding conventions
- Git workflow conventions
- Build environment requirements

### Step 6: Start Coding

Now you can work with your agent following the established guidelines:

```text
Help me create a library crate with proper error handling using Result types.
```

Your agent will follow the conventions in AGENTS.md, including:

- Using proper Rust style
- Following conventional commits
- Waiting for explicit commit confirmation
- Documenting decisions

### Step 7: Update Templates Later (Optional)

If templates are updated upstream:

```bash
# Update global templates
vibe-check update

# Then reinitialize the project (will skip customized AGENTS.md unless --force)
vibe-check init --lang rust
```

vibe-check will:

- Check if AGENTS.md has been customized (template marker removed)
- Skip customized AGENTS.md unless `--force` is used

### Common Scenarios

**Scenario: Modified AGENTS.md locally**

```bash
$ vibe-check init --lang rust
! Local AGENTS.md has been customized and will be skipped
→ Other files will still be updated
→ Use --force to overwrite AGENTS.md
```

**Solution:** Review changes, commit them, then use `--force`:

```bash
git diff AGENTS.md              # Review changes
git add AGENTS.md
git commit -m "docs: customize project instructions"
vibe-check init --lang rust --force
```

**Scenario: Clean up project templates**

```bash
# Remove all vibe-check files including AGENTS.md
vibe-check purge

# V1: Removes agent directories (.claude/, .github/, .cursor/) and AGENTS.md
# V2: Removes AGENTS.md and language config files
# Both: Preserves customized AGENTS.md unless --force is used
```

**Scenario: Remove only agent-specific files**

```bash
# Remove all agent files but keep AGENTS.md
vibe-check remove --all

# Remove only one agent's files
vibe-check remove --agent claude

# V1: Removes CLAUDE.md, .claude/commands/, etc.
# V2: Removes .cursor/commands/, .github/prompts/, etc. (agent prompts)
```

**Scenario: Switch from Cursor to Claude (keep Rust setup)**

```bash
# You have Rust + Cursor; want to add Claude prompts
vibe-check init --agent claude
# Uses existing Rust language; adds Claude prompts only
```

**Scenario: Language-independent project (e.g. docs-only repo)**

```bash
vibe-check init --no-lang
# AGENTS.md with mission, principles, integration only—no .rustfmt.toml, no coding-conventions
```

**Scenario: Use custom templates**

```bash
# Your team maintains custom v2 templates
vibe-check update --from https://github.com/yourteam/templates/tree/main/templates

# Then initialize (v2 style)
vibe-check init --lang rust

# Or v1 templates
vibe-check update --from https://github.com/yourteam/v1-templates/tree/main/templates
vibe-check init --lang rust --agent claude
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
  - **v6.0.0+**: `https://github.com/heikopanjas/vibe-check/tree/develop/templates/v2` (agents.md standard)
  - **v5.x**: `https://github.com/heikopanjas/vibe-check/tree/develop/templates/v1` (agent-specific files)
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
# Specify at least one of --lang, --agent, or --no-lang

# V2: With language conventions
vibe-check init --lang <language> [--agent <agent>] [--mission <text|@file>] [--force] [--dry-run]

# V2: Language-independent (no coding-conventions fragments)
vibe-check init --no-lang [--agent <agent>] [--mission <text|@file>] [--force] [--dry-run]

# V2: Switch agent only (preserves existing language)
vibe-check init --agent <agent> [--mission <text|@file>] [--force] [--dry-run]

# V1 templates (requires --agent)
vibe-check init --lang <language> --agent <agent> [--mission <text|@file>] [--force] [--dry-run]
```

**Options:**

- `--lang <string>` - Programming language or framework (e.g., c++, rust, swift, c). Mutually exclusive with `--no-lang`.
- `--agent <string>` - AI coding agent (e.g., claude, copilot, codex, cursor). Required for v1 templates, optional for v2.
- `--no-lang` - Skip language-specific setup (AGENTS.md with mission/principles/integration only, no coding-conventions). Mutually exclusive with `--lang`.
- `--mission <string>` - Custom mission statement to override the template default. Use `@filename` to read from a file (e.g., `--mission @mission.md`)
- `--force` - Force overwrite of local files without confirmation
- `--dry-run` - Preview changes without applying them

**Examples (V6.0.0+ with V2 templates):**

```bash
# Initialize Rust project (works with all agents)
vibe-check init --lang rust

# Initialize C++ project
vibe-check init --lang c++

# Language-independent setup (AGENTS.md + integration only, no .rustfmt.toml etc.)
vibe-check init --no-lang

# Language-independent + agent prompts (e.g. init-session command for Cursor)
vibe-check init --no-lang --agent cursor

# Switch from Cursor to Claude (keeps existing language e.g. Rust)
vibe-check init --agent claude

# Initialize with custom mission statement (inline)
vibe-check init --lang rust --mission "A CLI tool for managing AI agent instructions"

# Initialize with mission statement from file (multi-line support)
vibe-check init --lang rust --mission @mission.md

# Force overwrite existing local files
vibe-check init --lang swift --force

# Preview what would be created/modified
vibe-check init --lang rust --dry-run
```

**Examples (V1 templates or v5.x):**

```bash
# Initialize C++ project with Claude
vibe-check init --lang c++ --agent claude

# Initialize Rust project with Copilot
vibe-check init --lang rust --agent copilot

# Initialize with custom mission from file
vibe-check init --lang c++ --agent claude --mission @docs/mission.md
```

**Behavior:**

- Uses global templates to set up agent instructions in the current project
- If global templates do not exist, automatically downloads them from the default repository
- Detects template version (v1 or v2) from templates.yml
- **Must specify at least one** of `--lang`, `--agent`, or `--no-lang`; `--lang` and `--no-lang` cannot be used together
- **V2 with `--agent` only**: Preserves existing installation language (e.g. switch Cursor→Claude, keep Rust); falls back to first available language for fresh init
- **V2 with `--no-lang`**: Skips language fragments; creates AGENTS.md with mission, principles, integration only (no .rustfmt.toml, .editorconfig, etc.); optional `--agent` adds agent prompts
- **V2 with `--lang`**: Creates single AGENTS.md plus language config files; optional `--agent` adds agent prompts
- **V1 behavior**: Requires both `--lang` and `--agent`; creates AGENTS.md plus agent-specific files
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

→ Use 'vibe-check init --lang <lang>' or 'vibe-check init --no-lang' or 'vibe-check init --agent <agent>' to install
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

### `config` - Manage Configuration

Manage persistent configuration settings using Git-style dotted keys.

**Usage:**

```bash
vibe-check config <key> <value>    # Set a configuration value
vibe-check config <key>            # Get a configuration value
vibe-check config --list           # List all configuration values
vibe-check config --unset <key>    # Remove a configuration value
```

**Options:**

- `<key>` - Configuration key (e.g., source.url)
- `<value>` - Value to set (omit to get current value)
- `--list` - List all configuration values
- `--unset <key>` - Remove a configuration key

**Examples:**

```bash
# Set custom template source
vibe-check config source.url https://github.com/myteam/templates/tree/main/templates

# Get current source URL
vibe-check config source.url

# List all configuration
vibe-check config --list

# Remove custom source (revert to default)
vibe-check config --unset source.url

# Set fallback source for resilience
vibe-check config source.fallback https://github.com/heikopanjas/vibe-check/tree/develop/templates
```

**Valid Configuration Keys:**

- `source.url` - Default template download URL (used by `update` and `init` when `--from` not specified)
- `source.fallback` - Fallback URL used when primary source fails or is unreachable

**Configuration File Location:**

- Linux: `$XDG_CONFIG_HOME/vibe-check/config.yml` or `~/.config/vibe-check/config.yml`
- macOS: `~/.config/vibe-check/config.yml`

**Behavior:**

- Configuration persists between sessions
- `update` command uses `source.url` if set and `--from` not specified
- `init` command uses `source.url` when downloading missing global templates
- If primary source fails and `source.fallback` is configured, automatically tries the fallback
- Empty configuration file is valid (all defaults used)

## Core Governance Principles

All templates in this repository enforce these critical rules:

- **Never auto-commit** – Explicit human request required before any commit
- **Conventional commits** – Standardized commit message format (max 500 chars)
- **Change logging** – Maintain "Recent Updates & Decisions" log with timestamps
- **Single source of truth** – Update only `AGENTS.md`, not reference files
- **Structured updates** – Preserve file structure: header → timestamp → content → log
- **No secrets** – Never add credentials, API keys, or sensitive data

## Supported Agents

### V2 Templates (v6.0.0+ Default)

**Universal Support**: Single AGENTS.md works with all agents following the [agents.md](https://agents.md) standard:

- Claude (Anthropic)
- Cursor (AI code editor)
- GitHub Copilot (GitHub)
- Aider (command-line AI)
- Jules (coding assistant)
- Factory (AI dev tool)
- Any agent that reads AGENTS.md

No agent-specific configuration needed. One file, all agents.

### V1 Templates (v5.x Default)

Agent-specific files configuration:

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

The `templates.yml` file defines the template structure with a version field and multiple sections:

**Version Field:**
- `version: 1` - V1 templates with agent-specific files
- `version: 2` - V2 templates following agents.md standard (no agent-specific files)
- Missing version defaults to 1 for backward compatibility

**Main Sections:**

1. **main**: Main AGENTS.md instruction file (primary source of truth)
2. **agents**: (V1 only) Agent-specific files with `instructions` and `prompts`
3. **languages**: Language-specific coding standards fragments (merged into AGENTS.md)
4. **integration**: Tool/workflow integration fragments (merged into AGENTS.md, e.g., git workflows)
5. **principles**: Core principles and general guidelines fragments (merged into AGENTS.md)
6. **mission**: Mission statement, purpose, and project overview fragments (merged into AGENTS.md)

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

**Example V2 structure (agents.md standard):**

```yaml
version: 2

main:
    source: AGENTS.md
    target: '$workspace/AGENTS.md'

# No agents section in v2 - single AGENTS.md works for all agents

languages:
    rust:
        files:
            - source: rust-coding-conventions.md
              target: '$instructions'
            - source: rust-build-commands.md
              target: '$instructions'
            - source: rust-format-instructions.toml
              target: '$workspace/.rustfmt.toml'
            - source: rust-editor-config.ini
              target: '$workspace/.editorconfig'
            - source: rust-git-ignore.txt
              target: '$workspace/.gitignore'

principles:
    - source: core-principles.md
      target: '$instructions'

mission:
    - source: mission-statement.md
      target: '$instructions'
```

**Example V1 structure (agent-specific files):**

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

principles:
    - source: core-principles.md
      target: '$instructions'
```

### Template Versioning

Templates include a version field to support different format approaches:

- **Version 2** (default in v6.0.0+): agents.md standard - single AGENTS.md for all agents
- **Version 1** (default in v5.x): Agent-specific files with separate instruction files per agent
- Missing version field defaults to 1 for backward compatibility
- Different template engines handle each version format

The `status` command shows the template version currently installed.

**Version Detection:**
vibe-check automatically detects the template version from `templates.yml` and uses the appropriate template engine. This allows seamless support for both v1 (agent-specific) and v2 (agents.md standard) templates.

### Template Management

1. **First run**: `update` downloads `templates.yml` and all specified files from GitHub
2. **Local storage**: Templates are cached in platform-specific directory
3. **Protection**: Template marker in AGENTS.md detects customization and prevents accidental overwrites
4. **Updates**: Detect AGENTS.md customization and warn before overwriting
5. **Placeholders**: `$workspace` and `$userprofile` resolve to appropriate paths

### Project Initialization

**V2 Templates** (when you run `vibe-check init --lang rust`):

1. Checks if global templates exist (downloads v2 by default if needed)
2. Loads `templates.yml` configuration and detects version 2
3. Uses TemplateEngineV2 for agents.md standard
4. Downloads main AGENTS.md template
5. Merges fragments (mission, principles, language, integration) into AGENTS.md at insertion points
6. Copies language config files (.rustfmt.toml, .editorconfig, .gitignore, .gitattributes)
7. **No agent-specific files** - single AGENTS.md works with all agents
8. Optional `--agent` adds agent prompts (e.g. .cursor/commands/init-session.md)
9. You're ready to start coding with any agent

**V2 with `--no-lang`** (language-independent setup):

1. Same as above but skips language fragments and language config files
2. AGENTS.md contains mission, principles, integration (e.g. git, versioning) only
3. Optional `--agent` adds agent prompts

**V2 with `--agent` only** (switch agent, preserve language):

1. Detects existing installation language from file tracker
2. Uses that language; if none, uses first available from templates
3. Adds/updates agent prompts only

**V1 Templates** (when you run `vibe-check init --lang c++ --agent claude`):

1. Checks if global templates exist (downloads if needed)
2. Loads `templates.yml` configuration and detects version 1
3. Uses TemplateEngineV1 for agent-specific format
4. Downloads main AGENTS.md template
5. Merges fragments into AGENTS.md at insertion points
6. Copies language config files (.clang-format, .editorconfig, .gitignore, .gitattributes)
7. Copies Claude instruction file (CLAUDE.md) to project root
8. Creates `.claude/commands/` directory with prompts
9. You're ready to start coding with Claude

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
Use the `--dry-run` flag on any command: `vibe-check init --lang rust --dry-run` or `vibe-check init --no-lang --dry-run`

**How do I customize the mission statement?**
Use the `--mission` option with `init`. For inline text: `--mission "Your mission here"`. For multi-line content from a file: `--mission @mission.md`. The custom mission replaces the default template placeholder in AGENTS.md.

**What template version should I use?**
- **V2 (recommended for v6.0.0+)**: agents.md standard - simpler, single AGENTS.md for all agents
- **V1 (default in v5.x)**: Agent-specific files - separate files per agent
Run `vibe-check status` to see the installed template version.

**How do I switch between v1 and v2 templates?**
```bash
# Switch to v2 (agents.md standard)
vibe-check config source.url https://github.com/heikopanjas/vibe-check/tree/develop/templates/v2
vibe-check update

# Switch to v1 (agent-specific files)
vibe-check config source.url https://github.com/heikopanjas/vibe-check/tree/develop/templates/v1
vibe-check update
```

**What's the difference between v1 and v2 templates?**
- **V2**: One AGENTS.md file that works with all agents. Simpler, follows agents.md community standard.
- **V1**: Separate instruction files per agent (CLAUDE.md, copilot-instructions.md, etc.). More files to manage.

**When should I use --no-lang?**
Use `--no-lang` when you want AGENTS.md with mission, principles, and integration (e.g. git) only—no language-specific coding conventions or config files (.rustfmt.toml, .editorconfig, etc.). Good for documentation repositories, multi-language projects, or when you prefer a minimal setup.

**How do I switch agents without changing the language?**
Run `vibe-check init --agent <new-agent>`. vibe-check detects the existing language from the file tracker and uses it (e.g. switching from Cursor to Claude keeps your Rust setup).

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
cargo run -- init --lang rust

# Build in release mode (optimized, generates man pages)
cargo build --release

# Format code
cargo fmt

# Run linter
cargo clippy
```

---

<img src="docs/images/made-in-berlin-badge.jpg" alt="Made in Berlin" width="220" style="border: 5px solid white;">

Last updated: February 15, 2026
