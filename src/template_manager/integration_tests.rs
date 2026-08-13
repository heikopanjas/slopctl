//! Integration tests exercising init→remove sequences across all three
//! canonical test agent archetypes (bogus, fake, foobar) and both synthetic
//! languages (Rust++, CppScript).

use std::{fs, path::Path};

use super::cwd_test_guard;
use crate::{
    FileTracker, MergeOptions, TemplateManager, UpdateOptions,
    agent_defaults::AGENT_DEFAULTS_FILE,
    template_engine::{CHANGELOG_MARKER, TEMPLATE_MARKER}
};

// ── Fixture ──────────────────────────────────────────────────────────────────

/// Self-contained config directory for integration tests.
///
/// Holds a populated `config_dir` TempDir with templates.yml,
/// agent-defaults.yml, AGENTS.md, and every source file referenced by the
/// template config.  Each test creates its own fixture to avoid cross-test
/// contamination.
struct IntegrationFixture
{
    config_dir: tempfile::TempDir
}

impl IntegrationFixture
{
    fn new() -> anyhow::Result<Self>
    {
        let config_dir = tempfile::TempDir::new()?;
        let d = config_dir.path();

        // ── templates.yml ────────────────────────────────────────────────
        fs::write(
            d.join("templates.yml"),
            r#"version: 5

main:
  source: AGENTS.md
  target: '$workspace/AGENTS.md'

agents:
  bogus:
    instructions:
      - source: bogus/instructions.md
        target: '$workspace/.bogus/instructions.md'
  fake:
    prompts:
      - source: fake/commands/init-session.md
        target: '$workspace/.fake/commands/init-session.md'
  foobar: {}

shared:
  cmake:
    files:
      - source: cmake-hint.md
        target: '$instructions'
    skills:
      - source: 'skills/cmake-build-commands'

languages:
  Rust++:
    files:
      - source: rpp-format.toml
        target: '$workspace/.rpp.toml'
      - source: shared-gitignore
        target: '$workspace/.gitignore'
      - source: rpp-hint.md
        target: '$instructions'
    skills:
      - source: 'skills/rpp-coding-conventions'
  CppScript:
    includes: [cmake]
    files:
      - source: cppscript-format.json
        target: '$workspace/.cppscript-format'
      - source: shared-gitignore
        target: '$workspace/.gitignore'

skills:
  - source: 'skills/git-workflow'
  - source: 'skills/semantic-versioning'

integration:
  git:
    files:
      - source: git-workflow-summary.md
        target: '$instructions'
  updates:
    files:
      - source: UPDATES.md
        target: '$workspace/UPDATES.md'
      - source: recent-updates-summary.md
        target: '$instructions'

principles:
  - source: core-principles.md
    target: '$instructions'

mission:
  - source: mission-statement.md
    target: '$instructions'
"#
        )?;

        // ── AGENTS.md source with marker + insertion points ──────────────
        fs::write(
            d.join("AGENTS.md"),
            format!(
                "{}\n# Project\n\n<!-- {{preamble}} -->\n\n<!-- {{mission}} -->\n\n<!-- {{principles}} -->\n\n<!-- {{languages}} -->\n\n<!-- {{integration}} -->\n",
                TEMPLATE_MARKER
            )
        )?;

        // ── agent-defaults.yml ───────────────────────────────────────────
        fs::write(
            d.join(AGENT_DEFAULTS_FILE),
            r#"version: 1
agents:
  - name: bogus
    markers:
      - .bogus
    prompt_dir: '$workspace/.bogus/prompts'
    skill_dir: '$workspace/.bogus/skills'
    reads_cross_client_skills: false
  - name: fake
    markers:
      - .fake
    prompt_dir: '$workspace/.fake/commands'
    skill_dir: '$workspace/.fake/skills'
    reads_cross_client_skills: true
  - name: foobar
    markers:
      - .foobar
    prompt_dir: '$workspace/.foobar/commands'
    skill_dir: '$workspace/.agents/skills'
    reads_cross_client_skills: true
"#
        )?;

        // ── Agent source files ───────────────────────────────────────────
        fs::create_dir_all(d.join("bogus"))?;
        fs::write(d.join("bogus/instructions.md"), "# Bogus instructions\n")?;

        fs::create_dir_all(d.join("fake/commands"))?;
        fs::write(d.join("fake/commands/init-session.md"), "# Fake init-session\n")?;

        // ── Language source files ────────────────────────────────────────
        fs::write(d.join("rpp-format.toml"), "max_width = 167\n")?;
        fs::write(d.join("rpp-hint.md"), "## Rust++ Conventions\n")?;
        fs::write(d.join("cppscript-format.json"), "{}\n")?;
        fs::write(d.join("shared-gitignore"), "target/\n")?;
        fs::write(d.join("cmake-hint.md"), "## CMake Conventions\n")?;

        // ── Fragment source files ────────────────────────────────────────
        fs::write(d.join("git-workflow-summary.md"), "## Git Workflow\n")?;
        fs::write(d.join("recent-updates-summary.md"), "## Recent Updates\n")?;
        fs::write(d.join("core-principles.md"), "## Principles\n")?;
        fs::write(d.join("mission-statement.md"), "## Mission\n")?;

        // ── Changelog-marker file (user-owned log tail below the marker) ─
        fs::write(d.join("UPDATES.md"), format!("# Recent Updates & Decisions\n\n{}\n\n### 2025-01-01 (v0.1.0, seed)\n\n- seed entry\n", CHANGELOG_MARKER))?;

        // ── Skill directories ────────────────────────────────────────────
        for skill in &["git-workflow", "semantic-versioning", "rpp-coding-conventions", "cmake-build-commands"]
        {
            let skill_dir = d.join("skills").join(skill);
            fs::create_dir_all(&skill_dir)?;
            fs::write(skill_dir.join("SKILL.md"), format!("---\nname: {skill}\n---\n# {skill}\n"))?;
        }

        Ok(Self { config_dir })
    }

    fn manager(&self) -> TemplateManager
    {
        TemplateManager { config_dir: self.config_dir.path().to_path_buf() }
    }

    fn init(&self, agent: Option<&str>, lang: Option<&str>) -> anyhow::Result<()>
    {
        let options = UpdateOptions { lang, agent, mission: None, force: false, dry_run: false, partial: None, local_cache_only: false };
        self.manager().update(&options)
    }

    fn init_force(&self, agent: Option<&str>, lang: Option<&str>) -> anyhow::Result<()>
    {
        let options = UpdateOptions { lang, agent, mission: None, force: true, dry_run: false, partial: None, local_cache_only: false };
        self.manager().update(&options)
    }

    fn remove_all(&self) -> anyhow::Result<()>
    {
        self.manager().remove(None, None, true, false)
    }

    fn remove_agent(&self, agent: &str) -> anyhow::Result<()>
    {
        self.manager().remove(Some(agent), None, true, false)
    }

    fn remove_lang(&self, lang: &str) -> anyhow::Result<()>
    {
        self.manager().remove(None, Some(lang), true, false)
    }

    fn doctor(&self, fix: bool, dry_run: bool) -> anyhow::Result<()>
    {
        self.manager().doctor(fix, dry_run, false, false)
    }

    fn merge_dry_run(&self, agent: Option<&str>, lang: Option<&str>) -> anyhow::Result<()>
    {
        let options = MergeOptions { lang, agent, mission: None };
        self.manager().merge(&options, true, false, false)
    }

    fn verify(&self) -> anyhow::Result<()>
    {
        let source = self.config_dir.path().to_string_lossy().to_string();
        self.manager().verify(&source)
    }

    fn status(&self) -> anyhow::Result<()>
    {
        self.manager().status(false)
    }
}

