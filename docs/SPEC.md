# slopctl specification

**Spec version:** 1 (describes slopctl v23.3.5)
**Last reviewed:** 2026-11-18

This document defines what slopctl does, command by command and concept by concept. It describes the current behavior of the code and marks every place where that behavior is wrong, inconsistent, or undefined. Each such place has an issue box with a proposed solution. Section 11 collects all issues in one register.

## Contents

1. [Conventions](#1-conventions)
2. [Glossary](#2-glossary)
3. [Configuration](#3-configuration)
4. [Catalog model](#4-catalog-model)
5. [Workspace state](#5-workspace-state)
6. [AGENTS.md and protected files](#6-agentsmd-and-protected-files)
7. [Skills](#7-skills)
8. [Commands](#8-commands)
9. [Cross-cutting behavior](#9-cross-cutting-behavior)
10. [Target model](#10-target-model)
11. [Issue register](#11-issue-register)

---

## 1. Conventions

- **MUST**, **MUST NOT**, **SHOULD** and **MAY** follow RFC 2119. They appear only in rules that the code is meant to satisfy.
- Each subsection first states the **current behavior**. References like `template_engine.rs:1190` point at the code as of v23.3.3 and drift as the code changes. The function name next to a reference is the stable anchor.
- Issue boxes look like this:

  > **SPEC-00** · P1 · area
  > **Problem.** What is wrong and how it shows up.
  > **Proposal.** What slopctl should do instead.

- Severity:
  - **P0** loses or deletes user data without warning.
  - **P1** produces wrong results, contradicts a documented rule, or makes a command unusable in a common case.
  - **P2** is an inconsistency, a confusing message, missing validation, or stale documentation.
- Every issue was traced through the code. Issues marked *verified* were re-read line by line during this review. The rest were traced by reading the call chain and are very likely correct, but deserve a test before a fix.
- Issue IDs are stable and never reused. Gaps in the numbering are allowed.
- Section 10 describes the proposed target model that most fixes converge on. The issue boxes refer to it.

---

## 2. Glossary

These terms are normative. Code, docs and messages SHOULD use them with exactly this meaning.

| Term | Meaning |
|---|---|
| **Workspace** | The current working directory when slopctl runs. slopctl does not search upward for a workspace root. |
| **Global config dir** | `$XDG_CONFIG_HOME/slopctl`, else `~/.config/slopctl`. Holds `config.yml` and global agent overlays. |
| **Cache** | `$XDG_CACHE_HOME/slopctl/templates`, else `~/.cache/slopctl/templates`. Holds the fetched baseline, `agent-defaults.yml`, `model-defaults.yml` and the legacy tracker. Layers are cached in `layers/<id>/` beside it. |
| **Template system** | A complete catalog: `templates.yml`, the files and skills it references, and optionally `agent-defaults.yml`. |
| **Baseline** | The template system configured by `templates.uri`. Default: `heikopanjas/slopctl-starter`. |
| **Layer** | An extra template system configured under `templates.layers`, merged over the baseline in order. |
| **Overlay** | A user-defined agent in `agents/<name>/agent.yml`, in the global config dir or in `.slopctl/agents/`. |
| **Effective catalog** | The result of merging baseline, layers and overlays (`agent_overlay::load_effective_catalogs`). Every command that needs the catalog MUST load it this way. |
| **Agent** | A coding assistant listed in the effective agent catalog (`agent-defaults.yml` plus overlays). |
| **Marker** | A workspace-relative directory listed in an agent's `markers` (for example `.claude`). Its presence means the agent's tool has been used here. It does not mean slopctl installed anything. |
| **Installed** | A language or agent is installed when it owns at least one tracker entry (`FileTracker::get_installed_languages`, `get_installed_agents`). See [SPEC-19](#spec-19). |
| **Detected agent** | An agent whose marker exists in the workspace. |
| **Skill** | A directory containing `SKILL.md` and optional supporting files, per agentskills.io. Its name is the directory name, which MUST equal the `name` frontmatter. |
| **Skill directory** | A directory that holds skills: the cross-client `.agents/skills/`, or an agent's own `skill_dir`. |
| **Cross-client reader** | An agent with `reads_cross_client_skills: true`. It reads `.agents/skills/` and its own `skill_dir`. |
| **Native-only agent** | An agent with `reads_cross_client_skills: false`. It reads only its own `skill_dir`. |
| **Skill copy** | One skill installed into one skill directory. A skill can have several copies. |
| **Placement** | The rule that decides which skill directories receive a copy of a skill. Replaces the older words "adoption" and "distribution". |
| **Hydration** | Current code's name for back-filling an agent's skill directory when that agent is added later. The target model folds it into placement. |
| **Registration** | Recording a file that already exists on disk in the tracker. The code calls this "adoption" today. See [SPEC-18](#spec-18). |
| **Fragment** | A catalog entry whose target starts with `$instructions`. Its content is inserted into AGENTS.md instead of being written as a file. |
| **Main file** | `AGENTS.md`, assembled from the `main` template and fragments. |
| **Changelog-protected file** | A file containing the line `<!-- {changelog} -->` on its own (for example `UPDATES.md`). Everything below the marker belongs to the user. |
| **Owner** | A language or agent recorded in a tracker entry's `lang` or `agent` array. |
| **Origin** | The id of the template system that provided a tracked file. |
| **Pristine / modified** | A tracked file is pristine when its content equals what slopctl last wrote and modified otherwise. See [SPEC-10](#spec-10) for why "what slopctl last wrote" is not enough. |

---

## 3. Configuration

### 3.1 Files and scopes

| Scope | File | Notes |
|---|---|---|
| Global | `<global config dir>/config.yml` | Personal settings and private layers. |
| Workspace | `<workspace>/.slopctl/config.yml` | Usually committed, so it MUST only reference sources the whole team can reach. |

`slopctl config` without `-g` writes the workspace file and reads the effective view. With `-g` it reads and writes only the global file.

### 3.2 Keys

| Key | Meaning |
|---|---|
| `templates.uri` | Baseline source: local folder, GitHub tree URL, or git URL. |
| `templates.ref`, `templates.path` | Git ref and sub-path of the baseline. Read only from the scope that set `templates.uri`. |
| `templates.fallbackUri` | Source tried when the primary fetch fails. Used with no ref or path. |
| `templates.layers.<id>.uri` / `.ref` / `.path` | One layer. A bare `templates.layers.<id>` gets or deletes the whole layer. Setting ref or path requires an existing uri. |
| `agents.uri`, `agents.fallbackUri` | Source of `agent-defaults.yml`: local file or folder, or GitHub tree URL. |
| `models.uri`, `models.fallbackUri` | Source of `model-defaults.yml`, same forms. |
| `merge.provider`, `merge.model` | LLM provider and model for `merge` and `doctor --smart`. |

`Config::set` rejects unknown keys (`config.rs:363`). The YAML loader does not reject unknown fields, and no value is validated.

### 3.3 Precedence

- Every key: workspace beats global (`EffectiveConfig::get`).
- Baseline ref and path come from the scope that set the uri (`EffectiveConfig::baseline_source`).
- Layers concatenate: global layers first, then workspace layers. The same id in both scopes is an error.
- Template source for a fetch: `--from` (uri only), then the effective config, then the built-in default. The fallback is tried only after the primary fails.
- LLM provider: `merge.provider`, else the first API key found in the environment in the order Anthropic, OpenAI, Mistral. Model: `merge.model`, else the provider's default from `model-defaults.yml`.

> <a id="spec-01"></a>**SPEC-01** · P1 · config
>
> **Problem.** Source resolution and provider resolution call `EffectiveConfig::load(..).ok()` (`main.rs:66`, `:86`, `:216`, `merge.rs:429`). A config file with a YAML error is ignored. `templates --update` then fetches the default catalog, and `merge` falls back to whichever API key is in the environment. The user gets no hint.
>
> **Proposal.** A config file that exists but does not parse MUST be a hard error in every command, naming the file and the parse error.

> <a id="spec-02"></a>**SPEC-02** · P2 · config
>
> **Problem.** The config file accepts unknown fields, so a typo is ignored. Values are not checked: a git URL for `agents.uri` is accepted and fails later. `templates.ref` set in the workspace has no effect when `templates.uri` is global, but `config --list` shows it as if it were active. `config -g --set templates.layers.X.uri` succeeds when the workspace already defines layer X, and from then on every command fails on the duplicate id.
>
> **Proposal.** Deserialize with `deny_unknown_fields`. Validate values on `--set` (URI form per key, provider name against `model-defaults.yml`). `config --list` marks inactive keys. `--set` refuses a layer id that the other scope already defines.

> <a id="spec-03"></a>**SPEC-03** · P2 · config CLI
>
> **Problem.** `config` takes a key, `--set`, `--list` and `--delete` with no clap conflicts. It evaluates them in a fixed order and silently ignores the rest. `--set a b --set c d` produces four values, so nothing is set and help is printed (`main.rs:387`).
>
> **Proposal.** Make the four modes mutually exclusive in clap and allow `--set` once.

---

## 4. Catalog model

### 4.1 `templates.yml`

Parsed into `bom::TemplateConfig`. Unknown keys are ignored ([SPEC-04](#spec-04)).

| Field | Type | Meaning |
|---|---|---|
| `version` | integer, default 5 | Schema version. `init` accepts 2 to 5. |
| `id` | `[a-z0-9_-]+`, optional | Template system id. Required for layers. A baseline without one is `baseline`. |
| `main` | `{source, target}` | Template for AGENTS.md. |
| `preamble`, `principles`, `mission` | list of `{source, target}` | Usually fragments (`$instructions`). An entry with a real target is installed as a file. |
| `languages.<name>` | `{includes, files, skills}` | Language support. `includes` names shared groups or other languages. Included files and skills come first, depth-first. |
| `shared.<name>` | `{files, skills}` | Reusable groups referenced through `includes`. |
| `integration.<name>` | `{files}` | Always installed, independent of language and agent. Kept in name order. |
| `agents.<name>` | `{instructions, prompts, skills, directories}` | Agent-specific files, skills, and directories to create. |
| `skills` | list of skill definitions | Top-level skills, installed for every workspace. |

A skill definition is `{source, target?}`. Its name is the last path component of `source` (`SkillDefinition::derive_name`). For URL skills the installed name is whatever directory the fetch discovers, which can differ ([SPEC-06](#spec-06)).

Target placeholders:

| Placeholder | Resolves to |
|---|---|
| `$workspace` | The workspace root. As the whole skill `target`, it means "use placement" (same as omitting it). |
| `$userprofile` | The home directory. As the whole skill `target`, it means the agent's `userprofile_skill_dir`, else `~/.agents/skills`. |
| `$instructions` | Marks a fragment. The section the entry comes from (`mission`, `languages`, ...) selects the AGENTS.md marker it fills. |

Include resolution tracks the current include path as a stack, so a diamond is not a cycle. Identical entries reached twice are deduplicated; two different sources for one target are an error.

One source per skill name: top-level, language and shared skills install into the same directories, so the strict loader rejects a name with two sources (`bom::require_unique_skill_sources`). Agent skill lists may carry their own variant; `templates --verify` only warns about those.

> <a id="spec-04"></a>**SPEC-04** · P2 · catalog
>
> **Problem.** `TemplateConfig` and its sections accept unknown keys. A typo such as `skill:` instead of `skills:` silently drops the skills. An include name that is both a shared group and a language resolves files from the group but skills from the language when the group has none (`bom.rs:217-225` against `:280-289`).
>
> **Proposal.** Reject unknown keys in `templates.yml` (layers and baseline alike). Make a name that is both a shared group and a language a load error.

### 4.2 `agent-defaults.yml`

Parsed into `AgentCatalog { version: 1, agents: [...] }`. Agent order is the detection order.

| Field | Required | Meaning |
|---|---|---|
| `name` | yes | Unique agent id. |
| `markers` | yes, at least one | Workspace-relative directories that indicate the agent. No `$`, no `..`, no file extension. |
| `prompt_dir` | yes | Where the agent reads prompts or commands. Used by registration, `agents --list` and removal location checks. Installs use the catalog's own targets, not this field. |
| `skill_dir` | yes | The agent's own skill directory. |
| `userprofile_skill_dir` | no | Target for `$userprofile` skills. Default `~/.agents/skills`. |
| `reads_cross_client_skills` | yes | Whether the agent also reads `.agents/skills/`. |

The cross-client directory is fixed: `$workspace/.agents/skills` (`agent_defaults::CROSS_CLIENT_SKILL_DIR`).

Shipped agents (slopctl-starter):

| Kind | Agents |
|---|---|
| Cross-client reader with own dir | cursor, codex, copilot, vibe, opencode, pi, goose |
| Native-only | claude, kiro, cline |
| Cross-client only (`skill_dir` is `.agents/skills`) | none shipped; possible through layers or overlays |

An agent missing from the agent catalog counts as a cross-client reader (`reads_cross_client_skills_from_catalog` defaults to true). The effective catalogs MUST agree: an agent in `templates.yml` MUST also be in the agent catalog (`EffectiveCatalogs::require_known_agent`). `init`, `update` and `merge` hard-error on a mismatch; `remove` only warns.

### 4.3 `model-defaults.yml`

`{version, providers: [{name, api_key_env?, endpoint, models_endpoint, default_model}]}`. The provider list in code is a fixed enum (Anthropic, OpenAI, Mistral); other entries in the file cannot be used.

### 4.4 Layers

- Order: baseline, then global layers, then workspace layers. Later wins.
- `template_layer::apply_layers` is the only merge path. Rules:
  - `main` is replaced.
  - Fragments are keyed by source path relative to their system.
  - Other files are keyed by target.
  - Keyed sections merge field by field.
  - Top-level, language and shared skills are settled globally by name, so one name has one source.
  - Agent skill lists merge per agent, by name.
  - Layer `agent-defaults.yml` entries replace baseline entries whole, by name.
- Every replacement is recorded in `TemplateSystems.overrides` and shown by `templates --list` and `--verify`.
- Layer sources are rebased to absolute paths inside the layer cache. URL skills in a layer are rewritten to one definition per discovered skill using the manifest's `url_skills`. The baseline does not get this rewrite ([SPEC-05](#spec-05)).
- The strict loader refuses a layer that is not fetched or whose manifest no longer matches its config. The lenient loader warns and skips it.

### 4.5 Overlays

- `agents/<name>/agent.yml` combines the agent-defaults fields and the `templates.yml` agent sections. Unknown keys are rejected.
- Overlays are add-only. A name that clashes with a shipped agent is an error. Workspace overlays win over global ones on a name clash.
- Sources MUST be relative to the agent directory (no absolute paths, no `..`, no URLs) and are rewritten to absolute paths with `std::path::absolute`.
- Workspace overlays MUST use `$workspace` targets and directories only. `skill_dir: $workspace/.agents/skills` is allowed, which creates a cross-client-only agent ([SPEC-23](#spec-23)).

### 4.6 Fetching and the cache

- `fetch_catalog(&CatalogSource, dest)` is the only way a catalog enters a cache directory. It stages into a sibling directory and swaps it in after success, so files dropped upstream disappear and a failed fetch keeps the old cache.
- Source kinds: git URLs (`git::is_git_url`) are cloned with the user's `git`; other `http(s)` URLs are treated as GitHub tree URLs; anything else is a local folder.
- URL skills are downloaded at fetch time and cached under `skills/<discovered name>`.
- Each fetch writes `.slopctl-source.yml` (uri, ref, path, commit, fetch time, URL skills).
- `agent-defaults.yml`, `model-defaults.yml` and the legacy tracker survive a fetch.

> <a id="spec-05"></a>**SPEC-05** · P1 · URL skills
>
> **Problem.** For baseline URL skills, `init` and `merge` download from GitHub live (`template_engine.rs:1800-1828`) even though the cache holds them. `update` reads the cache at `skills/<derive_name>` (`:1789-1798`). Three things follow:
> - `init` can install a newer revision than the cache, and the next `update` reverts it.
> - A URL that yields several skills, or a skill whose directory name differs from `derive_name`, makes bare `update` fail with "not found in local template cache".
> - `init` needs the network although the cache is complete.
>
>
> **Proposal.** Apply the manifest-based URL rewrite (`template_layer.rs:285-311`) to the baseline too. After a fetch, every URL skill is a local cached skill with its discovered name. `init`, `update` and `merge` MUST read only the cache.

> <a id="spec-06"></a>**SPEC-06** · P2 · skill identity
>
> **Problem.** The uniqueness check and layer settling compare `derive_name` (`bom.rs:334`, `template_layer.rs:409`). The installed name is the discovered directory name. A URL skill whose discovered name differs can collide with a local skill undetected, and in the cache it overwrites the bundled `skills/<name>` because URL skills are cached after local ones (`download_manager.rs:162-168`). A `SKILL.md` at a repository root is named after the tarball's top-level directory (`owner-repo-<sha>`), so its name changes with every commit.
>
> **Proposal.** A skill's identity is its discovered directory name, resolved at fetch time and stored in the manifest. Uniqueness and layer settling use that name. A cache collision between a URL skill and a local skill is a fetch error. A root-level `SKILL.md` is named from its `name` frontmatter.

> <a id="spec-07"></a>**SPEC-07** · P2 · agent and model catalogs
>
> **Problem.** `agents.uri` and `models.uri` accept only local paths and GitHub tree URLs; a git URL is treated as a local path. `templates --update` downloads `agent-defaults.yml` and `model-defaults.yml` only if they are missing and never refreshes them. If a template system ships its own `agent-defaults.yml`, `fetch_catalog` takes it and `agents.uri` is ignored (`mod.rs:299-307`).
>
> **Proposal.** Use the `CatalogSource` fetch path for all three catalogs, so git URLs work everywhere. Define one precedence: `agents.uri` when set, else the template system's own file, else the default. `templates --update` reports when the agent or model catalog is older than its source.

---

## 5. Workspace state

### 5.1 Tracker file

`.slopctl/tracker.yml` maps a workspace-relative, forward-slash path to `FileMetadata`:

| Field | Meaning |
|---|---|
| `original_sha` | SHA-256 of what slopctl wrote. For copied files: the source's SHA. For AGENTS.md: the assembled content's SHA. After `merge`: the merged content's SHA. |
| `template_sha` | SHA-256 of the template content a merged file was based on. Stored only when it differs from `original_sha`; absent means the file is a plain template copy. Written by `merge`, cleared by any plain copy. |
| `template_version` | Catalog `version` at install; `0` for registered files. |
| `installed_date` | RFC 3339 timestamp. |
| `lang`, `agent` | Owner arrays, sorted and unique. |
| `ref_count` | Always `lang.len() + agent.len()`, recomputed on load and on every change (`normalize_ownership`). |
| `category` | Catalog section the file came from. |
| `origin` | Template system id. None for AGENTS.md, overlay files, registered and migrated entries. |

Paths outside the workspace (from `$userprofile` targets) are stored as absolute keys (`FileTracker::to_relative_key`).

An unreadable tracker loads as empty with a warning. The next save moves it to `tracker.yml.bak` instead of overwriting it.

Categories written today: `main`, `agent`, `language`, `skill`, `integration`, `preamble`, `principles`, `mission` (section entries with real targets, `tracker_category_for_section`), and `command`, written only by registration (`file_tracker.rs:632`). Categories MUST come from the catalog section, never from path substrings.

### 5.2 File status

`FileTracker::check_modification`:

| Status | Condition |
|---|---|
| `NotTracked` | No entry. |
| `Deleted` | Entry exists, file does not. |
| `Unmodified` | File SHA equals `original_sha`. |
| `Merged` | File SHA equals `original_sha` and the entry has a `template_sha`: an unedited merge result. |
| `Modified` | File SHA differs. |

> <a id="spec-10"></a>**SPEC-10** · P0 · tracker · *verified*
>
> **Problem.** `original_sha` means "what slopctl last wrote". It does not record which template content that write was based on. After `merge` writes a merged file it records the merged SHA (`merge.rs:361`), so the file reads as `Unmodified`. The next bare `update` sees source SHA ≠ file SHA and refreshes it (`partial_update.rs:361-369`). `init` does the same (`template_engine.rs:1416`, plan `Copy`). The merged customizations are replaced by the plain template with no warning. Only changelog-protected files escape, because they are checked by marker.
>
> **Proposal.** Store two hashes per entry:
> - `template_sha`: SHA of the template source used for the last write or merge.
> - `installed_sha`: SHA of what slopctl wrote.
>
> The status then answers two separate questions: did the user edit the file (`disk ≠ installed_sha`), and is the file a plain copy of its template (`installed_sha == template_sha`). Automatic refresh (`init`, bare `update`) MUST only replace a file that is unedited **and** a plain copy. A merged file whose template moved on is reported as "merged, template changed, run `slopctl merge`". Old entries load with `template_sha = original_sha`.
>
> **Resolved in v23.3.4.** `original_sha` keeps its role as the installed hash, and an optional `template_sha` is stored only for merged files, so existing trackers load unchanged. `check_modification` reports the new `FileStatus::Merged`.
> - `init`: a merged file with an unchanged template gets `RefreshTracker`, which records owners and keeps both hashes. With a changed template it is treated like a modified file (`SkipModified`, `Copy` with `--force`, error when owners would be added).
> - Bare `update`: up to date while the template is unchanged; skipped with a `merge` hint when it changed; `--force` refreshes. Stale merged skill files are kept like modified ones.
> - Partial `update`: a selected merged file needs `--force`.
> - `merge`: records `template_sha` and skips unedited merges of the current template instead of calling the LLM again (`settle_current_merges`).
> - `doctor`: reports merged files as OK.

### 5.3 Ownership

- `init` records owners from its flags. Language files get `[lang]`, agent files get `[agent]`. Integration and section files get no owners. Skills: see section 7.
- A file shared by two owners is only valid when the content is identical. If the incoming template SHA differs from the tracked one and the write would add a new owner, `init` fails preflight and points to `merge`.
- `remove --lang` / `--agent` release that owner on every tracker entry (`clear_lang_owner`, `clear_agent_owner`), not only on deleted files.

> <a id="spec-12"></a>**SPEC-12** · P1 · ownership
>
> **Problem.** `ref_count == 0` has two unrelated meanings. Integration and section files (`UPDATES.md`, workspace-target mission files) are installed with no owners because they belong to the workspace as a whole. Registered files have no owners because nobody knows their owner. `is_unreferenced` returns true for both, so any sweep that deletes unreferenced files treats shared infrastructure and user files the same. `--all` and `--agent` never collect `integration` or `command` entries; only `--purge` removes them.
>
> **Proposal.** Make the owner explicit. Add a `workspace` owner for files installed for every workspace (integration, sections, top-level skills) and a `user` origin for registered files. `ref_count == 0` then means only "nothing needs this file", and `remove` can rely on it.

> <a id="spec-13"></a>**SPEC-13** · P2 · tracker format
>
> **Problem.** The tracker is a `HashMap`, so key order changes between saves and committed trackers produce noisy diffs. The file has no format version. One malformed entry discards the whole file to `.bak`. A language or agent named `none` or `all` silently disappears from owner arrays because those are legacy sentinels.
>
> **Proposal.** Use a `BTreeMap`. Add `format: 2` at the top. Skip and report a malformed entry instead of the whole file. Reject `none` and `all` as language or agent names at catalog load.

### 5.4 Registration (code: "adoption") and migration

`TemplateManager::try_migrate_tracker` (`template_manager/mod.rs:174`) runs when `.slopctl/tracker.yml` does not exist. Callers ignore its result. It runs from `status`, `doctor` (also `--smart`), `remove` (except `--source`), `update`, `merge` and `init`, including their dry runs.

1. **Migration.** Entries in the legacy global `installed_files.json` whose workspace equals the current directory are converted. Agent owners are never set.
2. **Registration.** `FileTracker::adopt_untracked_files_from_catalog` records, for every agent in the catalog (installed or not):
   - `AGENTS.md` as `main`;
   - any marker that is a file, as `agent`;
   - every file in every skill subdirectory of every workspace `skill_dir` and of `.agents/skills`, as `skill`;
   - every top-level file in every workspace `prompt_dir`, as `command`.

   Each file is recorded with its current SHA, `template_version: 0`, and no owners (the `LANG_NONE`/`AGENT_ALL` sentinels it passes are dropped by normalization).

> <a id="spec-11"></a>**SPEC-11** · P0 · registration · *verified*
>
> **Problem.** Registered files look exactly like pristine slopctl installs with no owners. The consequences:
> - A bare `update` refreshes `Unmodified` files without `--force`. A user-written skill or prompt whose path matches a catalog target is overwritten.
> - `remove --purge` deletes every tracked file, including registered user prompts in `.claude/commands/` or `.github/prompts/`.
> - `remove --lang` deletes registered files in skill directories because they are "tracked".
> - `init` reports the misleading conflict "already owned by another language or agent".
> - `doctor` reports user files as `Orphaned`.
> - Registration runs from read-only commands. `slopctl status` in any directory writes `.slopctl/tracker.yml`. In `$HOME`, it registers `~/.claude/skills/*`.
> - The code comment at `file_tracker.rs:559` says agent instruction files are registered. `CLAUDE.md`, `.cursorrules` and `.github/copilot-instructions.md` are not markers, so they never are.
>
>
> **Proposal.**
> - Registration MUST NOT run implicitly. It becomes an explicit step (`slopctl doctor --fix`, or `init` when it finds no tracker, after printing what it will register).
> - Registered files get `origin: user`, which marks them as never being a plain template copy, even though they have no `template_sha` ([SPEC-10](#spec-10)). `update` and `init` treat them as modified. `remove` never deletes them.
> - Only files that match a catalog target are registered. Everything else in skill and prompt directories stays untracked and therefore untouchable.
> - Migration of the legacy JSON stays automatic but MUST NOT run in dry-run mode.

> <a id="spec-14"></a>**SPEC-14** · P1 · atomicity
>
> **Problem.** `init` writes AGENTS.md, then directories, then files, and saves the tracker once at the end (`template_engine.rs:1517-1527`). `update` does the same. If a write fails halfway, the files already written are on disk but untracked. The next `init` fails preflight with "exists but is not tracked", which `--force` cannot override. `remove` calls `remove_entry` before the delete (`remove.rs:497`), so a failed delete leaves a file on disk that is no longer tracked.
>
> Since v23.3.5 `remove` drops a tracker entry only after the file was deleted.
>
> **Proposal.** Write a file, then record it. Delete a file, then unrecord it. Save the tracker after each step, or keep a journal and save once in a `Drop` guard. A failed run then leaves a tracker that matches the disk.

---

## 6. AGENTS.md and protected files

### 6.1 Assembly

`generate_fresh_main` (`template_engine.rs:1283`):

1. Read the `main` template and strip `TEMPLATE_MARKER` followed by `\n`.
2. If no language was given, insert an empty `languages` group.
3. `--mission` replaces the `mission` fragments with `## Mission Statement\n\n<text>`.
4. For each category, replace every `<!-- {category} -->` with itself, a blank line, and the trimmed fragments joined by blank lines. Categories without a marker in the template are dropped silently.

Categories are the section names: `preamble`, `principles`, `mission`, `languages`, `integration`. Agent instructions never become fragments.

`handle_main_template` writes the result when there are fragments or a mission. Without them it copies the template as is, marker included. AGENTS.md is recorded as `main` with owners from the command's flags. It has no origin, because it combines every system.

### 6.2 Customized

| Where | Definition |
|---|---|
| `init`, `status`, `remove --purge` | The file does not contain `TEMPLATE_MARKER` (`is_file_customized`). |
| `doctor` | Tracker SHA differs; `Unmerged` only when the file is `Unmodified` and still has the marker. |
| `update` | AGENTS.md is never touched. |

Every assembled AGENTS.md lacks the marker, so it counts as customized from the first `init` on. `init` therefore skips it on every later run unless `--force` is given.

> <a id="spec-15"></a>**SPEC-15** · P1 · AGENTS.md
>
> **Problem.** Three definitions of "customized" disagree. An AGENTS.md the user edited but whose marker is still present shows as "(from template)" in `status`, passes `doctor`, and is deleted by `remove --purge` without `--force`. Marker stripping matches `\n` only, so a template with CRLF line endings keeps its marker and is never considered customized.
>
> **Proposal.** AGENTS.md uses the same two-hash status as every other file ([SPEC-10](#spec-10)). The template marker keeps one job: signalling to humans and `doctor` that a raw template was never assembled. Marker detection is line-based and accepts CRLF.

> <a id="spec-16"></a>**SPEC-16** · P1 · AGENTS.md
>
> **Problem.** AGENTS.md is assembled only from the current command's flags.
> - `init --agent X --force` wipes the languages section because no `--lang` was given.
> - `init --lang B --force` drops language A's fragment.
> - Leaving out `--mission` restores the template's mission.
> - Without `--force`, AGENTS.md is skipped as customized, so a second language's fragment never arrives.
> - `merge` without `--lang` assembles AGENTS.md with an empty languages section and asks the LLM to merge against that ([SPEC-36](#spec-36)).
>
> There is no way to regenerate AGENTS.md for a workspace with two languages.
>
> **Proposal.** AGENTS.md is always assembled from the full installed state: every installed language's fragments, every installed agent's fragments, and the stored mission. `--mission` is stored in `.slopctl/config.yml` (or the tracker) when given. `init --lang B` on a workspace with language A adds B's fragment through `merge` when AGENTS.md is modified, or by regenerating it when AGENTS.md is still a plain assembly.

### 6.3 Changelog-protected files

- A file is protected when a trimmed line equals `<!-- {changelog} -->` (`template_engine::is_changelog_protected`).
- `init` and `update` (full and `--file`) MUST NOT write a protected file, under any flag. `update --file` on one is a hard error pointing to `merge`.
- `merge` is the only command that may refresh the part above the marker. It keeps everything below.
- `remove` and `remove --purge` keep the whole file; `--purge --force` deletes it.

> <a id="spec-17"></a>**SPEC-17** · P1 · changelog protection
>
> **Problem.** The checks are not uniform.
> - `merge` splits on a raw substring (`merge.rs:463-470`). A document that mentions the marker as example text, like the `recent-updates` skill, is cut at the first mention, and template changes below it are never merged.
> - `update --file` checks the source for the marker (`partial_update.rs:138`); every other path checks the target.
> - `update --skill` refreshes skip the check entirely.
>
>
> **Proposal.** One function decides protection, line-based, on the target. `merge` splits with the same function. Skill files go through the same check.

---

## 7. Skills

This section answers the question this spec was started for: which skill copies exist, where, and why.

### 7.1 Sources of skills

| List | Purpose |
|---|---|
| `skills` (top level) | Skills every workspace gets. |
| `languages.<L>.skills`, including skills from included shared groups and languages | Skills for a language. |
| `agents.<A>.skills` | Skills one agent needs, for example `init-session` for agents without a prompt workflow. |

### 7.2 Current placement

`target` omitted or the bare string `$workspace` means "use placement". The set of directories is computed by `non_agent_skill_target_dirs` (`template_engine.rs:467`):

1. Start from the **detected** agents (marker directories) plus the agent named by `--agent`.
2. Add `.agents/skills/` if any of them is a cross-client reader, or if the set is empty.
3. Add each native-only agent's `skill_dir` if it starts with `$workspace`.
4. If nothing was added, use `.agents/skills/`.

| Skill list | Target | Copies go to | Owners on each copy |
|---|---|---|---|
| agent `A` | placement | A's `skill_dir` | `agent: [A]` |
| agent `A` | `$userprofile` | A's `userprofile_skill_dir`, else `~/.agents/skills` | `agent: [A]` |
| language `L` | placement | every directory from the rule above | `lang: [L]` on every copy, including copies in native-only dirs |
| language `L` | `$userprofile` | the `--agent` agent's userprofile dir, else `~/.agents/skills` | `lang: [L]` |
| top level | placement | every directory from the rule above | `lang: [--lang]`, `agent: [--agent]` of the running command, on every copy |
| any | full path | that path | as for the list |

Hydration: `init --agent <native-only>` **without** `--lang` copies the skills of every installed language into that agent's `skill_dir` (`hydrate_language_skills_for_native_agent`). No other agent change hydrates.

`merge_identical_entries` unions owners only when source and target are both identical, so copies in different directories stay separate entries.

### 7.3 Current lifecycle

| Event | Current behavior |
|---|---|
| `init --lang L` | Language and top-level skills go to the placement directories of the detected agents. |
| `init --agent <native-only>` (no `--lang`) | Agent skills to its dir. Top-level skills placed again with the new agent added as owner on every copy. Installed languages' skills hydrated into its dir. |
| `init --agent <native-only> --lang L2` with L1 installed | No hydration. L1's skills are missing from the new agent's dir until a bare `update`. |
| `init --agent <cross-client>` when only native-only agents exist | Language skills are not placed into `.agents/skills/`. A later bare `update` fills the gap. |
| bare `update` | Resolves every installed language against every detected agent and places skills accordingly. Prunes tracked copies of still-resolved skills whose target was dropped. |
| `update --skill S` | Resolves S for the detected agents. Prunes tracked copies of S that are not in the new set. Blocks on modified or untracked targets without `--force`. |
| `remove --agent A` | Deletes tracked, unedited files under A's workspace `skill_dir`, whatever their owners. If A is a cross-client reader and no other detected agent is, deletes tracked, unedited non-language files in `.agents/skills/`. Untracked files are kept; modified files only go with `--force` (since v23.3.5, [SPEC-39](#spec-39)). |
| `remove --lang L` | For each of L's skill names, deletes tracked, unedited files in all skill directories. Deletes any tracked file whose last owner was L. |
| skill removed upstream | Kept. `doctor` reports it as `Orphaned`. `remove` cleans it up. |
| user-added skill | Every `remove` mode keeps it, also with `--force`. First run without a tracker registers it, after which `remove` treats it as installed ([SPEC-11](#spec-11)). |

### 7.4 Problems

> <a id="spec-18"></a>**SPEC-18** · P1 · terminology · *verified*
>
> **Problem.** "Adopt" means three things:
> 1. `FileTracker::adopt_untracked_files_from_catalog`: recording files that already exist on disk.
> 2. The old copy from `.agents/skills/` into a native directory, replaced in v22.1.0 by placement and hydration. It survives in a comment (`template_engine.rs:1207`) and in test names (`test_init_cross_client_then_native_adopts_skills`, `test_remove_lang_removes_adopted_native_agent_skill_copies`).
> 3. "Workspace adoption" for `prompt_dir` targeting in an old UPDATES.md entry.
>
> Nobody reading the code can tell which one a message or a test refers to. The skill model built on top of them has no single rule for who owns a copy.
>
> **Proposal.** Retire the word. Use **registration** for (1) and **placement** for (2). Rename the function, the console message ("Adopted N existing file(s)") and the two tests.

> <a id="spec-19"></a>**SPEC-19** · P1 · installed agents · *verified*
>
> **Problem.** Two definitions of "which agents are here" are in use.
> - Markers decide placement, the agent scope of `update` and `merge`, the last-reader check in `remove`, and `status`.
> - Tracker ownership decides the `init` no-op guard and `update`'s gate on creating agent files.
>
> The split leaks. Bare `update` creates missing skill files for every detected agent, because the creation gate only checks `category == "agent"` (`partial_update.rs:589-592`) and skills are `skill`. The top-level copies it writes record the detected agent as an owner (`template_engine.rs:1201`, `partial_update.rs:464`). That makes the agent installed, so the **next** `update` creates its `CLAUDE.md` and prompts. A user who once opened Claude Code in the repository gets Claude's files after two updates. The existing test runs `update` once and misses this. `merge` without `--agent` creates instruction and prompt files for any detected agent outright.
>
> **Proposal.** One definition: installed means "owns a tracker entry" ([section 10.1](#101-one-definition-of-installed)). Placement, `update`, `merge`, `remove` and `status` use the installed set. Markers are hints only: `status` shows "detected, not installed: claude (run `slopctl init --agent claude`)".

> <a id="spec-20"></a>**SPEC-20** · P1 · skill ownership · *verified*
>
> **Problem.** Top-level skill copies take their owners from the running command's flags, on every copy in every directory (`template_engine.rs:1190-1204`). In this repository `.claude/skills/git-workflow` has `lang: [rust], agent: [claude]` and `.claude/skills/rust-coding-conventions` has `lang: [rust], agent: []`. As a result:
> - Whether `remove --lang L` deletes a top-level skill depends on whether `init` was called with `--agent`. The next `update` puts it back.
> - With one copy locally modified, `init --agent cursor` or `init --lang swift` fails preflight, because the write would add an owner to the modified copy.
>
>
> **Proposal.** Top-level skills are owned by the workspace ([SPEC-12](#spec-12)), never by a language or agent. They exist as long as the workspace has any slopctl install.

> <a id="spec-21"></a>**SPEC-21** · P1 · skill ownership
>
> **Problem.** Language-skill copies in a native-only agent's directory carry `lang: [L]` and no agent owner. Removing the agent does not release anything, so v22.0.1 added "delete everything physically inside the agent's directories" (`in_agent_dir` in `remove.rs`). Until v23.3.5 that rule also deleted untracked files and modified copies ([SPEC-39](#spec-39)); it now applies to tracked, unedited files only, but deletion is still decided by location instead of need. The ownership arrays cannot express "this copy exists because language L needs the skill **and** agent A reads this directory".
>
> **Proposal.** Derive the need for a copy instead of storing it ([section 10.2](#102-skill-placement-as-reconciliation)). A copy is needed when at least one requirer (language, agent, workspace) needs the skill **and** at least one installed agent reads the directory (or no agent is installed and the directory is `.agents/skills/`).

> <a id="spec-22"></a>**SPEC-22** · P0 · update · *verified*
>
> **Problem.** With `--agent X`, bare `update` resolves only X. It then collects the names of resolved skills and treats every tracked copy of those names outside the new set as stale (`partial_update.rs:316-319`, `collect_stale_skill_files` at `:634`). codex, vibe, goose and cline all ship an `init-session` skill, so `update --agent codex` deletes `.vibe/skills/init-session`, `.goose/skills/init-session` and `.cline/skills/init-session`. `update --agent codex --skill init-session` does the same. Reproduced: after `init --agent codex` and `init --agent vibe`, `update --agent codex --dry-run` lists `.vibe/skills/init-session/SKILL.md` as stale.
>
> **Proposal.** Staleness is decided per directory against the full desired state, never against a narrowed candidate set. `--agent` and `--lang` narrow what gets written, not what is compared. With [section 10.2](#102-skill-placement-as-reconciliation) a copy is stale only when the full placement no longer contains it.

> <a id="spec-23"></a>**SPEC-23** · P1 · remove
>
> **Problem.** For an agent whose `skill_dir` is `.agents/skills` (possible through overlays and layers), `remove --agent` deletes the shared directory's contents by location. That includes every language and top-level skill, even while other cross-client readers remain installed. Since v23.3.5 untracked and modified files survive ([SPEC-39](#spec-39)), but slopctl's own unedited copies are still wiped. The message "Keeping .agents/skills (still in use by ...)" is printed anyway.
>
> **Proposal.** Covered by [section 10.2](#102-skill-placement-as-reconciliation): `remove` deletes only copies that are no longer desired and are unedited. Until then, `remove --agent` MUST skip the location sweep when `skill_dir` equals the cross-client directory.

> <a id="spec-24"></a>**SPEC-24** · P1 · hydration
>
> **Problem.** Hydration covers one case out of four (section 7.3). It also iterates the installed languages without checking the catalog still has them, so `init --agent claude` fails once an installed language has been dropped upstream (`template_engine.rs:613-615`). Bare `update` filters correctly (`partial_update.rs:517`).
>
> **Proposal.** Remove hydration as a separate step. Placement after every `init` covers all agent and language changes ([section 10.2](#102-skill-placement-as-reconciliation)). Installed languages missing from the catalog are reported and skipped.

> <a id="spec-25"></a>**SPEC-25** · P0 · `$userprofile`
>
> **Problem.** Skills with a `$userprofile` target live outside the workspace but are tracked by each workspace's tracker with an absolute key. `remove --lang`, `--all` and `--purge` delete tracked files, so removing a language in one repository deletes a global skill every other repository and the user's own setup depend on. A language skill with `target: '$userprofile'` also lands in a different directory depending on the `--agent` of the command, and bare `update` writes one copy per detected agent.
>
> **Proposal.** Workspaces do not own files outside the workspace. `$userprofile` targets are installed and refreshed, recorded with `scope: user` for reporting, and never deleted by `remove`, which lists them instead. For language and top-level skills, `$userprofile` resolves to `~/.agents/skills` plus each installed native-only agent's `userprofile_skill_dir`, independent of the command's flags.

> <a id="spec-26"></a>**SPEC-26** · P1 · update --skill
>
> **Problem.** In partial mode a URL skill definition matches any requested name that exists anywhere in `<cache>/skills/` (`skill_definition_matches_partial`, `template_engine.rs:687-707`). In a Swift workspace with a URL skill in Swift's list, `update --skill git-workflow` also resolves through Swift and records `lang: swift` on `git-workflow`. `remove --lang swift` can then delete it.
>
> **Proposal.** Fixed by [SPEC-05](#spec-05): once URL skills are rewritten to named local skills at fetch time, matching is by name only.

> <a id="spec-27"></a>**SPEC-27** · P2 · skill variants
>
> **Problem.** A cross-client reader with its own directory sees both `.agents/skills/S` and `<own dir>/S` when a top-level or language skill and an agent skill share the name S. `templates --verify` only warns. Which copy the agent uses is up to the agent.
>
> **Proposal.** Treat an agent-list skill with the same name as a top-level or language skill as an **override for that agent**: when the agent is installed, placement puts the variant into the agent's directory and the agent no longer counts as a reader of `.agents/skills/S`. If no other reader needs it, the shared copy is not created. `verify` explains the override instead of warning.

---

## 8. Commands

All commands work on the current directory. Exit code 0 means success; any error exits 1.

### 8.1 `templates`

```text
slopctl templates [-u|--update] [-l|--list] [-V|--verify] [-f|--from <FROM>] [-n|--dry-run]
```

| Flag | Meaning |
|---|---|
| `-u`, `--update` | Fetch the baseline into the cache, bootstrap missing agent and model catalogs, refetch every layer. |
| `-l`, `--list` | Show template systems, agents (with installed flag), languages with includes, skills, contributing layers, overrides. |
| `-V`, `--verify` | Validate the cache and compare it with its source. |
| `-f`, `--from` | Source for `--update` and `--verify` (uri only). Not remembered. |
| `-n`, `--dry-run` | With `--update` only (clap `requires`). Prints source, fallback, bootstrap candidates and layers. |

At least one of `--update`, `--list`, `--verify` is required. When several are given they run in the order update, verify, list.

`--update` fetches with `fetch_catalog`. On failure it retries `templates.fallbackUri`.

`--verify` checks, in order:
1. YAML structure: version 2 to 5, `main.source` present, duplicate targets, skill-source conflicts.
2. File integrity: every referenced source exists.
3. Each layer: manifest matches config, id matches, version equals the baseline's, URL skills cached, integrity, freshness. Prints the override report.
4. Agent catalogs: mismatches between `templates.yml` and the agent catalog, overlays.
5. Baseline freshness. Git sources compare commits with `git ls-remote`. Other sources compare `templates.yml` byte for byte. An unreachable source is a warning.

Any issue makes the exit code non-zero.

> <a id="spec-08"></a>**SPEC-08** · P2 · verify
>
> **Problem.** An unreachable source is a warning in `templates --verify` (`verify.rs:452`) and an error in `agents --verify` and `models --verify` (`agents.rs:165-174`). Freshness for non-git sources compares only `templates.yml`, so changed skill files go unnoticed. `--from` is not remembered, so a `--verify` after `--update --from X` reports the cache as stale against the configured source.
>
> **Proposal.** Unreachable is a warning everywhere, with a non-zero exit only under a `--strict` flag. Non-git freshness compares a hash over every referenced file. The manifest records the uri actually fetched, and `--verify` without `--from` compares against that.

> <a id="spec-09"></a>**SPEC-09** · P2 · list
>
> **Problem.** `templates --list` treats only top-level skills as template skills (`list.rs:365-375`), so installed language and agent skills appear again as extra "(installed)" entries.
>
> **Proposal.** Group skills by the list that defines them and mark which ones are installed.

### 8.2 `agents` and `models`

```text
slopctl agents [-u] [-l] [-V] [-f <FROM>] [-n]
slopctl models [-u] [-l] [-V] [-f <FROM>] [-n]
```

Same flags and rules as `templates`.

- `--update` copies one YAML file from a local file, a local folder, or a GitHub tree URL, falling back to `*.fallbackUri`.
- `--list` prints the merged agent catalog including overlays (lenient loader), or the provider list.
- `--verify` parses the local file and compares it with the source byte for byte.

> <a id="spec-47"></a>**SPEC-47** · P2 · models
>
> **Problem.** `models --list` fails when `model-defaults.yml` is missing (`models.rs:71`), so its embedded-fallback branch never runs. LLM error messages tell the user to run `slopctl templates --update`, which does not refresh an existing model catalog ([SPEC-07](#spec-07)). `LlmClient::list_models` exists but no command calls it.
>
> **Proposal.** Fall back to the embedded catalog in `--list`. Point error messages to `slopctl models --update`. Either expose `list_models` (for example `models --list --remote`) or delete it.

### 8.3 `init`

```text
slopctl init [-l|--lang <LANG>] [-a|--agent <AGENT>] [-m|--mission <MISSION>] [-f|--force] [-n|--dry-run]
```

| Flag | Meaning |
|---|---|
| `-l`, `--lang` | One language to install. |
| `-a`, `--agent` | One agent to install. At least one of `--lang` and `--agent` is required. |
| `-m`, `--mission` | Mission text, or `@file` read relative to the current directory. Affects AGENTS.md only. |
| `-f`, `--force` | Skip the no-op guard, overwrite a customized AGENTS.md, turn `SkipModified` into `Copy`. Does not override untracked files, owner-expanding conflicts, or changelog protection. |
| `-n`, `--dry-run` | Show the plan, write nothing (see [SPEC-48](#spec-48)). |

Pipeline:
1. If the cache has no baseline, fetch it (with fallback) and bootstrap the agent and model catalogs. With `--dry-run`, print a message and exit 0.
2. Fetch layers that are configured but not cached.
3. Run migration and registration if no tracker exists ([SPEC-11](#spec-11)).
4. **No-op guard.** Unless `--force`, fail when every requested language and agent is already installed.
5. Check the baseline `version` (2 to 5).
6. Resolve files (`resolve_all_files`): sections, language files, integration files, agent directories, instructions and prompts, then agent, language and top-level skills, then hydration. Missing sources are dropped silently.
7. Preflight. Every hard error is collected; one error aborts the run before anything is written.

| Situation | Plan |
|---|---|
| Same target from two sources | error |
| Target is a directory | error |
| Target missing | `Copy` |
| Target changelog-protected | `SkipChangelog` |
| Unmodified, tracked SHA equals source SHA | `RefreshTracker` (record owners only) |
| Unmodified, SHA differs, adds owners | error, "use merge" |
| Unmodified, SHA differs, no new owners | `Copy` |
| Deleted (tracked, missing) | `Copy` |
| Not tracked | error, even with `--force` |
| Modified or merged with changed template, adds owners | error |
| Merged, template unchanged | `RefreshTracker` (owners only, merged content and hashes kept) |
| Modified or merged with changed template, `--force` | `Copy` |
| Modified or merged with changed template | `SkipModified` |

8. With `--dry-run`, print the plan and stop.
9. Write AGENTS.md (skipped when customized, unless `--force`), create directories, copy files, save the tracker.

> <a id="spec-28"></a>**SPEC-28** · P2 · init
>
> **Problem.** The `--lang` help says "use merge to add another", but `init --lang` is the documented way to add a language. The `--force` help says "without confirmation", but `init` never prompts. `--force` overwrites modified files but not untracked ones, while `update --force` overwrites both.
>
> **Proposal.** Help text: "--lang: language to install (repeat init to add another)", "--force: overwrite modified files and a customized AGENTS.md". Unify `--force` semantics ([SPEC-41](#spec-41)).

> <a id="spec-29"></a>**SPEC-29** · P2 · init
>
> **Problem.** `init --dry-run` with no cached baseline exits 0 without a preview (`main.rs:567-571`). With a newly configured layer it does not fetch the layer, and the strict loader then refuses it, so the dry run fails. Only `init` checks the catalog version, and only the baseline's. Only `init` creates agent `directories`. A missing source is silently dropped in full resolution and a hard error in partial resolution.
>
> **Proposal.** Dry runs fetch into a temporary cache when the real one is missing, so they can preview. Version checks happen in the loader for every system. `update` creates missing agent directories for installed agents. A missing source is an error in both modes, with `templates --verify` as the place to find it first.

### 8.4 `update`

```text
slopctl update [--file <FILE>]... [-s|--skill <SKILL>]... [-l|--lang <LANG>] [-a|--agent <AGENT>] [-f|--force] [-n|--dry-run]
```

| Flag | Meaning |
|---|---|
| `--file` | Workspace path to refresh. Repeatable. Selects partial mode. |
| `-s`, `--skill` | Skill name to refresh. Repeatable. Selects partial mode. |
| `-l`, `--lang` | Bare: replaces the installed languages. Partial: replaces the single installed language. Must exist in the catalog; need not be installed. |
| `-a`, `--agent` | Bare: must be installed, else error. Partial: must exist in the catalog. |
| `-f`, `--force` | Bare: overwrite modified and untracked files, delete modified stale skill files. Partial: allow modified and untracked targets. Never overrides changelog protection or the agent creation gate. |
| `-n`, `--dry-run` | Show the plan, write nothing. |

`update` never touches AGENTS.md, never fetches, and reads only the local cache.

**Bare update** (`update_full`, `partial_update.rs:282`):
1. Require the cache, migrate and register ([SPEC-11](#spec-11)), load the strict catalogs.
2. `resolve_workspace_sets`: every installed language (or `--lang`) crossed with every detected agent (or `--agent`).
3. Collect candidates keyed by normalized target. Last one wins ([SPEC-34](#spec-34)).
4. Compute stale skill copies ([SPEC-22](#spec-22)).
5. Classify each candidate:
   - Missing: create it if the category is not `agent`, or the target is still tracked, or an owning agent is installed. Otherwise report "run `slopctl init --agent X`".
   - Changelog-protected: skip.
   - Unmodified or Deleted: refresh when the source SHA differs.
   - Merged: up to date while the template is unchanged; otherwise skipped with a `merge` hint, or refreshed with `--force`.
   - Modified or not tracked: refresh only with `--force`.
6. Write, prune stale copies (modified and merged ones only with `--force`), save the tracker.

**Partial update** (`update_partial`, `partial_update.rs:45`):
1. Same setup. Language is `--lang` or the first installed language. Agents are the effective agent scope.
2. `--file` matches language files, integration files, agent instructions and prompts. `--skill` matches agent, language and top-level skills. A missing cached source is a hard error.
3. Errors: AGENTS.md selected; selected source contains the changelog marker; a file inside a skill directory (use `--skill`); no match (lists what exists).
4. Any modified, merged or untracked target without `--force` fails the whole run.
5. Write, prune, save.

> <a id="spec-31"></a>**SPEC-31** · P1 · update
>
> **Problem.** `update --lang go` in a Rust workspace installs Go's files and records `go` as owner (`partial_update.rs:507-513`), without Go's AGENTS.md fragment. `--agent` is checked against the installed set; `--lang` is not. This breaks the rule "init installs, update refreshes".
>
> **Proposal.** `update --lang L` MUST fail when L is not installed, with the hint `slopctl init --lang L`.

> <a id="spec-32"></a>**SPEC-32** · P1 · update · *verified*
>
> **Problem.** Partial update uses `get_installed_language()`, which returns the alphabetically first installed language (`file_tracker.rs:411`). In a Rust and Swift workspace `update --skill swift-coding-conventions` fails with "No template match" unless `--lang swift` is given. Bare update iterates all languages.
>
> **Proposal.** Partial update resolves against every installed language, exactly like bare update, and filters by selector afterwards.

> <a id="spec-33"></a>**SPEC-33** · P2 · update
>
> **Problem.** `update --file` searches only language, integration and agent entries (`template_engine.rs:881-952`). Preamble, principles and mission entries with real file targets can never be selected.
>
> **Proposal.** Search every non-fragment entry.

> <a id="spec-34"></a>**SPEC-34** · P1 · update
>
> **Problem.** Bare update keys candidates in a `BTreeMap` where the last insert wins (`partial_update.rs:307-314`). Two installed languages that provide different sources for one target never conflict; the one resolved last silently wins. `init` rejects the same situation.
>
> **Proposal.** Run the same duplicate-target check `init` runs. Identical sources merge owners; different sources are an error that names both owners.

> <a id="spec-35"></a>**SPEC-35** · P2 · update
>
> **Problem.** Bare update reports untracked files at catalog targets as "local modifications preserved", which suggests slopctl installed them.
>
> **Proposal.** Report them separately: "exists but not installed by slopctl: <path> (use `--force` to replace or `slopctl doctor --fix` to register)".

### 8.5 `merge`

```text
slopctl merge [-l|--lang <LANG>] [-a|--agent <AGENT>] [-m|--mission <MISSION>] [--preview] [-n|--dry-run] [-v|--verbose]
```

| Flag | Meaning |
|---|---|
| `-l`, `--lang` | Language whose files and fragments take part. No fallback today ([SPEC-36](#spec-36)). |
| `-a`, `--agent` | Limit to one agent. Without it, every detected agent; the first one wins on a shared target. |
| `-m`, `--mission` | Mission text or `@file`. |
| `--preview` | Write `<file>.merged` next to each original; skip files that already have one; do not update the tracker. |
| `-n`, `--dry-run` | List new and diverged files without calling the LLM. |
| `-v`, `--verbose` | Token counts, unchanged files, and the chat in both directions. |

Pipeline:
1. Migrate and register, load the strict catalogs. `merge` may use the network ([SPEC-05](#spec-05)).
2. Build the expected content per target: AGENTS.md from `generate_fresh_main`, every other file from its source.
3. Classify:
   - **New**: target missing.
   - **Unchanged**: identical, or both changelog-protected and identical above the marker.
   - **Diverged**: everything else. For changelog-protected files only the part above the marker is sent.
   - A diverged file that is an unedited merge (`Merged`) of the current template is moved back to **Unchanged** (`settle_current_merges`).
4. If anything diverged, resolve the LLM provider (section 3.3).
5. New files are written and recorded. Diverged files stream into `<file>.partial`; on success the result replaces the file (or goes to `.merged`), the partial is removed, and the merged SHA is recorded together with the template's SHA. A truncated response keeps the partial. A failure prints a warning. An existing `.partial` skips the file.
6. Save the tracker (also in dry run, [SPEC-48](#spec-48)).

For AGENTS.md the prompt includes an `<available_skills>` block so the model can drop prose that a skill already covers.

> <a id="spec-36"></a>**SPEC-36** · P1 · merge
>
> **Problem.** `MergeOptions.lang` is documented as "falls back to installed language from tracker" (`merge.rs:25`) but is passed through unchanged (`:114`). A plain `slopctl merge`:
> - assembles AGENTS.md with an empty languages section, so AGENTS.md is always "diverged" and the LLM merges against a template without language rules;
> - leaves out every language file and language skill.
>
>
> **Proposal.** Without `--lang`, merge covers every installed language. Without `--agent`, every installed (not detected) agent.

> <a id="spec-37"></a>**SPEC-37** · P2 · merge
>
> **Problem.**
> - Every file that differs from its template goes to the LLM, including unedited files that a plain copy would refresh. That costs tokens and risks hallucinated changes. (Since v23.3.4, unedited merges of the current template are skipped.)
> - New files are created for agents that are only detected, bypassing the `update` gate ([SPEC-19](#spec-19)).
> - `Unchanged` never repairs a stale tracker SHA.
> - A truncated or failed merge still exits 0.
> - A leftover `.partial` blocks that file until the user deletes it by hand.
>
>
> **Proposal.** Merge only files that are modified ([SPEC-10](#spec-10)); refresh plain copies directly. Use the installed agent set. Record the SHA for `Unchanged`. Exit non-zero when any merge failed or was truncated. Offer `--resume` (use the partial) and `--discard-partial`.

### 8.6 `remove`

```text
slopctl remove [-a|--agent <AGENT>] [-l|--lang <LANG>] [-s|--source <SOURCE>] [--all] [--purge] [-f|--force] [-n|--dry-run]
```

| Flag | Meaning |
|---|---|
| `-a`, `--agent` | Remove one agent. Conflicts with `--purge`. |
| `-l`, `--lang` | Remove one language. Conflicts with `--purge`. Combinable with `--agent`. |
| `-s`, `--source` | Remove every tracked file whose origin is this template system id. Conflicts with everything else. |
| `--all` | Remove every agent's files and every skill. Keeps AGENTS.md. |
| `--purge` | Remove everything slopctl installed, AGENTS.md included. Keeps a customized AGENTS.md and changelog-protected files. |
| `-f`, `--force` | Skip the confirmation prompt and also delete modified tracked files. With `--purge`, also delete a customized AGENTS.md and changelog-protected files. Never deletes untracked files. |
| `-n`, `--dry-run` | Show what would be removed. |

Precedence: `--purge`, `--source`, `--all` (error with `--agent` or `--lang`), then `--agent`/`--lang` (at least one required).

What each mode collects:

Every mode passes its candidates through one gate before deleting (`split_user_owned`, since v23.3.5): untracked files are kept, modified and merged files are kept unless `--force` is given, and kept files are listed with the reason.

| Mode | Candidates | Deleted when (after the gate) |
|---|---|---|
| `--agent A` | A's catalog instruction and prompt files on disk; tracked files owned only by A; every file under A's workspace `skill_dir`; tracked skill files inside A's directories; for the last cross-client reader, non-language files in `.agents/skills/` | inside A's directories, or no owners left |
| `--lang L` | L's catalog file targets; L's skill names in every skill directory (tracked or catalog-shipped only); every tracked file owned by L except `main` | no owners left |
| `--all` | every agent's catalog files; every tracked `agent` entry; every file in every skill directory; every tracked `skill` entry | always |
| `--source ID` | tracked files with `origin == ID` | always, except changelog-protected; entries for missing files are dropped |
| `--purge` | every agent's catalog files; every tracker entry; every skill directory; AGENTS.md | always, except customized AGENTS.md and changelog-protected files without `--force`; empty marker directories removed |

After `--agent` or `--lang`, the owner is released on every tracker entry.

> <a id="spec-39"></a>**SPEC-39** · P0 · remove · *verified*
>
> **Problem.** `remove` deletes files the user wrote.
> - `remove --agent A` deletes every file under A's `skill_dir`, tracked or not, modified or not (`remove.rs:133-162`, forced by `in_agent_dir` at `:492`).
> - The last-reader sweep counts untracked files in `.agents/skills/` as A's.
> - An untracked file at a catalog target (a `CLAUDE.md` or `.editorconfig` the user wrote) counts as unreferenced (`is_unreferenced` returns true for untracked paths, `file_tracker.rs:531`) and is deleted by `--agent` and `--lang`.
> - `--all` deletes every file in every skill directory.
>
> `remove --lang` keeps untracked files in skill directories, so the two modes follow opposite rules.
>
> **Proposal.** One rule for every mode except `--purge --force`: `remove` deletes only tracked files that are unedited. Modified tracked files are kept and listed with a `--force` hint. Untracked files are never deleted; they are listed as "left in place". `--purge` follows the same rule, and `--purge --force` deletes modified tracked files too, but still not untracked ones.
>
> **Resolved in v23.3.5.** `split_user_owned` runs on the candidates of every mode (`--agent`, `--lang`, `--all`, `--source`, `--purge`) after the changelog split and before the preview. Untracked files are kept with "not installed by slopctl", also under `--force`. Modified and merged files are kept with "modified, use --force to delete" unless `--force` is given, in every mode, so `--force` now also means "delete my edited copies" outside `--purge`. The `--lang` special case for catalog-shipped untracked files is gone. Workspaces without a tracker still register everything on disk before removal, so this protection depends on [SPEC-11](#spec-11) for them.

> <a id="spec-38"></a>**SPEC-38** · P1 · remove · *verified*
>
> **Problem.** Owners leak.
> - `remove --all` never releases owners (`remove.rs:524-533` runs only for `--agent` and `--lang`). AGENTS.md keeps its agent owners, so `init --agent X` afterwards fails the no-op guard and `update` still treats X as installed.
> - `remove --agent`/`--lang` returns early with "No files found" (`remove.rs:424-428`) before owners are released. If the user deleted the files by hand, the language or agent stays installed forever.
> - `--purge` leaves `.slopctl/tracker.yml` with entries for missing files and the owners of a kept AGENTS.md. The next `init` can hit the no-op guard.
>
>
> **Proposal.** Owner release runs in every mode, before the early return, and `--purge` deletes the tracker (or resets it to the kept files with no owners).

> <a id="spec-40"></a>**SPEC-40** · P1 · remove
>
> **Problem.** The dry-run list and the confirmation prompt show every candidate as "will be deleted". The real run keeps some as "still referenced" (`remove.rs:513`), because owner release is not simulated.
>
> **Proposal.** Compute the final plan (deletions, kept files with reason, owner releases) once, then print it for dry run and confirmation, then execute exactly that plan.

> <a id="spec-42"></a>**SPEC-42** · P2 · remove
>
> **Problem.** Entries for files that are already missing are never pruned. Their owners are released and they stay with no owners. Because `update` recreates any target that is still tracked (`partial_update.rs:589-592`), a later `update` with the agent's marker present can bring back files of a removed agent.
>
> **Proposal.** `remove` drops entries for missing files it would have deleted. `update` never recreates an entry with no owners.

> <a id="spec-43"></a>**SPEC-43** · P2 · remove
>
> **Problem.** BoM paths are canonicalized and tracker paths are not (`remove.rs:99` against `:116`). With a symlinked directory in the workspace the same file can appear twice in the list.
>
> **Proposal.** Normalize every path through one helper that does not follow symlinks ([SPEC-49](#spec-49)).

### 8.7 `doctor`

```text
slopctl doctor [-f|--fix] [-n|--dry-run] [-v|--verbose] [--smart]
```

| Flag | Meaning |
|---|---|
| `-f`, `--fix` | Prune `Missing` entries, strip the marker from `Unmerged` AGENTS.md. |
| `-n`, `--dry-run` | Preview `--fix`. Ignored without `--fix`. |
| `-v`, `--verbose` | Print every checked file. |
| `--smart` | After the checks, lint AGENTS.md with the LLM. Runs regardless of `--fix` and `--dry-run`. Errors are warnings. |

| Issue | Condition | `--fix` |
|---|---|---|
| Missing | tracked, not on disk | prune the entry |
| Unmerged | `main` file, unmodified, still has the template marker | strip the marker |
| Modified | not `main`, not changelog-protected, SHA differs | none |
| Orphaned | tracked, not `main`, not provided by the catalog for the installed set (needs the cache) | none |

`--smart` resolves the provider, picks the first `main` tracker entry (HashMap order) or `./AGENTS.md`, sends one request, and parses a JSON array sliced from the first `[` to the last `]`.

> <a id="spec-44"></a>**SPEC-44** · P2 · doctor
>
> **Problem.**
> - Two hints are wrong: "run `slopctl init --force` to overwrite" (`doctor.rs:250`), and "Run 'slopctl init' to re-merge..." (`:267`). `init` without flags exits with an error, and verb separation says `update --force` or `merge`.
> - `--fix` on `Missing` drops ownership instead of restoring the file.
> - `--dry-run` without `--fix` is silently ignored (no clap `requires`).
> - `fix_unmerged_template` rewrites with LF line endings and does not record a new SHA.
> - `doctor` does not check for: entries with no owners, absolute keys, a `.bak` tracker, stale or unfetched layers, catalog mismatches, untracked files at catalog targets, or detected-but-not-installed agents.
>
>
> **Proposal.** Correct the hints. `--fix` on `Missing` offers to restore the file with `update` and prunes only when the catalog no longer provides it. Add `requires = "fix"` to `--dry-run`. Preserve line endings and record the SHA. Add the missing checks, and make registration ([SPEC-11](#spec-11)) a `--fix` action.

> <a id="spec-45"></a>**SPEC-45** · P2 · doctor --smart
>
> **Problem.** The provider is resolved before checking that AGENTS.md exists. The response is not checked for truncation. A `[` in leading prose breaks the JSON slice. The `main` entry is picked in HashMap order. The module doc of `smart.rs` says it serves `init` too, which has no smart option.
>
> **Proposal.** Check the file first, request JSON with a schema or tool call, detect truncation, lint every `main` entry deterministically, fix the module doc.

### 8.8 `status`

```text
slopctl status [-v|--verbose]
```

Prints the cache location, its version and the available agents and languages; whether AGENTS.md exists and is customized; installed agents; installed languages; installed skills grouped by name. `-v` adds a "Managed Files" list built from every agent's catalog files on disk, every tracker entry, and AGENTS.md.

> <a id="spec-46"></a>**SPEC-46** · P1 · status · *verified*
>
> **Problem.** "Installed agents" comes from markers (`list.rs:172-179`), "installed languages" from the tracker. A repository where someone once ran Claude Code shows `claude` as installed. `status -v` lists the catalog files of every agent present on disk as managed, even files slopctl never wrote. `status` also writes a tracker through registration ([SPEC-11](#spec-11)).
>
> **Proposal.** Installed agents from the tracker. A separate line "Detected, not installed". Managed files from the tracker only. `status` is read-only.

### 8.9 `config`

```text
slopctl config [KEY] [-s|--set <KEY> <VALUE>] [-l|--list] [-d|--delete <KEY>] [-g|--global]
```

Modes are checked in the order list, set, delete, get; with none of them, help is printed. See section 3 and [SPEC-03](#spec-03).

### 8.10 `completions`

```text
slopctl completions <bash|fish|powershell|zsh>
```

Writes a clap_complete script to stdout.

---

## 9. Cross-cutting behavior

### 9.1 Dry run

> <a id="spec-48"></a>**SPEC-48** · P1 · dry run
>
> **Problem.** `--dry-run` writes to disk.
> - Migration and registration run before every dry-run branch (`update.rs:33`, `partial_update.rs:53`, `:290`, `merge.rs:124`, `smart.rs:93`) and save `tracker.yml` when they find anything. Migration also rewrites the legacy global `installed_files.json`.
> - `merge -n` always calls `file_tracker.save()` (`merge.rs:378`). That creates `.slopctl/tracker.yml` and can rename an unreadable tracker to `.bak`.
> - `remove -n` runs registration too.
>
>
> **Proposal.** A dry run MUST NOT write any file, in the workspace or outside it. Migration and registration compute an in-memory tracker in dry-run mode and report what they would record.

### 9.2 `--force`

| Command | `--force` effect today |
|---|---|
| `init` | skip no-op guard, overwrite modified files and customized AGENTS.md; untracked still an error |
| `update` (bare) | overwrite modified and untracked; delete modified stale skills |
| `update` (partial) | allow modified and untracked targets |
| `remove` | skip prompt and delete modified tracked files; with `--purge` also delete customized AGENTS.md and changelog files; never untracked files |

> <a id="spec-41"></a>**SPEC-41** · P2 · flags
>
> **Problem.** `--force` means different things per command. In `remove` it means both "don't ask" and "destroy customizations", so there is no way to purge non-interactively while keeping a customized AGENTS.md.
>
> **Proposal.** Two flags with one meaning each, in every command:
> - `--yes` / `-y`: answer the confirmation prompt.
> - `--force` / `-f`: replace modified and untracked files.
>
> Changelog protection is never overridden by either flag; deleting a protected file needs `remove --purge --force`.

### 9.3 Network

- `templates`, `agents` and `models` with `--update`, and `--verify` freshness checks, use the network.
- `init` uses the network when the cache is missing and for baseline URL skills ([SPEC-05](#spec-05)).
- `update` MUST NOT use the network.
- `merge` and `doctor --smart` call the LLM provider. `merge` also downloads baseline URL skills.
- GitHub access is unauthenticated (about 60 REST requests per hour per IP). Git sources use the user's `git`, which handles authentication.

### 9.4 LLM requests

The client uses a fixed 300 s timeout and a 32768-token output limit (`llm.rs:234`, `:389`, `:492`). A long merge can be truncated; see [SPEC-37](#spec-37) for the exit code.

### 9.5 Paths

> <a id="spec-49"></a>**SPEC-49** · P2 · paths
>
> **Problem.** `normalize_path` calls `canonicalize` (`template_engine.rs:1929`), and `update` and `merge` key candidates with it. When `CLAUDE.md` is a symlink to `AGENTS.md` (a common setup), CLAUDE.md's template is keyed as AGENTS.md. Bare `update` can then write CLAUDE.md's template over AGENTS.md, `merge` replaces AGENTS.md's expected content with CLAUDE.md's, and `update --file CLAUDE.md` errors with "is the merged AGENTS.md".
>
> **Proposal.** Normalize lexically (`std::path::absolute` plus component cleanup). Detect symlinks at a target and skip them with a message, since the user chose to link the file.

> <a id="spec-50"></a>**SPEC-50** · P2 · workspace root
>
> **Problem.** slopctl uses the current directory as the workspace. Running it from a subdirectory creates a second `.slopctl/` there, and `status` writes one through registration.
>
> **Proposal.** Search upward for `.slopctl/` (stop at the git root or `$HOME`) and use that directory. Without one, `init` uses the current directory and every other command errors with "not a slopctl workspace".

### 9.6 Documentation drift

> <a id="spec-51"></a>**SPEC-51** · P2 · docs
>
> **Problem.**
> - `bom.rs:42-48` and `:94-98` say language skills go to `.agents/skills/` and that `$workspace` means "agent's workspace skill dir". Neither has been true since v22.1.0.
> - README lines 1454 and 1731 list Vibe as native-only; v22.6.0 made it a cross-client reader.
> - The `CROSS_CLIENT_SKILL_DIR` doc comment names Gemini CLI, which is not in the catalog.
> - The `command` and section tracker categories are not documented in AGENTS.md.
> - README lists a `merge --list-models` / `-L` option that the CLI does not have.
>
>
> **Proposal.** Fix the comments and README in one docs pass. From now on, this spec is updated in the same change as the behavior.

---

## 10. Target model

The issue boxes above converge on a small set of rules. They are listed here so fixes can be done one at a time without losing the overall direction.

### 10.1 One definition of installed

- A language or agent is **installed** if and only if it owns at least one tracker entry.
- `init` is the only command that makes something installed. `remove` is the only command that makes something not installed.
- Markers are hints. They appear in `status` as "detected, not installed" and nowhere else.
- Placement, `update` scope, `merge` scope, `remove` checks and `status` all use the installed set.

Resolves: [SPEC-19](#spec-19), [SPEC-31](#spec-31), [SPEC-36](#spec-36), [SPEC-37](#spec-37), [SPEC-46](#spec-46).

### 10.2 Skill placement as reconciliation

The desired placement is a pure function:

```text
placement(catalog, installed languages, installed agents) -> set of (skill, directory)
```

For each skill:

| Skill comes from | Requirer | Directories |
|---|---|---|
| top-level list | workspace | the shared directory set |
| language L (incl. includes) | L | the shared directory set |
| agent A | A | A's `skill_dir` |
| explicit full-path target | its list's requirer | that path |

The **shared directory set** is:
- `.agents/skills/` if any installed agent is a cross-client reader that does not override the skill ([SPEC-27](#spec-27)), or if no agent is installed;
- plus the `skill_dir` of every installed native-only agent.

A skill is in the placement while at least one requirer is installed (the workspace requirer always is).

Every command that changes the installed set (`init`, `remove`) and bare `update` reconcile tracked skill copies against the placement **after** the change:

| Copy state | In placement | Not in placement |
|---|---|---|
| missing | create | nothing |
| tracked, pristine, template unchanged | nothing | delete |
| tracked, pristine, template changed | refresh | delete |
| tracked, modified | keep, report "modified, run merge" | keep, report "no longer needed, modified" |
| untracked | report, replace only with `--force` | never touch |

Ownership arrays on skill entries become informational: they record which requirers needed the copy when it was written, for `status` and `remove` reports. Deletion decisions come from the placement, never from `ref_count` or physical location.

This removes hydration, the delete-by-location rule, owner stamping from flags, and stale detection on narrowed sets. Resolves: [SPEC-20](#spec-20), [SPEC-21](#spec-21), [SPEC-22](#spec-22), [SPEC-23](#spec-23), [SPEC-24](#spec-24), [SPEC-27](#spec-27), and the skill part of [SPEC-39](#spec-39).

The same reconciliation can later replace the per-file logic for language and agent files. Skills come first because their placement depends on the agent set, which is where today's bugs cluster.

### 10.3 Two hashes per tracked file

Implemented in v23.3.4 as `original_sha` (installed) plus an optional `template_sha` ([SPEC-10](#spec-10)). Statuses:

| Status | Condition | Automatic refresh |
|---|---|---|
| pristine | disk = installed = template | yes, when the template changes |
| merged | disk = installed ≠ template | no; `update` points to `merge` when the template changes |
| modified | disk ≠ installed | no; `--force` or `merge` |
| registered | `origin: user` ([SPEC-11](#spec-11), not implemented yet) | no; treated as modified |

### 10.4 Files outside the workspace

`$userprofile` targets are installed and refreshed, marked `scope: user`, and never deleted by a workspace command ([SPEC-25](#spec-25)).

### 10.5 Safety guarantees

- Dry runs write nothing ([SPEC-48](#spec-48)).
- Write, then record. Delete, then unrecord ([SPEC-14](#spec-14)).
- Untracked files are never deleted ([SPEC-39](#spec-39), done).
- Registration is explicit ([SPEC-11](#spec-11)).
- `--yes` answers prompts; `--force` replaces customized files ([SPEC-41](#spec-41)).
- AGENTS.md is assembled from the full installed state and the stored mission ([SPEC-16](#spec-16)).

---

## 11. Issue register

All issues are open unless marked resolved.

| ID | Sev | Area | Summary | Fix direction |
|---|---|---|---|---|
| [SPEC-10](#spec-10) | P0 | tracker | `update`/`init` overwrite merged files | **resolved in v23.3.4** |
| [SPEC-11](#spec-11) | P0 | registration | user files registered as pristine, then overwritten or deleted; runs from read-only commands | explicit registration, `origin: user` |
| [SPEC-22](#spec-22) | P0 | update | `update --agent X` deletes other agents' same-named skills | reconcile against full placement |
| [SPEC-25](#spec-25) | P0 | `$userprofile` | workspace `remove` deletes global skills | never delete outside the workspace |
| [SPEC-39](#spec-39) | P0 | remove | deletes untracked and modified user files | **resolved in v23.3.5** |
| [SPEC-01](#spec-01) | P1 | config | malformed config silently ignored | hard error |
| [SPEC-05](#spec-05) | P1 | URL skills | `init` downloads live, `update` reads cache by wrong name | rewrite baseline URL skills at fetch |
| [SPEC-12](#spec-12) | P1 | ownership | `ref_count 0` means shared and unknown | `workspace` owner, `user` origin |
| [SPEC-14](#spec-14) | P1 | atomicity | tracker saved once; unrecord before delete | write then record |
| [SPEC-15](#spec-15) | P1 | AGENTS.md | three definitions of customized; CRLF marker | two-hash status |
| [SPEC-16](#spec-16) | P1 | AGENTS.md | assembled from flags only; languages and mission lost | assemble from installed state |
| [SPEC-17](#spec-17) | P1 | changelog | substring split in merge; source vs target check; skills unchecked | one line-based check |
| [SPEC-18](#spec-18) | P1 | terminology | "adopt" has three meanings | registration and placement |
| [SPEC-19](#spec-19) | P1 | installed | markers vs tracker; detected agents become installed via skills | tracker only (10.1) |
| [SPEC-20](#spec-20) | P1 | skills | top-level skill owners taken from flags | workspace owner |
| [SPEC-21](#spec-21) | P1 | skills | native copies have no agent owner; delete by location | placement (10.2) |
| [SPEC-23](#spec-23) | P1 | remove | cross-client-only agent wipes `.agents/skills` | placement (10.2) |
| [SPEC-24](#spec-24) | P1 | skills | hydration covers one case, fails on dropped language | placement (10.2) |
| [SPEC-26](#spec-26) | P1 | update | `--skill` matches through unrelated URL skills | fetch-time names |
| [SPEC-31](#spec-31) | P1 | update | `update --lang` installs a language | require installed |
| [SPEC-32](#spec-32) | P1 | update | partial update sees one language | all installed languages |
| [SPEC-34](#spec-34) | P1 | update | target conflicts resolved by last write | duplicate-target check |
| [SPEC-36](#spec-36) | P1 | merge | no language fallback | all installed languages |
| [SPEC-38](#spec-38) | P1 | remove | owners leak after `--all`, early return, `--purge` | release in every mode |
| [SPEC-40](#spec-40) | P1 | remove | preview overstates deletions | compute plan once |
| [SPEC-46](#spec-46) | P1 | status | agents from markers; writes tracker | tracker only, read-only |
| [SPEC-48](#spec-48) | P1 | dry run | dry runs write tracker files | in-memory only |
| [SPEC-02](#spec-02) | P2 | config | unknown fields, no value validation, scope confusion | validate |
| [SPEC-03](#spec-03) | P2 | config CLI | modes not exclusive | clap conflicts |
| [SPEC-04](#spec-04) | P2 | catalog | unknown keys ignored; ambiguous include names | reject |
| [SPEC-06](#spec-06) | P2 | skills | identity by `derive_name` vs discovered name | discovered name |
| [SPEC-07](#spec-07) | P2 | catalogs | no git for agents/models; never refreshed | `CatalogSource` everywhere |
| [SPEC-08](#spec-08) | P2 | verify | unreachable handling and freshness differ | unify |
| [SPEC-09](#spec-09) | P2 | list | language and agent skills listed twice | group by list |
| [SPEC-13](#spec-13) | P2 | tracker | unordered, unversioned, all-or-nothing load | BTreeMap, format version |
| [SPEC-27](#spec-27) | P2 | skills | same-name agent variants duplicate | override semantics |
| [SPEC-28](#spec-28) | P2 | init | wrong help text; `--force` scope | fix text |
| [SPEC-29](#spec-29) | P2 | init | dry run without cache; version check; directories | see box |
| [SPEC-33](#spec-33) | P2 | update | `--file` misses section entries | search all |
| [SPEC-35](#spec-35) | P2 | update | untracked reported as modified | separate message |
| [SPEC-37](#spec-37) | P2 | merge | merges plain copies; exit 0 on failure; stuck partials | see box |
| [SPEC-41](#spec-41) | P2 | flags | `--force` overloaded | `--yes` and `--force` |
| [SPEC-42](#spec-42) | P2 | remove | missing entries kept; removed files can return | prune |
| [SPEC-43](#spec-43) | P2 | remove | mixed canonicalization | one path helper |
| [SPEC-44](#spec-44) | P2 | doctor | wrong hints; missing checks | see box |
| [SPEC-45](#spec-45) | P2 | doctor --smart | fragile parsing | see box |
| [SPEC-47](#spec-47) | P2 | models | `--list` fails without file; wrong hints | fallback |
| [SPEC-49](#spec-49) | P2 | paths | symlinks canonicalized | lexical normalization |
| [SPEC-50](#spec-50) | P2 | workspace | no root discovery | search upward |
| [SPEC-51](#spec-51) | P2 | docs | stale comments and README | docs pass |

### Suggested order

1. **Stop data loss.** [SPEC-10](#spec-10) (done), [SPEC-39](#spec-39) (done), [SPEC-25](#spec-25), [SPEC-22](#spec-22), [SPEC-11](#spec-11), [SPEC-48](#spec-48). Each can ship alone.
2. **One definition of installed** (10.1): [SPEC-19](#spec-19), [SPEC-46](#spec-46), [SPEC-31](#spec-31), [SPEC-38](#spec-38).
3. **Skill placement** (10.2): [SPEC-18](#spec-18), [SPEC-20](#spec-20), [SPEC-21](#spec-21), [SPEC-23](#spec-23), [SPEC-24](#spec-24), [SPEC-27](#spec-27). This is a MAJOR version because skill ownership in the tracker changes meaning.
4. **URL skills and identity**: [SPEC-05](#spec-05), [SPEC-06](#spec-06), [SPEC-26](#spec-26).
5. **AGENTS.md**: [SPEC-15](#spec-15), [SPEC-16](#spec-16), [SPEC-17](#spec-17), [SPEC-36](#spec-36).
6. Everything else, P1 before P2.
