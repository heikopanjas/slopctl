---
name: Redesign merge command
overview: Extract the shared file-resolution pipeline from init into a reusable method on TemplateEngine, then rewrite merge to call that same method -- differing only in conflict strategy (LLM merge vs user prompt). Eliminates the duplicated templates.yml walk in merge.rs.
todos:
  - id: extract-resolve
    content: "Extract shared file-resolution logic from TemplateEngine::update() into a new public method resolve_all_files() that returns ResolvedFiles (TemplateContext, Vec<(PathBuf, PathBuf)>, Vec<PathBuf>)."
    status: done
  - id: refactor-init
    content: "Refactor TemplateEngine::update() to call resolve_all_files() instead of inline resolution, keeping its existing copy/prompt/track behavior."
    status: done
  - id: build-content-map
    content: "Add build_target_content_map() and generate_fresh_main() on TemplateEngine. Refactored merge_fragments to use generate_fresh_main internally."
    status: done
  - id: rewrite-merge
    content: "Rewrite merge() with New/Unchanged/Diverged classification using shared pipeline. Deleted ~400 lines of duplicated code."
    status: done
  - id: delete-dup
    content: "Deleted build_target_source_map, find_merge_candidates, generate_fresh_main, insert_source_content, insert_skill_sources, insert_skill_dir_recursive, resolve_target, normalize_path, sha256_string from merge.rs."
    status: done
  - id: test
    content: "All 246 tests pass. cargo fmt and cargo clippy clean."
    status: done
isProject: false
---

# Redesign merge to mirror init behavior (DRY)

## Problem

Two problems:

1. **Behavioral gap**: `merge` only considers files that are both user-modified AND template-changed since last install. Running `merge --agent copilot` to add a new agent does nothing because there are no tracked files to compare. It should work exactly like `init` but delegate conflict resolution to an LLM instead of prompting the user.

2. **DRY violation**: The file-resolution logic (walking templates.yml sections -- principles, mission, languages, integration, agents, skills) is duplicated between `TemplateEngine::update()` in [src/template_engine.rs](src/template_engine.rs) (lines 280-462) and `build_target_source_map()` in [src/template_manager/merge.rs](src/template_manager/merge.rs) (lines 380-505). Both walk the same config sections in the same order.

## Design

### Shared resolution pipeline

Extract the templates.yml walk into a reusable method on `TemplateEngine`:

```
resolve_all_files(&self, options: &UpdateOptions) 
    -> Result<(TemplateContext, Vec<(PathBuf, PathBuf)>, Vec<PathBuf>)>
```

This returns:
- `TemplateContext` -- main AGENTS.md source/target/fragments/version
- `Vec<(PathBuf, PathBuf)>` -- all (source, target) file pairs (agent files, language files, integration files, skills)
- `Vec<PathBuf>` -- directories to create

Both `update()` (init) and `merge` call this same method.

### How merge uses the shared pipeline

`merge` additionally needs a `HashMap<PathBuf, String>` (target -> content) so it can compare against disk and send to the LLM. A second method `build_target_content_map()` calls `resolve_all_files()`, reads each source file, and generates a fresh AGENTS.md (moving `generate_fresh_main` from merge.rs into template_engine.rs since it's template logic).

### Three-way classification in merge

For each (target, template_content) pair from the content map:

- **New** -- target does not exist on disk: write template content directly
- **Unchanged** -- target exists and SHA matches template: skip (report with `--verbose`)
- **Diverged** -- target exists and SHA differs: send to LLM for AI merge (or write sidecar with `--preview`)

All written files (New + Diverged) are recorded in FileTracker. To force-overwrite without AI, the user can use `init --force` instead.

## File changes

### [src/template_engine.rs](src/template_engine.rs)

- Extract the templates.yml walk (lines 280-462 of `update()`) into `resolve_all_files(&self, options: &UpdateOptions) -> Result<ResolvedFiles>` where `ResolvedFiles` is a struct holding `TemplateContext`, `files_to_copy`, and `directories_to_create`.
- Move `generate_fresh_main()` from merge.rs here as a method or associated function (it's pure template logic).
- Add `build_target_content_map()` that calls `resolve_all_files()`, reads sources, and returns `HashMap<PathBuf, String>`.
- Refactor `update()` to call `resolve_all_files()` then proceed with its existing copy/prompt/track logic.

### [src/template_manager/merge.rs](src/template_manager/merge.rs)

- Delete `build_target_source_map`, `find_merge_candidates`, `generate_fresh_main`, `insert_source_content`, `insert_skill_sources`, `insert_skill_dir_recursive`, `resolve_target`, `normalize_path` (all replaced by shared pipeline).
- Rewrite `merge()` to call `TemplateEngine::build_target_content_map()`, classify entries, and handle New/Unchanged/Diverged.
- Keep `MergeOptions`, `MERGE_SYSTEM_PROMPT`, `build_merge_messages`, LLM interaction, and token/sidecar helpers.

### [src/main.rs](src/main.rs)

- No changes needed (no new flags).
