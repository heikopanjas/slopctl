use std::io;

use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::generate;
use owo_colors::OwoColorize;
use vibe_check::{Config, Result, TemplateManager};

/// Supported shells for completion generation
#[derive(Clone, Copy, ValueEnum)]
enum ShellType
{
    Bash,
    Fish,
    Powershell,
    Zsh
}

impl From<ShellType> for clap_complete::Shell
{
    fn from(shell: ShellType) -> Self
    {
        match shell
        {
            | ShellType::Bash => clap_complete::Shell::Bash,
            | ShellType::Fish => clap_complete::Shell::Fish,
            | ShellType::Powershell => clap_complete::Shell::PowerShell,
            | ShellType::Zsh => clap_complete::Shell::Zsh
        }
    }
}

#[derive(Parser)]
#[command(name = "vibe-check")]
#[command(about = "A manager for coding agent instruction files", long_about = None)]
#[command(version)]
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
        /// Programming language or framework (e.g., rust, c++, swift)
        #[arg(long)]
        lang: String,

        /// AI coding agent (e.g., claude, copilot, codex, cursor). Required for v1 templates, optional for v2.
        #[arg(long)]
        agent: Option<String>,

        /// Force overwrite of local files without confirmation
        #[arg(long, default_value = "false")]
        force: bool,

        /// Preview changes without applying them
        #[arg(long, default_value = "false")]
        dry_run: bool
    },
    /// Update global templates from source
    Update
    {
        /// Path or URL to download/copy templates from
        #[arg(long)]
        from: Option<String>,

        /// Preview changes without applying them
        #[arg(long, default_value = "false")]
        dry_run: bool
    },
    /// Purge all vibe-check files from project
    Purge
    {
        /// Force purge without confirmation
        #[arg(long, default_value = "false")]
        force: bool,

        /// Preview changes without applying them
        #[arg(long, default_value = "false")]
        dry_run: bool
    },
    /// Remove agent-specific files from current directory
    Remove
    {
        /// AI coding agent (e.g., claude, copilot, codex, cursor)
        #[arg(long)]
        agent: Option<String>,

        /// Remove all agent-specific files (cannot be used with --agent)
        #[arg(long, default_value = "false")]
        all: bool,

        /// Force removal without confirmation
        #[arg(long, default_value = "false")]
        force: bool,

        /// Preview changes without applying them
        #[arg(long, default_value = "false")]
        dry_run: bool
    },
    /// Generate shell completions
    Completions
    {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: ShellType
    },
    /// Show current project status
    Status,
    /// List available agents and languages
    List,
    /// Manage configuration
    Config
    {
        /// Configuration key (e.g., source.url)
        key: Option<String>,

        /// Value to set (omit to get current value)
        value: Option<String>,

        /// List all configuration values
        #[arg(long, default_value = "false")]
        list: bool,

        /// Unset a configuration key
        #[arg(long)]
        unset: Option<String>
    }
}

/// Handle config command operations
fn handle_config(key: Option<String>, value: Option<String>, list: bool, unset: Option<String>) -> Result<()>
{
    // Handle --list flag
    if list == true
    {
        let config = Config::load()?;
        let values = config.list();

        if values.is_empty() == true
        {
            println!("{} No configuration values set", "→".blue());
            println!("{} Use 'vibe-check config <key> <value>' to set a value", "→".blue());
            println!("{} Valid keys: {}", "→".blue(), Config::valid_keys().join(", ").yellow());
        }
        else
        {
            println!("{}", "Configuration:".bold());
            for (k, v) in &values
            {
                println!("  {} = {}", k.green(), v.yellow());
            }
        }
        return Ok(());
    }

    // Handle --unset flag
    if let Some(unset_key) = unset
    {
        let mut config = Config::load()?;
        config.unset(&unset_key)?;
        config.save()?;
        println!("{} Unset {}", "✓".green(), unset_key.yellow());
        return Ok(());
    }

    // Handle key/value operations
    match (key, value)
    {
        | (Some(k), Some(v)) =>
        {
            // Set value
            let mut config = Config::load()?;
            config.set(&k, &v)?;
            config.save()?;
            println!("{} Set {} = {}", "✓".green(), k.yellow(), v.green());
        }
        | (Some(k), None) =>
        {
            // Get value
            let config = Config::load()?;
            if let Some(v) = config.get(&k)
            {
                println!("{}", v);
            }
            else
            {
                println!("{} Key '{}' is not set", "→".blue(), k.yellow());
            }
        }
        | (None, Some(_)) =>
        {
            return Err("Must specify a key when setting a value".into());
        }
        | (None, None) =>
        {
            // Show help
            println!("{}", "vibe-check config".bold());
            println!();
            println!("Usage:");
            println!("  vibe-check config <key> <value>  Set a configuration value");
            println!("  vibe-check config <key>          Get a configuration value");
            println!("  vibe-check config --list         List all configuration values");
            println!("  vibe-check config --unset <key>  Remove a configuration value");
            println!();
            println!("Valid keys:");
            for key in Config::valid_keys()
            {
                println!("  • {}", key.yellow());
            }
        }
    }
    Ok(())
}

