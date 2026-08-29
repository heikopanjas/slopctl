# Coding Agent Configuration Locations

Instruction files, custom prompts, skills & subagents for CLI / agentic coding tools — August 2026

Skills follow the [agentskills.io](https://agentskills.io/specification) open standard.
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
| **Instructions** ★ | `~/.claude/CLAUDE.md` — personal defaults, all projects | `<repo>/CLAUDE.md` or `<repo>/.claude/CLAUDE.md`, `<repo>/CLAUDE.local.md` — walks cwd up to `/`. Does **not** natively read `AGENTS.md`; use `@AGENTS.md` import inside `CLAUDE.md` to share with other agents. Subdirectory `CLAUDE.md` files load lazily |
| **Instructions** ◆ | Managed: `/etc/claude-code/CLAUDE.md` (Linux/WSL) · `/Library/Application Support/ClaudeCode/CLAUDE.md` (macOS) · `C:\Program Files\ClaudeCode\CLAUDE.md` (Windows) | — |
| **Rules** ★ | `~/.claude/rules/*.md` | `.claude/rules/*.md` — path-scoped via YAML frontmatter `paths:`; discovered recursively |
| **Settings** ★ | `~/.claude/settings.json`; Managed: `/etc/claude-code/managed-settings.json` + `managed-settings.d/*.json` (Linux/WSL) · `/Library/Application Support/ClaudeCode/managed-settings.json` + `managed-settings.d/` + MDM plist `com.anthropic.claudecode` (macOS) · `C:\Program Files\ClaudeCode\managed-settings.json` + `managed-settings.d\` + `HKLM\SOFTWARE\Policies\ClaudeCode` + `HKCU\SOFTWARE\Policies\ClaudeCode` (Windows; `C:\ProgramData\ClaudeCode\` deprecated since v2.1.75) | `.claude/settings.json`, `.claude/settings.local.json` (gitignored). Precedence: Managed > CLI args > Local > Project > User |
| **Skills** ★ | `~/.claude/skills/*/SKILL.md` | `.claude/skills/*/SKILL.md` — progressive disclosure per agentskills.io |
| **Subagents** ★ | `~/.claude/agents/*.md`; managed/enterprise subagents deployed inside the managed-settings directory take highest priority | `.claude/agents/*.md` — Markdown + YAML frontmatter; subdirectory discovery supported, but identity comes only from the `name` field — no directory-qualified naming on clash (that pattern, e.g. `apps/web:deploy`, applies to skills, not subagents); same-name files within one `.claude/agents/` tree resolve by unspecified filesystem read order, but across nested project `.claude/agents/` directories the definition closest to the working directory wins (v2.1.178+); only plugin subagents get scoped identifiers like `my-plugin:review:security`. `isolation: worktree` frontmatter runs the subagent in a temporary git worktree; `background` field; `--agents` CLI flag defines session-scoped JSON subagents; nested spawning up to depth 3 by default (v2.1.219+; was depth 5 in v2.1.172–v2.1.216, depth 1 in v2.1.217–v2.1.218) |
| **Agent memory** ◆ | `~/.claude/agent-memory/<agent-name>/` — subagent user-scoped (`memory: user`). Main-session auto-memory: `~/.claude/projects/<project>/memory/MEMORY.md` (v2.1.59+; `autoMemoryDirectory` setting overrides) | `.claude/agent-memory/<agent-name>/` (`memory: project`, committable), `.claude/agent-memory-local/<agent-name>/` (`memory: local`, gitignored) |
| **Commands** ★ | `~/.claude/commands/*.md` → `/<name>` (superseded by skills, still works) | `.claude/commands/*.md` → `/<name>` (superseded by skills, still works) ⁴ |
| **Workflows** ◆ | `~/.claude/workflows/*.js` — JavaScript multi-agent orchestration scripts | `.claude/workflows/*.js` — each saved file becomes a `/<name>` command; `/workflows` opens the separate run-management/progress view (list, watch, pause, stop). Dynamic workflows require v2.1.154+ and, on the Pro plan, manual enablement via `/config` |
| **Output styles** ◆ | `~/.claude/output-styles/*.md` — custom response format styles (frontmatter-based: `name`, `description`, `keep-coding-instructions`) | `.claude/output-styles/*.md`. Standalone `/output-style` command deprecated v2.1.73, removed v2.1.91 — set via `/config` or the `outputStyle` setting key |
| **Worktrees** ◆ | — | `<repo>/.worktreeinclude` — lists gitignored files to copy into new worktrees; `.gitignore` syntax. `--worktree`/`-w` flag or the `EnterWorktree` tool create worktrees under `.claude/worktrees/<name>/` on branch `worktree-<name>`; `worktree.baseRef` setting (`"fresh"` vs `"head"`) |
| **MCP** ★ | `~/.claude.json` — global MCP server config (also holds OAuth session, per-project trust state, caches); enterprise `managed-mcp.json` at `/etc/claude-code/managed-mcp.json` (Linux/WSL) · `/Library/Application Support/ClaudeCode/managed-mcp.json` (macOS) · `C:\Program Files\ClaudeCode\managed-mcp.json` (Windows). Policy settings `allowedMcpServers`/`deniedMcpServers`/`allowManagedMcpServersOnly`/`strictPluginOnlyCustomization` restrict MCP sources | `<repo>/.mcp.json` |

**Sources:**
[Memory & instructions](https://code.claude.com/docs/en/memory) ·
[Skills](https://code.claude.com/docs/en/skills) ·
[Sub-agents](https://code.claude.com/docs/en/sub-agents) ·
[Settings](https://code.claude.com/docs/en/settings) ·
[`.claude` directory explorer](https://code.claude.com/docs/en/claude-directory) ·
[Workflows](https://code.claude.com/docs/en/workflows) ·
[Output styles](https://code.claude.com/docs/en/output-styles) ·
[Worktrees](https://code.claude.com/docs/en/worktrees) ·
[Managed MCP](https://code.claude.com/docs/en/managed-mcp) ·
`/etc/claude-code/` path: [anthropics/claude-code#2274](https://github.com/anthropics/claude-code/issues/2274)

---

## Codex CLI — OpenAI

Home: `~/.codex/` (`$CODEX_HOME`)

> **Rust rewrite complete:** The TypeScript CLI was gradually phased out through late 2025 (no formal retirement date announced). The current codebase is ~96.5% Rust (`codex-rs/`, up from ~95.6% a week prior — a continuously drifting figure), production-stable since June 2025 (precise GA date unconfirmed by official sources), with a release cadence that has accelerated further — multiple releases/day are now routine (e.g. 10 tags across Aug 26–27, 2026, including alpha builds), up from ~1–2/day in July 2026. Core config paths are stable post-transition. (A separate TypeScript **SDK**, `sdk/typescript`, still exists and is unrelated to the retired TS CLI.)

| Feature | Global (user) | Project (repo) |
|---|---|---|
| **Instructions** ◆ | `~/.codex/AGENTS.override.md` (takes precedence), `~/.codex/AGENTS.md` — global user-level instructions | `<repo>/AGENTS.override.md`, `<repo>/AGENTS.md` — walks root→cwd; override file wins; first non-empty used; 1 file/dir; 32 KiB default cap (`project_doc_max_bytes`). Fallbacks via `project_doc_fallback_filenames` |
| **Config** ◆ | `~/.codex/config.toml`; `~/.codex/<profile>.config.toml` — named profile selected via `--profile` flag. Enterprise managed defaults: `/etc/codex/managed_config.toml` (Unix) · `~/.codex/managed_config.toml` (Windows/non-Unix) · macOS MDM `com.openai.codex` domain (`config_toml_base64`, `requirements_toml_base64` keys; highest precedence) | `.codex/config.toml` — walks root→cwd; closest wins (trusted projects only). System: `/etc/codex/config.toml`. Enterprise enforced: `/etc/codex/requirements.toml` (Unix) · `%ProgramData%\OpenAI\Codex\requirements.toml` (Windows) — cannot be overridden. `--strict-config` errors on unrecognized fields |
| **Skills** ◆ | `~/.agents/skills/*/SKILL.md` — user-level | `.agents/skills/*/SKILL.md` — walks CWD → parent → repo root; `/skills` or `$` to invoke. Toggle via `[[skills.config]]` (`path`, `enabled`) in config.toml. Admin: `/etc/codex/skills/`; System: bundled |
| **Subagents** ◆ | `~/.codex/agents/*.toml` — required fields `name`, `description`, `developer_instructions` | `.codex/agents/*.toml`; also `agents.<name>.config_file`/`agents.<name>.description` in `config.toml`. Global settings under `[agents]`: `enabled` (default `true`), `max_concurrent_threads_per_session` (legacy alias `max_threads`), `default_subagent_model`, `default_subagent_reasoning_effort`, `interrupt_message` (default `true`) |
| **Rules** ◆ | `~/.codex/rules/default.rules` — Starlark `prefix_rule()` syntax (`pattern`, `decision` [allow/prompt/forbidden], `justification`, `match`/`not_match`); written by TUI allow-command; validate via `codex execpolicy check` | `.codex/rules/` — loads only when the project is trusted |
| **Hooks** ◆ | `~/.codex/hooks.json` — or inline `[hooks]` table in `config.toml` (merges both if present in same layer, with a startup warning); gated by `features.hooks`. Events: `SessionStart`, `SessionEnd`, `PreToolUse`, `PermissionRequest`, `PostToolUse`, `UserPromptSubmit`, `PreCompact`/`PostCompact`, `SubagentStart`/`SubagentStop`, `Stop` | `.codex/hooks.json`; or inline `[hooks]` in `.codex/config.toml`. `--dangerously-bypass-hook-trust` runs hooks without persisted trust |
| **MCP** ◆ | `[mcp_servers]` in `~/.codex/config.toml` | `[mcp_servers]` in `.codex/config.toml` |
| **Plugins** ◆ | `~/.agents/plugins/marketplace.json` — personal plugin marketplace catalog; installed plugins cached at `~/.codex/plugins/cache/$MARKETPLACE_NAME/$PLUGIN_NAME/$VERSION/` (`local` version for local plugins) | `.codex-plugin/plugin.json` — manifest bundling `skills/`, `.mcp.json`, `hooks/hooks.json`, `.app.json`, `assets/` (not a top-level `hooks.json`); repo marketplace at `.agents/plugins/marketplace.json` with plugins under `plugins/` |

**Sources:** (`developers.openai.com/codex/*` now 308-redirects to `learn.chatgpt.com/docs/*`, rebranded "ChatGPT Learn"; canonical URLs below)
[AGENTS.md discovery](https://learn.chatgpt.com/docs/agent-configuration/agents-md) ·
[Skills](https://learn.chatgpt.com/docs/build-skills) ·
[Subagents](https://learn.chatgpt.com/docs/agent-configuration/subagents) ·
[Config basics](https://learn.chatgpt.com/docs/config-file/config-basic) ·
[Config reference](https://learn.chatgpt.com/docs/config-file/config-reference) ·
[Advanced config](https://learn.chatgpt.com/docs/config-file/config-advanced) ·
[Managed configuration](https://learn.chatgpt.com/docs/enterprise/managed-configuration) ·
[Hooks](https://learn.chatgpt.com/docs/hooks) ·
[Rules](https://learn.chatgpt.com/docs/agent-configuration/rules) ·
[Plugins](https://learn.chatgpt.com/docs/build-plugins) ·
[CLI reference](https://learn.chatgpt.com/docs/developer-commands?surface=cli)

---

## GitHub Copilot — GitHub / Microsoft

Home: `~/.copilot/` · `~/.github/`

| Feature | Global (user) | Project (repo) |
|---|---|---|
| **Instructions** ★ | `$HOME/.copilot/copilot-instructions.md` (CLI). VS Code: `~/.copilot/instructions/*.instructions.md`, `~/.claude/rules` — configurable via `chat.instructionsFilesLocations`. `$COPILOT_CUSTOM_INSTRUCTIONS_DIRS` | `.github/copilot-instructions.md` — repository-wide. Also reads `AGENTS.md` (**treated as primary instructions**; CLI + VS Code via `chat.useAgentsMdFile`; experimental nested: `chat.useNestedAgentsMdFiles`), `CLAUDE.md`, `GEMINI.md` at root. VS Code additionally reads `.claude/CLAUDE.md` and `CLAUDE.local.md` (workspace) and `~/.claude/CLAUDE.md` (user home) via `chat.useClaudeMdFile`. VS Code also reads a project-level `.claude/rules/` folder (uses a `paths` key instead of `applyTo`) |
| **Path-specific** ★ | — | `.github/instructions/*.instructions.md` — YAML frontmatter `applyTo:` glob; optional `excludeAgent:` key to exclude from `code-review` or `cloud-agent`; searched recursively |
| **Prompt files** ◆ | — | `.github/prompts/*.prompt.md` — invoke via `#prompt:` or `/`; public preview, subject to change; VS Code, Visual Studio, and JetBrains IDEs only — not yet supported in Copilot CLI |
| **Custom agents** ◆ | `~/.copilot/agents/` — user-level agents (CLI/VS Code); `~/.github/agents/` — user-level agents (VS 2026, added April 2026 update) | `.github/agents/*.agent.md` (legacy: `.chatmode.md` files must be renamed to `.agent.md`) — YAML: name, description, tools, model, mcp-servers, target (`vscode`\|`github-copilot`), disable-model-invocation, user-invocable, metadata. (`handoffs`, `agents`, and `argument-hint` are VS Code-only fields; not supported for cloud agents on GitHub.com. `hooks` is **VS Code preview** — requires `chat.useCustomAgentHooks` setting.) `infer` field is **retired**; use `disable-model-invocation` + `user-invocable` instead. `.claude/agents/` — VS Code workspace agents (Claude format). Org: `.github` or `.github-private` repo `agents/` dir; Enterprise-wide: `.github-private` repo of a designated org |
| **Skills** ★ | `~/.copilot/skills/*/SKILL.md`, `~/.agents/skills/*/SKILL.md` (CLI); also `~/.claude/skills/*/SKILL.md` (VS Code only) ¹ | `.github/skills/*/SKILL.md`, `.claude/skills/*/SKILL.md`, **`.agents/skills/*/SKILL.md`** — all three discovered ¹; `gh skill` CLI (GitHub CLI ≥ 2.90.0, public preview) |
| **MCP** ◆ | `~/.copilot/mcp-config.json` — CLI, `mcpServers` key. VS Code: user-profile `mcp.json` via "MCP: Open User Configuration" command, `servers` key | `.vscode/mcp.json` — VS Code project-level MCP, `servers` key (no longer read by Copilot CLI as of v1.0.22). Copilot CLI: `.mcp.json` (project root/cwd, shipped v1.0.22, 2026-04-09) and auto-loaded `.github/mcp.json` (shipped v1.0.61, 2026-06-09), `mcpServers` key — CLI walks cwd → git root loading every match found, closest wins ([github/copilot-cli#2528](https://github.com/github/copilot-cli/issues/2528), closed/shipped). Cloud agent's repo-level MCP config is entered separately as JSON directly via the repo's Settings → Copilot → MCP servers page on GitHub.com — not a committed file. Key name diverges by surface: CLI uses `mcpServers`, VS Code uses `servers` |

**Sources:**
[Custom instructions (CLI)](https://docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/add-custom-instructions) ·
[Custom instructions (VS Code)](https://code.visualstudio.com/docs/agent-customization/custom-instructions) ·
[About agent skills](https://docs.github.com/en/copilot/concepts/agents/about-agent-skills) ·
[Adding skills](https://docs.github.com/en/copilot/how-tos/use-copilot-agents/cloud-agent/create-skills) ·
[Custom agents (cloud)](https://docs.github.com/en/copilot/how-tos/copilot-on-github/customize-copilot/customize-cloud-agent/create-custom-agents) ·
[Custom agents (VS Code)](https://code.visualstudio.com/docs/copilot/customization/custom-agents) ·
[Custom agents config ref](https://docs.github.com/en/copilot/reference/custom-agents-configuration) ·
[Agent skills (VS Code)](https://code.visualstudio.com/docs/copilot/customization/agent-skills) ·
[VS 2026 April update](https://github.blog/changelog/2026-04-30-github-copilot-in-visual-studio-april-update/) ·
[MCP servers (CLI)](https://docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/add-mcp-servers) ·
[MCP servers (repository)](https://docs.github.com/en/copilot/how-tos/copilot-on-github/customize-copilot/configure-mcp-servers)

---

## Cursor — Anysphere

Home: `.cursor/` (project-centric)

| Feature | Global (user) | Project (repo) |
|---|---|---|
| **Instructions** ★ | User Rules (Customize → Rules) — plain text; always applied. Unified "Customize" page (introduced v3.9, Jun 22, 2026) centrally manages plugins/rules/skills/MCP/subagents/commands/hooks at user/team/workspace scope; v3.11 (Jul 10, 2026 — side chats, conversation search, expanded cloud-agent hooks) remains the newest numbered release as of Aug 2026, but frequent unversioned feature drops have continued: Slack integration improvements (Jul 17), Cursor Router auto-mode model routing (Jul 22), Cursor Start pricing tier for India (Jul 28), Cursor for iPad (Jul 29), Google Workspace Plugins — Gmail/Drive/Calendar access for agents (Aug 3); further unversioned drops continued: Grok 4.6 model (Aug 12), Origin code-hosting beta (Aug 17), Cloud Agents/Cursor Harness improvements — `/goal` command, PR/Slack subscriptions, isolated-VM subagents (Aug 19); no new numbered release found through Aug 27, 2026 | `<repo>/AGENTS.md` — root level and subdirectories; plain-markdown alternative to `.cursor/rules`. Legacy: `.cursorrules` (deprecated since ~v0.43; officially deprecated and undocumented but still functional as of Aug 2026 — migrate to `.cursor/rules/`) |
| **Rules** ★ | — | `.cursor/rules/*.mdc` — YAML frontmatter: `alwaysApply`, `description`, `globs`. 4 modes: Always Apply, Apply Intelligently, Apply to Specific Files, Apply Manually. (`.md` files in this directory are ignored; use `.mdc`.) Subdirectory grouping supported. Remote rules imported from GitHub ◆ — land at `.cursor/rules/imported/<repoName>/` (relative paths preserved, e.g. `dir/rule.mdc` → `.cursor/rules/imported/<repoName>/dir/rule.mdc`) via Customize → Rules → Add Rule → Remote Rule (GitHub). Known bug (community-reported, Jan 2026) ○: rules may instead cache at `~/.cursor/projects/<hashed-path>/rules/` (a path hash, not the literal project name) and fail to sync into `.cursor/rules/`; a fix shipped to the nightly channel Apr 2026, stable-channel status unconfirmed |
| **Commands** ◆ | `~/.cursor/commands/*.md` — user-global, all projects; still fully supported and documented (not deprecated) | `.cursor/commands/*.md` — invoke via `/`; filename becomes command name. Still fully supported — official "Agent Best Practices" docs recommend committing these to git. As of v3.9 (Jun 22, 2026) folded into the unified "Customize" page alongside skills/rules/MCP/subagents/hooks (a UI consolidation, not a deprecation). `/migrate-to-skills` (~v2.4) remains available as an *optional* converter for "Apply Intelligently" rules and slash commands into skills |
| **Skills** ◆ | `~/.cursor/skills/*/SKILL.md`, `~/.agents/skills/*/SKILL.md` — primary; `~/.claude/skills/*/SKILL.md`, `~/.codex/skills/*/SKILL.md` — legacy compat ² | `.cursor/skills/*/SKILL.md`, `.agents/skills/*/SKILL.md` — primary; `.claude/skills/*/SKILL.md`, `.codex/skills/*/SKILL.md` — legacy compat ² — agentskills.io; loaded on demand. SKILL.md: `paths` field (current) scopes activation by glob; `globs` is now the legacy alias |
| **Subagents** ◆ | `~/.cursor/agents/*.md` — user-level; compat: `~/.claude/agents/`, `~/.codex/agents/` | Markdown+YAML files in `.cursor/agents/` (project primary); compat: `.claude/agents/`, `.codex/agents/`; `.cursor/` takes precedence on name conflict. Fields: `name`, `description`, `model` (default `inherit`), `readonly`, `is_background`. Background agents write output to `~/.cursor/subagents/`. Parallel (v2.4+); nested tree (v2.5+). `.cursor/worktrees.json` — setup commands; searched in worktree path first, then project root |
| **Hooks** ◆ | `~/.cursor/hooks.json`; hook scripts in `~/.cursor/hooks/`; Enterprise: `/Library/Application Support/Cursor/hooks.json` (macOS) · `/etc/cursor/hooks.json` (Linux) · `C:\ProgramData\Cursor\hooks.json` (Windows) | `.cursor/hooks.json`; hook scripts in `.cursor/hooks/` — camelCase event names, current docs list (exact version each event landed is unconfirmed): `preToolUse`/`postToolUse`/`postToolUseFailure`/`beforeShellExecution`/`afterShellExecution`/`beforeMCPExecution`/`afterMCPExecution`/`beforeReadFile`/`afterFileEdit`/`beforeSubmitPrompt`/`afterAgentResponse`/`afterAgentThought`/`preCompact`/`stop`/`sessionStart`/`sessionEnd`/`subagentStart`/`subagentStop`, plus Tab inline-completion hooks `beforeTabFileRead`/`afterTabFileEdit` and app-lifecycle hook `workspaceOpen` (same config files); `loop_limit` defaults to 5; `failClosed` boolean (default `false`) blocks action on hook failure; supports `type: "prompt"` (LLM-evaluated condition); NOT identical to Claude Code hooks format (PascalCase, unlimited). Priority: Enterprise → Team → Project → User |
| **MCP** ◆ | `~/.cursor/mcp.json` | `.cursor/mcp.json` — `mcpServers` key; supports local, remote, remote+OAuth |
| **Team Rules** ◆ | Managed from Cursor dashboard (Team/Enterprise); pushed to members. Precedence: Team → Project → User | *(same)* |

**Sources:**
[Cursor docs home](https://cursor.com/docs) ·
[Rules](https://cursor.com/docs/rules) ·
[Agent skills](https://cursor.com/docs/context/skills) ·
[Hooks](https://cursor.com/docs/hooks) ·
[MCP](https://cursor.com/docs/mcp) ·
[Worktrees](https://cursor.com/docs/configuration/worktrees) ·
[Subagents](https://cursor.com/docs/subagents) ·
[Best practices (rules + skills)](https://cursor.com/blog/agent-best-practices) ·
[Changelog 2.4 (subagents, skills)](https://cursor.com/changelog/2-4)

---

## Mistral Vibe — Mistral AI

Home: `~/.vibe/` (`$VIBE_HOME`)

> **Note:** Current version: 2.24.4 (August 26, 2026). Core config paths stable since Vibe 2.0, though several features were added after: AGENTS.md parent-folder walking (2.5.0), `.agents/skills/` discovery (2.2.0), `~/.agents/skills` (2.11.0), hooks introduced (2.9.0) then breaking-changed (2.15.0), project-config persistence (2.18.3), hooks graduated from experimental to stable with renamed events (2.21.0, 2026-07-17), full migration to a `ConfigOrchestrator` for config handling (2.22.0), managed-shell/app-server refactor (2.23.0), built-in skill-creator skill (2.23.2), `/retry` and fork-vs-rewind-in-place `/rewind` (2.23.3, 2026-08-03), admin config layer for shared/enforced config overlaying user config (2.24.0, 2026-08-05, no new documented file path), inline ghost-text skill completion and faster resume via deferred sub-agent instantiation (2.24.1, 2026-08-11), in-app session picker/`/log-level`/worktree cleanup/admin-config retry improvements (2.24.2, 2026-08-18), process renamed "Vibe CLI" with PID in status bar plus live slash-command/setting pickers during agent runs (2.24.3, 2026-08-20), LLM-generated session titles/queued-message edit mode/git info in session header/reused model connections/fixed Nix-managed skills symlink loading (2.24.4, 2026-08-26). None of this changes documented config paths.

| Feature | Global (user) | Project (repo) |
|---|---|---|
| **Instructions** ◆ | `~/.vibe/AGENTS.md` (or `$VIBE_HOME/AGENTS.md`) — user-level instruction file; official docs confirm this path | `<repo>/AGENTS.md` — walks cwd upward within trusted folders (official docs confirm path traversal within trusted project directories; single-root workspace recommended) |
| **Config** ◆ | `~/.vibe/config.toml` — fallback | `.vibe/config.toml` — project-local, checked first |
| **System prompts** ◆ | `~/.vibe/prompts/*.md` — set `system_prompt_id` in config.toml (confirmed in docs); `compaction_prompt_id` ◆ (confirmed, added v2.11.1: custom compaction prompts resolved from `~/.vibe/prompts/` or `.vibe/prompts/`) | `.vibe/prompts/*.md` |
| **Skills** ◆ | `~/.vibe/skills/*/SKILL.md` — agentskills.io; invoke via `/`. Custom paths via `skill_paths` in config.toml, plus `enabled_skills`/`disabled_skills`. Discovery order: `skill_paths` → project (`.vibe/skills/`, `.agents/skills/`) → user (`~/.vibe/skills/`). Note: CHANGELOG 2.11.0 states `~/.agents/skills` was added as a *global* path, but the current live docs page no longer lists it at user scope (only at project scope) — discrepancy unresolved, treat `~/.agents/skills/*/SKILL.md` at the global/user level as unconfirmed | `.vibe/skills/*/SKILL.md`, **`.agents/skills/*/SKILL.md`** (trusted folders only) |
| **Agents** ◆ | `~/.vibe/agents/*.toml` — `display_name`, `safety` (safe/neutral/destructive/yolo; display-only), `enabled_tools` | `.vibe/agents/*.toml` — subagents: `agent_type = "subagent"`; user-facing selectable agents: `agent_type = "agent"` |
| **API keys** ◆ | `~/.vibe/.env` — auto-loaded; env vars (e.g. `MISTRAL_API_KEY`) take precedence | — |
| **Hooks** ◆ | `~/.vibe/hooks.toml` — stable since v2.21.0 (2026-07-17, graduated from experimental); current event names `post_agent` (after an assistant turn, was `post_agent_turn`, added v2.9.0), `pre_tool`/`post_tool` (tool-call hooks, can deny calls or rewrite tool inputs; renamed from `before_tool`/`after_tool`, added v2.15.0). `enable_experimental_hooks`/`VIBE_ENABLE_EXPERIMENTAL_HOOKS` gate removed in v2.21.0; hooks now load unconditionally when declared | `.vibe/hooks.toml` — checked before `~/.vibe/hooks.toml` (trusted folders only); same hook name in both, project wins |
| **Trust / misc** ◆ | `~/.vibe/trusted_folders.toml` — trust management (e.g. `trusted = ["~/projects", ...]`); `--trust` CLI flag grants session-only (non-persistent) trust. `~/.vibe/tools/` — custom tools. `~/.vibe/logs/` — session logs (○ sources disagree: official docs list only `logs/`; community/reverse-engineered sources variously claim a sibling `~/.vibe/sessions/` dir or `~/.vibe/logs/session(s)/`, configurable via `[session_logging] save_dir` — unresolved) | — |

**Sources:**
[Configuration](https://docs.mistral.ai/vibe/code/cli/configuration) ·
[Agents](https://docs.mistral.ai/vibe/code/cli/agents) ·
[Skills reference](https://docs.mistral.ai/vibe/code/cli/skills) ·
[Hooks](https://docs.mistral.ai/vibe/code/cli/hooks) ·
[GitHub: mistralai/mistral-vibe](https://github.com/mistralai/mistral-vibe) ·
[PyPI: mistral-vibe](https://pypi.org/project/mistral-vibe/) ·
[Vibe 2.0 announcement](https://mistral.ai/news/mistral-vibe-2-0) ·
[Remote agents + Medium 3.5](https://mistral.ai/news/vibe-remote-agents-mistral-medium-3-5)

---

## OpenCode — Anomaly (open source)

Home: `~/.config/opencode/`

> **Rebrand:** The parent company's official name has always been Anomaly (Anomaly Innovations), previously used the SST name publicly. In 2026 the company consolidated its GitHub presence under `anomalyco`; the repo moved from `sst/opencode` to `anomalyco/opencode` (old URL redirects). Functionality is unaffected.
>
> **V2 beta:** An `opencode2` binary and beta docs tree (`opencode.ai/v2/docs/`) exist alongside stable V1 as of August 2026, with a migration guide at `opencode.ai/v2/docs/migrate-v1`. V2 reportedly drops `CLAUDE.md` instruction-file discovery, consolidates `tui.json` into `cli.json`, and moves skills fully under `.opencode/skills`. The table below documents current stable V1 behavior only.

| Feature | Global (user) | Project (repo) |
|---|---|---|
| **Instructions** ◆ | `~/.config/opencode/AGENTS.md` — personal rules, all sessions. Compat fallback: `~/.claude/CLAUDE.md` ³ | `<repo>/AGENTS.md`, `CLAUDE.md` — walks cwd upward (docs confirm directory traversal but don't state the stopping boundary; commonly assumed git root); at each level AGENTS.md takes precedence over CLAUDE.md if both exist. Extra via `opencode.json` `instructions: […]`; supports remote URLs + globs (5s fetch timeout) |
| **Config** ★ | `~/.config/opencode/opencode.json` (or `.jsonc`). TUI settings: `tui.json` (or `.jsonc`; legacy `tui` key in `opencode.json` deprecated). System/managed: `/etc/opencode/` (Linux) · `/Library/Application Support/opencode/` (macOS) · `%ProgramData%\opencode` (Windows). macOS MDM (highest precedence): `/Library/Managed Preferences/<user>/ai.opencode.managed.plist`, `/Library/Managed Preferences/ai.opencode.managed.plist`. Env overrides: `OPENCODE_CONFIG` (custom file path), `OPENCODE_CONFIG_DIR` (supplemental dir mirroring `.opencode`'s `agents/`/`commands/`/`modes/`/`plugins/` subdirs, loaded after global config + `.opencode` so it can override them — does not cover `opencode.json`/`AGENTS.md`/`skills/`), `OPENCODE_CONFIG_CONTENT` (inline JSON), `OPENCODE_TUI_CONFIG`. Config values support `{env:VAR}` and `{file:path}` interpolation. Precedence (low→high): remote → global `opencode.json` → `OPENCODE_CONFIG` → project `opencode.json` → `.opencode/` dirs → `OPENCODE_CONFIG_CONTENT` → managed files → macOS MDM | `<repo>/opencode.json`. Remote/org config: `.well-known/opencode` (lowest precedence; fetched on provider auth) |
| **Commands** ★ | `~/.config/opencode/commands/*.md` — invoke via `/` in the TUI | `.opencode/commands/*.md` — filename becomes command name |
| **Skills** ★ | `~/.config/opencode/skills/*/SKILL.md`, `~/.claude/skills/*/SKILL.md`, `~/.agents/skills/*/SKILL.md` | `.opencode/skills/*/SKILL.md`, `.claude/skills/*/SKILL.md`, **`.agents/skills/*/SKILL.md`** — walks cwd→git root. Claude compat discovery gated by `OPENCODE_DISABLE_CLAUDE_CODE_SKILLS` / `OPENCODE_DISABLE_CLAUDE_CODE` |
| **Modes** ○ | *(deprecated)* `~/.config/opencode/modes/*.md` — no official docs page remains (404); folded into the Agents row's `mode` field (`primary`\|`subagent`\|`all`) | *(deprecated)* `.opencode/modes/*.md` — same; may still load for backward compatibility but is no longer independently documented |
| **Agents** ★ | `~/.config/opencode/agents/*.md` — YAML: description, model, temperature, top_p, mode (`primary`\|`subagent`\|`all`), permission, color, steps, disable, hidden | `.opencode/agents/*.md` — primary (Tab) or subagent (@ invoke) |

**Sources:**
[Rules (AGENTS.md)](https://opencode.ai/docs/rules/) ·
[Agent Skills](https://opencode.ai/docs/skills/) ·
[Agents](https://opencode.ai/docs/agents/) ·
[Config](https://opencode.ai/docs/config/) ·
[Commands](https://opencode.ai/docs/commands/) ·
[Getting started](https://opencode.ai/docs/)

---

## Pi — earendil-works (open source)

Home: `~/.pi/` (agent config under `~/.pi/agent/`)

> **Philosophy:** Pi is a minimal terminal harness — "primitives, not features." It deliberately ships **no built-in MCP, subagents, or plan mode**; those are added as TypeScript extensions. Distributed on npm as `@earendil-works/pi-coding-agent` (migrated from `@mariozechner/pi-coding-agent` at v0.74.0 — old package deprecated at v0.73.1, new repo at `earendil-works/pi`). Current version: 0.84.3 (npm, released ~2026-08-25). Core paths stable since the v0.74.0 rename; `trust.json` added v0.79.0.

| Feature | Global (user) | Project (repo) |
|---|---|---|
| **Instructions** ◆ | `~/.pi/agent/AGENTS.md` — also reads `CLAUDE.md`; all discovered files concatenated | `<repo>/AGENTS.md`, `.pi/AGENTS.md` — loaded from parent dirs + cwd at startup |
| **System prompt** ◆ | `~/.pi/agent/SYSTEM.md` (replace), `~/.pi/agent/APPEND_SYSTEM.md` (append) | `.pi/SYSTEM.md` (replace), `.pi/APPEND_SYSTEM.md` (append) — per-project |
| **Settings** ◆ | `~/.pi/agent/settings.json`; `~/.pi/agent/keybindings.json` — keyboard shortcuts; `~/.pi/agent/trust.json` — saved project trust decisions (v0.79.0) | `.pi/settings.json` — project overrides global |
| **Skills** ◆ | `~/.pi/agent/skills/*/SKILL.md`, `~/.agents/skills/*/SKILL.md` — bare root `.md` files count as skills in `~/.pi/agent/skills/` and `.pi/skills/` only (not in `.agents/skills/` locations); `settings.json` `skills: []` can add other paths (e.g. `~/.claude/skills`) | `.pi/skills/`, **`.agents/skills/*/SKILL.md`** — walks cwd→git root; `/skill:<name>` to invoke. Skill name need not match directory |
| **Prompt templates** ◆ | `~/.pi/agent/prompts/*.md` → `/<name>` | `.pi/prompts/*.md` |
| **Models** ◆ | `~/.pi/agent/models.json` — custom providers (Ollama, vLLM, LM Studio, proxies) | — |
| **Extensions** ◆ | `~/.pi/agent/extensions/*.ts` — TypeScript modules; how MCP, subagents, hooks, plan mode etc. are added | `.pi/extensions/` — auto-discovered |
| **MCP / Subagents** ○ | Not built in — provided via extensions. `~/.pi/agent/mcp.json` is a **proposed community extension convention** (unshipped; native MCP/ACP support is still only under discussion) | `.pi/mcp.json` — proposed project-level convention, a distinct path from the global one, not the same file |
| **Packages** ◆ | npm/git bundles declared via a `pi` key in `package.json` (`extensions`, `skills`, `prompts`, `themes`) | *(same)* |

**Sources:**
[Pi home](https://pi.dev/) ·
[Migration announcement](https://pi.dev/news/2026/5/7/pi-has-a-new-home) ·
[Skills doc](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/skills.md) ·
[README](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/README.md) ·
[npm: @earendil-works/pi-coding-agent](https://www.npmjs.com/package/@earendil-works/pi-coding-agent)

---

## Aider — open source

Home: no dedicated directory for user-editable config (`~/.aider.conf.yml`, `.env` sit directly in the home dir), but `~/.aider/` holds non-config state — `oauth-keys.env`, `caches/` (model pricing/context-window cache), `analytics.json` (opt-in anonymous analytics)

| Feature | Global (user) | Project (repo) |
|---|---|---|
| **Instructions / conventions** ◆ | Any file passed with `--read`; persist it via `read:` in `~/.aider.conf.yml` | Convention files such as `CONVENTIONS.md`, loaded with `/read` or `--read`; persist via `read:` in the repo's `.aider.conf.yml` |
| **Config** ◆ | `~/.aider.conf.yml`; environment variables `AIDER_*`; optional `.env` | `<git-root>/.aider.conf.yml`, `<cwd>/.aider.conf.yml`; `.env` follows the same three-location search order as `.aider.conf.yml` (home → git root → cwd, last-loaded wins), or override with `--env-file`. CLI flags override file settings |
| **Model config** ◆ | Paths selected by `model-settings-file` and `model-metadata-file`; commonly `.aider.model.settings.yml` and `.aider.model.metadata.json` | Same keys can point to repo-local files |
| **History** ◆ | — | `.aider.chat.history.md` and `.aider.input.history` by default; filenames are configurable |
| **Skills / subagents / MCP** ◆ | No native Agent Skills, subagent-definition, or MCP configuration convention documented (MCP requested via open issue/RFC #4506; a PR adding it was closed unmerged) | *(same)* |

**Sources:**
[Configuration](https://aider.chat/docs/config.html) ·
[YAML config reference](https://aider.chat/docs/config/aider_conf.html) ·
[Coding conventions](https://aider.chat/docs/usage/conventions.html)

---

## Cline — Cline Bot Inc.

Home: `~/.cline/` (CLI/SDK/Kanban; IDE integrations also use platform storage)

| Feature | Global (user) | Project (repo) |
|---|---|---|
| **Instructions / rules** ◆ | `~/.cline/rules/`, `~/.agents/AGENTS.md`; compatibility search path `~/Documents/Cline/Rules/` (Linux may also use `~/Cline/Rules/`) | `.cline/rules/` and primary legacy-compatible `.clinerules/`; also reads `AGENTS.md`, `.cursorrules`, and `.windsurfrules`. Conditional rules use `paths:` YAML frontmatter |
| **Settings** ◆ | `~/.cline/data/settings/providers.json`, `global-settings.json`, and `cline_mcp_settings.json`; `CLINE_DATA_DIR` replaces `~/.cline/data/` | Project behavior is stored in `.cline/` subdirectories; secrets stay in global provider settings |
| **Skills** ◆ | `~/.cline/skills/*/SKILL.md` | `.cline/skills/*/SKILL.md` (recommended), `.clinerules/skills/`, `.claude/skills/`; progressive disclosure (shipped v3.48.0, Jan 2026). Made always-on with the Settings → Features → Enable Skills toggle removed entirely in a single v3.57.0 change (~2026-02-05) — skills are now always on, with only per-skill toggles. (v3.56.0, ~2026-01-30, was unrelated to skills — it shipped GPT-5 OAuth and Jupyter notebook support.) A v4.x line (Customize marketplace) has since shipped, current ~v4.1.16 (~2026-08-26) |
| **Agents** ◆ | `~/.cline/agents/` | `.cline/agents/` |
| **Hooks** ◆ | `~/.cline/hooks/`; compatibility `~/Documents/Cline/Hooks/` | `.cline/hooks/` and `.clinerules/hooks/`; lifecycle scripts receive/return JSON |
| **Workflows / plugins** ◆ | `~/.cline/data/workflows/`, `~/.cline/plugins/`; compatibility under `~/Documents/Cline/` | `.cline/plugins/`; project workflows are supported by Cline's configuration system |
| **MCP** ◆ | `~/.cline/data/settings/cline_mcp_settings.json` (per the config/storage overview); the CLI-specific MCP doc instead cites `~/.cline/mcp.json` — the two official pages disagree on the canonical CLI path, worth confirming against an actual install | Managed through Cline settings; no separate committed project MCP filename documented |

**Sources:**
[Configuration and storage](https://docs.cline.bot/getting-started/config) ·
[Rules](https://docs.cline.bot/customization/cline-rules) ·
[Skills](https://docs.cline.bot/customization/skills) ·
[Hooks](https://docs.cline.bot/customization/hooks) ·
[MCP](https://docs.cline.bot/mcp/mcp-overview)

---

## Continue — Continue Dev, Inc.

Home: `~/.continue/`

> **Acquired / discontinued:** Continue.dev was acquired by Cursor (announced ~June 16, 2026). The `continuedev/continue` GitHub repo is now read-only and no longer actively maintained, with v2.0.0 (shipped 2026-06-19) as the final release. Continue's cloud data (conversation history, saved configs, team settings) is scheduled for deletion after 2026-07-15. Local extension/CLI config on users' own machines is unaffected, so the table below still describes the final shipped behavior.

| Feature | Global (user) | Project (repo) |
|---|---|---|
| **Config / agents** ◆ | `~/.continue/config.yaml` — models, context, rules, prompts, docs, and MCP servers; `config.json` and `config.ts` are legacy | Local blocks under `.continue/`; the CLI can load any YAML config with `--config <path>` |
| **Rules** ◆ | Rules can be embedded or referenced from `config.yaml` | `.continue/rules/*.md` (recommended; YAML is also accepted); optional `globs` frontmatter |
| **Skills** ○ | Not documented for global scope | `.continue/skills/*/SKILL.md` or `.claude/skills/*/SKILL.md` — YAML frontmatter + Markdown body, retrieved via a `read_skill` tool (merged [PR #9353](https://github.com/continuedev/continue/pull/9353), 2026-01-15); never made it into official docs before the product wound down, so sourced to the PR rather than docs.continue.dev |
| **Prompts** ◆ | Declared or referenced under `prompts:` in `config.yaml` | Local prompt blocks can be referenced from workspace configuration and invoked as slash commands |
| **Models / tools** ◆ | `~/.continue/config.yaml`; reusable global blocks | `.continue/mcpServers/`; no dedicated `.continue/models/` directory documented — project-level models are set via `config.yaml` blocks instead; workspace blocks apply to all configs |
| **MCP** ◆ | `mcpServers:` in `~/.continue/config.yaml` | `.continue/mcpServers/*.yaml`; MCP tools are available in Agent mode |
| **Secrets / permissions** ◆ | `~/.continue/.env`; Continue CLI decisions in `~/.continue/permissions.yaml` | `<repo>/.env` or `.continue/.env`; workspace secrets take precedence |

**Sources:**
[Configuration](https://docs.continue.dev/cli/configuration) ·
[`config.yaml` reference](https://docs.continue.dev/reference) ·
[Models, rules, and tools](https://docs.continue.dev/guides/configuring-models-rules-tools) ·
[Rules](https://docs.continue.dev/customize/rules) ·
[MCP examples](https://docs.continue.dev/customize/deep-dives/mcp-examples)

---

## Devin — Cognition

Home: cloud-managed; configuration is primarily organization- and repository-scoped in the Devin web app

| Feature | Global (user / organization) | Project (repo) |
|---|---|---|
| **Instructions / knowledge** ◆ | Knowledge and Enterprise Knowledge are managed in Settings & Library; items can be pinned to all repos, one repo, or retrieved by triggers | A centralized specialized file such as root `AGENTS.md` is recommended; Devin also auto-pulls updates from specialized files including `.rules`, `.mdc`, `.cursorrules`, `.windsurf`, `CLAUDE.md`, and `AGENTS.md` into repo knowledge |
| **Skills** ◆ | — | `.agents/skills/*/SKILL.md` (recommended); also discovered: `.devin/skills/`, `.github/skills/`, `.claude/skills/`, `.cursor/skills/`, `.codex/skills/`, `.cognition/skills/`, `.windsurf/skills/`, `.codeium/skills/` — nine paths scanned in every repo; discovered automatically or invoked with `@skills:<name>` |
| **Playbooks** ◆ | Managed in the Devin web app; reusable organization- or enterprise-scoped prompts attached manually to sessions | — |
| **Environment** ◆ | Environment snapshots, secrets, and repository access are managed in Devin settings | Declarative environment blueprints are version-controlled YAML; `.envrc` can provide repo environment variables and should normally be gitignored |
| **MCP** ◆ | Settings → Connections → MCP servers (page titled "MCP Marketplace"); custom stdio, SSE, and HTTP servers entered through a web form, gated by the "Manage MCP Servers" permission | No committed repo-level MCP file documented |

**Sources:**
[Knowledge](https://docs.devin.ai/product-guides/knowledge) ·
[Knowledge onboarding](https://docs.devin.ai/onboard-devin/knowledge-onboarding) ·
[Skills](https://docs.devin.ai/product-guides/skills) ·
[Environment configuration](https://docs.devin.ai/onboard-devin/environment) ·
[MCP Marketplace](https://docs.devin.ai/work-with-devin/mcp)

---

## Gemini CLI — Google

Home: `~/.gemini/`

> **Sunsetting for individual users:** Per Google's Developers Blog ("An important update: Transitioning Gemini CLI to Antigravity CLI"), Gemini CLI stopped serving free/Pro/Ultra individual-user requests on 2026-06-18, superseded by the closed-source, Go-based Antigravity CLI (which reportedly retains Skills, Hooks, Subagents, and Extensions concepts). Only Gemini Code Assist Standard/Enterprise/Google Cloud licensees retain access. The geminicli.com docs remain live and technically accurate as of today, so the table below still reflects current behavior for licensed users.

| Feature | Global (user) | Project (repo) |
|---|---|---|
| **Instructions** ◆ | `~/.gemini/GEMINI.md` | Hierarchical `GEMINI.md`; filename(s) configurable via `context.fileName`, enabling `AGENTS.md`. Supports `@file` imports |
| **Settings** ◆ | `~/.gemini/settings.json`; system defaults/overrides under `/etc/gemini-cli/` (Linux), `/Library/Application Support/GeminiCli/` (macOS), or `C:\ProgramData\gemini-cli\` (Windows) | `.gemini/settings.json`; project settings override user settings but are overridden by system policy, environment, and CLI arguments |
| **Skills** ◆ | `~/.gemini/skills/*/SKILL.md`, `~/.agents/skills/*/SKILL.md` | `.gemini/skills/*/SKILL.md`, `.agents/skills/*/SKILL.md`; workspace skills require trust; cross-agent alias wins within a scope |
| **Subagents** ◆ | `~/.gemini/agents/*.md` | `.gemini/agents/*.md` — Markdown + YAML; local or A2A remote agents; subagents cannot recursively call subagents |
| **Commands** ◆ | `~/.gemini/commands/*.toml` | `.gemini/commands/*.toml`; project commands override user commands; subdirectories create `:` namespaces |
| **Hooks** ◆ | `hooks` object in `~/.gemini/settings.json` | `hooks` in `.gemini/settings.json`; extension hooks are lowest precedence |
| **MCP** ◆ | `mcpServers` in `~/.gemini/settings.json` | `mcpServers` in `.gemini/settings.json`; agent definitions may contain isolated inline MCP servers |
| **Extensions** ◆ | `~/.gemini/extensions/<name>/gemini-extension.json` | Extensions can bundle context, commands, skills, hooks, themes, and MCP servers; workspace settings override conflicts |

**Sources:**
[Configuration](https://geminicli.com/docs/reference/configuration/) ·
[`GEMINI.md`](https://geminicli.com/docs/cli/gemini-md/) ·
[Agent Skills](https://geminicli.com/docs/cli/skills/) ·
[Subagents](https://geminicli.com/docs/core/subagents/) ·
[Custom commands](https://geminicli.com/docs/cli/custom-commands/) ·
[Hooks](https://geminicli.com/docs/hooks/) ·
[MCP](https://geminicli.com/docs/tools/mcp-server/) ·
[Extensions](https://geminicli.com/docs/extensions/reference/)

---

## Goose — Agentic AI Foundation

Home: `~/.config/goose/` (macOS/Linux) · `%APPDATA%\Block\goose\config\` (Windows)

| Feature | Global (user) | Project (repo) |
|---|---|---|
| **Instructions** ◆ | `~/.config/goose/.goosehints` is the only documented global hints file (no home-level `AGENTS.md` path is documented) | Hierarchical `.goosehints` and `AGENTS.md` from cwd to git root and nested directories, controlled by default `CONTEXT_FILE_NAMES` (`["AGENTS.md", ".goosehints"]`); `CONTEXT_FILE_NAMES` adds alternatives |
| **Config** ◆ | `~/.config/goose/config.yaml`; `permission.yaml`, `secrets.yaml`, and `permissions/tool_permissions.json` alongside it | Environment variables override global config; no separate project config file documented |
| **Skills** ◆ | `~/.agents/skills/*/SKILL.md` (recommended standard); backward-compatible discovery also checks `~/.claude/skills/*/SKILL.md` and other platform-specific config dirs (`~/.config/goose/skills/` is not a documented path) | `.agents/skills/*/SKILL.md` (recommended standard), `.goose/skills/*/SKILL.md`, `.claude/skills/*/SKILL.md`; docs do not state an explicit precedence rule between project and global paths on name conflict |
| **Recipes / commands** ◆ | `~/.config/goose/recipes/*.{yaml,json}`; recipe-backed slash commands configured in `config.yaml` | `.goose/recipes/*.{yaml,json}` |
| **MCP / extensions** ◆ | `extensions:` in `~/.config/goose/config.yaml`; built-in and stdio MCP extensions | Recipes can declare required extensions; no separate repo MCP file documented |
| **Prompts / permissions** ◆ | `~/.config/goose/prompts/`, `permission.yaml`, and runtime permission decisions | Project hints and recipes provide committed behavior |

**Sources:**
[Configuration files](https://github.com/aaif-goose/goose/blob/main/documentation/docs/guides/config-files.md) ·
[Project hints](https://github.com/aaif-goose/goose/blob/main/documentation/docs/guides/context-engineering/using-goosehints.md) ·
[Recipes](https://github.com/aaif-goose/goose/blob/main/documentation/docs/guides/recipes/storing-recipes.md) ·
[Goose repository](https://github.com/aaif-goose/goose)

---

## Hermes Agent — Nous Research

Home: `~/.hermes/` (profiles may use an alternate home)

| Feature | Global (user) | Project (repo) |
|---|---|---|
| **Instructions / identity** ◆ | `~/.hermes/SOUL.md` — primary identity (system-prompt slot #1) | Additional context files (`.hermes.md`, `AGENTS.md`, `CLAUDE.md`, `.cursorrules`) are all injected into the system prompt together, subject to `context_file_max_chars` truncation — no documented first-match-wins precedence |
| **Config / secrets** ◆ | `~/.hermes/config.yaml`, `.env`, `auth.json`; precedence: CLI → config.yaml → .env → defaults | No separate project configuration file documented; working directory and backend are selected globally or per invocation |
| **Skills** ◆ | `~/.hermes/skills/*/SKILL.md` — primary source of truth; external directories configurable | Plans created by the bundled plan skill land under `.hermes/plans/`; project-local skills (`.hermes/skills/`, `.agents/skills/`) are discovered but require explicit `hermes skills trust` before loading — not a default (auto-load) path |
| **Memory** ◆ | `~/.hermes/memories/MEMORY.md` and `USER.md`; optional external memory providers configured in `config.yaml` | Cross-session search and sessions are stored under `~/.hermes/`; no committed project memory convention documented |
| **MCP** ◆ | `mcp_servers:` in `~/.hermes/config.yaml`; catalog installations are managed with `hermes mcp` | No separate project MCP file documented |
| **Automation** ◆ | `~/.hermes/cron/`, sessions, logs, messaging gateway config, and tool settings | Workspace artifacts created by skills may live under `.hermes/` |

**Sources:**
[Configuration](https://hermes-agent.nousresearch.com/docs/user-guide/configuration/) ·
[Context and identity](https://hermes-agent.nousresearch.com/docs/user-guide/configuration/#personality--soulmd) ·
[Skills](https://hermes-agent.nousresearch.com/docs/user-guide/features/skills/) ·
[Memory](https://hermes-agent.nousresearch.com/docs/user-guide/features/memory/) ·
[MCP](https://hermes-agent.nousresearch.com/docs/user-guide/features/mcp/)

---

## Kiro — Amazon Web Services

Home: `~/.kiro/`

| Feature | Global (user) | Project (repo) |
|---|---|---|
| **Instructions / steering** ◆ | `~/.kiro/steering/*.md`; global `AGENTS.md` is supported there | `.kiro/steering/*.md`; `AGENTS.md` is discovered anywhere in the workspace tree, not just root (since CLI v2.18.0, 2026-08-12) — previously root-only; steering modes include always, automatic, file-match, and manual |
| **Settings** ◆ | `~/.kiro/settings/cli.json` | Project configuration is stored under `.kiro/`; no project `cli.json` documented |
| **Skills** ◆ | `~/.kiro/skills/*/SKILL.md` | `.kiro/skills/*/SKILL.md`; workspace skill wins on name conflict; Agent Skills standard |
| **Custom agents** ◆ | `~/.kiro/agents/` | `.kiro/agents/`; agent configuration can embed MCP servers, hooks, tools, permissions, and steering resources |
| **Prompts** ◆ | `~/.kiro/prompts/` | `.kiro/prompts/`; project prompts override global prompts |
| **MCP** ◆ | `~/.kiro/settings/mcp.json` | `.kiro/settings/mcp.json`; resolution: Agent > Project > Global |
| **Specs / hooks** ◆ | `~/.kiro/hooks/` — global hooks (CLI v3 preview, added v2.13.0, 2026-07-17): define once, apply across all workspaces; hooks may also be embedded in custom-agent configuration | `.kiro/specs/` and `.kiro/hooks/`; IDE and CLI support lifecycle/tool hooks, with the current CLI agent schema also allowing inline hooks |

**Sources:**
[CLI configuration](https://kiro.dev/docs/configuration/) ·
[Agent configuration](https://kiro.dev/docs/custom-agents/configuration-reference/) ·
[Agent Skills](https://kiro.dev/docs/skills/) ·
[Steering](https://kiro.dev/docs/steering/) ·
[Getting started](https://kiro.dev/docs/getting-started/first-project/) ·
[Changelog: global hooks (v2.13)](https://kiro.dev/changelog/cli/2-13/) ·
[Changelog: AGENTS.md workspace-wide (v2.18)](https://kiro.dev/changelog/cli/2-18/)

---

## OpenHands — OpenHands / All Hands AI

Home: `~/.openhands/` for local state; cloud settings are managed in the UI

| Feature | Global (user) | Project (repo) |
|---|---|---|
| **Instructions** ◆ | User skills under `~/.agents/skills/` | Root `AGENTS.md` (recommended); `GEMINI.md` and `CLAUDE.md` supported as model-specific permanent context |
| **Settings** ◆ | CLI: `~/.openhands/agent_settings.json` (LLM/agent config), `cli_config.json` (CLI/TUI preferences); `settings.json` is legacy pre-1.0 naming — reconfiguration is required on upgrade. State root controlled by `OH_PERSISTENCE_DIR` | `.openhands/setup.sh` runs when work begins; `.openhands/hooks.json` customizes lifecycle/tool execution |
| **Skills** ◆ | `~/.agents/skills/*/SKILL.md`; deprecated `~/.openhands/skills/` and `microagents/` fallbacks | `.agents/skills/*/SKILL.md` (recommended), including legacy `.md` skills; deprecated `.openhands/skills/` and `.openhands/microagents/` |
| **Agents** ◆ | `~/.agents/agents/*.md`, fallback `~/.openhands/agents/*.md` | `.agents/agents/*.md`, fallback `.openhands/agents/*.md`; project definitions take precedence |
| **MCP** ◆ | `~/.openhands/mcp.json` for CLI; cloud/local GUI settings through the UI | No separate committed project MCP filename documented |
| **History / persistence** ◆ | `~/.openhands/conversations/`; `OH_PERSISTENCE_DIR` changes the state root | Repository setup and hooks are committed under `.openhands/` |

**Sources:**
[CLI installation and settings](https://docs.openhands.dev/openhands/usage/cli/installation) ·
[CLI command reference](https://docs.openhands.dev/openhands/usage/cli/command-reference) ·
[Skills overview](https://docs.openhands.dev/overview/skills) ·
[File-based agents](https://docs.openhands.dev/sdk/guides/agent-file-based) ·
[Repository customization](https://docs.openhands.dev/openhands/usage/customization/repository) ·
[MCP](https://docs.openhands.dev/openhands/usage/cli/mcp-servers) ·
[Configuration](https://docs.openhands.dev/openhands/usage/advanced/configuration-options)

---

## Windsurf / Devin Desktop — Cognition

Home: `~/.codeium/windsurf/` (legacy Codeium/Windsurf paths; still read by the legacy Cascade agent bundled in Devin Desktop, not by Devin Local)

> **Rebrand complete:** Windsurf was renamed **Devin Desktop** (announced 2026-06-02) — same editor, same features, unified under the Devin brand. Its local agent, Cascade, remained available through July 2026 (per the official FAQ; no exact end-of-life day is published) before being replaced as the default local agent by **Devin Local** (a Rust rewrite, ~30% better token efficiency, adds subagent support, and supports the Agent Client Protocol at launch, running Codex/Claude Agent/OpenCode etc.). Devin Desktop also acts as a command center for Devin Cloud sessions, which remain a separate SKU (see the Devin section above). Legacy `~/.codeium/windsurf/` and `.windsurf/` paths are still read for backward compatibility.

| Feature | Global (user) | Project (repo) |
|---|---|---|
| **Instructions / rules** ◆ | `~/.codeium/windsurf/memories/global_rules.md`; system rules use OS-level `Devin/rules/` with `Windsurf/rules/` fallback | `.devin/rules/*.md` (preferred), `.windsurf/rules/*.md` fallback, legacy `.windsurfrules`; hierarchical `AGENTS.md` is supported |
| **Memory** ◆ | Workspace-scoped generated memories are stored under `~/.codeium/windsurf/memories/` | Memories are local and not committed; durable team context should use rules or `AGENTS.md` |
| **Skills** ◆ | `~/.codeium/windsurf/skills/*/SKILL.md`, `~/.agents/skills/*/SKILL.md` | `.windsurf/skills/*/SKILL.md`, `.agents/skills/*/SKILL.md`; optional Claude-compatible discovery |
| **Workflows** ◆ | `~/.codeium/windsurf/global_workflows/*.md` | `.windsurf/workflows/*.md`; manual `/name` invocation; discovered through subdirectories and parents to git root |
| **Hooks** ◆ | `~/.codeium/windsurf/hooks.json`; JetBrains plugin: `~/.codeium/hooks.json` | `.windsurf/hooks.json`; system → user → workspace merge order; pre-hooks can block with exit code 2 |
| **MCP** ◆ | `~/.codeium/windsurf/mcp_config.json` (legacy Cascade path, still read); Devin Local adds a user-level `~/.config/devin/mcp_config.json` (since the Local 3.6 / v3000.3 release, 2026-07-29; older `mcpServers` entries in `config.json` are auto-migrated); MCP Marketplace and enterprise allowlists | Devin Local uses dedicated MCP config files: `.devin/mcp_config.json` (team-shared, committed) and `.devin/mcp_config.local.json` (personal/gitignored) — superseding the earlier combined `.devin/config.json`/`.devin/config.local.json` |

**Sources:**
[Memories and rules](https://docs.devin.ai/desktop/cascade/memories) ·
[Skills](https://docs.devin.ai/desktop/cascade/skills) ·
[Workflows](https://docs.devin.ai/desktop/cascade/workflows) ·
[Hooks](https://docs.devin.ai/desktop/cascade/hooks) ·
[MCP](https://docs.devin.ai/desktop/cascade/mcp) ·
[AGENTS.md](https://docs.devin.ai/desktop/cascade/agents-md) ·
[Devin Local](https://docs.devin.ai/desktop/devin-local) ·
[Devin Desktop FAQ](https://docs.devin.ai/desktop/devin-desktop-faq) ·
[CLI extensibility / MCP config (Local 3.6)](https://docs.devin.ai/cli/extensibility/configuration)

---

## Notes

¹ VS 2026 and VS Code discover `.github/skills/`, `.claude/skills/`, and `.agents/skills/` at project scope; Copilot CLI discovers the same three project paths. User-scope: VS Code and VS 2026 discover `~/.copilot/skills/`, `~/.claude/skills/`, and `~/.agents/skills/`; Copilot CLI discovers only `~/.copilot/skills/` and `~/.agents/skills/` (not `~/.claude/skills/`). See [VS 2026 April update](https://github.blog/changelog/2026-04-30-github-copilot-in-visual-studio-april-update/).

² Cursor's official docs now label `.claude/skills/`, `~/.claude/skills/`, `.codex/skills/`, and `~/.codex/skills/` as legacy backward-compatibility paths. Primary locations are `.cursor/skills/` and `.agents/skills/` (project) and `~/.cursor/skills/` and `~/.agents/skills/` (user). See [Cursor agent skills docs](https://cursor.com/docs/context/skills).

³ OpenCode Claude compat can be disabled granularly: `OPENCODE_DISABLE_CLAUDE_CODE_PROMPT=1` (disables `~/.claude/CLAUDE.md` fallback), `OPENCODE_DISABLE_CLAUDE_CODE_SKILLS=1` (disables `.claude/skills/` discovery), `OPENCODE_DISABLE_CLAUDE_CODE=1` (all `.claude/` support). The previously documented `OPENCODE_DISABLE_CLAUDE_COMPAT=1` and `OPENCODE_DISABLE_EXTERNAL_SKILLS=1` are **not valid variables**. See [OpenCode rules docs](https://opencode.ai/docs/rules/).

⁴ In Claude Code, slash commands and skills have converged — a skill `deploy` and a command file `deploy.md` both produce `/deploy`. Existing `.claude/commands/` files keep working unchanged.

### Cross-agent standards

- **`.agents/skills/`** — The cross-agent convention from the [agentskills.io client implementation guide](https://agentskills.io/client-implementation/adding-skills-support), described there as "a widely-adopted convention for cross-client skill sharing" that makes skills "automatically visible" across compliant clients. The guide instructs all compliant clients to scan both their own native directory and `.agents/skills/`. Scanned by Codex, Copilot, OpenCode, Gemini CLI, Cursor, Vibe, and Pi among others. (A single canonical skill tree shared via symlinks is a natural consequence of this convention, though not stated verbatim by the spec.)

- **`AGENTS.md`** — Open standard ([agents.md](https://agents.md)), originated from collaborative efforts by OpenAI Codex, Amp, Jules (Google), Cursor, and Factory (August 2025); contributed by OpenAI and now stewarded by the **Agentic AI Foundation (AAIF)** under the Linux Foundation (founded December 2025). Tools listed on agents.md as of July 2026 (23 on main page; additional tools via 'View all supported agents' link): Codex (OpenAI), Jules (Google), Factory, Aider, goose, OpenCode, Zed, Warp, VS Code, Devin (Cognition), UiPath, Junie (JetBrains), Amp, Cursor, RooCode, Gemini CLI, Kilo Code, Phoenix, Semgrep, GitHub Copilot, Ona, Windsurf (Cognition), Augment Code. Over 60,000 open-source projects use AGENTS.md. (Note: Pi and Mistral Vibe support AGENTS.md but are not listed on agents.md.)

- **`SKILL.md`** — [Agent Skills spec](https://agentskills.io/specification). No explicit version number in the spec itself — there is no top-level `version:` frontmatter field; skill package versioning goes under the `metadata:` map (e.g., `metadata:\n  version: "1.0"`). Originally developed by Anthropic (released 2025-12-18); now hosted at the separate `agentskills` org at [github.com/agentskills/agentskills](https://github.com/agentskills/agentskills) (Apache 2.0 code / CC-BY-4.0 docs), open to community contribution; long-term governance — e.g. whether it folds under the AAIF — remains unsettled as of mid-2026. Structure: `skill-name/{SKILL.md, scripts/, references/, assets/}`. `allowed-tools` frontmatter field is Experimental. Adopted by **46 tools** on the live showcase at [agentskills.io/clients](https://agentskills.io/clients) (note: the path is `/clients`, not `/showcase`, which 404s; list continues to grow).

- **Subagent format split:** Claude Code, Copilot (`.agent.md`), OpenCode, and Cursor define agents as Markdown + YAML frontmatter. Codex uses Markdown+YAML for Agent Skills; TOML is used only for Codex's own internal subagent profile definitions (`.codex/agents/*.toml`). Vibe uses TOML for its internal subagent config profiles and Markdown+YAML for SKILL.md skills.

All paths use Unix notation; `~` = `%USERPROFILE%` on Windows. `$CODEX_HOME`, `$VIBE_HOME` override their respective defaults. `PI_CODING_AGENT_DIR` overrides `~/.pi/agent/`.

### slopctl agent defaults

slopctl keeps the agent filesystem conventions from this document in `agent-defaults.yml`, stored next to `templates.yml` in the global template cache. Use `slopctl agents --update` to refresh agent prompt, skill, marker, and cross-client-skill defaults independently from templates. `slopctl templates --update` bootstraps this file only when it is missing. Agent markers are workspace-relative directories that `slopctl init --agent` can safely create for detection; slopctl does not create agent config files such as `opencode.json`.

---

*Last verified: 2026-08-27*
