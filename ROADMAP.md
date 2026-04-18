# slopctl Roadmap

**Last updated:** 2026-04-10

This document indexes planned work for slopctl. Each plan originated
in a Cursor chat session; the links below point to the original
transcripts so future sessions can pick up context quickly.

---

## Plan 2 — AI-Powered Features (`merge`, `--smart`)

**Status:** `merge` command implemented in v13.1.0; `--smart` features not started
**Origin:** [AI merge discussion](47951d93-1cd8-4722-81c0-a1389d0b6a0d)

Three AI-assisted capabilities:

1. ~~**`merge` command**~~ — **Done (v13.1.0)**: LLM-assisted merge of
   customized files with updated templates
   - Providers: OpenAI, Anthropic, Ollama, Mistral
   - Config keys: `merge.provider`, `merge.model`
   - CLI flags: `--provider`, `--model`
   - Writes `.merged` sidecar files; skips if sidecar already exists
   - `--dry-run` shows what would be merged without calling an API
2. **`init --smart`** — AI auto-fill of mission statement and
   project-specific sections during initial install
3. **`doctor --smart`** — AI-powered linting of instruction files
   (contradictions, stale references, unclear instructions)

All features must remain language- and agent-agnostic (but may use the
`--lang` hint when provided).

---

## Future Considerations

- **Agent-agnostic config/subagent support**: When adding support for
  agent configuration files (e.g. `.codex/config.toml`) or custom
  subagents (e.g. `.codex/agents/*.toml`), design them as agent-agnostic
  features rather than Codex-specific fields. Cursor, Claude Code, and
  Copilot have their own emerging patterns; a good abstraction should
  cover all of them uniformly.

---

## Completed

| Version | Item | Date |
|---------|------|------|
| v13.1.0 | AI-assisted `merge` command with LLM provider abstraction (OpenAI, Anthropic, Ollama, Mistral) | 2026-04-10 |
| v13.0.0 | Rename `install` to `init`, Codex template cleanup, `merge` skeleton, Session Protocol | 2026-04-10 |
| v12.4.0 | `templates` command (replaces `update`), `status` (replaces `list`) | 2026-04-10 |
