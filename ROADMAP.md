# slopctl Roadmap

**Last updated:** 2026-05-16

This document indexes planned work for slopctl. Each plan originated
in a Cursor chat session; the links below point to the original
transcripts so future sessions can pick up context quickly.

---

## Future Considerations

- **Agent-agnostic config/subagent support**: When adding support for
  agent configuration files (e.g. `.codex/config.toml`) or custom
  subagents (e.g. `.codex/agents/*.toml`), design them as agent-agnostic
  features rather than Codex-specific fields. Cursor, Claude Code, and
  Copilot have their own emerging patterns; a good abstraction should
  cover all of them uniformly.

---

## Planned: release artifacts and corporate licensing

For downstream corporate use, legal and security teams often expect
self-contained artifacts and dependency transparency.

- **Template package (CI release asset):** Bundle the repository root
  `LICENSE` (MIT) inside the template tarball/zip alongside
  `templates/v5/` (and any other shipped paths) so installs that do not
  clone the full repo still carry explicit license text.
- **Binary releases:** Ship `LICENSE` next to each binary (standalone
  upload) or inside the same archive as the binary (e.g.
  `slopctl-<version>-<target>.tar.gz` containing `slopctl` + `LICENSE`).
  The binary itself does not embed license text; MIT does not require
  that, but bundling the file aids audits.
- **Third-party licenses (optional, enterprise-friendly):** Generate a
  transitive dependency license manifest (e.g. `cargo-about`) in CI and
  attach it as a release asset such as `THIRD-PARTY-LICENSES.html` for
  SBOM-style review.
- **Implementation hook:** Extend `.github/workflows/release.yml` (or
  the step that assembles artifacts) to copy `LICENSE` into staging
  before creating archives so this stays automatic per release.

---

## Completed

| Version | Item | Date |
| --------- | ------ | ------ |
| v15.3.0 | Merge command redesign: DRY shared pipeline with init, New/Unchanged/Diverged classification | 2026-04-18 |
| v15.2.0 | Init/merge redesign: AI-free init, AI-powered merge with --lang/--agent/--mission/--skill options | 2026-04-18 |
| v15.1.0 | Smart doctor (AI-assisted AGENTS.md linting via `doctor --smart`) | 2026-04-18 |
| v15.0.0 | Rebrand to slopctl | 2026-04-18 |
| v14.0.0 | Rebrand to slopcop | 2026-04-18 |
| v13.1.0 | AI-assisted `merge` command with LLM provider abstraction (OpenAI, Anthropic, Ollama, Mistral) | 2026-04-10 |
| v13.0.0 | Rename `install` to `init`, Codex template cleanup, `merge` skeleton, Session Protocol | 2026-04-10 |
| v12.4.0 | `templates` command (replaces `update`), `status` (replaces `list`) | 2026-04-10 |