// ── Single-operation sanity ──────────────────────────────────────────────────

#[test]
fn test_init_bogus_with_rpp() -> anyhow::Result<()>
{
    let _g = cwd_test_guard();
    let fixture = IntegrationFixture::new()?;
    let workspace = tempfile::TempDir::new()?;
    std::env::set_current_dir(workspace.path())?;

    fixture.init(Some("bogus"), Some("Rust++"))?;

    // Filesystem assertions
    assert!(workspace.path().join("AGENTS.md").exists() == true, "AGENTS.md must be created");
    assert!(workspace.path().join(".bogus/instructions.md").exists() == true, "agent instruction file must exist");
    assert!(workspace.path().join(".rpp.toml").exists() == true, "language config file must exist");

    // Native-only agent: skills go to .bogus/skills/, NOT .agents/skills/
    let bogus_skills = workspace.path().join(".bogus/skills");
    assert!(bogus_skills.exists() == true, "native skill dir must exist for bogus");
    assert!(has_skill_md_under(&bogus_skills) == true, "skills must be installed in .bogus/skills/");

    // Tracker assertions — use current_dir() for consistency with production code on Windows
    let cwd = std::env::current_dir()?;
    let tracker = FileTracker::new(&cwd)?;
    assert_eq!(tracker.get_installed_language(), Some("Rust++".to_string()));

    Ok(())
}

#[test]
fn test_init_fake_with_rpp() -> anyhow::Result<()>
{
    let _g = cwd_test_guard();
    let fixture = IntegrationFixture::new()?;
    let workspace = tempfile::TempDir::new()?;
    std::env::set_current_dir(workspace.path())?;

    fixture.init(Some("fake"), Some("Rust++"))?;

    // Filesystem assertions
    assert!(workspace.path().join("AGENTS.md").exists() == true);
    assert!(workspace.path().join(".fake/commands/init-session.md").exists() == true, "agent prompt must exist");
    assert!(workspace.path().join(".rpp.toml").exists() == true);

    // Cross-client agent: skills go to .agents/skills/, NOT .fake/skills/
    let cross_client_skills = workspace.path().join(".agents/skills");
    assert!(cross_client_skills.exists() == true, "cross-client skill dir must exist");
    assert!(has_skill_md_under(&cross_client_skills) == true, "skills must be installed in .agents/skills/");

    let tracker = FileTracker::new(&std::env::current_dir()?)?;
    assert_eq!(tracker.get_installed_language(), Some("Rust++".to_string()));

    Ok(())
}

#[test]
fn test_init_foobar_with_cppscript() -> anyhow::Result<()>
{
    let _g = cwd_test_guard();
    let fixture = IntegrationFixture::new()?;
    let workspace = tempfile::TempDir::new()?;
    std::env::set_current_dir(workspace.path())?;

    fixture.init(Some("foobar"), Some("CppScript"))?;

    assert!(workspace.path().join("AGENTS.md").exists() == true);
    assert!(workspace.path().join(".cppscript-format").exists() == true, "CppScript config must exist");

    // foobar's skill_dir IS .agents/skills (cross-client-only archetype)
    let cross_client_skills = workspace.path().join(".agents/skills");
    assert!(cross_client_skills.exists() == true);
    assert!(has_skill_md_under(&cross_client_skills) == true);

    // CppScript includes cmake shared group — cmake-build-commands skill must be present
    assert!(cross_client_skills.join("cmake-build-commands/SKILL.md").exists() == true, "cmake skill inherited via shared include must exist");

    let tracker = FileTracker::new(&std::env::current_dir()?)?;
    assert_eq!(tracker.get_installed_language(), Some("CppScript".to_string()));

    Ok(())
}

// ── Remove preserves sibling scope ───────────────────────────────────────────

#[test]
fn test_init_then_remove_agent_preserves_lang() -> anyhow::Result<()>
{
    let _g = cwd_test_guard();
    let fixture = IntegrationFixture::new()?;
    let workspace = tempfile::TempDir::new()?;
    std::env::set_current_dir(workspace.path())?;

    fixture.init(Some("fake"), Some("Rust++"))?;

    // Verify init succeeded
    assert!(workspace.path().join(".fake/commands/init-session.md").exists() == true);
    assert!(workspace.path().join(".rpp.toml").exists() == true);

    // Remove agent only
    fixture.remove_agent("fake")?;

    // Agent artifacts must be gone
    assert!(workspace.path().join(".fake/commands/init-session.md").exists() == false, "agent prompt must be deleted");

    // Language artifacts must survive
    assert!(workspace.path().join(".rpp.toml").exists() == true, "language config must survive agent removal");
    assert!(workspace.path().join("AGENTS.md").exists() == true, "AGENTS.md must survive agent removal");

    let tracker = FileTracker::new(&std::env::current_dir()?)?;
    assert_eq!(tracker.get_installed_language(), Some("Rust++".to_string()), "tracker must still report language");

    Ok(())
}

#[test]
fn test_init_then_remove_lang_preserves_agent() -> anyhow::Result<()>
{
    let _g = cwd_test_guard();
    let fixture = IntegrationFixture::new()?;
    let workspace = tempfile::TempDir::new()?;
    std::env::set_current_dir(workspace.path())?;

    fixture.init(Some("bogus"), Some("CppScript"))?;

    assert!(workspace.path().join(".bogus/instructions.md").exists() == true);
    assert!(workspace.path().join(".cppscript-format").exists() == true);

    fixture.remove_lang("CppScript")?;

    // Language artifacts must be gone
    assert!(workspace.path().join(".cppscript-format").exists() == false, "language file must be deleted");

    // Agent artifacts must survive
    assert!(workspace.path().join(".bogus/instructions.md").exists() == true, "agent file must survive lang removal");
    assert!(workspace.path().join("AGENTS.md").exists() == true, "AGENTS.md must survive lang removal");

    let tracker = FileTracker::new(&std::env::current_dir()?)?;
    assert!(tracker.get_installed_language().is_none() == true, "tracker must report no language after removal");

    Ok(())
}

#[test]
fn test_init_then_remove_agent_then_remove_lang_leaves_clean() -> anyhow::Result<()>
{
    let _g = cwd_test_guard();
    let fixture = IntegrationFixture::new()?;
    let workspace = tempfile::TempDir::new()?;
    std::env::set_current_dir(workspace.path())?;

    fixture.init(Some("fake"), Some("Rust++"))?;

    fixture.remove_agent("fake")?;
    fixture.remove_lang("Rust++")?;

    // Only AGENTS.md should remain (it is never deleted by remove --agent or --lang)
    assert!(workspace.path().join("AGENTS.md").exists() == true, "AGENTS.md must survive both removals");
    assert!(workspace.path().join(".rpp.toml").exists() == false, "language file must be gone");
    assert!(workspace.path().join(".fake").exists() == false, "agent marker dir must be gone");

    let tracker = FileTracker::new(&std::env::current_dir()?)?;
    assert!(tracker.get_installed_language().is_none() == true);

    let agent_entries = tracker.get_entries_by_category("agent");
    assert!(agent_entries.is_empty() == true, "no agent entries should remain in tracker");

    Ok(())
}

// ── Agent switching ──────────────────────────────────────────────────────────

#[test]
fn test_init_cross_client_then_native_adopts_skills() -> anyhow::Result<()>
{
    let _g = cwd_test_guard();
    let fixture = IntegrationFixture::new()?;
    let workspace = tempfile::TempDir::new()?;
    std::env::set_current_dir(workspace.path())?;

    // First: install cross-client agent → skills go to .agents/skills/
    fixture.init(Some("fake"), Some("Rust++"))?;
    let cross_client_skills = workspace.path().join(".agents/skills");
    assert!(has_skill_md_under(&cross_client_skills) == true, "cross-client skills must exist after first init");

    // Second: install native-only agent → should adopt skills into .bogus/skills/
    fixture.init(Some("bogus"), Some("Rust++"))?;
    let native_skills = workspace.path().join(".bogus/skills");
    assert!(has_skill_md_under(&native_skills) == true, "skills must be adopted into .bogus/skills/ for native-only agent");

    Ok(())
}

