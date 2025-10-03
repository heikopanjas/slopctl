# Project Instructions for AI Coding Agents

**Last updated:** 2025-10-03

## Project Overview

This repository contains **session initialization prompts for AI coding agents**. It provides a curated collection of ready-to-use initialization prompts for Claude, GitHub Copilot, Cursor, Codex, and other AI coding assistants with built-in governance guardrails and human-in-the-loop controls.

## Technology Stack

- **Language:** Markdown
- **Version Control:** Git
- **License:** MIT

## Repository Structure

```text
vibe-check/
├── LICENSE                     # MIT license
├── README.md                   # Main documentation
├── AGENTS.md                   # This file - primary instructions
├── CLAUDE.md                   # Claude-specific reference
├── .github/
│   └── copilot-instructions.md # GitHub Copilot reference
├── .cursor/
│   └── rules/
│       └── main.mdc            # Cursor AI reference
├── claude/
│   └── instructions.md         # Claude initialization prompts
├── copilot/
│   └── instructions.md         # GitHub Copilot initialization prompts
├── cursor/
│   └── instructions.md         # Cursor AI initialization prompts
├── codex/
│   └── instructions.md         # OpenAI Codex initialization prompts
└── universal/
    └── instructions.md         # Universal prompt for any agent
```

## Coding Conventions

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

- Keep agent-specific instructions in their respective directories
- Maintain consistency across all agent instruction files
- Use three versions of prompts: Quick Copy-Paste, Agent-Specific, Detailed
- Preserve file structure and formatting when updating

## Core Principles

1. **Human control first** – All prompts enforce explicit confirmation before commits
2. **Single source of truth** – This AGENTS.md file is the primary reference
3. **Transparency** – Every change logs rationale with date and reasoning
4. **Minimalism** – Only essential policies that deliver concrete safety or velocity
5. **Scalability** – Add new agents without policy drift
6. **No auto-commits** – CRITICAL: Never commit automatically without explicit confirmation

## Build Commands

This is a documentation-only repository with no build process.

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

1. **Maintain Consistency**: When updating prompts, ensure all three versions (Quick, Standard, Detailed) are aligned
2. **Test Instructions**: Verify that instruction files reference the correct paths and files
3. **Preserve Structure**: Keep the markdown structure consistent across all agent directories
4. **Update README**: Reflect significant changes in the README.md
5. **Date Changes**: Update the "Last updated" timestamp in this file when making changes
6. **Log Updates**: Add entries to "Recent Updates & Decisions" section below

### Content Guidelines

- Keep prompts clear, concise, and actionable
- Emphasize governance guardrails (no auto-commits, human confirmation)
- Reference AGENTS.md as single source of truth in all agent-specific files
- Maintain the three-version structure: Quick Copy-Paste, Agent-Specific, Detailed
- Use consistent terminology across all instruction files

### Security & Safety

- Never include API keys, tokens, or credentials
- Always require explicit human confirmation before commits
- Maintain conventional commit message standards
- Keep change history transparent through commit messages

---

## Recent Updates & Decisions

### 2025-10-03

- Initial AGENTS.md setup
- Established core coding standards and conventions
- Created agent-specific reference files (CLAUDE.md, .github/copilot-instructions.md, .cursor/rules/main.mdc)
- Defined repository structure and governance principles

