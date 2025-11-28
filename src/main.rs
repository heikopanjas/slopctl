use std::io;

use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::generate;
use owo_colors::OwoColorize;
use vibe_check::TemplateManager;

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

        /// AI coding agent (e.g., claude, copilot, codex, cursor)
        #[arg(long)]
        agent: String,

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
    }
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

                let source = "https://github.com/heikopanjas/vibe-check/tree/develop/templates";
                println!("{} Global templates not found, downloading from {}", "→".blue(), source.yellow());

                if let Err(e) = manager.download_or_copy_templates(source)
                {
                    eprintln!("{} Failed to download global templates: {}", "✗".red(), e.to_string().red());
                    std::process::exit(1);
                }
            }

            // Install templates to project
            if dry_run == true
            {
                println!("{} Dry run: previewing changes for {} with {}", "→".blue(), lang.green(), agent.green());
            }
            else
            {
                println!("{} Initializing project for {} with {}", "→".blue(), lang.green(), agent.green());
            }
            manager.update(&lang, &agent, force, dry_run)
        }
        | Commands::Update { from, dry_run } =>
        {
            let source = from.as_deref().unwrap_or("https://github.com/heikopanjas/vibe-check/tree/develop/templates");
            if dry_run == true
            {
                println!("{} Dry run: would update global templates from {}", "→".blue(), source.yellow());
                println!("{} Templates would be downloaded to: {}", "→".blue(), manager.get_config_dir().display().to_string().yellow());
                println!("\n{} Dry run complete. No files were modified.", "✓".green());
                Ok(())
            }
            else
            {
                println!("{} Updating global templates from {}", "→".blue(), source.yellow());
                manager.download_or_copy_templates(source)
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
    };

    if let Err(e) = result
    {
        eprintln!("{} {}", "✗".red(), e.to_string().red());
        std::process::exit(1);
    }
}
