---
name: init-session
description: Start an agentic coding session by reading the primary instruction files and confirming understanding.
license: MIT
metadata:
  source: slopctl
---

# Init Session

Use this skill when the user asks to initialize, start, or reset a coding-agent session for the current workspace.

Analyze the workspace and read the following instruction files in order:

1. AGENTS.md (primary instructions file)

Confirm you've read and understood these instructions before beginning work. Also remember to update the instructions as work progresses.

When making updates, in AGENTS.md maintain the "Last updated" timestamp at the top and add entries to the "Recent Updates & Decisions" log at the bottom with the date, brief description, and reasoning for each change. Ensure the file maintains this structure: title header, timestamp line, main instructions content, then the "Recent Updates & Decisions" section at the end.

Never commit automatically. Whenever the user asks you to commit changes, stage the changes, write a detailed but still concise commit message using conventional commits format, and commit the changes. The commit message must have a maximum length of 500 characters and must not contain special characters or quoting.

