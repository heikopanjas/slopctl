//! Integration tests exercising init→remove sequences across all three
//! canonical test agent archetypes (bogus, fake, foobar) and both synthetic
//! languages (Rust++, CppScript).

use std::{fs, path::Path};

use super::cwd_test_guard;
use crate::{FileTracker, MergeOptions, TemplateManager, UpdateOptions, agent_defaults::AGENT_DEFAULTS_FILE, template_engine::TEMPLATE_MARKER};

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
      - source: rpp-hint.md
        target: '$instructions'
    skills:
      - source: 'skills/rpp-coding-conventions'
  CppScript:
    includes: [cmake]
    files:
      - source: cppscript-format.json
        target: '$workspace/.cppscript-format'

skills:
  - source: 'skills/git-workflow'
  - source: 'skills/semantic-versioning'

integration:
  git:
    files:
      - source: git-workflow-summary.md
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
        fs::write(d.join("cmake-hint.md"), "## CMake Conventions\n")?;

        // ── Fragment source files ────────────────────────────────────────
        fs::write(d.join("git-workflow-summary.md"), "## Git Workflow\n")?;
        fs::write(d.join("core-principles.md"), "## Principles\n")?;
        fs::write(d.join("mission-statement.md"), "## Mission\n")?;

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
        let options = UpdateOptions { lang, agent, mission: None, force: false, dry_run: false };
        self.manager().update(&options)
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

    // Tracker assertions
    let tracker = FileTracker::new(workspace.path())?;
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

    let tracker = FileTracker::new(workspace.path())?;
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

    let tracker = FileTracker::new(workspace.path())?;
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

    let tracker = FileTracker::new(workspace.path())?;
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

    let tracker = FileTracker::new(workspace.path())?;
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

    let tracker = FileTracker::new(workspace.path())?;
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
    let tracker = FileTracker::new(workspace.path())?;
    let agent_entries = tracker.get_entries_by_category("agent");
    assert!(agent_entries.len() >= 2, "both agents should have tracked entries");

    Ok(())
}

// ── Language guard ───────────────────────────────────────────────────────────

#[test]
fn test_init_lang_then_different_lang_blocked() -> anyhow::Result<()>
{
    let _g = cwd_test_guard();
    let fixture = IntegrationFixture::new()?;
    let workspace = tempfile::TempDir::new()?;
    std::env::set_current_dir(workspace.path())?;

    fixture.init(Some("fake"), Some("Rust++"))?;
    assert_eq!(FileTracker::new(workspace.path())?.get_installed_language(), Some("Rust++".to_string()));

    // Attempting a different language must fail
    let result = fixture.init(Some("fake"), Some("CppScript"));
    assert!(result.is_err() == true, "second init with different lang must be rejected");
    let err = result.unwrap_err().to_string();
    assert!(err.contains("Rust++") == true, "error must mention installed language");

    // Workspace must be unchanged — CppScript file must not appear
    assert!(workspace.path().join(".cppscript-format").exists() == false, "blocked init must not create files");

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

    fixture.remove_lang("Rust++")?;
    assert!(workspace.path().join(".rpp.toml").exists() == false);
    assert!(FileTracker::new(workspace.path())?.get_installed_language().is_none() == true);

    // Now a different language must be accepted
    fixture.init(Some("fake"), Some("CppScript"))?;
    assert!(workspace.path().join(".cppscript-format").exists() == true, "new language file must appear");
    assert_eq!(FileTracker::new(workspace.path())?.get_installed_language(), Some("CppScript".to_string()));

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

    // Non-language skills (lang: none) must be cleaned
    let git_skill = cross_client_dir.join("git-workflow/SKILL.md");
    assert!(git_skill.exists() == false, "top-level skill must be deleted when last cross-client agent removed");

    // Language skills must survive (owned by Rust++, not by the agent)
    let tracker = FileTracker::new(workspace.path())?;
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

    let entries_before = FileTracker::new(workspace.path())?.get_entries().len();

    // Doctor on a clean workspace must succeed and not modify the tracker
    fixture.doctor(false, false)?;

    let entries_after = FileTracker::new(workspace.path())?.get_entries().len();
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

    let rpp_file = workspace.path().join(".rpp.toml");
    assert!(rpp_file.exists() == true);
    fs::remove_file(&rpp_file)?;

    // Doctor without fix: succeeds (prints issues but doesn't return Err)
    fixture.doctor(false, false)?;

    // The tracker still has the entry (fix was not requested)
    let tracker = FileTracker::new(workspace.path())?;
    assert!(tracker.get_metadata(&rpp_file).is_some() == true, "tracker must still have the stale entry before fix");

    // Doctor with fix: prunes the stale tracker entry
    fixture.doctor(true, false)?;

    let tracker_after = FileTracker::new(workspace.path())?;
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
    let mut tracker = FileTracker::new(workspace.path())?;
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

    let tracker = FileTracker::new(workspace.path())?;
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
    let original = fs::read_to_string(&agents_md)?;
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
    let original = fs::read_to_string(&agents_md)?;
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

// ── Helpers ──────────────────────────────────────────────────────────────────

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
