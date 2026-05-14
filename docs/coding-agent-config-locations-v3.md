# Coding Agent Configuration Locations

Instruction files, custom prompts, skills & subagents for CLI / agentic coding tools — May 2026

Skills follow the [agentskills.io](https://agentskills.io/specification) open standard (v1.0).
`.agents/` is the cross-agent convention path.

### Confidence markers

| Marker | Meaning |
|--------|---------|
| ★ | **Spec / stable** — defined in a published spec or long-standing official docs |
| ◆ | **Documented current** — in official docs but may shift across releases |
| ○ | **Observed** — community-verified, reverse-engineered, or very recent; may be volatile |

---

## Claude Code — Anthropic

Home: `~/.claude/`

| Feature | Global (user) | Project (repo) |
|---|---|---|
| **Instructions** ★ | `~/.claude/CLAUDE.md` — personal defaults, all projects | `<repo>/CLAUDE.md`, `<repo>/CLAUDE.local.md` — walks cwd up to `/`; also reads `AGENTS.md` |
| **Instructions** ○ | | System: `/etc/claude-code/CLAUDE.md` — installation-dependent; observed via strace, not in public docs |
| **Rules** ★ | `~/.claude/rules/*.md` | `.claude/rules/*.md` — path-scoped via YAML frontmatter `paths:` |
| **Settings** ★ | `~/.claude/settings.json` | `.claude/settings.json`, `.claude/settings.local.json` (gitignored, highest priority) |
| **Skills** ★ | `~/.claude/skills/*/SKILL.md` | `.claude/skills/*/SKILL.md` — progressive disclosure per agentskills.io |
| **Subagents** ★ | `~/.claude/agents/*.md` | `.claude/agents/*.md` — Markdown + YAML frontmatter |
| **Commands** ★ | `~/.claude/commands/*.md` → `/user:<name>` | `.claude/commands/*.md` → `/project:<name>` |
| **MCP** ★ | | `<repo>/.mcp.json` |

**Sources:**
[`.claude` directory explorer](https://code.claude.com/docs/en/claude-directory) ·
[Memory & instructions](https://code.claude.com/docs/en/memory) ·
[Skills](https://code.claude.com/docs/en/skills) ·
[Sub-agents](https://code.claude.com/docs/en/sub-agents) ·
[Settings](https://code.claude.com/docs/en/settings) ·
`/etc/claude-code/` path: [anthropics/claude-code#2274](https://github.com/anthropics/claude-code/issues/2274)

---

## Codex CLI — OpenAI

Home: `~/.codex/` (`$CODEX_HOME`)

> **Volatility note:** Codex CLI is under active Rust rewrite. Config surface changes frequently across releases. Paths below are from official OpenAI developer docs but should be re-verified after major updates.

| Feature | Global (user) | Project (repo) |
|---|---|---|
| **Instructions** ◆ | `~/.codex/AGENTS.override.md`, `~/.codex/AGENTS.md` — override wins; first non-empty used | `<repo>/AGENTS.override.md`, `<repo>/AGENTS.md` — walks root→cwd; 1 file/dir; 32 KiB default cap (`project_doc_max_bytes`). Fallbacks via `project_doc_fallback_filenames` |
| **Config** ◆ | `~/.codex/config.toml` | `.codex/config.toml` — walks root→cwd; closest wins (trusted projects only) |
| **Skills** ◆ | `~/.codex/skills/*/SKILL.md`, `~/.agents/skills/*/SKILL.md` | `.codex/skills/*/SKILL.md`, **`.agents/skills/*/SKILL.md`** — walks cwd→root; `/skills` or `$` to invoke. Admin: `/etc/codex/skills/`; System: bundled |
| **Subagents** ◆ | `~/.codex/agents/*.toml` | `.codex/agents/*.toml` — also `[agents.*]` in config.toml |
| **Rules / Hooks** ◆ | `~/.codex/rules/`, `~/.codex/hooks.json` | `.codex/rules/`, `.codex/hooks.json` |
| **MCP** ◆ | `[mcp_servers]` in config.toml | `[mcp_servers]` in `.codex/config.toml` |

**Sources:**
[AGENTS.md discovery](https://developers.openai.com/codex/guides/agents-md) ·
[Skills](https://developers.openai.com/codex/skills) ·
[Subagents](https://developers.openai.com/codex/subagents) ·
[Config basics](https://developers.openai.com/codex/config-basic) ·
[Config reference](https://developers.openai.com/codex/config-reference) ·
[Sample config](https://developers.openai.com/codex/config-sample) ·
[CLI reference](https://developers.openai.com/codex/cli/reference)

---

## GitHub Copilot — GitHub / Microsoft

Home: `~/.copilot/` · `~/.github/`

| Feature | Global (user) | Project (repo) |
|---|---|---|
| **Instructions** ★ | `$HOME/.copilot/copilot-instructions.md` (CLI). VS Code: user `*.instructions.md`. `$COPILOT_CUSTOM_INSTRUCTIONS_DIRS` | `.github/copilot-instructions.md` — repository-wide. Also reads `AGENTS.md`, `CLAUDE.md`, `GEMINI.md` at root |
| **Path-specific** ★ | — | `.github/instructions/*.instructions.md` — YAML frontmatter `applyTo:` glob |
| **Prompt files** ★ | — | `.github/prompts/*.prompt.md` — invoke via `#prompt:` or `/` |
| **Custom agents** ◆ | `~/.github/agents/*.agent.md` — user-level agents (VS 2026+ / VS Code) | `.github/agents/*.agent.md` — YAML: name, description, tools, model, mcp-servers, handoffs. Org: `.github-private` repo `agents/` dir |
| **Skills** ★ | `~/.copilot/skills/*/SKILL.md`, `~/.agents/skills/*/SKILL.md` | `.github/skills/*/SKILL.md`, `.claude/skills/*/SKILL.md`, **`.agents/skills/*/SKILL.md`** — all three discovered ¹; `gh skill` CLI |
| **MCP** ◆ | — | `.github/copilot/mcp.json` |

**Sources:**
[Custom instructions (CLI)](https://docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/add-custom-instructions) ·
[Custom instructions (VS Code)](https://code.visualstudio.com/docs/copilot/customization/custom-instructions) ·
[About agent skills](https://docs.github.com/en/copilot/concepts/agents/about-agent-skills) ·
[Adding skills](https://docs.github.com/en/copilot/how-tos/use-copilot-agents/cloud-agent/create-skills) ·
[Custom agents (cloud)](https://docs.github.com/en/copilot/how-tos/copilot-on-github/customize-copilot/customize-cloud-agent/create-custom-agents) ·
[Custom agents (VS Code)](https://code.visualstudio.com/docs/copilot/customization/custom-agents) ·
[Custom agents config ref](https://docs.github.com/en/copilot/reference/custom-agents-configuration) ·
[Agent skills (VS Code)](https://code.visualstudio.com/docs/copilot/customization/agent-skills) ·
[VS 2026 April update](https://github.blog/changelog/2026-04-30-github-copilot-in-visual-studio-april-update/)

---

## Cursor — Anysphere

Home: `.cursor/` (project-centric)

| Feature | Global (user) | Project (repo) |
|---|---|---|
| **Instructions** ★ | User Rules (Settings → Rules) — plain text; always applied | `<repo>/AGENTS.md` — root level; subdir behavior documented but evolving across versions. Legacy: `.cursorrules` (deprecated, still read) |
| **Rules** ★ | — | `.cursor/rules/*.md` — YAML frontmatter: `alwaysApply`, `description`, `globs`. 4 modes: Always, Auto Attached, Agent Requested, Manual. Legacy `.mdc` still supported |
| **Commands** ◆ | — | `.cursor/commands/*.md` — invoke via `/` in agent input |
| **Skills** ◆ | `~/.claude/skills/*/SKILL.md` — Claude plugins imported as agent-decided rules ² | `.cursor/skills/*/SKILL.md`, `.claude/skills/*/SKILL.md`, **`.agents/skills/*/SKILL.md`** — agentskills.io; loaded on demand |
| **Subagents** ◆ | — | Parallel subagents (v2.4+); each gets own context window. Custom definitions via docs |
| **Hooks** ◆ | — | `.cursor/hooks/` — compatible with Claude Code hooks format |
| **MCP** ◆ | — | `.cursor/*.json` — MCP defs; agents discover/load on demand |
| **Team Rules** ◆ | Import from GitHub repos (auto-synced). Precedence: Team → Project → User | *(same)* |

**Sources:**
[Cursor docs home](https://cursor.com/docs) ·
[Best practices (rules + skills)](https://cursor.com/blog/agent-best-practices) ·
[Changelog 2.4 (subagents, skills)](https://cursor.com/changelog/2-4) ·
[awesome-cursor-rules reference](https://github.com/sanjeed5/awesome-cursor-rules-mdc/blob/main/cursor-rules-reference.md) ·
[Cursor rules guide](https://www.vibecodingacademy.ai/blog/cursor-rules-complete-guide) ·
[Agent skills in Cursor (SkillsAuth)](https://skillsauth.com/skills/hub/for-cursor) ·
[Cursor community forum: agent plugins request](https://forum.cursor.com/t/agent-plugins-isolated-packaging-lifecycle-management-for-sub-agents-skills-hooks-rules-incl-agent-md-across-cursor-ide-cli/151250)

---

## Mistral Vibe — Mistral AI

Home: `~/.vibe/` (`$VIBE_HOME`)

> **Note:** Vibe is younger than the other agents listed here. Docs are thinner and the config surface is still stabilizing. Treat specifics as current observed behavior.

| Feature | Global (user) | Project (repo) |
|---|---|---|
| **Instructions** ◆ | — | `<repo>/AGENTS.md` — workspace root only (no recursive walk) |
| **Config** ◆ | `~/.vibe/config.toml` | `.vibe/config.toml` — project-local takes precedence |
| **System prompts** ◆ | `~/.vibe/prompts/*.md` — set `system_prompt_id` in config.toml | — |
| **Skills** ◆ | `~/.vibe/skills/*/SKILL.md` — agentskills.io; invoke via `/` | `.vibe/skills/*/SKILL.md` |
| **Agents** ◆ | `~/.vibe/agents/*.toml` — `display_name`, `safety`, `enabled_tools` | `.vibe/agents/*.toml` — subagents: `agent_type = "subagent"` |
| **API keys** ◆ | `~/.vibe/.env` | — |

**Sources:**
[Configuration](https://docs.mistral.ai/mistral-vibe/introduction/configuration) ·
[Agents & Skills](https://docs.mistral.ai/mistral-vibe/agents-skills) ·
[GitHub: mistralai/mistral-vibe](https://github.com/mistralai/mistral-vibe) ·
[PyPI: mistral-vibe](https://pypi.org/project/mistral-vibe/) ·
[Vibe 2.0 announcement](https://mistral.ai/news/mistral-vibe-2-0) ·
[Remote agents + Medium 3.5](https://mistral.ai/news/vibe-remote-agents-mistral-medium-3-5)

---

## OpenCode — SST (open source)

Home: `~/.config/opencode/`

| Feature | Global (user) | Project (repo) |
|---|---|---|
| **Instructions** ★ | `~/.config/opencode/AGENTS.md` — personal rules, all sessions. Compat fallback: `~/.claude/CLAUDE.md` ³ | `<repo>/AGENTS.md` — first match wins (AGENTS.md > CLAUDE.md). Extra via `opencode.json` `instructions: […]`; supports remote URLs + globs |
| **Config** ★ | `~/.config/opencode/opencode.json` | `<repo>/opencode.json` |
| **Commands** ★ | `~/.config/opencode/commands/*.md` — invoke via `/` in the TUI | `.opencode/commands/*.md` — filename becomes command name |
| **Skills** ★ | `~/.config/opencode/skills/*/SKILL.md`, `~/.claude/skills/*/SKILL.md`, `~/.agents/skills/*/SKILL.md` | `.opencode/skills/*/SKILL.md`, `.claude/skills/*/SKILL.md`, **`.agents/skills/*/SKILL.md`** — walks cwd→git root |
| **Agents** ★ | `~/.config/opencode/agents/*.md` — YAML: description, model, temperature, tools, mode | `.opencode/agents/*.md` — primary (Tab) or subagent (@ invoke) |

**Sources:**
[Rules (AGENTS.md)](https://opencode.ai/docs/rules/) ·
[Agent Skills](https://opencode.ai/docs/skills/) ·
[Agents](https://opencode.ai/docs/agents/) ·
[Getting started](https://opencode.ai/docs/)

---

## Notes

¹ VS 2026, VS Code, and Copilot CLI all discover `.github/skills/`, `.claude/skills/`, and `.agents/skills/`. See [VS 2026 April update](https://github.blog/changelog/2026-04-30-github-copilot-in-visual-studio-april-update/).

² Cursor imports Claude skills/plugins as agent-decided rules; toggleable but not always-apply. See [awesome-cursor-rules reference](https://github.com/sanjeed5/awesome-cursor-rules-mdc/blob/main/cursor-rules-reference.md).

³ OpenCode Claude compat disabled via `OPENCODE_DISABLE_CLAUDE_COMPAT=1`. See [OpenCode rules docs](https://opencode.ai/docs/rules/).

### Cross-agent standards

- **`.agents/skills/`** — The cross-agent convention from the [agentskills.io client implementation guide](https://agentskills.io/client-implementation/adding-skills-support). Scanned by Codex, Copilot, OpenCode, Gemini CLI, and Cursor. Symlink-friendly for a single canonical skill tree.

- **`AGENTS.md`** — Open standard ([agents.md](https://agents.md)). Fully or partially supported by a growing number of coding agents including Codex, Copilot, Cursor, OpenCode, Vibe, Zed, Warp, Jules, Factory, and Devin. Depth of support varies (native first-class to fallback-if-present).

- **`SKILL.md`** — [Agent Skills spec v1.0](https://agentskills.io/specification), maintained by Anthropic at [github.com/agentskills/agentskills](https://github.com/agentskills/agentskills). Structure: `skill-name/{SKILL.md, scripts/, references/, assets/}`. Adopted by 26+ tools.

- **Subagent format split:** Claude Code, Copilot, OpenCode, and Cursor define agents as Markdown + YAML frontmatter. Codex and Vibe use TOML.

All paths use Unix notation; `~` = `%USERPROFILE%` on Windows. `$CODEX_HOME`, `$VIBE_HOME` override their respective defaults.

### slopctl agent defaults

slopctl keeps the agent filesystem conventions from this document in `agent-defaults.yml`, stored next to `templates.yml` in the global template cache. Use `slopctl agents --update` to refresh agent prompt, skill, marker, and cross-client-skill defaults independently from templates. `slopctl templates --update` bootstraps this file only when it is missing.

---

*Last verified: 2026-05-09*
