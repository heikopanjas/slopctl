//! Integration tests exercising init→remove sequences across all three
//! canonical test agent archetypes (bogus, fake, foobar) and both synthetic
//! languages (Rust++, CppScript).

use std::{fs, path::Path};

use super::cwd_test_guard;
use crate::{FileTracker, TemplateManager, UpdateOptions, agent_defaults::AGENT_DEFAULTS_FILE, template_engine::TEMPLATE_MARKER};

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
