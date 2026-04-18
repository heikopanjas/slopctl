# slopctl

**A manager for coding agent instruction files** – A Rust CLI tool that provides a centralized system for managing, organizing, and maintaining initialization prompts and instruction files for AI coding assistants. Supports the [agents.md community standard](https://agents.md) where a single AGENTS.md file works across all agents (Claude Code, Cursor, GitHub Copilot, and Codex) with built-in governance guardrails and human-in-the-loop controls. Also supports [Agent Skills](https://agentskills.io) for extending agent capabilities with specialized knowledge and workflows.

[![Build and Test](https://github.com/heikopanjas/slopctl/actions/workflows/build.yml/badge.svg?branch=develop)](https://github.com/heikopanjas/slopctl/actions/workflows/build.yml)
![MIT License](https://img.shields.io/badge/-MIT%20License-000000?style=flat-square&logo=opensource&logoColor=white)
![CLI](https://img.shields.io/badge/-CLI-000000?style=flat-square&logo=zsh&logoColor=white)
![Rust](https://img.shields.io/badge/-Rust-000000?style=flat-square&logo=rust&logoColor=white)
![Claude](https://img.shields.io/badge/-Claude-000000?style=flat-square&logo=anthropic&logoColor=white)
![GitHub Copilot](https://img.shields.io/badge/-GitHub%20Copilot-000000?style=flat-square&logo=github&logoColor=white)
![Codex](https://img.shields.io/badge/-Codex-000000?style=flat-square&logo=openai&logoColor=white)
![Cursor](https://img.shields.io/badge/-Cursor-000000?style=flat-square&logo=visualstudiocode&logoColor=white)

## Overview

slopctl is a command-line tool that helps you:

- **Manage templates globally** – Store templates in platform-specific directories (e.g., `~/Library/Application Support/slopctl/templates` on macOS)
- **Configure via YAML** – Define template structure and file mappings in `templates.yml`
- **Initialize projects quickly** – Set up agent instructions with a single command
- **agents.md standard** – Follow the [agents.md](https://agents.md) community standard (single AGENTS.md for all agents)
- **Agent Skills support** – Define and install [Agent Skills](https://agentskills.io) (SKILL.md) from templates or GitHub repos
- **Independent skill loading** – Install skills standalone with `--skill user/repo` (no templates or agent required); uses cross-client `.agents/skills/` directory per agentskills.io spec
- **Keep templates synchronized** – Update global templates from remote sources
- **AI-assisted merge** – Merge customized files with updated templates using LLM providers (OpenAI, Anthropic, Ollama, Mistral)
- **Workspace health checks** – Detect and fix stale or broken managed files with `doctor --fix`
- **Enforce governance** – Built-in guardrails for no auto-commits and human confirmation
- **Support multiple agents** – Compatible with Claude Code, Cursor, GitHub Copilot, and Codex
- **Flexible file placement** – Use placeholders (`$workspace`, `$userprofile`) for custom locations
- **Template versioning** – V5 templates with shared file groups (with skill propagation), composable languages, agent/language skill associations, and agent directories

## Repository Structure

```text
slopctl/
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
│   ├── llm.rs                  # LLM provider abstraction (OpenAI, Anthropic, Ollama, Mistral)
│   ├── template_engine.rs      # TemplateEngine struct, fragment merging, update logic
│   ├── template_manager/       # TemplateManager implementation (directory module)
│   │   ├── mod.rs              # Struct, constructor, and helpers
│   │   ├── update.rs           # init/update command logic
│   │   ├── merge.rs            # AI-assisted merge command logic
│   │   ├── purge.rs            # Purge all slopctl files
│   │   ├── remove.rs           # Remove agent/language/skill files
│   │   ├── doctor.rs           # Workspace health checks and fixes
│   │   └── list.rs             # List available agents/languages and workspace status
│   └── utils.rs                # Utility functions
├── LICENSE                     # MIT license
├── README.md                   # You are here
├── AGENTS.md                   # Primary project instructions
├── templates/                  # Template files organized by version
│   └── v5/                     # Version 5 templates (agents.md standard, default)
│       ├── templates.yml       # V5 template configuration (version: 5)
│       ├── AGENTS.md           # Single instruction file for all agents
│       ├── claude/             # Claude-specific files (CLAUDE.md references AGENTS.md)
│       ├── copilot/            # GitHub Copilot files
│       ├── cursor/             # Cursor files
│       ├── skills/             # Agent Skills (coding conventions, build commands, etc.)
│       └── ...                 # Language config templates and skill hint fragments
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

## Template Format (V5)

slopctl uses the V5 template format following the [agents.md](https://agents.md) standard.

**Philosophy**: One AGENTS.md file that works across all agents.

- Follows the [agents.md](https://agents.md) community standard
- Single AGENTS.md file compatible with Claude Code, Cursor, GitHub Copilot, and Codex
- Agent-specific instruction files (e.g. CLAUDE.md) reference AGENTS.md when needed
- [Agent Skills](https://agentskills.io) support: define skills per agent, per language, or as top-level entries
- Shared file groups (`shared` section) and composable languages (`includes`) for reuse across languages
- Skills associated with agents, languages, or shared groups — skills propagate via `includes` (from shared groups and from included languages)
- Simpler initialization: `slopctl init --lang rust` or omit `--lang` for language-independent setup
- Optional `--lang` and `--agent` (specify at least one; `--agent` alone preserves existing language when switching)
- GitHub URL support: `source` fields in templates.yml accept full GitHub URLs for remote files
- Independent skill loading: `--skill` works standalone (no templates, no agent required) or combined with `--lang`/`--agent`
- Cross-client skill directory: standalone `--skill` installs to `.agents/skills/` per agentskills.io spec
- URL: `https://github.com/heikopanjas/slopctl/tree/develop/templates/v5`

**Usage:**

```bash
slopctl templates --update            # Downloads V5 templates
slopctl init --lang rust           # With language conventions
slopctl init --agent cursor        # Agent only (AGENTS.md + agent prompts, no language files)
slopctl init --skill user/repo     # Install a skill (standalone, to .agents/skills/)
```

### Migration from v12 to v13

**Upgrading from v12.x to v13.0.0:**

v13.0.0 is a major version bump with breaking changes:

1. **`install` renamed back to `init`**: Update any scripts or aliases.
2. **Codex templates modernized**: `CODEX.md` and `~/.codex/prompts/init-session.md` are no longer installed. Codex reads `AGENTS.md` natively.
3. **Session Protocol**: The AGENTS.md template now includes a Session Protocol section for agents that read it directly.
4. **`merge` command**: AI-assisted merge of customized workspace files with updated templates (supports OpenAI, Anthropic, Ollama, Mistral).

```bash
# Before (v12): slopctl install --lang rust --agent cursor
# After  (v13): slopctl init --lang rust --agent cursor
```

### Migration from v8 to v9

**Upgrading from v8.x to v9.0.0:** Use V5 templates (default source: `templates/v5`).

```bash
slopctl templates --update        # Gets V5 templates
slopctl init --lang rust          # Initialize with V5
```

### Migration from v7 to v8

**Upgrading from v7.x to v8.0.0:**

v8.0.0 renamed `init` to `install` (reversed in v13.0.0 back to `init`).

**New features in v8.0.0:**

- `--skill` flag: Install skills from GitHub repos or local paths
- GitHub URL support in templates.yml `source` fields (full URLs only, no shorthand)
- Top-level `skills` section in templates.yml for agent-agnostic skills
- `agent_defaults.rs`: built-in registry of agent paths (instructions, prompts, skills)
- `github.rs`: GitHub Contents API integration for on-the-fly downloads
- Automatic agent detection when `--agent` is not specified
- `UpdateOptions` struct now carries all parameters through the call chain

## Installation

### From Source

```bash
git clone https://github.com/heikopanjas/slopctl.git
cd slopctl
cargo build --release
sudo cp target/release/slopctl /usr/local/bin/
```

### Using Cargo

```bash
cargo install --path .
```

## Quick Start

```bash
# 1. Download global templates
slopctl templates --update

# 2. Initialize your project (choose one style)
cd your-project
slopctl init --lang rust         # With Rust conventions and config files
slopctl init --agent cursor     # Agent prompts + skills (AGENTS.md without language files)
slopctl init --skill user/repo  # Install a skill only (no templates needed)
```

With `--lang rust` this will:

1. Copy main AGENTS.md template to your project
2. Merge skill hint fragments into AGENTS.md (tells agents about available coding skills)
3. Copy language config files (.rustfmt.toml, .editorconfig, .gitignore)
4. Install language skills (rust-coding-conventions, rust-build-commands) to `.agents/skills/`
5. **Single AGENTS.md works with all agents** (Claude Code, Cursor, GitHub Copilot, and Codex)

Without `--lang`, you get AGENTS.md with mission, principles, and integration (e.g. git) only—no language-specific files.

### Initialize from a custom template source

```bash
# From a local path
slopctl templates --update --from /path/to/templates

# From a GitHub URL
slopctl templates --update --from https://github.com/user/repo/tree/branch/templates

# Then initialize the project
slopctl init --lang c++ --agent claude
```

**Note:** The custom source must include a `templates.yml` file that defines the template structure.

## Complete Walkthrough: Rust Project

This walkthrough demonstrates setting up a new Rust project using slopctl.

### Step 1: Create Your Project Directory

```bash
mkdir my-rust-project
cd my-rust-project
```

### Step 2: Initialize with slopctl

```bash
slopctl init --lang rust
```

**What happens:**

1. **Downloads templates** (first run only):
   - Fetches `templates.yml` from GitHub (V5 format)
   - Downloads all template files to platform-specific directory (e.g., `~/Library/Application Support/slopctl/templates/` on macOS)

2. **Processes configuration**:
   - Detects template version 5 (agents.md standard)
   - Identifies fragments marked with `$instructions` placeholder

3. **Creates main AGENTS.md**:
   - Downloads main AGENTS.md template
   - Merges fragments at insertion points:
     - **Mission section**: mission-statement.md, technology-stack.md
     - **Principles section**: core-principles.md, best-practices.md
     - **Languages section**: skill hint fragment (tells agents about available skills)
     - **Integration section**: git-workflow summary, semantic-versioning summary
   - Saves complete merged file to `./AGENTS.md`

4. **Installs language config files**:
   - Copies `.rustfmt.toml` for Rust formatting
   - Copies `.editorconfig` for editor configuration
   - Copies `.gitignore` for Rust artifacts

5. **Installs language skills** (as Agent Skills to `.agents/skills/`):
   - `rust-coding-conventions` — Rust coding standards and conventions
   - `rust-build-commands` — Cargo build commands and workflows

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
└── .agents/skills/                    # Cross-client Agent Skills directory
    ├── rust-coding-conventions/       # Rust coding standards skill
    │   └── SKILL.md
    └── rust-build-commands/           # Cargo build commands skill
        └── SKILL.md
```

### Step 4: Start Coding with Any Agent

**With Claude Code or Cursor:**

Open your agent and reference `AGENTS.md` in project settings. The single AGENTS.md works automatically.

**With GitHub Copilot:**

Copilot automatically reads AGENTS.md from your workspace.

**With Codex:**

Codex reads AGENTS.md from your workspace automatically.

### Step 5: Verify Agent Understands Instructions

Ask your agent to confirm:

```text
Please confirm you've read AGENTS.md and understand the project instructions.
```

The agent should acknowledge the:

- Commit protocol (no auto-commits)
- Available coding skills (Rust conventions, build commands)
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
slopctl templates --update

# Then reinitialize the project (will skip customized AGENTS.md unless --force)
slopctl init --lang rust
```

slopctl will:

- Check if AGENTS.md has been customized (template marker removed)
- Skip customized AGENTS.md unless `--force` is used

### Common Scenarios

**Scenario: Modified AGENTS.md locally**

```bash
$ slopctl init --lang rust
! Local AGENTS.md has been customized and will be skipped
→ Other files will still be updated
→ Use --force to overwrite AGENTS.md
```

**Solution:** Review changes, commit them, then use `--force`:

```bash
git diff AGENTS.md              # Review changes
git add AGENTS.md
git commit -m "docs: customize project instructions"
slopctl init --lang rust --force
```

**Scenario: Clean up project templates**

```bash
# Remove all slopctl files including AGENTS.md
slopctl purge

# Removes AGENTS.md, agent files (e.g. CLAUDE.md), and language config files
# Preserves customized AGENTS.md unless --force is used
```

**Scenario: Remove only agent-specific files**

```bash
# Remove all agent files but keep AGENTS.md
slopctl remove --all

# Remove only one agent's files
slopctl remove --agent claude

# Removes CLAUDE.md, .cursor/commands/, .github/prompts/, etc.
```

**Scenario: Remove language config files (switch languages)**

```bash
# Remove Rust config files (.rustfmt.toml, .editorconfig, .gitignore, etc.)
slopctl remove --lang rust

# Then install C++ config files
slopctl init --lang c++
```

**Scenario: Diagnose and fix workspace issues**

```bash
# Check for broken/stale managed files
slopctl doctor --verbose

# Fix what can be fixed automatically (prune stale entries, strip unmerged markers)
slopctl doctor --fix

# Re-merge language sections after fixing an unmerged AGENTS.md
slopctl init --lang rust
```

**Scenario: Switch from Cursor to Claude (keep Rust setup)**

```bash
# You have Rust + Cursor; want to add Claude prompts
slopctl init --agent claude
# Uses existing Rust language; adds Claude prompts only
```

**Scenario: Language-independent project (e.g. docs-only repo)**

```bash
slopctl init --agent cursor
# AGENTS.md with mission, principles, integration + agent prompts—no .rustfmt.toml, no coding-conventions
```

**Scenario: Use custom templates**

```bash
# Your team maintains custom templates
slopctl templates --update --from https://github.com/yourteam/templates/tree/main/templates

# Then initialize
slopctl init --lang rust
```

### Tips for Success

1. **Initialize early**: Run `slopctl init` at project start before adding code
2. **Commit instructions**: Add AGENTS.md and agent files to version control
3. **Team consistency**: All team members should use same template source
4. **Customize carefully**: Modify AGENTS.md as needed, but track changes in git
5. **Update periodically**: Check for template updates monthly or quarterly
6. **Use force sparingly**: Only use `--force` when you understand what you're overwriting
7. **Use version control**: Git is your primary safety net for tracking changes
8. **Preview first**: Use `--dry-run` to preview changes before applying them

## CLI Commands

### `templates` - Manage Global Template Catalog

Download, update, or browse the global template catalog.

**Usage:**

```bash
slopctl templates --update [--from <PATH or URL>] [--dry-run]
slopctl templates --list
slopctl templates --update --list
```

**Options:**

- `--update` / `-u` - Download or update global templates from source
- `--list` / `-l` - Show available agents, languages, and skills
- `--from` / `-f` - Path or URL to download/copy templates from (requires `--update`)
- `--dry-run` / `-n` - Preview what would be downloaded (requires `--update`)

At least one of `--update` or `--list` is required. Both can be combined to update and then show the catalog.

**Examples:**

```bash
# Update global templates from default repository
slopctl templates --update

# Update from custom URL
slopctl templates --update --from https://github.com/user/repo/tree/branch/templates

# Update from local path
slopctl templates --update --from /path/to/templates

# Preview what would be downloaded
slopctl templates --update --dry-run

# Browse available agents, languages, and skills
slopctl templates --list

# Update and then show what is available
slopctl templates --update --list
```

**Behavior:**

- Downloads templates from specified source or default GitHub repository
- If `--from` is not specified, downloads from:
  - **Default**: `https://github.com/heikopanjas/slopctl/tree/develop/templates/v5` (agents.md standard)
- Downloads `templates.yml` configuration file and all template files
- Stores templates in local data directory:
  - Linux: `$HOME/.local/share/slopctl/templates`
  - macOS: `$HOME/Library/Application Support/slopctl/templates`
- If `--dry-run` is specified, shows the source URL and target directory without downloading
- Overwrites existing global templates with new versions
- Does NOT modify any files in the current project directory

**Note:** Run `templates --update` first to download templates before using `init` to set up a project.

### `init` - Initialize Agent Instructions and Skills

Initialize instruction files and skills for AI coding agents in your project.

**Usage:**

```bash
# Specify at least one of --lang, --agent, or --skill

# With language conventions
slopctl init --lang <language> [--agent <agent>] [--skill <url>]... [--mission <text|@file>] [--force] [--dry-run]

# Agent only (preserves existing language, or language-independent if fresh)
slopctl init --agent <agent> [--skill <url>]... [--mission <text|@file>] [--force] [--dry-run]

# Install skills only (standalone, no templates or agent required)
slopctl init --skill <url> [--skill <url>]... [--force] [--dry-run]
```

**Options:**

- `--lang <string>` - Programming language or framework (e.g., c++, rust, swift, c). Optional; omit for language-independent setup.
- `--agent <string>` - AI coding agent (e.g., claude, copilot, codex, cursor). Optional; when specified alone, preserves existing language when switching agents.
- `--mission <string>` - Custom mission statement to override the template default. Use `@filename` to read from a file (e.g., `--mission @mission.md`)
- `--skill <string>` - Install skill(s) from GitHub or local paths (repeatable). Supports `user/repo` shorthand, full GitHub URLs, and local paths (`./skill`, `~/skills/my-skill`, `/absolute/path`). Works standalone (no `--lang` or `--agent` required).
- `--force` - Force overwrite of local files without confirmation
- `--dry-run` - Preview changes without applying them

**Examples:**

```bash
# Initialize Rust project (works with all agents)
slopctl init --lang rust

# Initialize C++ project
slopctl init --lang c++

# Agent only (AGENTS.md + agent prompts, no language files)
slopctl init --agent cursor

# Switch from Cursor to Claude (keeps existing language e.g. Rust)
slopctl init --agent claude

# Initialize with custom mission statement (inline)
slopctl init --lang rust --mission "A CLI tool for managing AI agent instructions"

# Initialize with mission statement from file (multi-line support)
slopctl init --lang rust --mission @mission.md

# Force overwrite existing local files
slopctl init --lang swift --force

# Install a skill standalone (to .agents/skills/ cross-client directory)
slopctl init --skill user/my-skill

# Install multiple skills standalone
slopctl init --skill user/skill-a --skill user/skill-b

# Install a skill with a specific agent (to agent-specific directory, e.g. .cursor/skills/)
slopctl init --agent cursor --skill user/my-skill

# Install a skill from a full GitHub URL
slopctl init --skill https://github.com/user/skills/tree/main/create-rule

# Install a skill from a local path
slopctl init --skill ./path/to/skill
slopctl init --skill ~/skills/my-skill

# Preview what would be created/modified
slopctl init --lang rust --dry-run
```

**Behavior:**

- Uses global templates to set up agent instructions in the current project
- If global templates do not exist, automatically downloads them from the default repository
- Detects template version from templates.yml
- **Must specify at least one** of `--lang`, `--agent`, or `--skill`
- **GitHub URL sources**: Any `source` field in templates.yml can be a full GitHub URL (downloaded on-the-fly)
- **`--skill` standalone**: When used without `--lang` or `--agent`, installs skills directly to the cross-client `$workspace/.agents/skills/` directory without downloading global templates or creating AGENTS.md
- **`--skill` with `--agent`**: Installs skills to the agent-specific directory (e.g. `.cursor/skills/`) alongside agent templates
- **`--skill` with `--lang`** (no agent): Installs skills to the cross-client `.agents/skills/` directory alongside language templates
- **With `--agent` only** (no `--lang`): Creates AGENTS.md with mission, principles, integration (no language files); preserves existing language if previously installed; installs agent-associated skills from templates.yml; creates agent-declared directories (e.g. `.cursor/plans`)
- **With `--lang`**: Creates single AGENTS.md plus language config files; installs language-associated skills (own + inherited from shared groups) from templates.yml to cross-client directory; optional `--agent` adds agent prompts and agent skills
- Checks for local modifications to AGENTS.md (detects if template marker has been removed)
- If local AGENTS.md has been customized and `--force` is not specified, skips AGENTS.md
- If `--force` is specified, overwrites local files regardless of modifications
- If `--dry-run` is specified, shows what would be created/modified without making changes
- Files are placed according to `templates.yml` configuration with placeholder resolution:
  - `$workspace` resolves to current directory
  - `$userprofile` resolves to user's home directory
- Merges language-specific and integration fragments into AGENTS.md

### `purge` - Purge All Slopctl Files

Purge all slopctl files from the current project directory.

**Usage:**

```bash
slopctl purge [--force] [--dry-run]
```

**Options:**

- `--force` - Force purge without confirmation and delete customized AGENTS.md
- `--dry-run` - Preview what would be deleted without making changes

**Examples:**

```bash
# Purge all slopctl files with confirmation prompt
slopctl purge

# Force purge without confirmation
slopctl purge --force

# Preview what would be deleted
slopctl purge --dry-run
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

### `remove` - Remove Agent, Language, or Skill Files

Remove agent-specific, language-specific, or named skill files from the current directory.

**Usage:**

```bash
# Remove specific agent's files
slopctl remove --agent <agent> [--force] [--dry-run]

# Remove language disk files (e.g. .rustfmt.toml, .editorconfig)
slopctl remove --lang <lang> [--force] [--dry-run]

# Remove a named skill
slopctl remove --skill <name> [--force] [--dry-run]

# Remove all agent-specific files and skills (keeps AGENTS.md)
slopctl remove --all [--force] [--dry-run]
```

**Options:**

- `--agent <string>` - AI coding agent (e.g., claude, copilot, codex, cursor)
- `--lang <string>` - Language to remove disk files for (e.g., rust, c++, swift). Skips `$instructions` fragments (merged into AGENTS.md) and `$userprofile` paths.
- `--skill <string>` - Skill name to remove (repeatable). Scans all agent skill dirs and the cross-client `.agents/skills/` directory.
- `--all` - Remove all agent-specific files and skills (cannot be used with `--agent` or `--lang`)
- `--force` - Force removal without confirmation
- `--dry-run` - Preview what would be deleted without making changes

**Examples:**

```bash
# Remove Claude-specific files with confirmation
slopctl remove --agent claude

# Remove Rust language files (.rustfmt.toml, .editorconfig, etc.)
slopctl remove --lang rust

# Remove a named skill
slopctl remove --skill create-rule

# Remove language files and agent files together
slopctl remove --lang rust --agent cursor

# Remove all agent-specific files (keeps AGENTS.md)
slopctl remove --all

# Remove all agents with force
slopctl remove --all --force

# Preview what would be deleted
slopctl remove --lang rust --dry-run
slopctl remove --all --dry-run
```

**Behavior:**

- Loads templates.yml from global storage to build Bill of Materials (BoM)
- `--agent`: removes agent instruction, prompt, and skill files; BoM is the source of truth
- `--lang`: resolves the language's complete file list via `resolve_language_files` (honours `includes` chains); skips `$instructions` fragments and `$userprofile` paths; validates the language name against templates.yml
- `--skill`: scans all agent skill directories and the cross-client `.agents/skills/` directory for that skill name; also sweeps FileTracker for any tracked paths outside standard directories; stale tracker entries are silently pruned
- Only removes files that exist in the current directory
- Shows list of files to be removed before deletion
- Asks for confirmation unless `--force` is specified
- If `--dry-run` is specified, shows files that would be deleted without removing them
- Automatically cleans up empty parent directories
- **NEVER touches AGENTS.md** (use `purge` command to remove AGENTS.md)
- Does NOT affect global templates in local data directory
- If agent/language not found in BoM/templates, shows a list of available options
- `--all` is mutually exclusive with `--agent` and `--lang`
- Must specify at least one of `--agent`, `--lang`, `--skill`, or `--all`

### `doctor` - Check Workspace Health

Check the workspace for stale or broken managed files and optionally fix them.

**Usage:**

```bash
slopctl doctor [--fix] [--dry-run] [--verbose]
```

**Options:**

- `--fix` - Automatically repair detected issues where safe to do so
- `--dry-run` - Preview what would be fixed without applying changes
- `--verbose` - Print every checked file and its result during the scan

**Issue categories detected:**

| Kind | Condition | Symbol |
|------|-----------|--------|
| **Missing** | File is tracked but no longer exists on disk (stale tracker entry) | `✗` |
| **Unmerged** | AGENTS.md (main file) exists but still contains the template marker | `✗` |
| **Modified** | File exists but SHA changed since installation (informational) | `!` |

**What `--fix` repairs:**

- **Missing** — Prunes the stale FileTracker entry. No filesystem change; run `slopctl init` to reinstall.
- **Unmerged** — Strips the template marker from the file in-place, marking it as customized so future installs won't silently overwrite it. Run `slopctl init` afterward for a full re-merge with language sections.
- **Modified** — No automatic fix; shown as informational. Use `slopctl init --force` to overwrite if intended.

**Examples:**

```bash
# Check workspace for issues
slopctl doctor

# Show every file checked alongside its result
slopctl doctor --verbose

# Automatically fix what can be fixed
slopctl doctor --fix

# Preview fixes without applying them
slopctl doctor --fix --dry-run
```

**Example output (with --verbose and issues present):**

```
Checking workspace files:

  ✓ OK:       .cursor/commands/init-session.md
  ✓ OK:       CLAUDE.md
  ✗ Missing:  .cursorrules
  ✗ Unmerged: AGENTS.md
  ! Modified: .rustfmt.toml

Issues found:

  ✗ Missing:  .cursorrules (tracked but deleted)
  ✗ Unmerged: AGENTS.md (template marker still present)
  ! Modified: .rustfmt.toml (changed since install)

  ✗ 1 stale tracker entry
  ✗ 1 file with unmerged template marker
  ! 1 modified file (no automatic fix available)

→ Run 'slopctl doctor --fix' to automatically fix issues
```

### `status` - Show Workspace Status

Display the current status of slopctl in the project.

**Usage:**

```bash
slopctl status              # Workspace status
slopctl status -v           # Workspace status with managed files
```

To browse the available template catalog, use `slopctl templates --list`.

**Default output includes:**

- **Global Templates:** Whether templates are installed and their location
  - Template version
  - Available agents (from templates.yml)
  - Available languages (from templates.yml)
- **Project Status:**
  - AGENTS.md existence and customization status
  - Which agents are currently installed
  - Installed language (from FileTracker metadata)
  - Installed skills (grouped by name)
- **Managed Files:** List of all slopctl managed files in current directory (with `--verbose`)

**Example output:**

```
slopctl status

Global Templates:
  ✓ Installed at: /Users/.../slopctl/templates
  → Template version: 5
  → Available agents: claude, copilot, codex, cursor
  → Available languages: c, c++, rust, swift

Project Status:
  ✓ AGENTS.md: exists (customized)
  ✓ Installed agents: claude, cursor
  ✓ Installed language: rust
  ✓ Installed skills: 2
    • create-rule
    • create-skill
```

### `completions` - Generate Shell Completions

Generate shell completion scripts for various shells.

**Usage:**

```bash
slopctl completions <shell>
```

**Arguments:**

- `<shell>` - Shell to generate completions for: `bash`, `zsh`, `fish`, `powershell`

**Examples:**

```bash
# Generate zsh completions
slopctl completions zsh > ~/.zsh/completions/_slopctl

# Generate bash completions
slopctl completions bash > ~/.bash_completion.d/slopctl

# Generate fish completions
slopctl completions fish > ~/.config/fish/completions/slopctl.fish

# Generate PowerShell completions
slopctl completions powershell > slopctl.ps1
```

### `merge` - AI-Assisted Merge

Merge customized workspace files with updated templates using AI assistance. By default, merged content replaces the original file directly. Use `--preview` to write `.merged` sidecar files for manual review instead.

The provider can be specified via `--provider`, the `merge.provider` config key, or auto-detected from environment variables (`ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, `MISTRAL_API_KEY` — checked in that order). The model can be set via `--model`, the `merge.model` config key, or the provider's default.

**Usage:**

```bash
slopctl merge                                    # Auto-detect provider from env
slopctl merge --provider anthropic
slopctl merge --provider openai --model gpt-4o
slopctl merge --provider anthropic --preview
slopctl merge --provider anthropic --dry-run
slopctl merge --provider anthropic --verbose
slopctl merge --list-models
```

**Options:**

- `--provider` / `-p` - LLM provider (openai, anthropic, ollama, mistral). Falls back to config `merge.provider`, then auto-detection from environment API keys.
- `--model` / `-m` - Model to use for merging. Falls back to config `merge.model`, then provider default.
- `--preview` - Write `.merged` sidecar files instead of replacing originals
- `--dry-run` / `-n` - Show merge candidates without calling the LLM
- `--list-models` / `-L` - List available models from the selected provider
- `--verbose` / `-v` - Show token usage summary after merging (input/output tokens, stop reason). Warns if any file was truncated due to max token limits.

**Provider priority:** CLI `--provider` > config `merge.provider` > environment auto-detect > error

**Merge candidates:** Files that are both user-modified (SHA changed since install) AND have an updated template source. Includes tracked files, skill files, and untracked files that exist on disk with a matching template source.

### `config` - Manage Configuration

Manage persistent configuration settings using Git-style dotted keys.

**Usage:**

```bash
slopctl config --add <key> <value>  # Set a configuration value
slopctl config <key>                # Get a configuration value
slopctl config --list               # List all configuration values
slopctl config --remove <key>       # Remove a configuration value
```

**Options:**

- `<key>` - Configuration key to get (e.g., source.url)
- `--add <key> <value>` (`-a`) - Set a configuration value
- `--list` (`-l`) - List all configuration values
- `--remove <key>` (`-r`) - Remove a configuration key

**Examples:**

```bash
# Set custom template source
slopctl config --add source.url https://github.com/myteam/templates/tree/main/templates

# Get current source URL
slopctl config source.url

# List all configuration
slopctl config --list

# Remove custom source (revert to default)
slopctl config --remove source.url

# Set fallback source for resilience
slopctl config --add source.fallback https://github.com/heikopanjas/slopctl/tree/develop/templates

# Set default LLM provider for merge
slopctl config --add merge.provider anthropic

# Set default model for merge
slopctl config --add merge.model claude-sonnet-4-20250514
```

**Valid Configuration Keys:**

- `source.url` - Default template download URL (used by `templates --update` and `init` when `--from` not specified)
- `source.fallback` - Fallback URL used when primary source fails or is unreachable
- `merge.provider` - Default LLM provider for the `merge` command (openai, anthropic, ollama, mistral)
- `merge.model` - Default model for the `merge` command (e.g., `gpt-4o`, `claude-sonnet-4-20250514`)

**Configuration File Location:**

- Linux: `$XDG_CONFIG_HOME/slopctl/config.yml` or `~/.config/slopctl/config.yml`
- macOS: `~/.config/slopctl/config.yml`

**Behavior:**

- Configuration persists between sessions
- `templates --update` command uses `source.url` if set and `--from` not specified
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

**Universal Support**: Single AGENTS.md works with all agents following the [agents.md](https://agents.md) standard:

- Claude Code (Anthropic)
- Cursor (AI code editor)
- GitHub Copilot (GitHub)
- Codex (OpenAI)

One AGENTS.md for all agents. Agent-specific files (e.g. CLAUDE.md) reference AGENTS.md when needed. Agent-specific [skills](https://agentskills.io) (SKILL.md) can also be defined per agent.

## Supported Languages

Currently configured in `templates.yml`:

- **C** - C programming language (skills: `c-coding-conventions`, `cmake-build-commands`; config files: `.clang-format`, `.editorconfig`)
- **C++** - C++ programming language (skills: `c++-coding-conventions`, `cmake-build-commands`; config files: `.clang-format`, `.editorconfig`)
- **Rust** - Rust programming language (skills: `rust-coding-conventions`, `rust-build-commands`; config files: `.rustfmt.toml`, `.editorconfig`, `.gitignore`)
- **Swift** - Swift programming language (skills: `swift-coding-conventions`, `swift-build-commands`, `swift-concurrency-pro`; config files: `.swift-format`, `.editorconfig`, `.gitignore`)
- **SwiftUI** - SwiftUI framework (includes all Swift skills and config files plus `swiftui-pro` skill)

Coding conventions and build commands are installed as [Agent Skills](https://agentskills.io) rather than fragments merged into AGENTS.md. A slim hint fragment is merged into AGENTS.md to inform agents that skills are available. Additional language templates can be added to `templates.yml` configuration.

## How It Works

### Template Storage

Templates are stored in platform-specific directories:

- **macOS**: `~/Library/Application Support/slopctl/templates/`
- **Linux**: `~/.local/share/slopctl/templates/`
- **Windows**: `%LOCALAPPDATA%\slopctl\templates\`

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

slopctl supports [Agent Skills](https://agentskills.io) – an open format for extending AI agent capabilities with specialized knowledge and workflows.

A skill is a directory containing a `SKILL.md` file with YAML frontmatter (name, description) and Markdown instructions. Skills can optionally include `scripts/`, `references/`, and `assets/` subdirectories.

**Skills can be defined in five ways:**

1. **Per-agent in templates.yml** – Using `name`/`source` under `agents.<name>.skills` (installed to agent-specific skill directory)
2. **Per-language in templates.yml** – Using `name`/`source` under `languages.<name>.skills` (installed to cross-client `.agents/skills/`)
3. **Per-shared group in templates.yml** – Using `name`/`source` under `shared.<name>.skills` (propagated to including languages via `includes`)
4. **Top-level in templates.yml** – Agent-agnostic skills under the `skills` section (installed to agent-specific or cross-client directory)
5. **Ad-hoc via CLI** – Using `--skill user/repo`, `--skill https://github.com/...`, or `--skill ./local/path` on the `init` command

**How skills work:**

- All skill definitions use the same format: `name` + `source` (GitHub URL or local path)
- **Agent skills** (`agents.<name>.skills`): installed to the agent-specific directory (e.g. `.cursor/skills/`) when that agent is selected
- **Language skills** (`languages.<name>.skills`): installed to the cross-client `.agents/skills/` directory when that language is selected
- **Shared group skills** (`shared.<name>.skills`): propagated to any language that includes the shared group via `includes`
- **Language include skills**: skills from an included *language* are also propagated depth-first (e.g. `swiftui` including `swift` inherits `swift`'s skills); cycle detection prevents infinite recursion. See the [`includes` section](#includes-composable-languages-and-shared-groups) for full details.
- **Top-level skills** (`skills`): installed to agent-specific directory if `--agent` is specified, otherwise cross-client
- **CLI `--skill`**: supports `user/repo` shorthand (expanded to full GitHub URL), full `https://github.com/...` URLs, and local filesystem paths (`./path`, `~/path`, `/absolute/path`)
- **Standalone mode**: `--skill` can be used without `--lang` or `--agent` — skills are installed to the cross-client `$workspace/.agents/skills/` directory without requiring global templates, AGENTS.md, or an agent. This follows the [agentskills.io](https://agentskills.io) cross-client interoperability spec.
- GitHub skills are downloaded on-the-fly via the GitHub Contents API (no local cache)
- Skills are tracked with the `"skill"` category in the file tracker for modification detection
- The `templates --list` command shows available skills (including agent and language skill counts); `status` shows installed skills
- Removing an agent (`slopctl remove --agent <name>`) also removes its skills

**Example per-agent skills in templates.yml:**

```yaml
agents:
  cursor:
    skills:
      - name: create-rule
        source: 'https://github.com/user/cursor-skills/tree/main/create-rule'
```

**Example per-language skills in templates.yml:**

```yaml
languages:
  rust:
    files:
      - source: rust-coding-conventions.md
        target: '$instructions'
    skills:
      - name: rust-analyzer
        source: 'https://github.com/user/rust-skills/tree/main/rust-analyzer'
```

**Example shared group skills in templates.yml (propagated to including languages):**

```yaml
shared:
  cmake:
    files:
      - source: cmake-build-commands.md
        target: '$instructions'
    skills:
      - name: cmake-skill
        source: 'https://github.com/user/cmake-skills/tree/main/cmake-skill'

languages:
  c:
    includes: [cmake]         # inherits cmake files AND skills
    files:
      - source: c-coding-conventions.md
        target: '$instructions'
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
# Standalone (cross-client .agents/skills/ directory)
slopctl init --skill user/my-skill

# With agent (agent-specific directory, e.g. .cursor/skills/)
slopctl init --agent cursor --skill user/my-skill

# From full GitHub URL
slopctl init --skill https://github.com/user/skills/tree/main/create-rule

# From local path
slopctl init --skill ./my-local-skill
```

### Agent Directories

Agents can declare workspace directories that should be created during `init`. This is useful for directories that the agent expects to exist but that are not tracked by version control — for example, Cursor's `.cursor/plans` directory for storing agent-generated plans.

**Example in templates.yml:**

```yaml
agents:
  cursor:
    instructions:
      - source: cursor/cursorrules
        target: '$workspace/.cursorrules'
    directories:
      - target: '$workspace/.cursor/plans'
```

When a user runs `slopctl init --agent cursor`, the `.cursor/plans` directory is created in the workspace alongside the usual instruction and prompt files. If the directory already exists, the step is silently skipped. Directories are also shown in `--dry-run` output.

Each entry in `directories` has a single field:

- `target` — Destination path using the standard placeholders (`$workspace`, `$userprofile`)

### Template Configuration (templates.yml)

The `templates.yml` file defines the template structure with a version field and multiple sections:

**Version Field:**

- `version: 5` (default) - Agent, language, and shared group skill associations, composable languages
- Missing version defaults to 5
- slopctl automatically detects the version from `templates.yml` and uses the appropriate template engine
- The `status` command shows the installed template version

**Main Sections:**

1. **main**: Main AGENTS.md instruction file (primary source of truth)
2. **agents**: Agent-specific files with `instructions`, `prompts`, `skills` (name + source), and `directories` (workspace paths to create during init)
3. **shared**: Reusable file groups with `files` and optional `skills` (skills propagate to including languages via `includes`)
4. **languages**: Language-specific coding standards fragments (merged into AGENTS.md), with optional `includes` and `skills`
5. **integration**: Tool/workflow integration fragments (merged into AGENTS.md, e.g., git workflows)
6. **principles**: Core principles and general guidelines fragments (merged into AGENTS.md)
7. **mission**: Mission statement, purpose, and project overview fragments (merged into AGENTS.md)
8. **skills**: Agent-agnostic skill definitions with `name` and `source` (installed to active agent's skill directory)

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

**Example V5 structure (agents.md standard):**

```yaml
version: 5

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
            - name: create-rule
              source: 'https://github.com/user/cursor-skills/tree/main/create-rule'
        directories:
            - target: '$workspace/.cursor/plans'

shared:
    cmake:
        files:
            - source: cmake-build-commands.md
              target: '$instructions'
        skills:
            - name: cmake-skill
              source: 'https://github.com/user/cmake-skills/tree/main/cmake-skill'

languages:
    c:
        includes: [cmake]
        files:
            - source: c-coding-conventions.md
              target: '$instructions'
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
        skills:
            - name: rust-analyzer
              source: 'https://github.com/user/rust-skills/tree/main/rust-analyzer'

principles:
    - source: core-principles.md
      target: '$instructions'

mission:
    - source: mission-statement.md
      target: '$instructions'
```

### `includes`: Composable Languages and Shared Groups

The `includes` key on a language entry lets you pull in files and skills from other definitions so you don't repeat yourself. There are two kinds of targets you can include, and they behave slightly differently.

#### Kind 1 — Shared groups (`shared` section)

A shared group is a named bucket of files and skills that has no meaning on its own; it only exists to be reused. Think of it like a mixin.

```yaml
shared:
  cmake:
    files:
      - source: cmake-build-commands.md
        target: '$instructions'
    skills:
      - name: cmake-skill
        source: 'https://github.com/user/cmake-skills/tree/main/cmake-skill'

languages:
  c:
    includes: [cmake]        # pulls in cmake files AND cmake skills
    files:
      - source: c-coding-conventions.md
        target: '$instructions'

  c++:
    includes: [cmake]        # same cmake files and skills, no duplication
    files:
      - source: cpp-coding-conventions.md
        target: '$instructions'
```

When a user runs `slopctl init --lang c++`, they get:

- `cmake-build-commands.md` merged into AGENTS.md (from the cmake shared group)
- `cpp-coding-conventions.md` merged into AGENTS.md (own file)
- `cmake-skill` installed (propagated from the shared group)

#### Kind 2 — Other languages (`languages` section)

A language can also include another language. This is useful when one language is a superset of another — for example, SwiftUI is Swift plus extra conventions.

```yaml
languages:
  swift:
    files:
      - source: swift-coding-conventions.md
        target: '$instructions'
      - source: swift-format-instructions.json
        target: '$workspace/.swiftformat'
    skills:
      - name: swift-analyzer
        source: 'https://github.com/user/swift-skills/tree/main/swift-analyzer'

  swiftui:
    includes: [swift]        # inherits swift's files AND skills
    files:
      - source: swiftui-coding-conventions.md
        target: '$instructions'
    skills:
      - name: swiftui-components
        source: 'https://github.com/user/swift-skills/tree/main/swiftui-components'
```

When a user runs `slopctl init --lang swiftui`, they get everything from `swift` first, then `swiftui`'s own additions on top:

| What gets installed | Source |
|---|---|
| `swift-coding-conventions.md` → AGENTS.md | inherited from `swift` |
| `.swiftformat` | inherited from `swift` |
| `swiftui-coding-conventions.md` → AGENTS.md | own |
| `swift-analyzer` skill | inherited from `swift` |
| `swiftui-components` skill | own |

#### Resolution order

Included items always come **before** the language's own items. For multiple includes, they are resolved left to right, depth-first. Example:

```yaml
languages:
  base:
    files: [base.md → $instructions]

  mid:
    includes: [base]
    files: [mid.md → $instructions]

  top:
    includes: [mid]
    files: [top.md → $instructions]
```

Installing `top` produces: `base.md`, then `mid.md`, then `top.md` — in that order.

Multiple includes in one language follow the same left-to-right, depth-first rule:

```yaml
languages:
  full:
    includes: [base, mid]   # base resolved first (depth-first), then mid, then own
    files: [full.md → $instructions]
```

#### Key rules

| Rule | Detail |
|---|---|
| **Shared groups propagate skills** | `includes: [my-shared]` → inherits both files and skills from the shared group |
| **Languages propagate skills** | `includes: [swift]` → inherits both files and skills from `swift` |
| **No duplicate disk targets** | Two entries targeting the same `$workspace/` path cause an error at init time; `$instructions` fragments are exempt |
| **Cycle detection** | Circular includes (e.g. `a` includes `b` includes `a`) are caught and reported as an error |
| **Mixing both kinds** | A language can include a mix of shared groups and other languages: `includes: [cmake, swift]` |

### Template Management

1. **First run**: `templates --update` downloads `templates.yml` and all specified files from GitHub
2. **Local storage**: Templates are cached in platform-specific directory
3. **Protection**: Template marker in AGENTS.md detects customization and prevents accidental overwrites
4. **Updates**: Detect AGENTS.md customization and warn before overwriting
5. **Placeholders**: `$workspace` and `$userprofile` resolve to appropriate paths

### Project Initialization

When you run `slopctl init --lang rust`:

1. Checks if global templates exist (downloads V5 by default if needed)
2. Loads `templates.yml` configuration and detects version
3. Uses TemplateEngine for agents.md standard
4. Downloads main AGENTS.md template
5. Merges fragments (mission, principles, skill hints, integration) into AGENTS.md at insertion points
6. Copies language config files (.rustfmt.toml, .editorconfig, .gitignore)
7. Installs language skills (e.g. rust-coding-conventions, rust-build-commands) to `.agents/skills/`
8. Single AGENTS.md works with all agents
9. Optional `--agent` adds agent-specific files (e.g. CLAUDE.md, .cursor/commands/init-session.md), agent skills, and creates agent directories (e.g. `.cursor/plans`)
10. You're ready to start coding with any agent

**Without `--lang`** (language-independent setup):

1. Same as above but skips language skill hints, config files, and language skills
2. AGENTS.md contains mission, principles, integration (e.g. git, versioning) only
3. Requires `--agent` to specify which agent prompts to set up

**With `--agent` only** (switch agent, preserve language):

1. Detects existing installation language from file tracker
2. Uses that language; if none, uses first available from templates
3. Adds/updates agent prompts only

The resulting AGENTS.md contains the complete merged content with all relevant sections for your project.

### Modification Detection

slopctl detects if you've customized AGENTS.md by checking for the template marker:

```bash
$ slopctl init --lang c++ --agent claude
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
slopctl templates --update --from /path/to/your/templates

# From a GitHub repository
slopctl templates --update --from https://github.com/yourname/your-templates/tree/main/templates

# Then initialize your project
slopctl init --lang c++ --agent claude
```

**Note:** Your custom template repository must include a `templates.yml` file that defines the template structure and file mappings.

### Modifying Global Templates

1. Navigate to platform-specific template directory:
   - macOS: `~/Library/Application Support/slopctl/templates/`
   - Linux: `~/.local/share/slopctl/templates/`
   - Windows: `%LOCALAPPDATA%\slopctl\templates\`
2. Edit the templates as needed
3. Run `slopctl init` to apply changes to your projects

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
- **Serialization:** serde v1.0, serde_yaml v0.9, serde_json v1.0
- **Error Handling:** anyhow v1.0
- **Hashing:** sha2 v0.10
- **Timestamps:** chrono v0.4
- **Directory Paths:** dirs v5.0
- **Temp Files:** tempfile v3.13
- **Man Pages:** clap_mangen v0.2 (build dependency)

## FAQ

**Where are templates stored?**

- Global templates (macOS): `~/Library/Application Support/slopctl/templates/`
- Global templates (Linux): `~/.local/share/slopctl/templates/`
- Global templates (Windows): `%LOCALAPPDATA%\slopctl\templates\`

**What happens if I modify AGENTS.md?**
slopctl detects customization via template marker removal and skips AGENTS.md when updating. Use `--force` to override.

**Can I use my own template repository?**
Yes! Use the `--from` option with the `templates --update` command to specify a local path or GitHub URL.

**Why AGENTS.md as single source of truth?**
Centralized updates prevent drift and make it easier to maintain consistency across sessions.

**Can I use this in commercial projects?**
Yes! MIT license allows commercial use. Attribution appreciated but not required.

**How do I update templates?**
Run `slopctl templates --update` to download the latest global templates, then `slopctl init` to apply to your project.

**How do I remove local templates?**
Run `slopctl purge` to remove all agent files and AGENTS.md, or `slopctl remove --all` to keep AGENTS.md.

**How do I remove language config files?**
Run `slopctl remove --lang <language>` (e.g. `slopctl remove --lang rust`). This removes disk files like `.rustfmt.toml` and `.editorconfig` but does NOT remove language fragments already merged into AGENTS.md.

**How do I fix stale or broken managed files?**
Run `slopctl doctor` to list issues, or `slopctl doctor --fix` to repair them automatically. Issues detected: missing tracked files (stale tracker entries), unmerged AGENTS.md templates, and modified files (informational). Use `--verbose` to see the result for every tracked file.

**How do I preview changes before applying?**
Use the `--dry-run` flag on any command: `slopctl init --lang rust --dry-run` or `slopctl init --agent cursor --dry-run`

**How do I customize the mission statement?**
Use the `--mission` option with `init`. For inline text: `--mission "Your mission here"`. For multi-line content from a file: `--mission @mission.md`. The custom mission replaces the default template placeholder in AGENTS.md.

**What template version should I use?**
V5 (default) is recommended. It follows the agents.md standard with agent/language skill associations, shared file groups, and composable languages. Run `slopctl status` to see the installed template version.

**What if I don't specify --lang?**
Omitting `--lang` gives you AGENTS.md with mission, principles, and integration (e.g. git) only—no language-specific coding conventions or config files (.rustfmt.toml, .editorconfig, etc.). Good for documentation repositories, multi-language projects, or when you prefer a minimal setup. Just use `--agent` alone: `slopctl init --agent cursor`.

**How do I switch agents without changing the language?**
Run `slopctl init --agent <new-agent>`. slopctl detects the existing language from the file tracker and uses it (e.g. switching from Cursor to Claude keeps your Rust setup).

**What are Agent Skills?**
[Agent Skills](https://agentskills.io) are an open format for giving agents specialized capabilities via SKILL.md files. Skills can be defined in `templates.yml` (per-agent, per-language, or top-level) or installed ad-hoc using `--skill user/repo`. Skills are downloaded on-the-fly and tracked like other template files.

**How do I install a skill?**
Use the `--skill` flag: `slopctl init --skill user/my-skill`. This installs to the cross-client `.agents/skills/` directory without needing templates or an agent. To install to an agent-specific directory, add `--agent`: `slopctl init --agent cursor --skill user/my-skill`. You can also use full GitHub URLs (`--skill https://github.com/user/repo/tree/main/path`) or local paths (`--skill ./my-skill`, `--skill ~/skills/my-skill`).

**Where are skills installed?**
It depends on how you invoke `--skill`:

- **Standalone** (`--skill` alone or with `--lang`): Cross-client directory `$workspace/.agents/skills/` per the [agentskills.io](https://agentskills.io) spec. All compliant agents scan this path.
- **With `--agent`** (e.g. `--agent cursor`): Agent-specific directory (e.g. `.cursor/skills/`).

## License

MIT License - See [LICENSE](LICENSE) for details.

## Building from Source

```bash
# Clone the repository
git clone https://github.com/heikopanjas/slopctl.git
cd slopctl

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

Last updated: April 10, 2026 (v13.3.0)
