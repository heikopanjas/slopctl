# slopctl Roadmap

**Last updated:** 2026-11-18

Forward-looking plan for slopctl: work that is planned or under
consideration but not yet shipped. This file is intentionally short.

Completed and released changes are recorded chronologically in
[UPDATES.md](UPDATES.md) — the append-only "Recent Updates & Decisions"
log maintained per the `recent-updates` skill. The roadmap does **not**
duplicate that history; delegating it there is what keeps this file
usable instead of perpetually stale.

---

## Open items

### Review whether `$userprofile` installs are still needed

`$userprofile` targets install files into the user's home directory instead
of the workspace. The feature exists to compensate for agents that could not
read project-specific instructions and skills. At the time that was Codex:
its skill directory was `~/.codex/skills` until v19.1.1 (2026-05-14), when
Codex gained workspace-local skills and slopctl moved it to `.codex/skills`.
Since v20.0.0 `$userprofile` is only an opt-in exception (explicit
`target: '$userprofile'`, plus `userprofile_skill_dir` in
`agent-defaults.yml`), kept for global or corporate policy skills.

- **Status:** open. Check every supported agent for project-local
  instructions and skills. If all of them support it, decide whether the
  remaining policy use case justifies the feature.
- **Why it matters:** files outside the workspace are the source of
  SPEC-25 in `docs/SPEC.md`. Workspaces track them although other projects
  share them, so `remove` must never delete them (fixed in v23.3.8), and
  `$userprofile` language skills still land in a directory that depends on
  the command's `--agent` flag (open). The slopctl-starter catalog uses no
  `$userprofile` target today.
- **If it goes:** drop the `$userprofile` skill target and the
  `userprofile_skill_dir` field (MAJOR version), and close the open part of
  SPEC-25 by removal instead of a placement fix.

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