#[test]
fn test_init_agent_then_different_agent_coexist() -> anyhow::Result<()>
{
    let _g = cwd_test_guard();
    let fixture = IntegrationFixture::new()?;
    let workspace = tempfile::TempDir::new()?;
    std::env::set_current_dir(workspace.path())?;

    // First: install native-only agent with a language
    fixture.init(Some("bogus"), Some("Rust++"))?;
    assert!(workspace.path().join(".bogus/instructions.md").exists() == true);

    // Second: install cross-client agent (agent-only, no lang)
    fixture.init(Some("fake"), None)?;
    assert!(workspace.path().join(".fake/commands/init-session.md").exists() == true);

    // Both agent marker dirs must exist
    assert!(workspace.path().join(".bogus").exists() == true, "bogus marker must survive second init");
    assert!(workspace.path().join(".fake").exists() == true, "fake marker must exist after second init");

    // Both agents have tracked files
    let tracker = FileTracker::new(&std::env::current_dir()?)?;
    let agent_entries = tracker.get_entries_by_category("agent");
    assert!(agent_entries.len() >= 2, "both agents should have tracked entries");

    Ok(())
}

// ── Multi-language installation ──────────────────────────────────────────────

#[test]
fn test_init_lang_then_different_lang_succeeds_when_targets_do_not_conflict() -> anyhow::Result<()>
{
    let _g = cwd_test_guard();
    let fixture = IntegrationFixture::new()?;
    let workspace = tempfile::TempDir::new()?;
    std::env::set_current_dir(workspace.path())?;

    fixture.init(Some("fake"), Some("Rust++"))?;
    assert_eq!(FileTracker::new(&std::env::current_dir()?)?.get_installed_language(), Some("Rust++".to_string()));

    fixture.init(Some("fake"), Some("CppScript"))?;

    assert!(workspace.path().join(".rpp.toml").exists() == true, "first language file must remain");
    assert!(workspace.path().join(".cppscript-format").exists() == true, "second language file must be installed");
    let tracker = FileTracker::new(&std::env::current_dir()?)?;
    let installed = tracker.get_installed_languages();
    assert_eq!(installed, vec!["CppScript".to_string(), "Rust++".to_string()]);
    let gitignore_meta = tracker.get_metadata(&workspace.path().join(".gitignore")).ok_or_else(|| anyhow::anyhow!("missing shared gitignore metadata"))?;
    assert_eq!(gitignore_meta.lang, vec!["CppScript".to_string(), "Rust++".to_string()]);
    assert_eq!(gitignore_meta.ref_count, 2);

    Ok(())
}

#[test]
fn test_init_second_lang_blocks_conflicting_shared_file() -> anyhow::Result<()>
{
    let _g = cwd_test_guard();
    let fixture = IntegrationFixture::new()?;
    let workspace = tempfile::TempDir::new()?;
    std::env::set_current_dir(workspace.path())?;

    fixture.init(Some("fake"), Some("Rust++"))?;

    fs::write(fixture.config_dir.path().join("cppscript-gitignore"), "build/\n")?;
    let templates_path = fixture.config_dir.path().join("templates.yml");
    let templates = fs::read_to_string(&templates_path)?;
    let updated = templates.replace(
        "      - source: cppscript-format.json\n        target: '$workspace/.cppscript-format'\n      - source: shared-gitignore\n        target: \
         '$workspace/.gitignore'",
        "      - source: cppscript-format.json\n        target: '$workspace/.cppscript-format'\n      - source: cppscript-gitignore\n        target: \
         '$workspace/.gitignore'"
    );
    fs::write(&templates_path, updated)?;

    let result = fixture.init(Some("fake"), Some("CppScript"));

    assert!(result.is_err() == true, "different shared file content must be rejected");
    let message = result.unwrap_err().to_string();
    assert!(message.contains(".gitignore") == true);
    assert!(message.contains("slopctl merge") == true);
    assert!(workspace.path().join(".cppscript-format").exists() == false, "failed preflight must not write second language files");

    Ok(())
}

#[test]
fn test_remove_lang_then_init_different_lang_succeeds() -> anyhow::Result<()>
{
    let _g = cwd_test_guard();
    let fixture = IntegrationFixture::new()?;
    let workspace = tempfile::TempDir::new()?;
    std::env::set_current_dir(workspace.path())?;

    fixture.init(Some("fake"), Some("Rust++"))?;
    assert!(workspace.path().join(".rpp.toml").exists() == true);
    assert!(workspace.path().join(".gitignore").exists() == true);
    assert!(workspace.path().join(".agents/skills/rpp-coding-conventions/SKILL.md").exists() == true);

    fixture.remove_lang("Rust++")?;
    assert!(workspace.path().join(".rpp.toml").exists() == false);
    assert!(workspace.path().join(".gitignore").exists() == false);
    assert!(workspace.path().join(".agents/skills/rpp-coding-conventions/SKILL.md").exists() == false);
    assert!(FileTracker::new(&std::env::current_dir()?)?.get_installed_language().is_none() == true);

    // Now a different language must be accepted
    fixture.init(Some("fake"), Some("CppScript"))?;
    assert!(workspace.path().join(".cppscript-format").exists() == true, "new language file must appear");
    assert!(workspace.path().join(".gitignore").exists() == true, "new language's shared file must appear");
    assert!(workspace.path().join(".agents/skills/cmake-build-commands/SKILL.md").exists() == true, "included language skill must appear");
    assert!(workspace.path().join(".agents/skills/rpp-coding-conventions/SKILL.md").exists() == false, "removed language skill must stay absent");
    assert_eq!(FileTracker::new(&std::env::current_dir()?)?.get_installed_language(), Some("CppScript".to_string()));

    Ok(())
}

// ── Cross-client cleanup edge cases ──────────────────────────────────────────

#[test]
fn test_remove_last_cross_client_cleans_agents_skills() -> anyhow::Result<()>
{
    let _g = cwd_test_guard();
    let fixture = IntegrationFixture::new()?;
    let workspace = tempfile::TempDir::new()?;
    std::env::set_current_dir(workspace.path())?;

    fixture.init(Some("fake"), Some("Rust++"))?;

    let cross_client_dir = workspace.path().join(".agents/skills");
    assert!(cross_client_dir.exists() == true, "cross-client skills must exist after init");

    // Track a language skill manually so we can verify it survives
    let lang_skill_dir = cross_client_dir.join("rpp-coding-conventions");
    assert!(lang_skill_dir.exists() == true, "language skill must be installed");

    // Remove fake — it is the last (only) cross-client agent
    fixture.remove_agent("fake")?;

    // Top-level skills are still referenced by the language install, so removing
    // the last cross-client agent must not delete them.
    let git_skill = cross_client_dir.join("git-workflow/SKILL.md");
    assert!(git_skill.exists() == true, "top-level skill must survive while still language-owned");

    // Language skills must survive (owned by Rust++, not by the agent)
    let tracker = FileTracker::new(&std::env::current_dir()?)?;
    assert_eq!(tracker.get_installed_language(), Some("Rust++".to_string()), "language must still be installed");

    Ok(())
}

