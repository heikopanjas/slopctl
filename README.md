# vibe-check

**A manager for coding agent instruction files** – A Rust CLI tool that provides a centralized system for managing, organizing, and maintaining initialization prompts and instruction files for AI coding assistants (Claude, GitHub Copilot, Cursor, Codex, and others) with built-in governance guardrails and human-in-the-loop controls.

## Overview

vibe-check is a command-line tool that helps you:

- **Manage templates globally** – Store templates in `~/.config/vibe-check/templates`
- **Initialize projects quickly** – Set up agent instructions with a single command
- **Keep templates synchronized** – Update local templates from global storage
- **Enforce governance** – Built-in guardrails for no auto-commits and human confirmation
- **Support multiple agents** – Works with Claude, Copilot, Cursor, Codex, and more

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

### Initialize a Rust project with Claude

```bash
cd your-project
vibe-check init --lang rust --agent claude
```

This will:

1. Download templates from the default repository (if not already cached)
2. Copy the Rust language template and Claude instructions to your project
3. Create `.claude/instructions.md` for Claude-specific setup

### Initialize from a custom template source

```bash
# From a local path
vibe-check init --lang rust --agent copilot --from /path/to/templates

# From a GitHub URL
vibe-check init --lang python --agent cursor --from https://github.com/user/repo/tree/branch/templates
```

### Update existing templates

```bash
# Update templates for current project
vibe-check update --lang rust --agent claude

# Force update (overwrite local modifications)
vibe-check update --lang rust --agent claude --force
```

### Clear local templates

```bash
# Remove local templates (with confirmation)
vibe-check clear

# Force clear without confirmation
vibe-check clear --force
```

## CLI Commands

### `init` - Initialize Agent Instructions

Initialize instruction files for AI coding agents in your project.

```bash
vibe-check init --lang <language> --agent <agent> [--from <PATH or URL>]
```

**Options:**

- `--lang <string>` - Programming language or framework (e.g., rust, python, typescript, cmake)
- `--agent <string>` - AI coding agent (e.g., claude, copilot, cursor, codex)
- `--from <string>` - Optional path or URL to copy/download templates from

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

| Agent | Status | Template Directory | Notes |
|-------|--------|-------------------|-------|
| Claude | Active | [`templates/claude/`](templates/claude/) | Anthropic's Claude (Code, Sonnet, Opus) |
| GitHub Copilot | Active | [`templates/copilot/`](templates/copilot/) | VS Code Copilot Chat & inline suggestions |
| Cursor | Active | [`templates/cursor/`](templates/cursor/) | Cursor IDE AI assistant |
| Codex | Active | [`templates/codex/`](templates/codex/) | OpenAI Codex-based agents |

## Supported Languages

- **Rust** - Rust programming language
- **C++** - C++ programming language
- **Swift** - Swift programming language
- **CMake** - CMake build system
- **General** - General coding guidelines
- **Git** - Git workflow and commit conventions

## How It Works

### Template Storage

Templates are stored globally in `~/.config/vibe-check/templates/` and include:

- **Language templates**: Language-specific coding standards and conventions
- **Agent templates**: Agent-specific initialization prompts
- **General templates**: AGENTS.md, Git guidelines, and more

### Template Management

1. **First run**: Downloads templates from the default GitHub repository
2. **Local storage**: Templates are cached in `~/.config/vibe-check/templates/`
3. **Checksums**: SHA-256 checksums verify template integrity
4. **Backups**: Automatic backups before any modifications in `~/.cache/vibe-check/backups/`
5. **Updates**: Detect local modifications and warn before overwriting

### Project Initialization

When you run `vibe-check init --lang rust --agent claude`:

1. Checks if global templates exist (downloads if needed)
2. Copies the Rust language template to your project root (`Rust.md`)
3. Creates `.claude/` directory with `instructions.md`
4. You're ready to start coding with proper agent instructions

### Modification Detection

vibe-check detects if you've modified local templates:

```bash
$ vibe-check update --lang rust --agent claude
→ Updating templates for rust with claude
! Local modifications detected:
  - /path/to/Rust.md
  - /path/to/.claude/instructions.md
→ Use --force to overwrite
✗ Local modifications detected. Aborting.
```

Use `--force` to override and update anyway.

## Customization

### Using Custom Templates

You can use your own template repository:

```bash
# From a local path
vibe-check init --lang rust --agent claude --from /path/to/your/templates

# From a GitHub repository
vibe-check init --lang rust --agent claude --from https://github.com/yourname/your-templates/tree/main/templates
```

### Modifying Global Templates

1. Navigate to `~/.config/vibe-check/templates/`
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

- **Language:** Rust (Edition 2021)
- **CLI Framework:** clap v4.5.20
- **Terminal Colors:** owo-colors v4.1.0
- **HTTP Client:** reqwest v0.12 (blocking, json)
- **Checksums:** sha2 v0.10, hex v0.4
- **Date/Time:** chrono v0.4
- **Serialization:** serde v1.0, serde_json v1.0

## FAQ

**Where are templates stored?**
Global templates: `~/.config/vibe-check/templates/`
Backups: `~/.cache/vibe-check/backups/`

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

## Contributing

Contributions are welcome! Please:

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run tests and formatting
5. Submit a pull request

## Acknowledgments

Inspired by the need for consistent, safe, and auditable AI-assisted coding workflows across multiple projects and agents.

---

Last updated: November 9, 2025
