# slopctl Roadmap

**Last updated:** 2026-07-22

Forward-looking plan for slopctl: work that is planned or under
consideration but not yet shipped. This file is intentionally short.

Completed and released changes are recorded chronologically in
[UPDATES.md](UPDATES.md) — the append-only "Recent Updates & Decisions"
log maintained per the `recent-updates` skill. The roadmap does **not**
duplicate that history; delegating it there is what keeps this file
usable instead of perpetually stale.

---

## Open items

### Third-party license manifest (optional)

Generate a transitive dependency license manifest in CI (e.g. via
`cargo-about`) and attach it as a release asset such as
`THIRD-PARTY-LICENSES.html` for SBOM-style review by downstream legal and
security teams.

- **Status:** open, optional. The MIT `LICENSE` is already bundled into
  every release archive (templates zip and per-OS binary zips), so the
  core licensing need is met; this item only adds transitive-dependency
  transparency.
- **Hook:** add a `cargo-about` step to
  `.github/workflows/release.yml` and attach the generated file as a
  release asset.

### Agent-agnostic config and subagent support (design note)

A guardrail for *if* slopctl ever manages agent **configuration files**
(e.g. `.claude/settings.json`, `.codex/config.toml`) or distributes
custom **subagents** (e.g. `.claude/agents/*.md`, `.codex/agents/*.toml`).

- **Status:** speculative — no committed work, no user waiting on it.
- **Design constraint:** model these as agent-agnostic features driven by
  `agent-defaults.yml` conventions (a config/subagent directory per
  agent, mirroring the existing `skill_dir` / `prompt_dir`) plus generic
  `templates.yml` sections — never as Codex-specific or Claude-specific
  fields. Every agent has its own emerging pattern; a good abstraction
  should cover them uniformly.

---

## Shipped work

See [UPDATES.md](UPDATES.md) for the full, newest-first history of
released changes.