#[test]
fn test_remove_one_cross_client_preserves_agents_skills() -> anyhow::Result<()>
{
    let _g = cwd_test_guard();
    let fixture = IntegrationFixture::new()?;
    let workspace = tempfile::TempDir::new()?;
    std::env::set_current_dir(workspace.path())?;

    // Install two cross-client agents
    fixture.init(Some("fake"), Some("Rust++"))?;
    fixture.init(Some("foobar"), None)?;

    let cross_client_dir = workspace.path().join(".agents/skills");
    assert!(cross_client_dir.exists() == true);

    let git_skill = cross_client_dir.join("git-workflow/SKILL.md");
    assert!(git_skill.exists() == true, "top-level skill must exist before removal");

    // Remove fake — foobar still reads .agents/skills/
    fixture.remove_agent("fake")?;

    assert!(git_skill.exists() == true, "top-level skill must survive when another cross-client agent remains");
    assert!(workspace.path().join(".foobar").exists() == true, "foobar marker must still exist");

    Ok(())
}

// ── Doctor after init ────────────────────────────────────────────────────────

#[test]
fn test_doctor_clean_after_init() -> anyhow::Result<()>
{
    let _g = cwd_test_guard();
    let fixture = IntegrationFixture::new()?;
    let workspace = tempfile::TempDir::new()?;
    std::env::set_current_dir(workspace.path())?;

    fixture.init(Some("fake"), Some("Rust++"))?;

    let entries_before = FileTracker::new(&std::env::current_dir()?)?.get_entries().len();

    // Doctor on a clean workspace must succeed and not modify the tracker
    fixture.doctor(false, false)?;

    let entries_after = FileTracker::new(&std::env::current_dir()?)?.get_entries().len();
    assert_eq!(entries_before, entries_after, "doctor must not modify tracker on a clean workspace");

    Ok(())
}

#[test]
fn test_doctor_detects_missing_file_after_deletion() -> anyhow::Result<()>
{
    let _g = cwd_test_guard();
    let fixture = IntegrationFixture::new()?;
    let workspace = tempfile::TempDir::new()?;
    std::env::set_current_dir(workspace.path())?;

    fixture.init(Some("fake"), Some("Rust++"))?;

    let cwd = std::env::current_dir()?;
    let rpp_file = cwd.join(".rpp.toml");
    assert!(rpp_file.exists() == true);
    fs::remove_file(&rpp_file)?;

    // Doctor without fix: succeeds (prints issues but doesn't return Err)
    fixture.doctor(false, false)?;

    // The tracker still has the entry (fix was not requested)
    let tracker = FileTracker::new(&cwd)?;
    assert!(tracker.get_metadata(&rpp_file).is_some() == true, "tracker must still have the stale entry before fix");

    // Doctor with fix: prunes the stale tracker entry
    fixture.doctor(true, false)?;

    let tracker_after = FileTracker::new(&cwd)?;
    assert!(tracker_after.get_metadata(&rpp_file).is_none() == true, "tracker must prune missing file after doctor --fix");

    Ok(())
}

#[test]
fn test_doctor_detects_modified_file() -> anyhow::Result<()>
{
    let _g = cwd_test_guard();
    let fixture = IntegrationFixture::new()?;
    let workspace = tempfile::TempDir::new()?;
    std::env::set_current_dir(workspace.path())?;

    fixture.init(Some("bogus"), Some("CppScript"))?;

    let config_file = workspace.path().join(".cppscript-format");
    assert!(config_file.exists() == true);
    fs::write(&config_file, "{ \"modified\": true }\n")?;

    // Doctor reports modified files as informational — does not return error
    fixture.doctor(false, false)?;

    // Doctor must NOT modify or delete the file (modified files have no automatic fix)
    let content = fs::read_to_string(&config_file)?;
    assert!(content.contains("modified") == true, "doctor must not touch modified files");

    Ok(())
}

#[test]
fn test_doctor_fix_strips_unmerged_marker() -> anyhow::Result<()>
{
    let _g = cwd_test_guard();
    let fixture = IntegrationFixture::new()?;
    let workspace = tempfile::TempDir::new()?;
    std::env::set_current_dir(workspace.path())?;

    fixture.init(Some("fake"), Some("Rust++"))?;

    // After init, AGENTS.md has fragments merged and marker stripped.
    // Re-insert the marker to simulate an unmerged template state, then
    // update the tracker SHA so doctor sees it as Unmodified (not Modified).
    let agents_md = workspace.path().join("AGENTS.md");
    let content = fs::read_to_string(&agents_md)?;
    fs::write(&agents_md, format!("{}\n{}", TEMPLATE_MARKER, content))?;

    let new_sha = FileTracker::calculate_sha256(&agents_md)?;
    let mut tracker = FileTracker::new(&std::env::current_dir()?)?;
    tracker.record_installation(&agents_md, new_sha, 5, "Rust++".into(), "all".into(), "main".into());
    tracker.save()?;

    let content_before = fs::read_to_string(&agents_md)?;
    assert!(content_before.contains(TEMPLATE_MARKER) == true, "AGENTS.md must have the marker for this test");

    // Doctor with fix should strip the template marker
    fixture.doctor(true, false)?;

    let content_after = fs::read_to_string(&agents_md)?;
    assert!(content_after.contains(TEMPLATE_MARKER) == false, "marker must be stripped after doctor --fix");
    assert!(content_after.contains("# Project") == true, "content must be preserved after marker stripping");

    Ok(())
}

#[test]
fn test_doctor_clean_after_remove() -> anyhow::Result<()>
{
    let _g = cwd_test_guard();
    let fixture = IntegrationFixture::new()?;
    let workspace = tempfile::TempDir::new()?;
    std::env::set_current_dir(workspace.path())?;

    fixture.init(Some("fake"), Some("Rust++"))?;
    fixture.remove_agent("fake")?;

    // Doctor after removal must not crash from stale state
    fixture.doctor(false, false)?;

    Ok(())
}

// ── Status after init/remove ─────────────────────────────────────────────────

#[test]
fn test_status_after_init() -> anyhow::Result<()>
{
    let _g = cwd_test_guard();
    let fixture = IntegrationFixture::new()?;
    let workspace = tempfile::TempDir::new()?;
    std::env::set_current_dir(workspace.path())?;

    fixture.init(Some("bogus"), Some("Rust++"))?;

    fixture.status()?;
    assert!(TemplateManager::is_workspace_initialized(workspace.path()) == true, "workspace must be initialized after init");

    Ok(())
}

#[test]
fn test_status_after_remove_agent() -> anyhow::Result<()>
{
    let _g = cwd_test_guard();
    let fixture = IntegrationFixture::new()?;
    let workspace = tempfile::TempDir::new()?;
    std::env::set_current_dir(workspace.path())?;

    fixture.init(Some("fake"), Some("Rust++"))?;
    fixture.remove_agent("fake")?;

    fixture.status()?;
    assert!(
        TemplateManager::is_workspace_initialized(workspace.path()) == true,
        "workspace must still be initialized after agent removal (AGENTS.md + tracker remain)"
    );

    Ok(())
}

#[test]
fn test_status_not_initialized_on_empty_workspace() -> anyhow::Result<()>
{
    let workspace = tempfile::TempDir::new()?;

    assert!(TemplateManager::is_workspace_initialized(workspace.path()) == false, "empty workspace must not be reported as initialized");

    Ok(())
}

// ── Merge dry-run after init ─────────────────────────────────────────────────

#[test]
fn test_merge_dry_run_all_unchanged() -> anyhow::Result<()>
{
    let _g = cwd_test_guard();
    let fixture = IntegrationFixture::new()?;
    let workspace = tempfile::TempDir::new()?;
    std::env::set_current_dir(workspace.path())?;

    fixture.init(Some("fake"), Some("Rust++"))?;

    // Immediately after init, all files match the template — merge should find nothing to do
    fixture.merge_dry_run(Some("fake"), Some("Rust++"))?;

    Ok(())
}

