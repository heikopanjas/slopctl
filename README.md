# vibe-coding

Operational playbooks & governance for AI coding agents (Claude Code, GitHub Copilot, and future peers). This repository centralizes standards, workflows, and per‑agent operating contracts so multi‑agent collaboration stays predictable, auditable, and safe.

## Goals

- Provide a clear, enforceable contract for each coding agent.
- Keep human maintainer control over commits & change history.
- Standardize structure so adding a new agent guide is low-friction.
- Encourage minimal, testable, reversible changes.
- Scale to multiple agents without policy drift.

## Repository Structure

```text
./
├── .gitignore              # Baseline ignore rules
├── LICENSE                 # MIT license
├── README.md               # You are here
├── claude-code/
│   └── CLAUDE.md           # Claude Code operating contract (source of truth for Claude)
└── copilot-chat/
    └── copilot-instructions.md # GitHub Copilot instruction file
```

Planned (not yet present):

```text
AGENT_TEMPLATE.md           # Copy baseline when introducing a new agent (planned)
CONTRIBUTING.md             # (May be extracted from README when policies grow) (planned)
```

## Philosophy

1. Human-in-the-loop for any persistent change.
2. **Single source of truth per agent**: Each agent only defers to its own contract file plus shared repo conventions here.
3. Transparency: Every edit to an agent contract logs rationale (date + why).
4. Minimalism first: Add only policies that deliver concrete safety or velocity.

## Governance Principles

- Never auto-commit; explicit human request required (reinforced in `CLAUDE.md`).
- No secrets ever added (future: add baseline `.gitignore` + secret scanning suggestion).
- Conventional Commits for all commit messages.
- Change logs live inside each agent file under a "Recent Updates & Decisions" section.

## Current Agent Guides

| Agent | Status | File | Notes |
|-------|--------|------|-------|
| Claude Code | Active | `claude-code/CLAUDE.md` | Includes operating contract & update protocol |
| GitHub Copilot | Active | `copilot-chat/copilot-instructions.md` | Instruction file for Chat code generation |

## Adding a New Agent Guide

Follow this lightweight process:

1. Copy the template (when added) or clone `claude-code/CLAUDE.md` structure.
2. Rename headings & references to the new agent (e.g., `GITHUB_COPILOT.md`).
3. Fill mandatory sections:
   - Operating Contract / Scope
   - File structure & maintenance rules
   - Task Workflow (tailored if agent differs)
   - Guardrails / Prohibited actions
   - Recent Updates & Decisions (start with initialization entry)
4. Add directory: `agent-name-code/` (kebab or consistent pattern) and place file inside.
5. Update this README table (Current Agent Guides).
6. Open a pull request; request review focusing on clarity & safety.

## GitHub Copilot Setup

GitHub Copilot Chat can use instruction files to provide consistent guidance across projects. To set this up:

1. **Copy the instruction file**: Place `copilot-instructions.md` in the `.github/` directory of your project root
2. **Enable the feature**: Set `"github.copilot.chat.codeGeneration.useInstructionFiles": true` in your VS Code settings
3. **Restart VS Code**: The instruction file will be automatically loaded for Copilot Chat sessions

The instruction file provides Copilot with project-specific context, coding standards, and operational guidelines similar to the Claude operating contract but tailored for Copilot's interaction model.

## (Planned) Agent Template Outline

The forthcoming `AGENT_TEMPLATE.md` will include:

```markdown
# <Agent Name> Operating Contract
_Last updated: YYYY-MM-DD (Time Zone)_

## Operating Contract
<Authoritative instructions, hierarchy, and guardrails>

## Workspace Overview
<Structure, build / run commands if repo includes code; or N/A>

## Coding Guidelines
<Formatting, style, testing expectations>

## Task Workflow
<Stepwise process tailored to agent capabilities>

## Safety & Guardrails
<Never do X, require approval for Y>

## Recent Updates & Decisions
- YYYY-MM-DD — Initialized template. _Reasoning: …_
```

## FAQ

**Why separate files per agent?** Isolation reduces cross-policy contamination and cognitive load.

**Why keep logs inside each file instead of a central CHANGELOG?** Proximity + immediate audit trail when reading instructions.

**Can agents modify each other's contracts?** Only if explicitly directed; default: no.

## License

MIT — see `LICENSE`.

## Feedback

Open a PR. Keep proposals concise and outcome-focused.

---

*This README will evolve as additional agent contracts are added.*
