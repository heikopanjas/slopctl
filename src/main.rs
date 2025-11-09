use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf}
};

use clap::{Parser, Subcommand};
use owo_colors::OwoColorize;
use sha2::{Digest, Sha256};

#[derive(Parser)]
#[command(name = "vibe-check")]
#[command(about = "A manager for coding agent instruction files", long_about = None)]
struct Cli
{
    #[command(subcommand)]
    command: Commands
}

#[derive(Subcommand)]
enum Commands
{
    /// Initialize agent instructions for a project
    Init
    {
        /// Programming language or framework (e.g., rust, python, typescript)
        #[arg(long)]
        lang: String,

        /// AI coding agent (e.g., claude, copilot, cursor, codex)
        #[arg(long)]
        agent: String
    },
    /// Update templates from global storage
    Update
    {
        /// Programming language or framework
        #[arg(long)]
        lang: String,

        /// AI coding agent
        #[arg(long)]
        agent: String,

        /// Force overwrite without confirmation
        #[arg(long, default_value = "false")]
        force: bool
    },
    /// Clear all templates from storage
    Clear
    {
        /// Force clear without confirmation
        #[arg(long, default_value = "false")]
        force: bool
    }
}

struct TemplateManager
{
    config_dir: PathBuf,
    cache_dir:  PathBuf
}

impl TemplateManager
{
    fn new() -> Result<Self, Box<dyn std::error::Error>>
    {
        let home = std::env::var("HOME")?;
        let config_dir = PathBuf::from(home.clone()).join(".config/vibe-check/templates");
        let cache_dir = PathBuf::from(home).join(".cache/vibe-check/backups");

        Ok(Self { config_dir, cache_dir })
    }

    fn get_timestamp() -> String
    {
        use std::time::SystemTime;

        use chrono::{DateTime, Utc};

        let now = SystemTime::now();
        let datetime: DateTime<Utc> = now.into();
        datetime.format("%Y-%m-%d_%H_%M_%S").to_string()
    }

    fn calculate_checksum(&self, file_path: &Path) -> Result<String, Box<dyn std::error::Error>>
    {
        let content = fs::read(file_path)?;
        let mut hasher = Sha256::new();
        hasher.update(&content);
        Ok(hex::encode(hasher.finalize()))
    }

    fn verify_or_create_checksum(&self, template_path: &Path) -> Result<(), Box<dyn std::error::Error>>
    {
        let checksum_path = template_path.with_extension("sha");

        if !checksum_path.exists()
        {
            println!("{} Creating missing checksum for {}", "→".blue(), template_path.display().to_string().yellow());

            let checksum = self.calculate_checksum(template_path)?;
            fs::write(&checksum_path, checksum)?;
        }

        Ok(())
    }

    fn has_local_modifications(&self, local_path: &Path, global_path: &Path) -> Result<bool, Box<dyn std::error::Error>>
    {
        if !local_path.exists()
        {
            return Ok(false);
        }

        let local_checksum = self.calculate_checksum(local_path)?;
        let global_checksum = self.calculate_checksum(global_path)?;

        Ok(local_checksum != global_checksum)
    }

    fn create_backup(&self, source_dir: &Path) -> Result<(), Box<dyn std::error::Error>>
    {
        if !source_dir.exists()
        {
            return Ok(());
        }

        let timestamp = Self::get_timestamp();
        let backup_dir = self.cache_dir.join(timestamp);

        fs::create_dir_all(&backup_dir)?;

        println!("{} Creating backup in {}", "→".blue(), backup_dir.display().to_string().yellow());

        copy_dir_all(source_dir, &backup_dir)?;

        Ok(())
    }

    fn update(&self, lang: &str, agent: &str, force: bool) -> Result<(), Box<dyn std::error::Error>>
    {
        println!("{} Updating templates for {} with {}", "→".blue(), lang.green(), agent.green());

        // Build paths
        let lang_template = self.config_dir.join(format!("{}.md", lang));
        let agent_template = self.config_dir.join(agent).join("instructions.md");

        // Verify global template existence
        if !lang_template.exists()
        {
            return Err(format!("Language template not found: {}", lang).into());
        }
        if !agent_template.exists()
        {
            return Err(format!("Agent template not found: {}", agent).into());
        }

        // Verify or create checksums
        self.verify_or_create_checksum(&lang_template)?;
        self.verify_or_create_checksum(&agent_template)?;

        // Check for local modifications
        let current_dir = std::env::current_dir()?;
        let local_lang = current_dir.join(format!("{}.md", lang));
        let local_agent_dir = current_dir.join(format!(".{}", agent));
        let local_agent = local_agent_dir.join("instructions.md");

        let has_lang_mods = self.has_local_modifications(&local_lang, &lang_template)?;
        let has_agent_mods = self.has_local_modifications(&local_agent, &agent_template)?;

        if (has_lang_mods || has_agent_mods) && !force
        {
            println!("{} Local modifications detected:", "!".yellow());
            if has_lang_mods
            {
                println!("  - {}", local_lang.display().to_string().yellow());
            }
            if has_agent_mods
            {
                println!("  - {}", local_agent.display().to_string().yellow());
            }
            println!("{} Use --force to overwrite", "→".blue());
            return Err("Local modifications detected. Aborting.".into());
        }

        // Create backup of existing local files
        self.create_backup(&current_dir)?;

        // Copy templates
        println!("{} Copying templates to current directory", "→".blue());
        fs::copy(&lang_template, &local_lang)?;
        fs::create_dir_all(&local_agent_dir)?;
        fs::copy(&agent_template, &local_agent)?;

        println!("{} Templates updated successfully", "✓".green());

        Ok(())
    }

    fn clear(&self, force: bool) -> Result<(), Box<dyn std::error::Error>>
    {
        if !force
        {
            print!("{} Are you sure you want to clear all templates? (y/N): ", "?".yellow());
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;

            if !input.trim().eq_ignore_ascii_case("y")
            {
                println!("{} Operation cancelled", "→".blue());
                return Ok(());
            }
        }

        // Create backup before clearing
        self.create_backup(&self.config_dir)?;

        if self.config_dir.exists()
        {
            println!("{} Clearing templates from {}", "→".blue(), self.config_dir.display().to_string().yellow());
            fs::remove_dir_all(&self.config_dir)?;
        }

        println!("{} Templates cleared successfully", "✓".green());

        Ok(())
    }
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<(), Box<dyn std::error::Error>>
{
    fs::create_dir_all(dst)?;

    for entry in fs::read_dir(src)?
    {
        let entry = entry?;
        let path = entry.path();
        let file_name = entry.file_name();
        let dst_path = dst.join(file_name);

        if path.is_dir()
        {
            copy_dir_all(&path, &dst_path)?;
        }
        else
        {
            fs::copy(&path, &dst_path)?;
        }
    }

    Ok(())
}

fn main()
{
    let cli = Cli::parse();

    let result = match cli.command
    {
        | Commands::Init { lang, agent } =>
        {
            let manager = TemplateManager::new().unwrap();
            manager.update(&lang, &agent, false)
        }
        | Commands::Update { lang, agent, force } =>
        {
            let manager = TemplateManager::new().unwrap();
            manager.update(&lang, &agent, force)
        }
        | Commands::Clear { force } =>
        {
            let manager = TemplateManager::new().unwrap();
            manager.clear(force)
        }
    };

    if let Err(e) = result
    {
        eprintln!("{} {}", "✗".red(), e.to_string().red());
        std::process::exit(1);
    }
}