#[test]
fn test_merge_dry_run_detects_diverged_file() -> anyhow::Result<()>
{
    let _g = cwd_test_guard();
    let fixture = IntegrationFixture::new()?;
    let workspace = tempfile::TempDir::new()?;
    std::env::set_current_dir(workspace.path())?;

    fixture.init(Some("fake"), Some("Rust++"))?;

    // Modify AGENTS.md to simulate user customization
    let agents_md = workspace.path().join("AGENTS.md");
    let original = fs::read_to_string(&agents_md)?;
    fs::write(&agents_md, format!("{original}\n## My Custom Section\n"))?;

    // Merge with dry_run=true: detects the divergence but writes nothing
    fixture.merge_dry_run(Some("fake"), Some("Rust++"))?;

    // Verify AGENTS.md is unchanged by the dry run
    let after = fs::read_to_string(&agents_md)?;
    assert!(after.contains("My Custom Section") == true, "dry-run must not modify diverged files");

    Ok(())
}

#[test]
fn test_merge_dry_run_after_remove_lang() -> anyhow::Result<()>
{
    let _g = cwd_test_guard();
    let fixture = IntegrationFixture::new()?;
    let workspace = tempfile::TempDir::new()?;
    std::env::set_current_dir(workspace.path())?;

    fixture.init(Some("fake"), Some("Rust++"))?;
    fixture.remove_lang("Rust++")?;

    // Merge with a different language in dry-run — should detect new files
    fixture.merge_dry_run(Some("fake"), Some("CppScript"))?;

    // Dry-run must not create the CppScript config file
    assert!(workspace.path().join(".cppscript-format").exists() == false, "dry-run merge must not write new files to disk");

    Ok(())
}

// ── Verify with local source ─────────────────────────────────────────────────

#[test]
fn test_verify_passes_with_complete_config() -> anyhow::Result<()>
{
    let _g = cwd_test_guard();
    let fixture = IntegrationFixture::new()?;
    let workspace = tempfile::TempDir::new()?;
    std::env::set_current_dir(workspace.path())?;

    // Verify uses the fixture's config_dir as both the template cache and the source
    fixture.verify()?;

    Ok(())
}

#[test]
fn test_verify_detects_missing_source_file() -> anyhow::Result<()>
{
    let _g = cwd_test_guard();
    let fixture = IntegrationFixture::new()?;
    let workspace = tempfile::TempDir::new()?;
    std::env::set_current_dir(workspace.path())?;

    // Delete an entire skill source directory from the config dir
    let skill_dir = fixture.config_dir.path().join("skills/git-workflow");
    assert!(skill_dir.exists() == true);
    fs::remove_dir_all(&skill_dir)?;

    // Verify should detect the missing source and return an error
    let result = fixture.verify();
    assert!(result.is_err() == true, "verify must fail when a source file is missing");

    Ok(())
}

// ── Merge with LLM hook ──────────────────────────────────────────────────────

#[test]
fn test_merge_writes_diverged_file_via_llm_hook() -> anyhow::Result<()>
{
    let _g = cwd_test_guard();
    let fixture = IntegrationFixture::new()?;
    let workspace = tempfile::TempDir::new()?;
    std::env::set_current_dir(workspace.path())?;

    fixture.init(Some("fake"), Some("Rust++"))?;

    // Customize AGENTS.md to create divergence
    let agents_md = workspace.path().join("AGENTS.md");
    fs::write(&agents_md, "# My Customized Project\n\n## Custom Rules\nDo things my way.\n")?;

    // Write workspace config with merge.provider = ollama (no API key needed)
    let slopctl_dir = workspace.path().join(".slopctl");
    fs::write(slopctl_dir.join("config.yml"), "merge:\n  provider: ollama\n")?;

    let merged_content = "# My Customized Project\n\n## Updated Rules\nDo things the merged way.\n";
    let _hook = crate::llm::set_chat_test_hook(Box::new(move |_msgs| {
        Ok(crate::llm::ChatResponse {
            content:       merged_content.to_string(),
            input_tokens:  Some(100),
            output_tokens: Some(50),
            stop_reason:   Some("end_turn".to_string())
        })
    }));

    let options = crate::MergeOptions { lang: Some("Rust++"), agent: Some("fake"), mission: None };
    fixture.manager().merge(&options, false, false, false)?;

    let final_content = fs::read_to_string(&agents_md)?;
    assert!(final_content.contains("merged way") == true, "AGENTS.md must contain the LLM-merged content");

    let tracker = FileTracker::new(&std::env::current_dir()?)?;
    let meta = tracker.get_metadata(&agents_md);
    assert!(meta.is_some() == true, "merged file must be tracked");

    Ok(())
}

#[test]
fn test_merge_preview_writes_sidecar() -> anyhow::Result<()>
{
    let _g = cwd_test_guard();
    let fixture = IntegrationFixture::new()?;
    let workspace = tempfile::TempDir::new()?;
    std::env::set_current_dir(workspace.path())?;

    fixture.init(Some("fake"), Some("Rust++"))?;

    let agents_md = workspace.path().join("AGENTS.md");
    let _original = fs::read_to_string(&agents_md)?;
    fs::write(&agents_md, "# Diverged content\n")?;

    let slopctl_dir = workspace.path().join(".slopctl");
    fs::write(slopctl_dir.join("config.yml"), "merge:\n  provider: ollama\n")?;

    let _hook = crate::llm::set_chat_test_hook(Box::new(|_msgs| {
        Ok(crate::llm::ChatResponse {
            content:       "# Preview merged\n".to_string(),
            input_tokens:  Some(50),
            output_tokens: Some(20),
            stop_reason:   Some("stop".to_string())
        })
    }));

    let options = crate::MergeOptions { lang: Some("Rust++"), agent: Some("fake"), mission: None };
    fixture.manager().merge(&options, false, true, false)?;

    // Preview mode: original file unchanged, sidecar created
    let after = fs::read_to_string(&agents_md)?;
    assert!(after.contains("Diverged content") == true, "original file must not be overwritten in preview mode");

    let sidecar = workspace.path().join("AGENTS.md.merged");
    assert!(sidecar.exists() == true, "sidecar .merged file must be created in preview mode");

    // Clean up sidecar for test isolation
    let _ = fs::remove_file(&sidecar);
    Ok(())
}

#[test]
fn test_merge_truncated_response_keeps_partial() -> anyhow::Result<()>
{
    let _g = cwd_test_guard();
    let fixture = IntegrationFixture::new()?;
    let workspace = tempfile::TempDir::new()?;
    std::env::set_current_dir(workspace.path())?;

    fixture.init(Some("fake"), Some("Rust++"))?;

    let agents_md = workspace.path().join("AGENTS.md");
    let _original = fs::read_to_string(&agents_md)?;
    fs::write(&agents_md, "# Diverged for truncation test\n")?;

    let slopctl_dir = workspace.path().join(".slopctl");
    fs::write(slopctl_dir.join("config.yml"), "merge:\n  provider: ollama\n")?;

    let _hook = crate::llm::set_chat_test_hook(Box::new(|_msgs| {
        Ok(crate::llm::ChatResponse {
            content:       "# Partial content that got cut off".to_string(),
            input_tokens:  Some(100),
            output_tokens: Some(32768),
            stop_reason:   Some("max_tokens".to_string())
        })
    }));

    let options = crate::MergeOptions { lang: Some("Rust++"), agent: Some("fake"), mission: None };
    fixture.manager().merge(&options, false, false, false)?;

    // Truncated: original file must not be overwritten
    let after = fs::read_to_string(&agents_md)?;
    assert!(after.contains("truncation test") == true, "truncated merge must not overwrite target");

    // .partial file must be preserved for user inspection
    let partial = workspace.path().join("AGENTS.md.partial");
    assert!(partial.exists() == true, ".partial file must be kept on truncation");

    // Clean up
    let _ = fs::remove_file(&partial);
    Ok(())
}

