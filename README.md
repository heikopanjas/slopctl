# vibe-cop

**A manager for coding agent instruction files** – A Rust CLI tool that provides a centralized system for managing, organizing, and maintaining initialization prompts and instruction files for AI coding assistants. Supports the [agents.md community standard](https://agents.md) where a single AGENTS.md file works across all agents (Claude Code, Cursor, GitHub Copilot, and Codex) with built-in governance guardrails and human-in-the-loop controls. Also supports [Agent Skills](https://agentskills.io) for extending agent capabilities with specialized knowledge and workflows.

![MIT License](https://img.shields.io/badge/-MIT%20License-000000?style=flat-square&logo=opensource&logoColor=white)
![CLI](https://img.shields.io/badge/-CLI-000000?style=flat-square&logo=zsh&logoColor=white)
![Rust](https://img.shields.io/badge/-Rust-000000?style=flat-square&logo=rust&logoColor=white)
![Claude](https://img.shields.io/badge/-Claude-000000?style=flat-square&logo=anthropic&logoColor=white)
![GitHub Copilot](https://img.shields.io/badge/-GitHub%20Copilot-000000?style=flat-square&logo=github&logoColor=white)
![Codex](https://img.shields.io/badge/-Codex-000000?style=flat-square&logo=openai&logoColor=white)
![Cursor](https://img.shields.io/badge/-Cursor-000000?style=flat-square&logo=visualstudiocode&logoColor=white)

## Overview

vibe-cop is a command-line tool that helps you:

- **Manage templates globally** – Store templates in platform-specific directories (e.g., `~/Library/Application Support/vibe-cop/templates` on macOS)
- **Configure via YAML** – Define template structure and file mappings in `templates.yml`
- **Initialize projects quickly** – Set up agent instructions with a single command
- **agents.md standard** – Follow the [agents.md](https://agents.md) community standard (single AGENTS.md for all agents)
- **Agent Skills support** – Define and install [Agent Skills](https://agentskills.io) (SKILL.md) from templates or GitHub repos
- **Independent skill loading** – Install skills standalone with `--skill user/repo` (no templates or agent required); uses cross-client `.agents/skills/` directory per agentskills.io spec
- **Keep templates synchronized** – Update global templates from remote sources
- **Workspace health checks** – Detect and fix stale or broken managed files with `doctor --fix`
- **Enforce governance** – Built-in guardrails for no auto-commits and human confirmation
- **Support multiple agents** – Compatible with Claude Code, Cursor, GitHub Copilot, and Codex
- **Flexible file placement** – Use placeholders (`$workspace`, `$userprofile`) for custom locations
- **Template versioning** – V4 templates with shared file groups (with skill propagation), composable languages, and agent/language skill associations

## Repository Structure

```text
vibe-cop/
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
│   │   ├── purge.rs            # Purge all vibe-cop files
│   │   ├── remove.rs           # Remove agent/language/skill files
│   │   ├── doctor.rs           # Workspace health checks and fixes
│   │   ├── status.rs           # Show project status
│   │   └── list.rs             # List available agents/languages
│   └── utils.rs                # Utility functions
├── LICENSE                     # MIT license
├── README.md                   # You are here
├── AGENTS.md                   # Primary project instructions
├── templates/                  # Template files organized by version
│   └── v4/                     # Version 4 templates (agents.md standard, default)
│       ├── templates.yml       # V4 template configuration (version: 4)
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

## Template Format (V4)

vibe-cop uses the V4 template format following the [agents.md](https://agents.md) standard.

**Philosophy**: One AGENTS.md file that works across all agents.

- Follows the [agents.md](https://agents.md) community standard
- Single AGENTS.md file compatible with Claude Code, Cursor, GitHub Copilot, and Codex
- Agent-specific instruction files (e.g. CLAUDE.md) reference AGENTS.md when needed
- [Agent Skills](https://agentskills.io) support: define skills per agent, per language, or as top-level entries
- Shared file groups (`shared` section) and composable languages (`includes`) for reuse across languages
- Skills associated with agents, languages, or shared groups — shared group skills propagate via `includes`
- Simpler initialization: `vibe-cop install --lang rust` or omit `--lang` for language-independent setup
- Optional `--lang` and `--agent` (specify at least one; `--agent` alone preserves existing language when switching)
- GitHub URL support: `source` fields in templates.yml accept full GitHub URLs for remote files
- Independent skill loading: `--skill` works standalone (no templates, no agent required) or combined with `--lang`/`--agent`
- Cross-client skill directory: standalone `--skill` installs to `.agents/skills/` per agentskills.io spec
- URL: `https://github.com/heikopanjas/vibe-cop/tree/develop/templates/v4`

**Usage:**

```bash
vibe-cop update                    # Downloads V4 templates
vibe-cop install --lang rust          # With language conventions
vibe-cop install --agent cursor       # Agent only (AGENTS.md + agent prompts, no language files)
vibe-cop install --skill user/repo    # Install a skill (standalone, to .agents/skills/)
```

### Migration from v8 to v9

**Upgrading from v8.x to v9.0.0:** Use V4 templates (default source: `templates/v4`).

```bash
vibe-cop update                    # Gets V4 templates
vibe-cop install --lang rust       # Initialize with V4
```

### Migration from v7 to v8

**Upgrading from v7.x to v8.0.0:**

v8.0.0 is a major version bump with one breaking change: **the `init` command has been renamed to `install`**. Update any scripts or aliases accordingly.

```bash
# Before (v7):  vibe-cop init --lang rust --agent cursor
# After  (v8):  vibe-cop install --lang rust --agent cursor
```

**New features in v8.0.0:**

- `--skill` flag: Install skills from GitHub repos or local paths (`--skill user/repo`, `--skill https://github.com/...`, `--skill ./path`)
- GitHub URL support in templates.yml `source` fields (full URLs only, no shorthand)
- Top-level `skills` section in templates.yml for agent-agnostic skills
- `agent_defaults.rs`: built-in registry of agent paths (instructions, prompts, skills)
- `github.rs`: GitHub Contents API integration for on-the-fly downloads
- Automatic agent detection when `--agent` is not specified (for template-based install; `--skill` alone needs no agent)
- `UpdateOptions` struct now carries all parameters through the call chain

## Installation

### From Source

```bash
git clone https://github.com/heikopanjas/vibe-cop.git
cd vibe-cop
cargo build --release
sudo cp target/release/vibe-cop /usr/local/bin/
```

### Using Cargo

```bash
cargo install --path .
```

## Quick Start

```bash
# 1. Download global templates
vibe-cop update

# 2. Initialize your project (choose one style)
cd your-project
vibe-cop install --lang rust         # With Rust conventions and config files
vibe-cop install --agent cursor     # Agent prompts + skills (AGENTS.md without language files)
vibe-cop install --skill user/repo  # Install a skill only (no templates needed)
```

With `--lang rust` this will:

1. Copy main AGENTS.md template to your project
2. Merge language-specific fragments (Rust conventions, build commands) into AGENTS.md
3. Copy language config files (.rustfmt.toml, .editorconfig, .gitignore, .gitattributes)
4. **Single AGENTS.md works with all agents** (Claude Code, Cursor, GitHub Copilot, and Codex)

Without `--lang`, you get AGENTS.md with mission, principles, and integration (e.g. git) only—no language-specific files.

### Initialize from a custom template source

```bash
# From a local path
vibe-cop update --from /path/to/templates

# From a GitHub URL
vibe-cop update --from https://github.com/user/repo/tree/branch/templates

# Then initialize the project
vibe-cop install --lang c++ --agent claude
```

**Note:** The custom source must include a `templates.yml` file that defines the template structure.

## Complete Walkthrough: Rust Project

This walkthrough demonstrates setting up a new Rust project using vibe-cop.

### Step 1: Create Your Project Directory

```bash
mkdir my-rust-project
cd my-rust-project
```

### Step 2: Initialize with vibe-cop

```bash
vibe-cop install --lang rust
```

**What happens:**

1. **Downloads templates** (first run only):
   - Fetches `templates.yml` from GitHub (V4 format)
   - Downloads all template files to platform-specific directory (e.g., `~/Library/Application Support/vibe-cop/templates/` on macOS)

2. **Processes configuration**:
   - Detects template version 4 (agents.md standard)
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
vibe-cop update

# Then reinitialize the project (will skip customized AGENTS.md unless --force)
vibe-cop install --lang rust
```

vibe-cop will:

- Check if AGENTS.md has been customized (template marker removed)
- Skip customized AGENTS.md unless `--force` is used

### Common Scenarios

**Scenario: Modified AGENTS.md locally**

```bash
$ vibe-cop install --lang rust
! Local AGENTS.md has been customized and will be skipped
→ Other files will still be updated
→ Use --force to overwrite AGENTS.md
```

**Solution:** Review changes, commit them, then use `--force`:

```bash
git diff AGENTS.md              # Review changes
git add AGENTS.md
git commit -m "docs: customize project instructions"
vibe-cop install --lang rust --force
```

**Scenario: Clean up project templates**

```bash
# Remove all vibe-cop files including AGENTS.md
vibe-cop purge

# Removes AGENTS.md, agent files (e.g. CLAUDE.md), and language config files
# Preserves customized AGENTS.md unless --force is used
```

**Scenario: Remove only agent-specific files**

```bash
# Remove all agent files but keep AGENTS.md
vibe-cop remove --all

# Remove only one agent's files
vibe-cop remove --agent claude

# Removes CLAUDE.md, .cursor/commands/, .github/prompts/, etc.
```

**Scenario: Remove language config files (switch languages)**

```bash
# Remove Rust config files (.rustfmt.toml, .editorconfig, .gitignore, etc.)
vibe-cop remove --lang rust

# Then install C++ config files
vibe-cop install --lang c++
```

**Scenario: Diagnose and fix workspace issues**

```bash
# Check for broken/stale managed files
vibe-cop doctor --verbose

# Fix what can be fixed automatically (prune stale entries, strip unmerged markers)
vibe-cop doctor --fix

# Re-merge language sections after fixing an unmerged AGENTS.md
vibe-cop install --lang rust
```

**Scenario: Switch from Cursor to Claude (keep Rust setup)**

```bash
# You have Rust + Cursor; want to add Claude prompts
vibe-cop install --agent claude
# Uses existing Rust language; adds Claude prompts only
```

**Scenario: Language-independent project (e.g. docs-only repo)**

```bash
vibe-cop install --agent cursor
# AGENTS.md with mission, principles, integration + agent prompts—no .rustfmt.toml, no coding-conventions
```

**Scenario: Use custom templates**

```bash
# Your team maintains custom templates
vibe-cop update --from https://github.com/yourteam/templates/tree/main/templates

# Then initialize
vibe-cop install --lang rust
```

### Tips for Success

1. **Initialize early**: Run `vibe-cop install` at project start before adding code
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
vibe-cop update [--from <PATH or URL>] [--dry-run]
```

**Options:**

- `--from <string>` - Optional path or URL to download/copy templates from
- `--dry-run` - Preview what would be downloaded without making changes

**Examples:**

```bash
# Update global templates from default repository
vibe-cop update

# Update from custom URL
vibe-cop update --from https://github.com/user/repo/tree/branch/templates

# Update from local path
vibe-cop update --from /path/to/templates

# Preview what would be downloaded
vibe-cop update --dry-run
```

**Behavior:**

- Downloads templates from specified source or default GitHub repository
- If `--from` is not specified, downloads from:
  - **Default**: `https://github.com/heikopanjas/vibe-cop/tree/develop/templates/v4` (agents.md standard)
- Downloads `templates.yml` configuration file and all template files
- Stores templates in local data directory:
  - Linux: `$HOME/.local/share/vibe-cop/templates`
  - macOS: `$HOME/Library/Application Support/vibe-cop/templates`
- If `--dry-run` is specified, shows the source URL and target directory without downloading
- Overwrites existing global templates with new versions
- Does NOT modify any files in the current project directory

**Note:** Run `update` first to download templates before using `install` to set up a project.

### `install` - Install Agent Instructions and Skills

Install instruction files and skills for AI coding agents in your project.

**Usage:**

```bash
# Specify at least one of --lang, --agent, or --skill

# With language conventions
vibe-cop install --lang <language> [--agent <agent>] [--skill <url>]... [--mission <text|@file>] [--force] [--dry-run]

# Agent only (preserves existing language, or language-independent if fresh)
vibe-cop install --agent <agent> [--skill <url>]... [--mission <text|@file>] [--force] [--dry-run]

# Install skills only (standalone, no templates or agent required)
vibe-cop install --skill <url> [--skill <url>]... [--force] [--dry-run]
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
vibe-cop install --lang rust

# Initialize C++ project
vibe-cop install --lang c++

# Agent only (AGENTS.md + agent prompts, no language files)
vibe-cop install --agent cursor

# Switch from Cursor to Claude (keeps existing language e.g. Rust)
vibe-cop install --agent claude

# Initialize with custom mission statement (inline)
vibe-cop install --lang rust --mission "A CLI tool for managing AI agent instructions"

# Initialize with mission statement from file (multi-line support)
vibe-cop install --lang rust --mission @mission.md

# Force overwrite existing local files
vibe-cop install --lang swift --force

# Install a skill standalone (to .agents/skills/ cross-client directory)
vibe-cop install --skill user/my-skill

# Install multiple skills standalone
vibe-cop install --skill user/skill-a --skill user/skill-b

# Install a skill with a specific agent (to agent-specific directory, e.g. .cursor/skills/)
vibe-cop install --agent cursor --skill user/my-skill

# Install a skill from a full GitHub URL
vibe-cop install --skill https://github.com/user/skills/tree/main/create-rule

# Install a skill from a local path
vibe-cop install --skill ./path/to/skill
vibe-cop install --skill ~/skills/my-skill

# Preview what would be created/modified
vibe-cop install --lang rust --dry-run
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
- **With `--agent` only** (no `--lang`): Creates AGENTS.md with mission, principles, integration (no language files); preserves existing language if previously installed; installs agent-associated skills from templates.yml
- **With `--lang`**: Creates single AGENTS.md plus language config files; installs language-associated skills (own + inherited from shared groups) from templates.yml to cross-client directory; optional `--agent` adds agent prompts and agent skills
- Checks for local modifications to AGENTS.md (detects if template marker has been removed)
- If local AGENTS.md has been customized and `--force` is not specified, skips AGENTS.md
- If `--force` is specified, overwrites local files regardless of modifications
- If `--dry-run` is specified, shows what would be created/modified without making changes
- Files are placed according to `templates.yml` configuration with placeholder resolution:
  - `$workspace` resolves to current directory
  - `$userprofile` resolves to user's home directory
- Merges language-specific and integration fragments into AGENTS.md

### `purge` - Purge All Vibe-Cop Files

Purge all vibe-cop files from the current project directory.

**Usage:**

```bash
vibe-cop purge [--force] [--dry-run]
```

**Options:**

- `--force` - Force purge without confirmation and delete customized AGENTS.md
- `--dry-run` - Preview what would be deleted without making changes

**Examples:**

```bash
# Purge all vibe-cop files with confirmation prompt
vibe-cop purge

# Force purge without confirmation
vibe-cop purge --force

# Preview what would be deleted
vibe-cop purge --dry-run
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
vibe-cop remove --agent <agent> [--force] [--dry-run]

# Remove language disk files (e.g. .rustfmt.toml, .editorconfig)
vibe-cop remove --lang <lang> [--force] [--dry-run]

# Remove a named skill
vibe-cop remove --skill <name> [--force] [--dry-run]

# Remove all agent-specific files and skills (keeps AGENTS.md)
vibe-cop remove --all [--force] [--dry-run]
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
vibe-cop remove --agent claude

# Remove Rust language files (.rustfmt.toml, .editorconfig, etc.)
vibe-cop remove --lang rust

# Remove a named skill
vibe-cop remove --skill create-rule

# Remove language files and agent files together
vibe-cop remove --lang rust --agent cursor

# Remove all agent-specific files (keeps AGENTS.md)
vibe-cop remove --all

# Remove all agents with force
vibe-cop remove --all --force

# Preview what would be deleted
vibe-cop remove --lang rust --dry-run
vibe-cop remove --all --dry-run
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
vibe-cop doctor [--fix] [--dry-run] [--verbose]
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

- **Missing** — Prunes the stale FileTracker entry. No filesystem change; run `vibe-cop install` to reinstall.
- **Unmerged** — Strips the template marker from the file in-place, marking it as customized so future installs won't silently overwrite it. Run `vibe-cop install` afterward for a full re-merge with language sections.
- **Modified** — No automatic fix; shown as informational. Use `vibe-cop install --force` to overwrite if intended.

**Examples:**

```bash
# Check workspace for issues
vibe-cop doctor

# Show every file checked alongside its result
vibe-cop doctor --verbose

# Automatically fix what can be fixed
vibe-cop doctor --fix

# Preview fixes without applying them
vibe-cop doctor --fix --dry-run
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

→ Run 'vibe-cop doctor --fix' to automatically fix issues
```

### `status` - Show Project Status

Display the current status of vibe-cop in the project.

**Usage:**

```bash
vibe-cop status
```

**Output includes:**

- **Global Templates:** Whether templates are installed and their location
  - Template version
  - Available agents (from templates.yml)
  - Available languages (from templates.yml)
- **Project Status:**
  - AGENTS.md existence and customization status
  - Which agents are currently installed
  - Installed language (from FileTracker metadata)
  - Installed skills (grouped by name)
- **Managed Files:** List of all vibe-cop managed files in current directory

**Example output:**

```
vibe-cop status

Global Templates:
  ✓ Installed at: /Users/.../vibe-cop/templates
  → Template version: 4
  → Available agents: claude, copilot, codex, cursor
  → Available languages: c, c++, rust, swift

Project Status:
  ✓ AGENTS.md: exists (customized)
  ✓ Installed agents: claude, cursor
  ✓ Installed language: rust
  ✓ Installed skills: 2
    • create-rule
    • create-skill

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
vibe-cop list
```

**Output includes:**

- **Available Agents:** All agents defined in templates.yml with installation status and skill counts
- **Available Languages:** All languages defined in templates.yml

**Example output:**

```
vibe-cop list

Available Agents:
  ✓ claude (installed)
  ○ codex
  ✓ copilot (installed)
  ○ cursor (2 skill(s))

Available Languages:
  • c
  • c++
  • rust (1 skill(s))
  • swift

→ Use 'vibe-cop install --lang <lang> --agent <agent>' to install
```

### `completions` - Generate Shell Completions

Generate shell completion scripts for various shells.

**Usage:**

```bash
vibe-cop completions <shell>
```

**Arguments:**

- `<shell>` - Shell to generate completions for: `bash`, `zsh`, `fish`, `powershell`

**Examples:**

```bash
# Generate zsh completions
vibe-cop completions zsh > ~/.zsh/completions/_vibe-cop

# Generate bash completions
vibe-cop completions bash > ~/.bash_completion.d/vibe-cop

# Generate fish completions
vibe-cop completions fish > ~/.config/fish/completions/vibe-cop.fish

# Generate PowerShell completions
vibe-cop completions powershell > vibe-cop.ps1
```

### `config` - Manage Configuration

Manage persistent configuration settings using Git-style dotted keys.

**Usage:**

```bash
vibe-cop config <key> <value>    # Set a configuration value
vibe-cop config <key>            # Get a configuration value
vibe-cop config --list           # List all configuration values
vibe-cop config --unset <key>    # Remove a configuration value
```

**Options:**

- `<key>` - Configuration key (e.g., source.url)
- `<value>` - Value to set (omit to get current value)
- `--list` - List all configuration values
- `--unset <key>` - Remove a configuration key

**Examples:**

```bash
# Set custom template source
vibe-cop config source.url https://github.com/myteam/templates/tree/main/templates

# Get current source URL
vibe-cop config source.url

# List all configuration
vibe-cop config --list

# Remove custom source (revert to default)
vibe-cop config --unset source.url

# Set fallback source for resilience
vibe-cop config source.fallback https://github.com/heikopanjas/vibe-cop/tree/develop/templates
```

**Valid Configuration Keys:**

- `source.url` - Default template download URL (used by `update` and `install` when `--from` not specified)
- `source.fallback` - Fallback URL used when primary source fails or is unreachable

**Configuration File Location:**

- Linux: `$XDG_CONFIG_HOME/vibe-cop/config.yml` or `~/.config/vibe-cop/config.yml`
- macOS: `~/.config/vibe-cop/config.yml`

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

**Universal Support**: Single AGENTS.md works with all agents following the [agents.md](https://agents.md) standard:

- Claude Code (Anthropic)
- Cursor (AI code editor)
- GitHub Copilot (GitHub)
- Codex (OpenAI)

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

- **macOS**: `~/Library/Application Support/vibe-cop/templates/`
- **Linux**: `~/.local/share/vibe-cop/templates/`
- **Windows**: `%LOCALAPPDATA%\vibe-cop\templates\`

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

vibe-cop supports [Agent Skills](https://agentskills.io) – an open format for extending AI agent capabilities with specialized knowledge and workflows.

A skill is a directory containing a `SKILL.md` file with YAML frontmatter (name, description) and Markdown instructions. Skills can optionally include `scripts/`, `references/`, and `assets/` subdirectories.

**Skills can be defined in five ways:**

1. **Per-agent in templates.yml** – Using `name`/`source` under `agents.<name>.skills` (installed to agent-specific skill directory)
2. **Per-language in templates.yml** – Using `name`/`source` under `languages.<name>.skills` (installed to cross-client `.agents/skills/`)
3. **Per-shared group in templates.yml** – Using `name`/`source` under `shared.<name>.skills` (propagated to including languages via `includes`)
4. **Top-level in templates.yml** – Agent-agnostic skills under the `skills` section (installed to agent-specific or cross-client directory)
5. **Ad-hoc via CLI** – Using `--skill user/repo`, `--skill https://github.com/...`, or `--skill ./local/path` on the `install` command

**How skills work:**

- All skill definitions use the same format: `name` + `source` (GitHub URL or local path)
- **Agent skills** (`agents.<name>.skills`): installed to the agent-specific directory (e.g. `.cursor/skills/`) when that agent is selected
- **Language skills** (`languages.<name>.skills`): installed to the cross-client `.agents/skills/` directory when that language is selected
- **Shared group skills** (`shared.<name>.skills`): propagated to any language that includes the shared group via `includes`. Skills from included *languages* are NOT propagated — only shared groups propagate skills.
- **Top-level skills** (`skills`): installed to agent-specific directory if `--agent` is specified, otherwise cross-client
- **CLI `--skill`**: supports `user/repo` shorthand (expanded to full GitHub URL), full `https://github.com/...` URLs, and local filesystem paths (`./path`, `~/path`, `/absolute/path`)
- **Standalone mode**: `--skill` can be used without `--lang` or `--agent` — skills are installed to the cross-client `$workspace/.agents/skills/` directory without requiring global templates, AGENTS.md, or an agent. This follows the [agentskills.io](https://agentskills.io) cross-client interoperability spec.
- GitHub skills are downloaded on-the-fly via the GitHub Contents API (no local cache)
- Skills are tracked with the `"skill"` category in the file tracker for modification detection
- The `list` command shows available skills (including agent and language skill counts); the `status` command shows installed skills
- Removing an agent (`vibe-cop remove --agent <name>`) also removes its skills

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
vibe-cop install --skill user/my-skill

# With agent (agent-specific directory, e.g. .cursor/skills/)
vibe-cop install --agent cursor --skill user/my-skill

# From full GitHub URL
vibe-cop install --skill https://github.com/user/skills/tree/main/create-rule

# From local path
vibe-cop install --skill ./my-local-skill
```

### Template Configuration (templates.yml)

The `templates.yml` file defines the template structure with a version field and multiple sections:

**Version Field:**

- `version: 4` (default) - Agent, language, and shared group skill associations, composable languages
- Missing version defaults to 4
- vibe-cop automatically detects the version from `templates.yml` and uses the appropriate template engine
- The `status` command shows the installed template version

**Main Sections:**

1. **main**: Main AGENTS.md instruction file (primary source of truth)
2. **agents**: Agent-specific files with `instructions`, `prompts`, and `skills` (name + source)
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

**Example V4 structure (agents.md standard):**

```yaml
version: 4

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

### Template Management

1. **First run**: `update` downloads `templates.yml` and all specified files from GitHub
2. **Local storage**: Templates are cached in platform-specific directory
3. **Protection**: Template marker in AGENTS.md detects customization and prevents accidental overwrites
4. **Updates**: Detect AGENTS.md customization and warn before overwriting
5. **Placeholders**: `$workspace` and `$userprofile` resolve to appropriate paths

### Project Initialization

When you run `vibe-cop install --lang rust`:

1. Checks if global templates exist (downloads V4 by default if needed)
2. Loads `templates.yml` configuration and detects version
3. Uses TemplateEngine for agents.md standard
4. Downloads main AGENTS.md template
5. Merges fragments (mission, principles, language, integration) into AGENTS.md at insertion points
6. Copies language config files (.rustfmt.toml, .editorconfig, .gitignore, .gitattributes)
7. Single AGENTS.md works with all agents
8. Optional `--agent` adds agent-specific files (e.g. CLAUDE.md, .cursor/commands/init-session.md) and agent skills
9. Language-associated skills are installed to the cross-client `.agents/skills/` directory
10. You're ready to start coding with any agent

**Without `--lang`** (language-independent setup):

1. Same as above but skips language fragments and language config files
2. AGENTS.md contains mission, principles, integration (e.g. git, versioning) only
3. Requires `--agent` to specify which agent prompts to install

**With `--agent` only** (switch agent, preserve language):

1. Detects existing installation language from file tracker
2. Uses that language; if none, uses first available from templates
3. Adds/updates agent prompts only

The resulting AGENTS.md contains the complete merged content with all relevant sections for your project.

### Modification Detection

vibe-cop detects if you've customized AGENTS.md by checking for the template marker:

```bash
$ vibe-cop install --lang c++ --agent claude
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
vibe-cop update --from /path/to/your/templates

# From a GitHub repository
vibe-cop update --from https://github.com/yourname/your-templates/tree/main/templates

# Then initialize your project
vibe-cop install --lang c++ --agent claude
```

**Note:** Your custom template repository must include a `templates.yml` file that defines the template structure and file mappings.

### Modifying Global Templates

1. Navigate to platform-specific template directory:
   - macOS: `~/Library/Application Support/vibe-cop/templates/`
   - Linux: `~/.local/share/vibe-cop/templates/`
   - Windows: `%LOCALAPPDATA%\vibe-cop\templates\`
2. Edit the templates as needed
3. Run `vibe-cop install` to apply changes to your projects

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

- Global templates (macOS): `~/Library/Application Support/vibe-cop/templates/`
- Global templates (Linux): `~/.local/share/vibe-cop/templates/`
- Global templates (Windows): `%LOCALAPPDATA%\vibe-cop\templates\`

**What happens if I modify AGENTS.md?**
vibe-cop detects customization via template marker removal and skips AGENTS.md when updating. Use `--force` to override.

**Can I use my own template repository?**
Yes! Use the `--from` option with the `update` command to specify a local path or GitHub URL.

**Why AGENTS.md as single source of truth?**
Centralized updates prevent drift and make it easier to maintain consistency across sessions.

**Can I use this in commercial projects?**
Yes! MIT license allows commercial use. Attribution appreciated but not required.

**How do I update templates?**
Run `vibe-cop update` to download the latest global templates, then `vibe-cop install` to apply to your project.

**How do I remove local templates?**
Run `vibe-cop purge` to remove all agent files and AGENTS.md, or `vibe-cop remove --all` to keep AGENTS.md.

**How do I remove language config files?**
Run `vibe-cop remove --lang <language>` (e.g. `vibe-cop remove --lang rust`). This removes disk files like `.rustfmt.toml` and `.editorconfig` but does NOT remove language fragments already merged into AGENTS.md.

**How do I fix stale or broken managed files?**
Run `vibe-cop doctor` to list issues, or `vibe-cop doctor --fix` to repair them automatically. Issues detected: missing tracked files (stale tracker entries), unmerged AGENTS.md templates, and modified files (informational). Use `--verbose` to see the result for every tracked file.

**How do I preview changes before applying?**
Use the `--dry-run` flag on any command: `vibe-cop install --lang rust --dry-run` or `vibe-cop install --agent cursor --dry-run`

**How do I customize the mission statement?**
Use the `--mission` option with `install`. For inline text: `--mission "Your mission here"`. For multi-line content from a file: `--mission @mission.md`. The custom mission replaces the default template placeholder in AGENTS.md.

**What template version should I use?**
V4 (default) is recommended. It follows the agents.md standard with agent/language skill associations, shared file groups, and composable languages. Run `vibe-cop status` to see the installed template version.

**What if I don't specify --lang?**
Omitting `--lang` gives you AGENTS.md with mission, principles, and integration (e.g. git) only—no language-specific coding conventions or config files (.rustfmt.toml, .editorconfig, etc.). Good for documentation repositories, multi-language projects, or when you prefer a minimal setup. Just use `--agent` alone: `vibe-cop install --agent cursor`.

**How do I switch agents without changing the language?**
Run `vibe-cop install --agent <new-agent>`. vibe-cop detects the existing language from the file tracker and uses it (e.g. switching from Cursor to Claude keeps your Rust setup).

**What are Agent Skills?**
[Agent Skills](https://agentskills.io) are an open format for giving agents specialized capabilities via SKILL.md files. Skills can be defined in `templates.yml` (per-agent, per-language, or top-level) or installed ad-hoc using `--skill user/repo`. Skills are downloaded on-the-fly and tracked like other template files.

**How do I install a skill?**
Use the `--skill` flag: `vibe-cop install --skill user/my-skill`. This installs to the cross-client `.agents/skills/` directory without needing templates or an agent. To install to an agent-specific directory, add `--agent`: `vibe-cop install --agent cursor --skill user/my-skill`. You can also use full GitHub URLs (`--skill https://github.com/user/repo/tree/main/path`) or local paths (`--skill ./my-skill`, `--skill ~/skills/my-skill`).

**Where are skills installed?**
It depends on how you invoke `--skill`:

- **Standalone** (`--skill` alone or with `--lang`): Cross-client directory `$workspace/.agents/skills/` per the [agentskills.io](https://agentskills.io) spec. All compliant agents scan this path.
- **With `--agent`** (e.g. `--agent cursor`): Agent-specific directory (e.g. `.cursor/skills/`).

## License

MIT License - See [LICENSE](LICENSE) for details.

## Building from Source

```bash
# Clone the repository
git clone https://github.com/heikopanjas/vibe-cop.git
cd vibe-cop

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

Last updated: April 2, 2026 (v11.8.0)
