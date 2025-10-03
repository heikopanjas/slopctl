# Agent Session Initialization Prompt

## For Any Coding Agent (Universal Version)

```
Analyze the workspace and read the following instruction files in order:
1. AGENTS.md (primary instructions file)
2. [Agent-specific file: CLAUDE.md, .github/copilot-instructions.md, or .cursor/rules/main.mdc]

The agent-specific file references AGENTS.md as the single source of truth. Please confirm you've read and understood these instructions before we begin.

As we work together, update ONLY the AGENTS.md file when coding standards, conventions, or project decisions evolve. The agent-specific reference files should rarely need changes.

When updating AGENTS.md:
- Maintain the "Last updated" timestamp at the top
- Add entries to the "Recent Updates & Decisions" log at the bottom with:
  - Date
  - Brief description
  - Reasoning for the change
- Preserve this structure: title header → timestamp → main instructions → "Recent Updates & Decisions" section

Whenever I ask you to commit changes:
- Stage the changes
- Write a detailed but concise commit message using conventional commits format
- NEVER commit automatically - always wait for explicit confirmation
- This is CRITICAL!
```

---

## AGENTS.md Template with Required Structure

To support this workflow, your AGENTS.md should follow this structure:

```markdown
# Project Instructions for AI Coding Agents

**Last updated:** 2025-01-15

## Project Overview
[Your content here]

## Technology Stack
[Your content here]

## Coding Conventions
[Your content here]

## Build Commands
[Your content here]

## Best Practices
[Your content here]