// ── Changelog-marker file lifecycle (UPDATES.md) ─────────────────────────────

#[test]
fn test_init_installs_updates_file() -> anyhow::Result<()>
{
    let _g = cwd_test_guard();
    let fixture = IntegrationFixture::new()?;
    let workspace = tempfile::TempDir::new()?;
    std::env::set_current_dir(workspace.path())?;

    fixture.init(Some("bogus"), None)?;

    let updates = workspace.path().join("UPDATES.md");
    assert!(updates.exists() == true, "UPDATES.md must be installed by init");
    let content = fs::read_to_string(&updates)?;
    assert!(content.contains(CHANGELOG_MARKER) == true, "installed UPDATES.md must carry the changelog marker");

    let cwd = std::env::current_dir()?;
    let tracker = FileTracker::new(&cwd)?;
    let metadata = tracker.get_metadata(&cwd.join("UPDATES.md")).expect("UPDATES.md must be tracked");
    assert!(metadata.is_unreferenced() == true, "integration file must have no language or agent owners");
    assert_eq!(metadata.category, "integration", "UPDATES.md must carry its templates.yml section category");

    Ok(())
}

#[test]
fn test_reinit_after_updates_append_preserves_entries() -> anyhow::Result<()>
{
    let _g = cwd_test_guard();
    let fixture = IntegrationFixture::new()?;
    let workspace = tempfile::TempDir::new()?;
    std::env::set_current_dir(workspace.path())?;

    fixture.init(Some("bogus"), None)?;

    let updates = workspace.path().join("UPDATES.md");
    let appended = format!("{}\n### 2026-02-02 (v1.2.3, user change)\n\n- user-authored entry\n", fs::read_to_string(&updates)?);
    fs::write(&updates, &appended)?;

    // A later init (different agent) must not fail preflight and must leave the log untouched.
    fixture.init(Some("fake"), None)?;

    assert_eq!(fs::read_to_string(&updates)?, appended, "user log entries must survive re-init");

    Ok(())
}

#[test]
fn test_reinit_force_preserves_updates_log() -> anyhow::Result<()>
{
    let _g = cwd_test_guard();
    let fixture = IntegrationFixture::new()?;
    let workspace = tempfile::TempDir::new()?;
    std::env::set_current_dir(workspace.path())?;

    fixture.init(Some("bogus"), None)?;

    let updates = workspace.path().join("UPDATES.md");
    let appended = format!("{}\n### 2026-02-02 (v1.2.3, user change)\n\n- user-authored entry\n", fs::read_to_string(&updates)?);
    fs::write(&updates, &appended)?;

    // Changelog files are blocked like AGENTS.md: --force does not override this.
    // 'slopctl merge' is the only command that may refresh the template half.
    fixture.init_force(Some("bogus"), None)?;

    assert_eq!(fs::read_to_string(&updates)?, appended, "--force must not overwrite the changelog log");

    Ok(())
}

#[test]
fn test_reinit_after_merge_resync_preserves_updates_log() -> anyhow::Result<()>
{
    let _g = cwd_test_guard();
    let fixture = IntegrationFixture::new()?;
    let workspace = tempfile::TempDir::new()?;
    std::env::set_current_dir(workspace.path())?;

    fixture.init(Some("bogus"), None)?;

    let updates = workspace.path().join("UPDATES.md");
    let appended = format!("{}\n### 2026-02-02 (v1.2.3, user change)\n\n- user-authored entry\n", fs::read_to_string(&updates)?);
    fs::write(&updates, &appended)?;
    simulate_merge_resync(&updates)?;

    // Regression: after a merge re-records the tracker SHA, the file reads as
    // Unmodified while still diverging from the template source. A second init
    // (different agent) must not fall through to an unconditional overwrite.
    fixture.init(Some("fake"), None)?;

    assert_eq!(fs::read_to_string(&updates)?, appended, "post-merge log must survive a later init with no flag involved");

    Ok(())
}

#[test]
fn test_update_full_after_merge_resync_preserves_updates_log() -> anyhow::Result<()>
{
    let _g = cwd_test_guard();
    let fixture = IntegrationFixture::new()?;
    let workspace = tempfile::TempDir::new()?;
    std::env::set_current_dir(workspace.path())?;

    fixture.init(Some("bogus"), None)?;

    let updates = workspace.path().join("UPDATES.md");
    let appended = format!("{}\n### 2026-02-02 (v1.2.3, user change)\n\n- user-authored entry\n", fs::read_to_string(&updates)?);
    fs::write(&updates, &appended)?;
    simulate_merge_resync(&updates)?;

    // Same regression via 'slopctl update' (no lang/agent selectors), with and
    // without --force: neither may touch a changelog-marker file's log.
    fixture.manager().update_full(None, Some("bogus"), false, false)?;
    assert_eq!(fs::read_to_string(&updates)?, appended, "plain update must not overwrite the post-merge log");

    fixture.manager().update_full(None, Some("bogus"), true, false)?;
    assert_eq!(fs::read_to_string(&updates)?, appended, "update --force must not overwrite the post-merge log");

    Ok(())
}

#[test]
fn test_update_file_updates_md_is_rejected() -> anyhow::Result<()>
{
    let _g = cwd_test_guard();
    let fixture = IntegrationFixture::new()?;
    let workspace = tempfile::TempDir::new()?;
    std::env::set_current_dir(workspace.path())?;

    fixture.init(Some("bogus"), None)?;

    // Mirrors the existing AGENTS.md single-file rejection: changelog-marker
    // files are refreshed only via 'slopctl merge', never as a --file selector.
    let err = fixture
        .manager()
        .update_partial(&["UPDATES.md".to_string()], &[], None, Some("bogus"), false, false)
        .expect_err("UPDATES.md must be rejected as a --file selector");
    let message = err.to_string();
    assert!(message.contains("UPDATES.md") == true, "error must name the rejected file: {}", message);
    assert!(message.contains("slopctl merge") == true, "error must point to merge: {}", message);

    let updates = workspace.path().join("UPDATES.md");
    let original = fs::read_to_string(&updates)?;
    let err_force =
        fixture.manager().update_partial(&["UPDATES.md".to_string()], &[], None, Some("bogus"), true, false).expect_err("--force must not bypass the rejection");
    assert!(err_force.to_string().contains("slopctl merge") == true);
    assert_eq!(fs::read_to_string(&updates)?, original, "rejected selector must not touch the file");

    Ok(())
}

#[test]
fn test_remove_all_preserves_unmodified_per_tracker_updates_file() -> anyhow::Result<()>
{
    let _g = cwd_test_guard();
    let fixture = IntegrationFixture::new()?;
    let workspace = tempfile::TempDir::new()?;
    std::env::set_current_dir(workspace.path())?;

    fixture.init(Some("bogus"), None)?;

    let updates = workspace.path().join("UPDATES.md");
    let appended = format!("{}\n### 2026-02-02 (v1.2.3, user change)\n\n- user-authored entry\n", fs::read_to_string(&updates)?);
    fs::write(&updates, &appended)?;
    simulate_merge_resync(&updates)?;

    // Regression: split_changelog_preserved used to key on FileStatus::Modified,
    // which never fires once the tracker SHA matches the on-disk content.
    fixture.remove_all()?;

    assert!(updates.exists() == true, "UPDATES.md must survive remove --all even when Unmodified per tracker");
    assert_eq!(fs::read_to_string(&updates)?, appended, "post-merge log must be untouched");

    Ok(())
}

