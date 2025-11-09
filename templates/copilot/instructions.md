# Agent Session Initialization Prompt

## Agent-Specific Variations for Copilot

```
Analyze the workspace and read the following instruction files in order:
1. AGENTS.md (primary instructions file)
2. .github/copilot-instructions.md (references AGENTS.md)

Please confirm you've read and understood these instructions before we begin.

As we work together, update ONLY the AGENTS.md file when coding standards, conventions, or project decisions evolve. Do not modify .github/copilot-instructions.md unless the reference mechanism itself needs changes.

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

## Detailed Initial Prompts for Copilot

```
Analyze the workspace and read and follow the instructions in the following files in order:
1. AGENTS.md (primary instructions file - this is your primary reference)
2. .github/copilot-instructions.md (references AGENTS.md)

Please confirm you've read and understood these instructions before we begin.

As we work together, remember to update ONLY the AGENTS.md file as coding standards, conventions, or project decisions evolve. The .github/copilot-instructions.md file is just a reference pointer and should rarely need changes unless the reference mechanism itself changes.

When making updates to AGENTS.md, maintain the "Last updated" timestamp at the top and add entries to the "Recent Updates & Decisions" log at the bottom with the date, brief description, and reasoning for each change. Ensure the file maintains this structure: title header, timestamp line, main instructions content, then the "Recent Updates & Decisions" section at the end.

Whenever I ask you to commit the changes, stage the changes, write a detailed but still concise commit message using conventional commits format and commit the changes. NEVER commit automatically. This is CRITICAL!
```

## Quick Copy-Paste Version for Copilot

```
Read AGENTS.md (primary instructions) and any agent-specific reference file. Confirm understanding. Update ONLY AGENTS.md as we work—maintain timestamp and add dated entries to "Recent Updates & Decisions" log. For commits: stage changes, write conventional commit message, NEVER auto-commit, ALWAYS wait for confirmation. CRITICAL!
```







