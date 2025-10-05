# Project Instructions for AI Coding Agents

**Last updated:** 2025-10-05

## Project Overview

[Describe your project here - what it does, its purpose, and key features]

## Technology Stack

- **Language:** [e.g., Python, TypeScript, JavaScript]
- **Framework:** [e.g., React, Next.js, Django, FastAPI]
- **Version Control:** Git
- **Package Manager:** [e.g., npm, pip, poetry, yarn]
- **License:** [e.g., MIT, Apache 2.0]

## Repository Structure

```text
your-project/
├── [Describe your main directories]
├── [and their purposes]
└── [key files]
```

## Coding Conventions

### Code Style

[Describe your coding style guidelines]

- Naming conventions (camelCase, snake_case, PascalCase)
- Indentation (spaces vs tabs, how many)
- Line length limits
- Comment style
- File organization

### Git Commit Messages

- Follow **conventional commits** format: `type(scope): description`
- Types: `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`
- Keep subject line under 72 characters
- Use imperative mood ("add" not "added" or "adds")
- Add detailed body for complex changes

### File Organization

[Describe how files should be organized]

- Directory structure rules
- Naming conventions for files
- Module organization
- Test file placement

## Core Principles

1. **Human control first** – All prompts enforce explicit confirmation before commits
2. **Single source of truth** – This AGENTS.md file is the primary reference
3. **Transparency** – Every change logs rationale with date and reasoning
4. **Minimalism** – Only essential policies that deliver concrete safety or velocity
5. **Scalability** – Maintain consistency as the project grows
6. **No auto-commits** – CRITICAL: Never commit automatically without explicit confirmation

## Build Commands

### Setup

```bash
# Install dependencies
[your install command, e.g., npm install, pip install -r requirements.txt]

# Initialize project
[your init command, if any]
```

### Development

```bash
# Start development server
[your dev command, e.g., npm run dev, python manage.py runserver]

# Run tests
[your test command, e.g., npm test, pytest]

# Lint code
[your lint command, e.g., npm run lint, flake8]
```

### Build & Deploy

```bash
# Build for production
[your build command, e.g., npm run build]

# Deploy
[your deploy command, if any]
```

## Best Practices

### When Updating This Repository

1. **Maintain Consistency**: Keep code style consistent across the codebase
2. **Test First**: Write tests before implementing features when applicable
3. **Document Changes**: Update documentation when changing functionality
4. **Code Review**: [Describe your code review process]
5. **Date Changes**: Update the "Last updated" timestamp in this file when making changes
6. **Log Updates**: Add entries to "Recent Updates & Decisions" section below

### Development Guidelines

[Add project-specific development guidelines]

- [Guideline 1]
- [Guideline 2]
- [Guideline 3]

### Security & Safety

- Never include API keys, tokens, or credentials in code
- Always require explicit human confirmation before commits
- Maintain conventional commit message standards
- Keep change history transparent through commit messages
- [Add project-specific security guidelines]

### Testing

[Describe your testing approach]

- Unit tests: [location and conventions]
- Integration tests: [location and conventions]
- Test coverage requirements: [if any]
- Testing framework: [e.g., Jest, pytest, JUnit]

### Documentation

[Describe your documentation requirements]

- Code comments: [when and how]
- API documentation: [format and location]
- README updates: [when required]
- Changelog: [if maintained]

---

## Agent-Specific Instructions

### For Claude Users

Use this initialization prompt:

```text
Analyze the workspace and read the following instruction files in order:
1. AGENTS.md (primary instructions file)
2. CLAUDE.md (references AGENTS.md)

Please confirm you've read and understood these instructions before we begin.

As we work together, update ONLY the AGENTS.md file when coding standards, conventions, or project decisions evolve. Do not modify CLAUDE.md unless the reference mechanism itself needs changes.

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

**Quick Copy-Paste:** Read AGENTS.md and CLAUDE.md. Confirm understanding. Update ONLY AGENTS.md—maintain timestamp and add dated entries to "Recent Updates & Decisions" log. For commits: stage, write conventional commit message, NEVER auto-commit, ALWAYS wait for confirmation. CRITICAL!

### For GitHub Copilot Users

Use this initialization prompt:

```text
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

**Quick Copy-Paste:** Read AGENTS.md (primary instructions) and any agent-specific reference file. Confirm understanding. Update ONLY AGENTS.md as we work—maintain timestamp and add dated entries to "Recent Updates & Decisions" log. For commits: stage changes, write conventional commit message, NEVER auto-commit, ALWAYS wait for confirmation. CRITICAL!

### For Cursor Users

Use this initialization prompt:

```text
Analyze the workspace and read the following instruction files in order:
1. AGENTS.md (primary instructions file)
2. .cursor/rules/main.mdc (references AGENTS.md)

Please confirm you've read and understood these instructions before we begin.

As we work together, update ONLY the AGENTS.md file when coding standards, conventions, or project decisions evolve. Do not modify .cursor/rules/main.mdc unless the reference mechanism itself needs changes.

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

**Quick Copy-Paste:** Read AGENTS.md and .cursor/rules/main.mdc. Confirm understanding. Update ONLY AGENTS.md—maintain timestamp and add dated entries to "Recent Updates & Decisions" log. For commits: stage, write conventional commit message, NEVER auto-commit, ALWAYS wait for confirmation. CRITICAL!

### For Codex Users

Use this initialization prompt:

```text
Analyze the workspace and read the following instruction files in order:
1. AGENTS.md (primary instructions file)
2. .github/copilot-instructions.md or AGENTS.md directly (Codex uses similar structure to Copilot)

Please confirm you've read and understood these instructions before we begin.

As we work together, update ONLY the AGENTS.md file when coding standards, conventions, or project decisions evolve. Do not modify reference files unless the reference mechanism itself needs changes.

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

**Quick Copy-Paste:** Read AGENTS.md and .github/copilot-instructions.md. Confirm understanding. Update ONLY AGENTS.md—maintain timestamp and add dated entries to "Recent Updates & Decisions" log. For commits: stage, write conventional commit message, NEVER auto-commit, ALWAYS wait for confirmation. CRITICAL!

### For Universal/Other Agents

Use this initialization prompt:

```text
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

## Recent Updates & Decisions

### 2025-10-05

- Initial AGENTS.md setup
- Established core coding standards and conventions
- Created agent-specific reference files
- Defined repository structure and governance principles
