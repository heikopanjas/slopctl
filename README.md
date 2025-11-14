# vibe-check

**A manager for coding agent instruction files** – A Rust CLI tool that provides a centralized system for managing, organizing, and maintaining initialization prompts and instruction files for AI coding assistants (Claude, GitHub Copilot, Codex, and others) with built-in governance guardrails and human-in-the-loop controls.

## Overview

vibe-check is a command-line tool that helps you:

- **Manage templates globally** – Store templates in platform-specific directories (e.g., `~/Library/Application Support/vibe-check/templates` on macOS)
- **Configure via YAML** – Define template structure and file mappings in `templates.yml`
- **Initialize projects quickly** – Set up agent instructions with a single command
- **Keep templates synchronized** – Update local templates from global storage
- **Enforce governance** – Built-in guardrails for no auto-commits and human confirmation
- **Support multiple agents** – Works with Claude, Copilot, Codex, and more
- **Flexible file placement** – Use placeholders (`$workspace`, `$userprofile`) for custom locations

## Repository Structure

```text
vibe-check/
├── Cargo.toml                  # Rust project manifest
├── Cargo.lock                  # Dependency lock file
├── .rustfmt.toml               # Rust formatting configuration
├── src/                        # Rust source code
│   ├── main.rs                 # Application entry point and CLI
│   ├── lib.rs                  # Library public API
│   ├── template_manager.rs    # TemplateManager implementation
│   └── utils.rs                # Utility functions
├── LICENSE                     # MIT license
├── README.md                   # You are here
├── AGENTS.md                   # Primary project instructions
├── templates/                  # Template files for various languages and frameworks
│   ├── templates.yml           # Template configuration (defines structure and mappings)
│   ├── AGENTS.md               # Template for project-specific agent instructions
│   ├── best-practices.md       # Best practices template (fragment)
│   ├── build-environment.md    # Generic build environment template (fragment)
│   ├── c++-coding-conventions.md  # C++ coding standards template (fragment)
│   ├── cmake-build-commands.md # CMake/C++ build commands template (fragment)
│   ├── core-principles.md      # Core principles template (fragment)
│   ├── git-workflow-conventions.md  # Git workflow template (fragment)
│   ├── mission-statement.md    # Mission statement template (fragment)
│   ├── rust-coding-conventions.md  # Rust coding standards template (fragment)
│   ├── rust-build-commands.md  # Rust build commands template (fragment)
│   ├── technology-stack.md     # Technology stack template (fragment)
│   ├── claude/
│   │   ├── CLAUDE.md           # Claude main instruction file
│   │   └── commands/
│   │       └── init-session.md # Claude session initialization prompt
│   ├── codex/
│   │   └── prompts/
│   │       └── init-session.md # Codex session initialization prompt
│   └── copilot/
│       ├── copilot-instructions.md # Copilot main instruction file
│       └── prompts/
│           └── init-session.prompt.md # Copilot session prompt
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

### Initialize a C++ project with Claude

```bash
cd your-project
vibe-check init --lang c++ --agent claude
```

This will:

1. Download templates from the default repository (if not already cached)
2. Download and parse `templates.yml` configuration
3. Copy general templates (AGENTS.md) to your project
4. Copy Claude instruction file (CLAUDE.md) to your project root
5. Create `.claude/commands/` directory with session initialization prompt

### Initialize from a custom template source

```bash
# From a local path
vibe-check init --lang c++ --agent copilot --from /path/to/templates

# From a GitHub URL
vibe-check init --lang c++ --agent claude --from https://github.com/user/repo/tree/branch/templates
```

**Note:** The custom source must include a `templates.yml` file that defines the template structure.

### Update existing templates

```bash
# Update templates for current project
vibe-check update --lang c++ --agent claude

# Force update (overwrite local modifications)
vibe-check update --lang c++ --agent claude --force
```

### Clear local templates

```bash
# Remove local templates (with confirmation)
vibe-check clear

# Force clear without confirmation
vibe-check clear --force
```

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
   - Creates SHA checksums for integrity verification

2. **Processes configuration**:
   - Parses `templates.yml` to determine which files to install
   - Identifies fragments marked with `$instructions` placeholder

3. **Creates main AGENTS.md**:
   - Downloads main AGENTS.md template
   - Merges fragments at insertion points:
     - **Mission section**: mission-statement.md, technology-stack.md
     - **Principles section**: core-principles.md, best-practices.md
     - **Languages section**: c++-coding-conventions.md, cmake-build-commands.md (C++ specific)
     - **Integration section**: git-workflow-conventions.md
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

```
Please confirm you've read AGENTS.md and understand the project instructions.
```

Claude should acknowledge the:
- Commit protocol (no auto-commits)
- C++ coding conventions
- Git workflow conventions
- Build environment requirements

### Step 6: Start Coding

Now you can work with Claude following the established guidelines:

```
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
# Update from global storage
vibe-check update --lang c++ --agent claude

