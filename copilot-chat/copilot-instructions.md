# Project Guide for GitHub Copilot

_Last updated: 2025-08-22 (Europe/Berlin)_

> **Copilot, before doing any task:**
> 1) Confirm you have read and understood the current version of this file.
> 2) If you change this file, update the timestamp above and add a log entry in **Recent Updates & Decisions**.

## Operating Contract

- **Source of truth:** Follow the instructions in this file for all work in this repository.
- **File structure must stay exactly:**
  1. Title header (H1)
  2. Single timestamp line in the format: `_Last updated: YYYY-MM-DD (Time Zone)_`
  3. Main instructions content (sections below)
  4. **Recent Updates & Decisions** section at the very end

- **Maintaining this file:**
  - When you make changes to *this* file, you must:
    - Update the timestamp line.
    - Append a new entry under **Recent Updates & Decisions** with:
      - Date (YYYY-MM-DD)
      - Brief description of what changed
      - Short reasoning

- **Committing on request (“commit the latest changes”):**
  - ⚠️ **NEVER commit automatically. Wait for my explicit request and confirmation.**
  - Stage modified files only when I ask you to.
  - Write a **Conventional Commits** message (e.g., `feat: …`, `fix: …`, `docs: …`, `refactor: …`).
  - Keep the message detailed yet concise; include scope when useful (e.g., `feat(ui): …`).
  - If there are **no changes**, state that explicitly instead of committing.

- **Guardrails (never stage/commit):**
  - Secrets or credentials (`.env`, keychains, tokens, certificates, private keys).
  - Local caches, build artifacts, or system/user settings (`DerivedData/`, `.DS_Store`, editor configs not versioned here unless explicitly allowed).
  - Any files listed in `.gitignore`.

- **Safety checks before committing:**
  - Run lightweight validations where applicable (format, lint, typecheck).
  - If validations fail, report the failure and do not commit.

## Workspace Overview
_(Fill in with a short description of the repo, main modules, build commands, test commands, and how to run locally.)_

## Coding Guidelines
_(Keep these concise; link to fuller docs if needed.)_

## Task Workflow
1. Clarify the goal succinctly (one paragraph max). If ambiguous, propose a best-effort plan and proceed.
2. Make the smallest useful change set; include tests when practical.
3. Run validations (format/lint/tests) relevant to the change.
4. On request to “commit the latest changes,” follow the **Committing** rules above.

## Context Files to Preferentially Use
_(Optional: list docs Copilot should read first as context. Keep short.)_

---

## Recent Updates & Decisions

- **2025-08-22** — Initialize Copilot operating contract.
  _Reasoning:_ Adapted from Claude Code template, added explicit safeguard to NEVER commit automatically.