fn main()
{
    let cli = Cli::parse();

    let manager = match TemplateManager::new()
    {
        | Ok(m) => m,
        | Err(e) =>
        {
            eprintln!("{} Failed to initialize template manager: {}", "✗".red(), e.to_string().red());
            std::process::exit(1);
        }
    };

    let result = match cli.command
    {
        | Commands::Init { lang, agent, force, dry_run } =>
        {
            // Check if global templates exist, download if not
            if manager.has_global_templates() == false
            {
                if dry_run == true
                {
                    println!("{} Global templates not found (would download in non-dry-run mode)", "→".yellow());
                    return;
                }

                // Use configured source or default (v2 is default in v6.x - agents.md standard)
                let default_source = "https://github.com/heikopanjas/vibe-check/tree/develop/templates/v2".to_string();
                let config = Config::load().ok();
                let configured_source = config.as_ref().and_then(|c| c.get("source.url"));
                let fallback_source = config.as_ref().and_then(|c| c.get("source.fallback"));

                let source = configured_source.clone().unwrap_or(default_source);

                if configured_source.is_some() == true
                {
                    println!("{} Using configured source", "→".blue());
                }
                println!("{} Global templates not found, downloading from {}", "→".blue(), source.yellow());

                // Try primary source, fall back if configured and primary fails
                let download_result = match manager.download_or_copy_templates(&source)
                {
                    | Ok(()) => Ok(()),
                    | Err(e) =>
                    {
                        if let Some(fallback) = fallback_source
                        {
                            println!("{} Primary source failed: {}", "!".yellow(), e);
                            println!("{} Trying fallback source: {}", "→".blue(), fallback.yellow());
                            manager.download_or_copy_templates(&fallback)
                        }
                        else
                        {
                            Err(e)
                        }
                    }
                };

                if let Err(e) = download_result
                {
                    eprintln!("{} Failed to download global templates: {}", "✗".red(), e);
                    std::process::exit(1);
                }
            }

            // Install templates to project
            if dry_run == true
            {
                if let Some(agent_name) = &agent
                {
                    println!("{} Dry run: previewing changes for {} with {}", "→".blue(), lang.green(), agent_name.green());
                }
                else
                {
                    println!("{} Dry run: previewing changes for {}", "→".blue(), lang.green());
                }
            }
            else if let Some(agent_name) = &agent
            {
                println!("{} Initializing project for {} with {}", "→".blue(), lang.green(), agent_name.green());
            }
            else
            {
                println!("{} Initializing project for {}", "→".blue(), lang.green());
            }
            manager.update(&lang, agent.as_deref(), force, dry_run)
        }
        | Commands::Update { from, dry_run } =>
        {
            // Determine source: CLI --from > config source.url > default (v2 is default in v6.x - agents.md standard)
            let default_source = "https://github.com/heikopanjas/vibe-check/tree/develop/templates/v2".to_string();
            let config = Config::load().ok();
            let configured_source = config.as_ref().and_then(|c| c.get("source.url"));
            let fallback_source = config.as_ref().and_then(|c| c.get("source.fallback"));

            let (source, is_configured) = if let Some(from_url) = from
            {
                (from_url, false)
            }
            else if let Some(config_url) = configured_source.clone()
            {
                (config_url, true)
            }
            else
            {
                (default_source.clone(), false)
            };

            if dry_run == true
            {
                if is_configured == true
                {
                    println!("{} Using configured source", "→".blue());
                }
                println!("{} Dry run: would update global templates from {}", "→".blue(), source.yellow());
                if let Some(ref fallback) = fallback_source
                {
                    println!("{} Fallback source configured: {}", "→".blue(), fallback.yellow());
                }
                println!("{} Templates would be downloaded to: {}", "→".blue(), manager.get_config_dir().display().to_string().yellow());
                println!("\n{} Dry run complete. No files were modified.", "✓".green());
                Ok(())
            }
            else
            {
                if is_configured == true
                {
                    println!("{} Using configured source", "→".blue());
                }
                println!("{} Updating global templates from {}", "→".blue(), source.yellow());

                // Try primary source, fall back if configured and primary fails
                match manager.download_or_copy_templates(&source)
                {
                    | Ok(()) => Ok(()),
                    | Err(e) =>
                    {
                        if let Some(fallback) = fallback_source
                        {
                            println!("{} Primary source failed: {}", "!".yellow(), e);
                            println!("{} Trying fallback source: {}", "→".blue(), fallback.yellow());
                            manager.download_or_copy_templates(&fallback)
                        }
                        else
                        {
                            Err(e)
                        }
                    }
                }
            }
        }
        | Commands::Purge { force, dry_run } => manager.purge(force, dry_run),
        | Commands::Remove { agent, all, force, dry_run } =>
        {
            // Validate mutually exclusive options
            if all == true && agent.is_some() == true
            {
                Err("Cannot specify both --agent and --all options".to_string().into())
            }
            else if all == false && agent.is_none() == true
            {
                Err("Must specify either --agent <name> or --all".to_string().into())
            }
            else
            {
                // Pass None for --all, or Some(&agent) for specific agent
                manager.remove(agent.as_deref(), force, dry_run)
            }
        }
        | Commands::Completions { shell } =>
        {
            let shell: clap_complete::Shell = shell.into();
            generate(shell, &mut Cli::command(), "vibe-check", &mut io::stdout());
            Ok(())
        }
        | Commands::Status => manager.status(),
        | Commands::List => manager.list(),
        | Commands::Config { key, value, list, unset } => handle_config(key, value, list, unset)
    };

    if let Err(e) = result
    {
        eprintln!("{} {}", "✗".red(), e.to_string().red());
        std::process::exit(1);
    }
}