# Update from specific source
vibe-check update --lang c++ --agent claude --from https://github.com/user/repo/tree/main/templates
```

vibe-check will:
- Check for local modifications
- Create timestamped backup in `~/.cache/vibe-check/backups/`
- Prompt for confirmation unless `--force` is used

### Step 8: Working with Multiple Agents

You can initialize multiple agents for the same project:

```bash
# Add GitHub Copilot
vibe-check init --lang c++ --agent copilot

# Project now has both Claude and Copilot instructions
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
$ vibe-check update --lang c++ --agent claude
→ Updating templates for c++ with claude
! Local modifications detected:
  - /path/to/my-cpp-project/AGENTS.md
→ Backup created: ~/.cache/vibe-check/backups/2025-11-12_14_30_45/
→ Use --force to overwrite
✗ Local modifications detected. Aborting.
```

**Solution:** Review changes, commit them, then use `--force`:

```bash
git diff AGENTS.md              # Review changes
git add AGENTS.md
git commit -m "docs: customize project instructions"
vibe-check update --lang c++ --agent claude --force
```

**Scenario: Clean up project templates**

```bash
# Remove agent directories and language templates
vibe-check clear

# Removes: .claude/, .github/, .codex/, c++-coding-conventions.md, etc.
# Preserves: AGENTS.md, README.md, LICENSE, source code
```

**Scenario: Use custom templates**

```bash
# Your team maintains custom templates
vibe-check init --lang c++ --agent claude --from https://github.com/yourteam/templates/tree/main/templates

# Or from local path
vibe-check init --lang c++ --agent claude --from ~/company/coding-standards/templates
```

### Tips for Success

1. **Initialize early**: Run `vibe-check init` at project start before adding code
2. **Commit instructions**: Add AGENTS.md and agent files to version control
3. **Team consistency**: All team members should use same template source
4. **Customize carefully**: Modify AGENTS.md as needed, but track changes in git
5. **Update periodically**: Check for template updates monthly or quarterly
6. **Use force sparingly**: Only use `--force` when you understand what you're overwriting
7. **Backup important**: vibe-check creates backups, but git is your primary safety net

## CLI Commands

### `init` - Initialize Agent Instructions

Initialize instruction files for AI coding agents in your project.

```bash
vibe-check init --lang <language> --agent <agent> [--from <PATH or URL>]
```

**Options:**

- `--lang <string>` - Programming language or framework (e.g., c++, cmake)
- `--agent <string>` - AI coding agent (e.g., claude, copilot, codex)
- `--from <string>` - Optional path or URL to copy/download templates from

**Behavior:**

- Downloads `templates.yml` first to determine which files to download
- If `templates.yml` fails to download, the operation stops with an error
- Downloads all files specified in the YAML configuration
- Copies files to locations specified by `target` paths in YAML (using placeholders)

### `update` - Update Templates

Update local templates from global storage.

```bash
vibe-check update --lang <language> --agent <agent> [--force] [--from <PATH or URL>]
```

**Options:**

- `--lang <string>` - Programming language or framework
- `--agent <string>` - AI coding agent
- `--force` - Force overwrite without confirmation
- `--from <string>` - Optional path or URL to copy/download templates from

**Behavior:**

- Loads `templates.yml` from global storage to determine which files to copy
- Resolves placeholders in target paths (`$workspace` → current directory, `$userprofile` → home directory)
- Copies files from global storage to locations specified in YAML
- Detects local modifications using checksums and prompts for confirmation
- Creates timestamped backups before overwriting files

### `clear` - Clear Local Templates

Remove local templates from current directory.

```bash
vibe-check clear [--force]
```

**Options:**

- `--force` - Force clear without confirmation

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

## Supported Languages

Currently configured in `templates.yml`:

- **C++** - C++ programming language (includes `c++-coding-conventions.md` and `cmake-build-commands.md`)
- **Rust** - Rust programming language (includes `rust-coding-conventions.md` and `rust-build-commands.md`)

Additional language templates can be added to `templates.yml` configuration.

## How It Works

### Template Storage

Templates are stored in platform-specific directories:

- **macOS**: `~/Library/Application Support/vibe-check/templates/`
- **Linux**: `~/.local/share/vibe-check/templates/`
- **Windows**: `%LOCALAPPDATA%\vibe-check\templates\`

Templates include:

- **templates.yml**: Configuration file defining structure and file mappings
- **Main template**: AGENTS.md (primary instruction file)
- **Language templates**: Language-specific coding standards and build commands (e.g., c++-coding-conventions.md, cmake-build-commands.md, rust-coding-conventions.md, rust-build-commands.md)
- **Integration templates**: Tool/workflow templates (e.g., git-workflow-conventions.md)
- **Principle templates**: Core principles and best practices
- **Mission templates**: Mission statement, technology stack
- **Agent templates**: Agent-specific instruction files and prompts

### Template Configuration (templates.yml)

The `templates.yml` file defines the template structure with six main sections:

1. **main**: Main AGENTS.md instruction file (primary source of truth)
2. **agents**: Agent-specific files with `instructions` (main file) and `prompts` (custom commands)
3. **languages**: Language-specific coding standards templates
4. **integration**: Tool/workflow integration templates (e.g., git workflows)
5. **principles**: Core principles and general guidelines
6. **mission**: Mission statement, purpose, and project overview

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
main:
    source: AGENTS.md
    target: '$workspace/AGENTS.md'

agents:
    claude:
        instructions:
            source: claude/CLAUDE.md
            target: '$workspace/CLAUDE.md'
        prompts:
            - source: claude/commands/init-session.md
              target: '$workspace/.claude/commands/init-session.md'

languages:
    c++:
        files:
            - source: c++-coding-conventions.md
              target: '$instructions'

principles:
    - source: core-principles.md
      target: '$instructions'
```