#[test]
fn test_remove_lang_preserves_updates_file() -> anyhow::Result<()>
{
    let _g = cwd_test_guard();
    let fixture = IntegrationFixture::new()?;
    let workspace = tempfile::TempDir::new()?;
    std::env::set_current_dir(workspace.path())?;

    fixture.init(Some("bogus"), Some("Rust++"))?;

    let updates = workspace.path().join("UPDATES.md");
    let appended = format!("{}\n### 2026-02-02 (v1.2.3, user change)\n\n- user-authored entry\n", fs::read_to_string(&updates)?);
    fs::write(&updates, &appended)?;

    fixture.remove_lang("Rust++")?;

    assert!(updates.exists() == true, "UPDATES.md must survive remove --lang");
    assert_eq!(fs::read_to_string(&updates)?, appended, "log entries must be untouched by remove --lang");

    Ok(())
}

#[test]
fn test_remove_all_preserves_updates_file() -> anyhow::Result<()>
{
    let _g = cwd_test_guard();
    let fixture = IntegrationFixture::new()?;
    let workspace = tempfile::TempDir::new()?;
    std::env::set_current_dir(workspace.path())?;

    fixture.init(Some("bogus"), Some("Rust++"))?;

    let updates = workspace.path().join("UPDATES.md");
    let appended = format!("{}\n### 2026-02-02 (v1.2.3, user change)\n\n- user-authored entry\n", fs::read_to_string(&updates)?);
    fs::write(&updates, &appended)?;

    fixture.remove_all()?;

    assert!(updates.exists() == true, "UPDATES.md must survive remove --all");
    assert_eq!(fs::read_to_string(&updates)?, appended, "log entries must be untouched by remove --all");

    Ok(())
}

#[test]
fn test_remove_all_preserves_updates_file_in_agent_named_path() -> anyhow::Result<()>
{
    let _g = cwd_test_guard();
    let fixture = IntegrationFixture::new()?;
    let parent = tempfile::TempDir::new()?;

    // Tracker categories are authoritative (recorded from the templates.yml section),
    // so a workspace path containing the agent name no longer miscategorizes files;
    // the changelog guard remains as defense in depth for user-owned log entries.
    let workspace = parent.path().join("bogus-nest");
    fs::create_dir_all(&workspace)?;
    std::env::set_current_dir(&workspace)?;

    fixture.init(Some("bogus"), None)?;

    let updates = workspace.join("UPDATES.md");
    let appended = format!("{}\n### 2026-02-02 (v1.2.3, user change)\n\n- user-authored entry\n", fs::read_to_string(&updates)?);
    fs::write(&updates, &appended)?;

    fixture.remove_all()?;

    assert!(updates.exists() == true, "UPDATES.md must survive remove --all even when categorized as agent");
    assert_eq!(fs::read_to_string(&updates)?, appended, "log entries must be untouched");

    Ok(())
}

#[test]
fn test_init_in_agent_named_path_records_section_categories() -> anyhow::Result<()>
{
    let _g = cwd_test_guard();
    let fixture = IntegrationFixture::new()?;
    let parent = tempfile::TempDir::new()?;

    // The workspace path contains the agent name; categories must still come from
    // the templates.yml section each file resolves from, not from path substrings.
    let workspace = parent.path().join("bogus-nest");
    fs::create_dir_all(&workspace)?;
    std::env::set_current_dir(&workspace)?;

    fixture.init(Some("bogus"), Some("Rust++"))?;

    let cwd = std::env::current_dir()?;
    let tracker = FileTracker::new(&cwd)?;
    let category_of = |rel: &str| tracker.get_metadata(&cwd.join(rel)).unwrap_or_else(|| panic!("{} must be tracked", rel)).category.clone();

    assert_eq!(category_of(".bogus/instructions.md"), "agent");
    assert_eq!(category_of(".rpp.toml"), "language");
    assert_eq!(category_of("UPDATES.md"), "integration");
    assert_eq!(category_of(".bogus/skills/git-workflow/SKILL.md"), "skill");

    Ok(())
}

// ── No-op re-init guard ──────────────────────────────────────────────────────

#[test]
fn test_reinit_same_agent_and_lang_errors_with_guidance() -> anyhow::Result<()>
{
    let _g = cwd_test_guard();
    let fixture = IntegrationFixture::new()?;
    let workspace = tempfile::TempDir::new()?;
    std::env::set_current_dir(workspace.path())?;

    fixture.init(Some("bogus"), Some("Rust++"))?;

    let err = fixture.init(Some("bogus"), Some("Rust++")).expect_err("no-op re-init must be rejected");
    let message = err.to_string();
    assert!(message.contains("already initialized") == true, "message must state the workspace is initialized: {}", message);
    assert!(message.contains("slopctl update") == true, "message must point to the update command: {}", message);
    assert!(message.contains("slopctl merge") == true, "message must point to the merge command: {}", message);

    Ok(())
}

#[test]
fn test_reinit_same_agent_only_errors() -> anyhow::Result<()>
{
    let _g = cwd_test_guard();
    let fixture = IntegrationFixture::new()?;
    let workspace = tempfile::TempDir::new()?;
    std::env::set_current_dir(workspace.path())?;

    fixture.init(Some("bogus"), None)?;

    let err = fixture.init(Some("bogus"), None).expect_err("agent-only no-op re-init must be rejected");
    assert!(err.to_string().contains("agent 'bogus'") == true);

    Ok(())
}

#[test]
fn test_init_new_agent_on_initialized_workspace_succeeds() -> anyhow::Result<()>
{
    let _g = cwd_test_guard();
    let fixture = IntegrationFixture::new()?;
    let workspace = tempfile::TempDir::new()?;
    std::env::set_current_dir(workspace.path())?;

    fixture.init(Some("bogus"), Some("Rust++"))?;

    // Anything new keeps init working: same language, new agent.
    fixture.init(Some("fake"), Some("Rust++"))?;
    assert!(workspace.path().join(".fake/commands/init-session.md").exists() == true);

    Ok(())
}

#[test]
fn test_reinit_force_bypasses_noop_guard() -> anyhow::Result<()>
{
    let _g = cwd_test_guard();
    let fixture = IntegrationFixture::new()?;
    let workspace = tempfile::TempDir::new()?;
    std::env::set_current_dir(workspace.path())?;

    fixture.init(Some("bogus"), Some("Rust++"))?;
    fixture.init_force(Some("bogus"), Some("Rust++"))?;

    Ok(())
}

// ── Modified tracked files never block re-init ───────────────────────────────

#[test]
fn test_add_second_agent_with_modified_language_file_succeeds() -> anyhow::Result<()>
{
    let _g = cwd_test_guard();
    let fixture = IntegrationFixture::new()?;
    let workspace = tempfile::TempDir::new()?;
    std::env::set_current_dir(workspace.path())?;

    fixture.init(Some("bogus"), Some("Rust++"))?;

    // A locally modified tracked file unrelated to the new agent must not block the install.
    let format_file = workspace.path().join(".rpp.toml");
    fs::write(&format_file, "max_width = 99\n")?;

    fixture.init(Some("fake"), Some("Rust++"))?;

    assert!(workspace.path().join(".fake/commands/init-session.md").exists() == true, "second agent files must be installed");
    assert_eq!(fs::read_to_string(&format_file)?, "max_width = 99\n", "local modifications must be preserved");

    Ok(())
}

#[test]
fn test_reinit_force_overwrites_modified_language_file() -> anyhow::Result<()>
{
    let _g = cwd_test_guard();
    let fixture = IntegrationFixture::new()?;
    let workspace = tempfile::TempDir::new()?;
    std::env::set_current_dir(workspace.path())?;

    fixture.init(Some("bogus"), Some("Rust++"))?;

    let format_file = workspace.path().join(".rpp.toml");
    fs::write(&format_file, "max_width = 99\n")?;

    fixture.init_force(Some("bogus"), Some("Rust++"))?;

    assert_eq!(fs::read_to_string(&format_file)?, "max_width = 167\n", "with --force the template must overwrite the modified file");

    Ok(())
}

