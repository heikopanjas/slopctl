# regulator

**A manager for coding agent instruction files** – A Rust CLI tool that provides a centralized system for managing, organizing, and maintaining initialization prompts and instruction files for AI coding assistants. Supports the [agents.md community standard](https://agents.md) where a single AGENTS.md file works across all agents (Claude, Cursor, Copilot, Aider, Jules, Factory, and others) with built-in governance guardrails and human-in-the-loop controls. Also supports [Agent Skills](https://agentskills.io) for extending agent capabilities with specialized knowledge and workflows.

![MIT License](https://img.shields.io/badge/-MIT%20License-000000?style=flat-square&logo=opensource&logoColor=white)
![CLI](https://img.shields.io/badge/-CLI-000000?style=flat-square&logo=zsh&logoColor=white)
![Rust](https://img.shields.io/badge/-Rust-000000?style=flat-square&logo=rust&logoColor=white)
![Claude](https://img.shields.io/badge/-Claude-000000?style=flat-square&logo=anthropic&logoColor=white)
![GitHub Copilot](https://img.shields.io/badge/-GitHub%20Copilot-000000?style=flat-square&logo=github&logoColor=white)
![Codex](https://img.shields.io/badge/-Codex-000000?style=flat-square&logo=openai&logoColor=white)
![Cursor](https://img.shields.io/badge/-Cursor-000000?style=flat-square&logo=visualstudiocode&logoColor=white)

## Overview

regulator is a command-line tool that helps you:

- **Manage templates globally** – Store templates in platform-specific directories (e.g., `~/Library/Application Support/regulator/templates` on macOS)
- **Configure via YAML** – Define template structure and file mappings in `templates.yml`
- **Initialize projects quickly** – Set up agent instructions with a single command
- **agents.md standard** – V2 templates follow the [agents.md](https://agents.md) community standard (single AGENTS.md for all agents)
- **Agent Skills support** – Define and install [Agent Skills](https://agentskills.io) (SKILL.md) from templates or GitHub repos
- **GitHub skill loading** – Install skills ad-hoc from GitHub with `--skill user/repo` shorthand or full URLs
- **Keep templates synchronized** – Update global templates from remote sources
- **Enforce governance** – Built-in guardrails for no auto-commits and human confirmation
- **Support multiple agents** – Compatible with Claude, Cursor, Copilot, Aider, Jules, Factory, and more
- **Flexible file placement** – Use placeholders (`$workspace`, `$userprofile`) for custom locations
- **Template versioning** – V2/V3 templates with shared file groups and composable languages

## Repository Structure

```text
regulator/
├── Cargo.toml                  # Rust project manifest
├── Cargo.lock                  # Dependency lock file
├── build.rs                    # Build script for man page generation
├── .rustfmt.toml               # Rust formatting configuration
├── src/                        # Rust source code
│   ├── main.rs                 # Application entry point and CLI
│   ├── lib.rs                  # Library public API
│   ├── agent_defaults.rs       # Agent path registry (instructions, prompts, skills per agent)
│   ├── bom.rs                  # Bill of Materials structures (AgentConfig, TemplateConfig)
│   ├── config.rs               # Configuration management
│   ├── download_manager.rs     # DownloadManager for URL downloads
│   ├── file_tracker.rs         # SHA-256 file tracking for modification detection
│   ├── github.rs               # GitHub API integration (URL parsing, Contents API, downloads)
│   ├── template_engine.rs      # TemplateEngine struct, fragment merging, update logic
│   ├── template_manager/       # TemplateManager implementation (directory module)
│   │   ├── mod.rs              # Struct, constructor, and helpers
│   │   ├── update.rs           # install/update command logic
│   │   ├── purge.rs            # Purge all regulator files
│   │   ├── remove.rs           # Remove agent-specific files
│   │   ├── status.rs           # Show project status
│   │   └── list.rs             # List available agents/languages
│   └── utils.rs                # Utility functions
├── LICENSE                     # MIT license
├── README.md                   # You are here
├── AGENTS.md                   # Primary project instructions
├── templates/                  # Template files organized by version
│   └── v3/                     # Version 3 templates (agents.md standard, default)
│       ├── templates.yml       # V3 template configuration (version: 3)
│       ├── AGENTS.md           # Single instruction file for all agents
│       ├── claude/             # Claude-specific files (CLAUDE.md references AGENTS.md)
│       ├── copilot/            # GitHub Copilot files
│       ├── codex/              # Codex files
│       ├── cursor/             # Cursor files
│       └── ...                 # Language templates (coding conventions, build commands, etc.)
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

regulator uses the V3 template format (agents.md standard):

### Version 3 (Default) - agents.md Standard

**Philosophy**: One AGENTS.md file that works across all agents. V3 adds shared file groups and composable languages via `includes`.

- Follows the [agents.md](https://agents.md) community standard
- Single AGENTS.md file compatible with Claude, Cursor, Copilot, Aider, Jules, Factory, and more
- Agent-specific instruction files (e.g. CLAUDE.md) reference AGENTS.md when needed
- [Agent Skills](https://agentskills.io) support: define SKILL.md files per agent for specialized capabilities
- Simpler initialization: `regulator install --lang rust` or `regulator install --no-lang` for language-independent setup
- Optional `--lang` and `--agent` (specify at least one; `--agent` alone preserves existing language when switching)
- GitHub URL support: `source` fields in templates.yml accept full GitHub URLs for remote files
- Ad-hoc skill loading: `--skill user/repo` shorthand or full GitHub URLs
- URL: `https://github.com/heikopanjas/regulator/tree/develop/templates/v3`

**V3 additions:** Shared file groups (`shared` section in templates.yml) and composable languages (`includes` on language configs) let you reuse files (e.g. cmake) across C and C++ without duplication.

**Usage:**

```bash
regulator update                    # Downloads V3 templates
regulator install --lang rust          # With language conventions
regulator install --no-lang            # Language-independent (AGENTS.md only)
regulator install --agent cursor       # Switch agent, keep existing language
regulator install --skill user/repo    # Install a skill from GitHub
```

### Migration from v8 to v9

**Upgrading from v8.x to v9.0.0:** Only V2/V3 templates are supported. Use V3 templates (default source: `templates/v3`).

```bash
regulator update                    # Gets V3 templates
regulator install --lang rust       # Initialize with V3
```

### Migration from v7 to v8

**Upgrading from v7.x to v8.0.0:**

v8.0.0 is a major version bump with one breaking change: **the `init` command has been renamed to `install`**. Update any scripts or aliases accordingly.

```bash
# Before (v7):  regulator init --lang rust --agent cursor
# After  (v8):  regulator install --lang rust --agent cursor
```

**New features in v8.0.0:**

- `--skill` flag: Install skills from GitHub repos (`--skill user/repo` or `--skill https://github.com/...`)
- GitHub URL support in templates.yml `source` fields (full URLs only, no shorthand)
- Top-level `skills` section in templates.yml for agent-agnostic skills
- `agent_defaults.rs`: built-in registry of agent paths (instructions, prompts, skills)
- `github.rs`: GitHub Contents API integration for on-the-fly downloads
- Automatic agent detection when `--agent` is not specified (checks workspace for known agent files)
- `UpdateOptions` struct now carries all parameters through the call chain

## Installation

### From Source

```bash
git clone https://github.com/heikopanjas/regulator.git
cd regulator
cargo build --release
sudo cp target/release/regulator /usr/local/bin/
```

### Using Cargo

```bash
cargo install --path .
```

## Quick Start

### V2 Templates (Default)

```bash
# 1. Download global templates (v2 by default)
regulator update

# 2. Initialize your project (choose one style)
cd your-project
regulator install --lang rust         # With Rust conventions and config files
regulator install --no-lang          # Language-independent (AGENTS.md + integration only)
regulator install --agent cursor     # Agent prompts + skills (preserves existing language)
```

With `--lang rust` this will:

1. Copy main AGENTS.md template to your project
2. Merge language-specific fragments (Rust conventions, build commands) into AGENTS.md
3. Copy language config files (.rustfmt.toml, .editorconfig, .gitignore, .gitattributes)
4. **Single AGENTS.md works with all agents** (Claude, Cursor, Copilot, Aider, Jules, Factory, etc.)

With `--no-lang` you get AGENTS.md with mission, principles, and integration (e.g. git) only—no language-specific files.

### Initialize from a custom template source

```bash
# From a local path
regulator update --from /path/to/templates

# From a GitHub URL
regulator update --from https://github.com/user/repo/tree/branch/templates

# Then initialize the project
regulator install --lang c++ --agent claude
```

**Note:** The custom source must include a `templates.yml` file that defines the template structure.

## Complete Walkthrough: Rust Project (V2 Templates)

This walkthrough demonstrates setting up a new Rust project using regulator with v2 templates (agents.md standard).

### Step 1: Create Your Project Directory

```bash
mkdir my-rust-project
cd my-rust-project
```

### Step 2: Initialize with regulator

```bash
regulator install --lang rust
```

**What happens:**

1. **Downloads templates** (first run only):
   - Fetches `templates.yml` from GitHub (v2 format)
   - Downloads all template files to platform-specific directory (e.g., `~/Library/Application Support/regulator/templates/` on macOS)

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
regulator update

# Then reinitialize the project (will skip customized AGENTS.md unless --force)
regulator install --lang rust
```

regulator will:

- Check if AGENTS.md has been customized (template marker removed)
- Skip customized AGENTS.md unless `--force` is used

### Common Scenarios

**Scenario: Modified AGENTS.md locally**

```bash
$ regulator install --lang rust
! Local AGENTS.md has been customized and will be skipped
→ Other files will still be updated
→ Use --force to overwrite AGENTS.md
```

**Solution:** Review changes, commit them, then use `--force`:

```bash
git diff AGENTS.md              # Review changes
git add AGENTS.md
git commit -m "docs: customize project instructions"
regulator install --lang rust --force
```

**Scenario: Clean up project templates**

```bash
# Remove all regulator files including AGENTS.md
regulator purge

# Removes AGENTS.md, agent files (e.g. CLAUDE.md), and language config files
# Preserves customized AGENTS.md unless --force is used
```

**Scenario: Remove only agent-specific files**

```bash
# Remove all agent files but keep AGENTS.md
regulator remove --all

# Remove only one agent's files
regulator remove --agent claude

# Removes CLAUDE.md, .cursor/commands/, .github/prompts/, etc.
```

**Scenario: Switch from Cursor to Claude (keep Rust setup)**

```bash
# You have Rust + Cursor; want to add Claude prompts
regulator install --agent claude
# Uses existing Rust language; adds Claude prompts only
```

**Scenario: Language-independent project (e.g. docs-only repo)**

```bash
regulator install --no-lang
# AGENTS.md with mission, principles, integration only—no .rustfmt.toml, no coding-conventions
```

**Scenario: Use custom templates**

```bash
# Your team maintains custom v2 templates
regulator update --from https://github.com/yourteam/templates/tree/main/templates

# Then initialize (v2 style)
regulator install --lang rust
```

### Tips for Success

1. **Initialize early**: Run `regulator install` at project start before adding code
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
regulator update [--from <PATH or URL>] [--dry-run]
```

**Options:**

- `--from <string>` - Optional path or URL to download/copy templates from
- `--dry-run` - Preview what would be downloaded without making changes

**Examples:**

```bash
# Update global templates from default repository
regulator update

# Update from custom URL
regulator update --from https://github.com/user/repo/tree/branch/templates

# Update from local path
regulator update --from /path/to/templates

# Preview what would be downloaded
regulator update --dry-run
```

**Behavior:**

- Downloads templates from specified source or default GitHub repository
- If `--from` is not specified, downloads from:
  - **Default**: `https://github.com/heikopanjas/regulator/tree/develop/templates/v3` (agents.md standard)
- Downloads `templates.yml` configuration file and all template files
- Stores templates in local data directory:
  - Linux: `$HOME/.local/share/regulator/templates`
  - macOS: `$HOME/Library/Application Support/regulator/templates`
- If `--dry-run` is specified, shows the source URL and target directory without downloading
- Overwrites existing global templates with new versions
- Does NOT modify any files in the current project directory

**Note:** Run `update` first to download templates before using `install` to set up a project.

### `install` - Install Agent Instructions and Skills

Install instruction files and skills for AI coding agents in your project.

**Usage:**

```bash
# Specify at least one of --lang, --agent, --no-lang, or --skill

# V2: With language conventions
regulator install --lang <language> [--agent <agent>] [--skill <url>]... [--mission <text|@file>] [--force] [--dry-run]

# V2: Language-independent (no coding-conventions fragments)
regulator install --no-lang [--agent <agent>] [--skill <url>]... [--mission <text|@file>] [--force] [--dry-run]

# V2: Switch agent only (preserves existing language)
regulator install --agent <agent> [--skill <url>]... [--mission <text|@file>] [--force] [--dry-run]

# V2: Install skills only (auto-detects agent from workspace)
regulator install --skill <url> [--skill <url>]...
```

**Options:**

- `--lang <string>` - Programming language or framework (e.g., c++, rust, swift, c). Mutually exclusive with `--no-lang`.
- `--agent <string>` - AI coding agent (e.g., claude, copilot, codex, cursor). Optional; when specified alone, preserves existing language when switching agents.
- `--no-lang` - Skip language-specific setup (AGENTS.md with mission/principles/integration only, no coding-conventions). Mutually exclusive with `--lang`.
- `--mission <string>` - Custom mission statement to override the template default. Use `@filename` to read from a file (e.g., `--mission @mission.md`)
- `--skill <string>` - Install skill(s) from GitHub (repeatable). Supports `user/repo` shorthand and full GitHub URLs.
- `--force` - Force overwrite of local files without confirmation
- `--dry-run` - Preview changes without applying them

**Examples (V2 templates):**

```bash
# Initialize Rust project (works with all agents)
regulator install --lang rust

# Initialize C++ project
regulator install --lang c++

# Language-independent setup (AGENTS.md + integration only, no .rustfmt.toml etc.)
regulator install --no-lang

# Language-independent + agent prompts (e.g. init-session command for Cursor)
regulator install --no-lang --agent cursor

# Switch from Cursor to Claude (keeps existing language e.g. Rust)
regulator install --agent claude

# Initialize with custom mission statement (inline)
regulator install --lang rust --mission "A CLI tool for managing AI agent instructions"

# Initialize with mission statement from file (multi-line support)
regulator install --lang rust --mission @mission.md

# Force overwrite existing local files
regulator install --lang swift --force

# Install a skill from GitHub (user/repo shorthand)
regulator install --agent cursor --skill user/my-skill

# Install a skill from a full GitHub URL
regulator install --agent cursor --skill https://github.com/user/skills/tree/main/create-rule

# Install multiple skills at once
regulator install --agent cursor --skill user/skill-a --skill user/skill-b

# Install skills only (auto-detects agent from workspace)
regulator install --skill user/my-skill

# Preview what would be created/modified
regulator install --lang rust --dry-run
```

**Behavior:**

- Uses global templates to set up agent instructions in the current project
- If global templates do not exist, automatically downloads them from the default repository
- Detects template version from templates.yml
- **Must specify at least one** of `--lang`, `--agent`, `--no-lang`, or `--skill`; `--lang` and `--no-lang` cannot be used together
- **GitHub URL sources**: Any `source` field in templates.yml can be a full GitHub URL (downloaded on-the-fly)
- **`--skill` flag**: Downloads skill directories from GitHub and installs to the active agent's skill directory; auto-detects agent if `--agent` not specified
- **V2 with `--agent` only**: Preserves existing installation language (e.g. switch Cursor→Claude, keep Rust); falls back to first available language for fresh init
- **V2 with `--no-lang`**: Skips language fragments; creates AGENTS.md with mission, principles, integration only (no .rustfmt.toml, .editorconfig, etc.); optional `--agent` adds agent prompts
- **V2 with `--lang`**: Creates single AGENTS.md plus language config files; optional `--agent` adds agent prompts
- Checks for local modifications to AGENTS.md (detects if template marker has been removed)
- If local AGENTS.md has been customized and `--force` is not specified, skips AGENTS.md
- If `--force` is specified, overwrites local files regardless of modifications
- If `--dry-run` is specified, shows what would be created/modified without making changes
- Files are placed according to `templates.yml` configuration with placeholder resolution:
  - `$workspace` resolves to current directory
  - `$userprofile` resolves to user's home directory
- Merges language-specific and integration fragments into AGENTS.md

### `purge` - Purge All Vibe-Check Files

Purge all regulator files from the current project directory.

**Usage:**

```bash
regulator purge [--force] [--dry-run]
```

**Options:**

- `--force` - Force purge without confirmation and delete customized AGENTS.md
- `--dry-run` - Preview what would be deleted without making changes

**Examples:**

```bash
# Purge all regulator files with confirmation prompt
regulator purge

# Force purge without confirmation
regulator purge --force

# Preview what would be deleted
regulator purge --dry-run
```

**Behavior:**

- Uses Bill of Materials (BoM) from templates.yml to discover all agent-specific files
- Removes all agent-specific files from all agents (instructions, prompts, skills, directories)
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
regulator remove --agent <agent> [--force] [--dry-run]

# Remove all agent-specific files (keeps AGENTS.md)
regulator remove --all [--force] [--dry-run]
```

**Options:**

- `--agent <string>` - AI coding agent (e.g., claude, copilot, codex, cursor)
- `--all` - Remove all agent-specific files (cannot be used with --agent)
- `--force` - Force removal without confirmation
- `--dry-run` - Preview what would be deleted without making changes

**Examples:**

```bash
# Remove Claude-specific files with confirmation
regulator remove --agent claude

# Remove Copilot files without confirmation
regulator remove --agent copilot --force

# Remove all agent-specific files (keeps AGENTS.md)
regulator remove --all

# Remove all agents with force
regulator remove --all --force

# Preview what would be deleted
regulator remove --all --dry-run
```

**Behavior:**

- Loads templates.yml from global storage to build Bill of Materials (BoM)
- BoM maps agent names to their target file paths in the workspace
- Only removes files that exist in the current directory
- Shows list of files to be removed before deletion
- Asks for confirmation unless `--force` is specified
- If `--dry-run` is specified, shows files that would be deleted without removing them
- Removes agent-specific files (instructions, prompts, and skills)
- Automatically cleans up empty parent directories
- **NEVER touches AGENTS.md** (use `purge` command to remove AGENTS.md)
- Does NOT affect global templates in local data directory
- If agent not found in BoM, shows list of available agents
- Cannot specify both `--agent` and `--all` (mutually exclusive)
- Must specify either `--agent` or `--all`

### `status` - Show Project Status

Display the current status of regulator in the project.

**Usage:**

```bash
regulator status
```

**Output includes:**

- **Global Templates:** Whether templates are installed and their location
  - Template version
  - Available agents (from templates.yml)
  - Available languages (from templates.yml)
- **Project Status:**
  - AGENTS.md existence and customization status
  - Which agents are currently installed
- **Managed Files:** List of all regulator managed files in current directory

**Example output:**

```
regulator status

Global Templates:
  ✓ Installed at: /Users/.../regulator/templates
  → Template version: 2
  → Available agents: claude, copilot, codex, cursor
  → Available languages: c, c++, rust, swift

Project Status:
  ✓ AGENTS.md: exists (customized)
  ✓ Installed agents: claude, cursor
  ✓ Installed skills: 2
    • .cursor/skills/create-rule/SKILL.md
    • .cursor/skills/create-skill/SKILL.md

Managed Files:
  • AGENTS.md
  • .claude/commands/init-session.md
  • CLAUDE.md
  • .cursor/skills/create-rule/SKILL.md
  • .cursor/skills/create-skill/SKILL.md
```

### `list` - List Available Options

List all available agents and languages from global templates.

**Usage:**

```bash
regulator list
```

**Output includes:**

- **Available Agents:** All agents defined in templates.yml with installation status and skill counts
- **Available Languages:** All languages defined in templates.yml

**Example output:**

```
regulator list

Available Agents:
  ✓ claude (installed)
  ○ codex
  ✓ copilot (installed)
  ○ cursor (2 skill(s))

Available Languages:
  • c
  • c++
  • rust
  • swift

→ Use 'regulator install --lang <lang> --agent <agent>' to install
```

### `completions` - Generate Shell Completions

Generate shell completion scripts for various shells.

**Usage:**

```bash
regulator completions <shell>
```

**Arguments:**

- `<shell>` - Shell to generate completions for: `bash`, `zsh`, `fish`, `powershell`

**Examples:**

```bash
# Generate zsh completions
regulator completions zsh > ~/.zsh/completions/_regulator

# Generate bash completions
regulator completions bash > ~/.bash_completion.d/regulator

# Generate fish completions
regulator completions fish > ~/.config/fish/completions/regulator.fish

# Generate PowerShell completions
regulator completions powershell > regulator.ps1
```

### `config` - Manage Configuration

Manage persistent configuration settings using Git-style dotted keys.

**Usage:**

```bash
regulator config <key> <value>    # Set a configuration value
regulator config <key>            # Get a configuration value
regulator config --list           # List all configuration values
regulator config --unset <key>    # Remove a configuration value
```

**Options:**

- `<key>` - Configuration key (e.g., source.url)
- `<value>` - Value to set (omit to get current value)
- `--list` - List all configuration values
- `--unset <key>` - Remove a configuration key

**Examples:**

```bash
# Set custom template source
regulator config source.url https://github.com/myteam/templates/tree/main/templates

# Get current source URL
regulator config source.url

# List all configuration
regulator config --list

# Remove custom source (revert to default)
regulator config --unset source.url

# Set fallback source for resilience
regulator config source.fallback https://github.com/heikopanjas/regulator/tree/develop/templates
```

**Valid Configuration Keys:**

- `source.url` - Default template download URL (used by `update` and `install` when `--from` not specified)
- `source.fallback` - Fallback URL used when primary source fails or is unreachable

**Configuration File Location:**

- Linux: `$XDG_CONFIG_HOME/regulator/config.yml` or `~/.config/regulator/config.yml`
- macOS: `~/.config/regulator/config.yml`

**Behavior:**

- Configuration persists between sessions
- `update` command uses `source.url` if set and `--from` not specified
- `install` command uses `source.url` when downloading missing global templates
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

### V2 Templates (Default)

**Universal Support**: Single AGENTS.md works with all agents following the [agents.md](https://agents.md) standard:

- Claude (Anthropic)
- Cursor (AI code editor)
- GitHub Copilot (GitHub)
- OpenAI Codex
- Aider (command-line AI)
- Jules (coding assistant)
- Factory (AI dev tool)
- Any agent that reads AGENTS.md

One AGENTS.md for all agents. Agent-specific files (e.g. CLAUDE.md) reference AGENTS.md when needed. Agent-specific [skills](https://agentskills.io) (SKILL.md) can also be defined per agent.

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

- **macOS**: `~/Library/Application Support/regulator/templates/`
- **Linux**: `~/.local/share/regulator/templates/`
- **Windows**: `%LOCALAPPDATA%\regulator\templates\`

Templates include:

- **templates.yml**: Configuration file defining structure and file mappings (with version field)
- **Main template**: AGENTS.md (primary instruction file)
- **Language fragments**: Language-specific coding standards and build commands - merged into AGENTS.md
- **Integration fragments**: Tool/workflow templates (e.g., git-workflow-conventions.md) - merged into AGENTS.md
- **Principle fragments**: Core principles and best practices - merged into AGENTS.md
- **Mission fragments**: Mission statement, technology stack - merged into AGENTS.md
- **Agent templates**: Agent-specific instruction files, prompts, and skills (copied to project directories)
- **Config files**: EditorConfig, format configurations, .gitignore, .gitattributes

### Agent Skills

regulator supports [Agent Skills](https://agentskills.io) – an open format for extending AI agent capabilities with specialized knowledge and workflows.

A skill is a directory containing a `SKILL.md` file with YAML frontmatter (name, description) and Markdown instructions. Skills can optionally include `scripts/`, `references/`, and `assets/` subdirectories.

**Skills can be defined in three ways:**

1. **Per-agent in templates.yml** – Using `source`/`target` mapping under `agents.<name>.skills`
2. **Top-level in templates.yml** – Agent-agnostic skills under the `skills` section (installed to the active agent's skill directory)
3. **Ad-hoc via CLI** – Using `--skill user/repo` or `--skill https://github.com/...` on the `install` command

**How skills work:**

- Per-agent skills use the same `source`/`target` mapping as instructions and prompts
- Top-level skills specify `name` and `source`; the target directory comes from `agent_defaults.rs` based on the active agent
- `--skill` CLI flag supports `user/repo` shorthand (expanded to full GitHub URL) or full `https://github.com/...` URLs
- GitHub skills are downloaded on-the-fly via the GitHub Contents API (no local cache)
- Skills are tracked with the `"skill"` category in the file tracker for modification detection
- The `list` command shows available skills; the `status` command shows installed skills
- Removing an agent (`regulator remove --agent <name>`) also removes its skills
- When `--skill` is used without `--agent`, the active agent is auto-detected from workspace files

**Example per-agent skills in templates.yml:**

```yaml
agents:
  cursor:
    skills:
      - source: cursor/skills/create-rule/SKILL.md
        target: '$workspace/.cursor/skills/create-rule/SKILL.md'
```

**Example top-level skills in templates.yml (agent-agnostic):**

```yaml
skills:
  - name: create-rule
    source: 'https://github.com/user/cursor-skills/tree/main/create-rule'
  - name: my-local-skill
    source: 'skills/my-local-skill'
```

**Example ad-hoc skill installation:**

```bash
regulator install --agent cursor --skill user/my-skill
regulator install --skill https://github.com/user/skills/tree/main/create-rule
```

### Template Configuration (templates.yml)

The `templates.yml` file defines the template structure with a version field and multiple sections:

**Version Field:**

- `version: 2` - V2 templates following agents.md standard
- `version: 3` - V3 templates (superset of V2: shared file groups, composable languages via `includes`)
- Missing version defaults to 3

**Main Sections:**

1. **main**: Main AGENTS.md instruction file (primary source of truth)
2. **agents**: Agent-specific files with `instructions`, `prompts`, and `skills`
3. **languages**: Language-specific coding standards fragments (merged into AGENTS.md)
4. **integration**: Tool/workflow integration fragments (merged into AGENTS.md, e.g., git workflows)
5. **principles**: Core principles and general guidelines fragments (merged into AGENTS.md)
6. **mission**: Mission statement, purpose, and project overview fragments (merged into AGENTS.md)
7. **skills**: Agent-agnostic skill definitions with `name` and `source` (installed to active agent's skill directory)

Each file entry specifies:

- `source`: Path in the template repository, or a full GitHub URL (e.g., `https://github.com/user/repo/tree/main/file.md`)
- `target`: Destination path using placeholders

**Note:** Only full GitHub URLs are supported in `source` fields. The `user/repo` shorthand is CLI-only (`--skill` flag).

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

agents:
    claude:
        instructions:
            - source: claude/CLAUDE.md
              target: '$workspace/CLAUDE.md'
        prompts:
            - source: claude/commands/init-session.md
              target: '$workspace/.claude/commands/init-session.md'
    cursor:
        instructions:
            - source: cursor/cursorrules
              target: '$workspace/.cursorrules'
        prompts:
            - source: cursor/commands/init-session.md
              target: '$workspace/.cursor/commands/init-session.md'
        skills:
            - source: cursor/skills/create-rule/SKILL.md
              target: '$workspace/.cursor/skills/create-rule/SKILL.md'

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

### Template Versioning

Templates include a version field to support different format approaches:

- **Version 2**: agents.md standard - single AGENTS.md for all agents, with Agent Skills support
- **Version 3** (default): Superset of V2 - adds shared file groups and composable languages via `includes`
- Missing version field defaults to 3

The `status` command shows the template version currently installed.

**Version Detection:**
regulator automatically detects the template version from `templates.yml` and uses the appropriate template engine.

### Template Management

1. **First run**: `update` downloads `templates.yml` and all specified files from GitHub
2. **Local storage**: Templates are cached in platform-specific directory
3. **Protection**: Template marker in AGENTS.md detects customization and prevents accidental overwrites
4. **Updates**: Detect AGENTS.md customization and warn before overwriting
5. **Placeholders**: `$workspace` and `$userprofile` resolve to appropriate paths

### Project Initialization

**V2 Templates** (when you run `regulator install --lang rust`):

1. Checks if global templates exist (downloads v2 by default if needed)
2. Loads `templates.yml` configuration and detects version 2
3. Uses TemplateEngine for agents.md standard
4. Downloads main AGENTS.md template
5. Merges fragments (mission, principles, language, integration) into AGENTS.md at insertion points
6. Copies language config files (.rustfmt.toml, .editorconfig, .gitignore, .gitattributes)
7. Single AGENTS.md works with all agents
8. Optional `--agent` adds agent-specific files (e.g. CLAUDE.md, .cursor/commands/init-session.md) and skills (SKILL.md)
9. You're ready to start coding with any agent

**V2 with `--no-lang`** (language-independent setup):

1. Same as above but skips language fragments and language config files
2. AGENTS.md contains mission, principles, integration (e.g. git, versioning) only
3. Optional `--agent` adds agent prompts

**V2 with `--agent` only** (switch agent, preserve language):

1. Detects existing installation language from file tracker
2. Uses that language; if none, uses first available from templates
3. Adds/updates agent prompts only

The resulting AGENTS.md contains the complete merged content with all relevant sections for your project.

### Modification Detection

regulator detects if you've customized AGENTS.md by checking for the template marker:

```bash
$ regulator install --lang c++ --agent claude
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
regulator update --from /path/to/your/templates

# From a GitHub repository
regulator update --from https://github.com/yourname/your-templates/tree/main/templates

# Then initialize your project
regulator install --lang c++ --agent claude
```

**Note:** Your custom template repository must include a `templates.yml` file that defines the template structure and file mappings.

### Modifying Global Templates

1. Navigate to platform-specific template directory:
   - macOS: `~/Library/Application Support/regulator/templates/`
   - Linux: `~/.local/share/regulator/templates/`
   - Windows: `%LOCALAPPDATA%\regulator\templates\`
2. Edit the templates as needed
3. Run `regulator install` to apply changes to your projects

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
- **Temp Files:** tempfile v3.13
- **Man Pages:** clap_mangen v0.2 (build dependency)

## FAQ

**Where are templates stored?**

- Global templates (macOS): `~/Library/Application Support/regulator/templates/`
- Global templates (Linux): `~/.local/share/regulator/templates/`
- Global templates (Windows): `%LOCALAPPDATA%\regulator\templates\`

**What happens if I modify AGENTS.md?**
regulator detects customization via template marker removal and skips AGENTS.md when updating. Use `--force` to override.

**Can I use my own template repository?**
Yes! Use the `--from` option with the `update` command to specify a local path or GitHub URL.

**Why AGENTS.md as single source of truth?**
Centralized updates prevent drift and make it easier to maintain consistency across sessions.

**Can I use this in commercial projects?**
Yes! MIT license allows commercial use. Attribution appreciated but not required.

**How do I update templates?**
Run `regulator update` to download the latest global templates, then `regulator install` to apply to your project.

**How do I remove local templates?**
Run `regulator purge` to remove all agent files and AGENTS.md, or `regulator remove --all` to keep AGENTS.md.

**How do I preview changes before applying?**
Use the `--dry-run` flag on any command: `regulator install --lang rust --dry-run` or `regulator install --no-lang --dry-run`

**How do I customize the mission statement?**
Use the `--mission` option with `install`. For inline text: `--mission "Your mission here"`. For multi-line content from a file: `--mission @mission.md`. The custom mission replaces the default template placeholder in AGENTS.md.

**What template version should I use?**

- **V3** (default): agents.md standard with shared file groups and composable languages. Single AGENTS.md for all agents, Agent Skills support.
- **V2**: Same as V3 but without shared groups and includes.
Run `regulator status` to see the installed template version.

**When should I use --no-lang?**
Use `--no-lang` when you want AGENTS.md with mission, principles, and integration (e.g. git) only—no language-specific coding conventions or config files (.rustfmt.toml, .editorconfig, etc.). Good for documentation repositories, multi-language projects, or when you prefer a minimal setup.

**How do I switch agents without changing the language?**
Run `regulator install --agent <new-agent>`. regulator detects the existing language from the file tracker and uses it (e.g. switching from Cursor to Claude keeps your Rust setup).

**What are Agent Skills?**
[Agent Skills](https://agentskills.io) are an open format for giving agents specialized capabilities via SKILL.md files. Skills can be defined in `templates.yml` (per-agent or top-level) or installed ad-hoc from GitHub using `--skill user/repo`. Skills are downloaded on-the-fly and tracked like other template files.

**How do I install a skill from GitHub?**
Use the `--skill` flag: `regulator install --agent cursor --skill user/my-skill`. The shorthand `user/repo` is expanded to a GitHub URL. You can also use full URLs: `--skill https://github.com/user/repo/tree/main/path`. If `--agent` is not specified, the active agent is auto-detected from workspace files.

## License

MIT License - See [LICENSE](LICENSE) for details.

## Building from Source

```bash
# Clone the repository
git clone https://github.com/heikopanjas/regulator.git
cd regulator

# Build in debug mode (for development)
cargo build

# Run tests
cargo test

# Run the application
cargo run -- install --lang rust

# Build in release mode (optimized, generates man pages)
cargo build --release

# Format code
cargo fmt

# Run linter
cargo clippy
```

---

<img src="docs/images/made-in-berlin-badge.jpg" alt="Made in Berlin" width="220" style="border: 5px solid white;">

Last updated: March 13, 2026 (v10.0.0)