### Template Management

1. **First run**: Downloads `templates.yml` and all specified files from GitHub
2. **Local storage**: Templates are cached in platform-specific directory
3. **Checksums**: SHA-256 checksums verify template integrity
4. **Backups**: Automatic timestamped backups in cache directory before any modifications
5. **Updates**: Detect local modifications and warn before overwriting
6. **Placeholders**: `$workspace` and `$userprofile` resolve to appropriate paths

### Project Initialization

When you run `vibe-check init --lang c++ --agent claude`:

1. Checks if global templates exist (downloads if needed)
2. Loads `templates.yml` configuration
3. Downloads main AGENTS.md template
4. Downloads and merges fragments (mission, principles, language, integration) into AGENTS.md at insertion points
5. Copies Claude instruction file (CLAUDE.md) to project root
6. Creates `.claude/commands/` directory with prompts
7. You're ready to start coding with proper agent instructions

The resulting AGENTS.md contains the complete merged content with all relevant sections for your project.

### Modification Detection

vibe-check detects if you've modified local templates:

```bash
$ vibe-check update --lang c++ --agent claude
→ Updating templates for c++ with claude
! Local modifications detected:
  - /path/to/CLAUDE.md
  - /path/to/.claude/commands/init-session.md
→ Use --force to overwrite
✗ Local modifications detected. Aborting.
```

Use `--force` to override and update anyway.

## Customization

### Using Custom Templates

You can use your own template repository:

```bash
# From a local path
vibe-check init --lang c++ --agent claude --from /path/to/your/templates

# From a GitHub repository
vibe-check init --lang c++ --agent claude --from https://github.com/yourname/your-templates/tree/main/templates
```

**Note:** Your custom template repository must include a `templates.yml` file that defines the template structure and file mappings.

### Modifying Global Templates

1. Navigate to platform-specific template directory:
   - macOS: `~/Library/Application Support/vibe-check/templates/`
   - Linux: `~/.local/share/vibe-check/templates/`
   - Windows: `%LOCALAPPDATA%\vibe-check\templates\`
2. Edit the templates as needed
3. Run `vibe-check update` to sync changes to your projects

### Creating New Templates

To add a new language or agent template:

1. Fork this repository
2. Add your template to the `templates/` directory
3. For languages: Create `Language.md` (e.g., `Python.md`)
4. For agents: Create `agent-name/instructions.md`
5. Submit a pull request

## Technology Stack

- **Language:** Rust (Edition 2024)
- **CLI Framework:** clap v4.5.20
- **Terminal Colors:** owo-colors v4.1.0
- **HTTP Client:** reqwest v0.12 (blocking, json)
- **Checksums:** sha2 v0.10, hex v0.4
- **Date/Time:** chrono v0.4
- **Serialization:** serde v1.0, serde_json v1.0

## FAQ

**Where are templates stored?**
- Global templates (macOS): `~/Library/Application Support/vibe-check/templates/`
- Global templates (Linux): `~/.local/share/vibe-check/templates/`
- Global templates (Windows): `%LOCALAPPDATA%\vibe-check\templates\`
- Backups: Platform-specific cache directory

**What happens if I modify local templates?**
vibe-check detects modifications and warns you before overwriting. Use `--force` to override.

**Can I use my own template repository?**
Yes! Use the `--from` option to specify a local path or GitHub URL.

**Why AGENTS.md as single source of truth?**
Centralized updates prevent drift and make it easier to maintain consistency across sessions.

**Can I use this in commercial projects?**
Yes! MIT license allows commercial use. Attribution appreciated but not required.

**How do I update templates?**
Run `vibe-check update --lang <language> --agent <agent>` to sync from global storage.

**How do I remove local templates?**
Run `vibe-check clear` to remove agent directories and language files from your project.

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

# Build in release mode (optimized)
cargo build --release

# Format code
cargo fmt

# Run linter
cargo clippy
```

---

Last updated: November 14, 2025
