use clap::{Parser, Subcommand};
use owo_colors::OwoColorize;
use vibe_check::TemplateManager;

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
        force: bool
    },
    /// Update global templates from source
    Update
    {
        /// Path or URL to download/copy templates from
        #[arg(long)]
        from: Option<String>
    },
    /// Purge all vibe-check files from project
    Purge
    {
        /// Force purge without confirmation
        #[arg(long, default_value = "false")]
        force: bool
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
        force: bool
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
        | Commands::Init { lang, agent, force } =>
        {
            // Check if global templates exist, download if not
            if manager.has_global_templates() == false
            {
                let source = "https://github.com/heikopanjas/vibe-check/tree/develop/templates";
                println!("{} Global templates not found, downloading from {}", "→".blue(), source.yellow());

                if let Err(e) = manager.download_or_copy_templates(source)
                {
                    eprintln!("{} Failed to download global templates: {}", "✗".red(), e.to_string().red());
                    std::process::exit(1);
                }
            }

            // Install templates to project
            println!("{} Initializing project for {} with {}", "→".blue(), lang.green(), agent.green());
            manager.update(&lang, &agent, force)
        }
        | Commands::Update { from } =>
        {
            let source = from.as_deref().unwrap_or("https://github.com/heikopanjas/vibe-check/tree/develop/templates");
            println!("{} Updating global templates from {}", "→".blue(), source.yellow());
            manager.download_or_copy_templates(source)
        }
        | Commands::Purge { force } => manager.purge(force),
        | Commands::Remove { agent, all, force } =>
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
                manager.remove(agent.as_deref(), force)
            }
        }
    };

    if let Err(e) = result
    {
        eprintln!("{} {}", "✗".red(), e.to_string().red());
        std::process::exit(1);
    }
}
