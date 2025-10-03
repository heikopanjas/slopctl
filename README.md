# vibe-coding

**Session initialization prompts for AI coding agents** – A curated collection of ready-to-use initialization prompts for Claude, GitHub Copilot, Cursor, Codex, and other AI coding assistants. This repository provides consistent, safe, and standardized prompts to kickstart agent sessions with governance guardrails built in.

## Overview

This repository contains session initialization prompts designed to establish clear operating contracts with AI coding agents. Each prompt ensures:

- **Human-in-the-loop governance** – No automatic commits without explicit confirmation
- **Standardized workflow** – Conventional commits, structured updates, and change tracking
- **Agent-specific optimization** – Tailored prompts for different AI assistants
- **Universal fallback** – Generic prompts that work with any coding agent

## Repository Structure

```text
./
├── LICENSE                     # MIT license
├── README.md                   # You are here
├── claude/
│   └── instructions.md         # Claude-specific initialization prompts
├── copilot/
│   └── instructions.md         # GitHub Copilot initialization prompts
├── cursor/
│   └── instructions.md         # Cursor AI initialization prompts
├── codex/
│   └── instructions.md         # OpenAI Codex initialization prompts
└── universal/
    └── instructions.md         # Universal prompt for any agent
```

## Philosophy

1. **Human control first** – All prompts enforce explicit confirmation before commits
2. **Single source of truth** – Centralized `AGENTS.md` file for project instructions
3. **Transparency** – Every change logs rationale with date and reasoning
4. **Minimalism** – Only essential policies that deliver concrete safety or velocity
5. **Scalability** – Add new agents without policy drift

## Quick Start

### For Claude (Anthropic)

Copy and paste from [`claude/instructions.md`](claude/instructions.md):

```text
Read AGENTS.md and CLAUDE.md. Confirm understanding. Update ONLY AGENTS.md—maintain timestamp and add dated entries to "Recent Updates & Decisions" log. For commits: stage, write conventional commit message, NEVER auto-commit, ALWAYS wait for confirmation. CRITICAL!
```

### For GitHub Copilot

Copy and paste from [`copilot/instructions.md`](copilot/instructions.md):

```text
Read AGENTS.md (master instructions) and any agent-specific reference file. Confirm understanding. Update ONLY AGENTS.md as we work—maintain timestamp and add dated entries to "Recent Updates & Decisions" log. For commits: stage changes, write conventional commit message, NEVER auto-commit, ALWAYS wait for confirmation. CRITICAL!
```

### For Cursor

Copy and paste from [`cursor/instructions.md`](cursor/instructions.md):

```text
Read AGENTS.md and .cursor/rules/main.mdc. Confirm understanding. Update ONLY AGENTS.md—maintain timestamp and add dated entries to "Recent Updates & Decisions" log. For commits: stage, write conventional commit message, NEVER auto-commit, ALWAYS wait for confirmation. CRITICAL!
```

### For Any Agent (Universal)

Copy and paste from [`universal/instructions.md`](universal/instructions.md) for a generic initialization that works with any AI coding assistant.

## Core Governance Principles

All prompts in this repository enforce these critical rules:

- ✅ **Never auto-commit** – Explicit human request required before any commit
- ✅ **Conventional commits** – Standardized commit message format
- ✅ **Change logging** – Maintain "Recent Updates & Decisions" log with timestamps
- ✅ **Single source of truth** – Update only `AGENTS.md`, not reference files
- ✅ **Structured updates** – Preserve file structure: header → timestamp → content → log
- ✅ **No secrets** – Never add credentials, API keys, or sensitive data

## Agent Instruction Files

Each agent directory contains three versions of the initialization prompt:

| Version | Use Case | Length |
|---------|----------|--------|
| **Quick Copy-Paste** | Fast session start | 1-2 sentences |
| **Agent-Specific** | Standard initialization | 1 paragraph |
| **Detailed** | Comprehensive setup | Full context |

Choose the version that best fits your workflow and comfort level.

## Supported Agents

| Agent | Status | Directory | Notes |
|-------|--------|-----------|-------|
| Claude | ✅ Active | [`claude/`](claude/) | Anthropic's Claude (Code, Sonnet, Opus) |
| GitHub Copilot | ✅ Active | [`copilot/`](copilot/) | VS Code Copilot Chat & inline suggestions |
| Cursor | ✅ Active | [`cursor/`](cursor/) | Cursor IDE AI assistant |
| Codex | ✅ Active | [`codex/`](codex/) | OpenAI Codex-based agents |
| Universal | ✅ Active | [`universal/`](universal/) | Generic prompt for any agent |

## Usage in Your Projects

### Step 1: Set Up AGENTS.md

Create an `AGENTS.md` file in your project root with this structure:

```markdown
# Project Instructions for AI Coding Agents

**Last updated:** 2025-10-03

## Project Overview
[Your project description]

## Technology Stack
[Your tech stack]

## Coding Conventions
[Your conventions]

## Build Commands
[Your commands]

## Best Practices
[Your practices]

---

## Recent Updates & Decisions

### 2025-10-03
- Initial setup
- Established core coding standards
```

### Step 2: Create Agent-Specific Reference Files

For **Claude**, create `CLAUDE.md`:

```markdown
# Claude Operating Contract

Please read and follow the instructions in `AGENTS.md` as your primary reference.

This file exists only to point you to the single source of truth.
```

For **Copilot**, create `.github/copilot-instructions.md`:

```markdown
Please read and follow the instructions in `AGENTS.md` as your primary reference.
```

For **Cursor**, create `.cursor/rules/main.mdc`:

```markdown
Please read and follow the instructions in `AGENTS.md` as your primary reference.
```

For **Codex**: No reference file needed – Codex works directly with `AGENTS.md` out-of-the-box.

### Step 3: Initialize Your Session

Use the appropriate prompt from this repository to start your agent session with governance guardrails enabled.

## Customization

All prompts are designed to be customized for your specific needs:

1. **Fork this repository** or copy the relevant instruction files
2. **Modify the prompts** to match your project's requirements
3. **Add project-specific rules** to your `AGENTS.md` file
4. **Keep the core governance** principles intact (no auto-commits, conventional commits, etc.)

## Adding a New Agent

To add support for a new AI coding agent:

1. Create a new directory: `agent-name/`
2. Add an `instructions.md` file with three prompt versions:
   - Quick Copy-Paste (minimal)
   - Agent-Specific (standard)
   - Detailed (comprehensive)
3. Follow the existing format from other agent directories
4. Update this README's "Supported Agents" table
5. Submit a pull request

## FAQ

**Why separate files per agent?**
Isolation reduces cross-policy contamination and allows agent-specific optimizations.

**Why AGENTS.md as single source of truth?**
Centralized updates prevent drift and make it easier to maintain consistency across sessions.

**Can I use these prompts in commercial projects?**
Yes! MIT license allows commercial use. Attribution appreciated but not required.

**Do these prompts work with future AI agents?**
The `universal/` directory provides a generic prompt that should work with most agents. Customize as needed.

## Contributing

This is a personal collection, but suggestions are welcome! If you have:

- **Improved prompts** for existing agents
- **New agent support** (Tabnine, Cody, Replit AI, etc.)
- **Better governance patterns** or safety mechanisms

Feel free to open an issue or submit a pull request.

## License

MIT License - See [LICENSE](LICENSE) for details.

Copyright (c) 2025 ultralove

## Acknowledgments

Inspired by the need for consistent, safe, and auditable AI-assisted coding workflows across multiple projects and agents.

---

Last updated: October 3, 2025