#[test]
fn test_merge_dry_run_after_updates_append_succeeds() -> anyhow::Result<()>
{
    let _g = cwd_test_guard();
    let fixture = IntegrationFixture::new()?;
    let workspace = tempfile::TempDir::new()?;
    std::env::set_current_dir(workspace.path())?;

    fixture.init(Some("bogus"), None)?;

    let updates = workspace.path().join("UPDATES.md");
    let appended = format!("{}\n### 2026-02-02 (v1.2.3, user change)\n\n- user-authored entry\n", fs::read_to_string(&updates)?);
    fs::write(&updates, &appended)?;

    // Tail-only diff classifies as Unchanged, so the dry run needs no LLM provider.
    fixture.merge_dry_run(Some("bogus"), None)?;

    assert_eq!(fs::read_to_string(&updates)?, appended, "merge dry run must not touch the log");

    Ok(())
}

// ── Native-only agent ownership and path-matching regressions ────────────────

#[test]
fn test_remove_native_agent_then_reinit_succeeds() -> anyhow::Result<()>
{
    let _g = cwd_test_guard();
    let fixture = IntegrationFixture::new()?;
    let workspace = tempfile::TempDir::new()?;
    std::env::set_current_dir(workspace.path())?;

    // Mixed workspace: cross-client fake first, then native-only bogus.
    fixture.init(Some("fake"), None)?;
    fixture.init(Some("bogus"), None)?;

    fixture.remove_agent("bogus")?;

    let cwd = std::env::current_dir()?;
    let tracker = FileTracker::new(&cwd)?;
    assert!(tracker.get_installed_agents().iter().any(|agent| agent == "bogus") == false, "removed agent must not linger as an owner anywhere in the tracker");

    // Re-adding the agent must not be rejected by the no-op init guard.
    fixture.init(Some("bogus"), None)?;

    Ok(())
}

#[test]
fn test_remove_native_agent_releases_owner_on_cross_client_copies() -> anyhow::Result<()>
{
    let _g = cwd_test_guard();
    let fixture = IntegrationFixture::new()?;
    let workspace = tempfile::TempDir::new()?;
    std::env::set_current_dir(workspace.path())?;

    fixture.init(Some("fake"), None)?;
    fixture.init(Some("bogus"), None)?;

    fixture.remove_agent("bogus")?;

    let cwd = std::env::current_dir()?;
    let tracker = FileTracker::new(&cwd)?;
    let shared = cwd.join(".agents/skills/git-workflow/SKILL.md");
    assert!(shared.exists() == true, "shared cross-client skill must survive native-agent removal");
    let meta = tracker.get_metadata(&shared).expect("shared skill must stay tracked");
    assert!(meta.has_agent("bogus") == false, "cross-client copy must not keep the removed agent as owner");
    assert!(meta.has_agent("fake") == true, "remaining agent ownership must be preserved");

    Ok(())
}

#[test]
fn test_remove_agent_ignores_agent_named_ancestor_dir() -> anyhow::Result<()>
{
    let _g = cwd_test_guard();
    let fixture = IntegrationFixture::new()?;
    let parent = tempfile::TempDir::new()?;

    // The workspace lives under a directory component equal to the agent name;
    // location-based force-deletion must not treat shared files as agent-owned.
    let workspace = parent.path().join("bogus").join("ws");
    fs::create_dir_all(&workspace)?;
    std::env::set_current_dir(&workspace)?;

    fixture.init(Some("fake"), Some("Rust++"))?;
    fixture.init(Some("bogus"), None)?;

    fixture.remove_agent("bogus")?;

    let shared_lang_skill = workspace.join(".agents/skills/rpp-coding-conventions/SKILL.md");
    assert!(shared_lang_skill.exists() == true, "shared language skill outside the agent dirs must survive remove --agent");

    Ok(())
}

#[test]
fn test_update_full_skips_never_installed_agent_files() -> anyhow::Result<()>
{
    let _g = cwd_test_guard();
    let fixture = IntegrationFixture::new()?;
    let workspace = tempfile::TempDir::new()?;
    std::env::set_current_dir(workspace.path())?;

    // The agent app created its own marker dir; slopctl never installed the agent.
    fs::create_dir_all(workspace.path().join(".fake"))?;
    fixture.init(None, Some("Rust++"))?;

    fixture.manager().update_full(None, None, false, false)?;

    assert!(workspace.path().join(".fake/commands/init-session.md").exists() == false, "update must not create agent files for a marker-only agent");
    assert!(workspace.path().join(".agents/skills/git-workflow/SKILL.md").exists() == true, "skill distribution stays marker-based");

    Ok(())
}

#[test]
fn test_merge_recreates_prompt_files_for_all_detected_agents() -> anyhow::Result<()>
{
    let _g = cwd_test_guard();
    let fixture = IntegrationFixture::new()?;
    let workspace = tempfile::TempDir::new()?;
    std::env::set_current_dir(workspace.path())?;

    fixture.init(Some("bogus"), None)?;
    fixture.init(Some("fake"), None)?;

    let bogus_file = workspace.path().join(".bogus/instructions.md");
    let fake_file = workspace.path().join(".fake/commands/init-session.md");
    fs::remove_file(&bogus_file)?;
    fs::remove_file(&fake_file)?;

    // Deleted files classify as New (no LLM needed); the hook guards against any
    // unexpected divergence reaching a real provider.
    let _hook = crate::llm::set_chat_test_hook(Box::new(|_msgs| {
        Ok(crate::llm::ChatResponse { content: String::new(), input_tokens: None, output_tokens: None, stop_reason: Some("end_turn".to_string()) })
    }));

    let options = crate::MergeOptions { lang: None, agent: None, mission: None };
    fixture.manager().merge(&options, false, false, false)?;

    assert!(bogus_file.exists() == true, "merge without --agent must recreate the first agent's file");
    assert!(fake_file.exists() == true, "merge without --agent must recreate the second agent's file");

    Ok(())
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Re-records `target`'s tracker entry to match its current on-disk content.
///
/// Mirrors what `slopctl merge` does after splicing a changelog-marker file's
/// template half: the tracker's `original_sha` is updated to the post-merge
/// content, so the file reads as `FileStatus::Unmodified` even though it now
/// diverges from the template source. This is the exact precondition that
/// exposed the changelog data-loss bug (init/update fell through to an
/// unconditional overwrite because `FileStatus::Modified` never fired).
fn simulate_merge_resync(target: &Path) -> anyhow::Result<()>
{
    let sha = FileTracker::calculate_sha256(target)?;
    let mut tracker = FileTracker::new(&std::env::current_dir()?)?;
    tracker.record_installation_with_owners(target, sha, 5, &[], &[], "integration".to_string());
    tracker.save()?;
    Ok(())
}

/// Returns `true` if any `SKILL.md` exists recursively under `dir`.
fn has_skill_md_under(dir: &Path) -> bool
{
    if dir.exists() == false
    {
        return false;
    }
    walkdir(dir).iter().any(|p| p.file_name().is_some_and(|n| n == "SKILL.md"))
}

/// Collect all file paths recursively under `dir`.
fn walkdir(dir: &Path) -> Vec<std::path::PathBuf>
{
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(dir)
    {
        for entry in entries.flatten()
        {
            let path = entry.path();
            if path.is_dir() == true
            {
                files.extend(walkdir(&path));
            }
            else
            {
                files.push(path);
            }
        }
    }
    files
}
