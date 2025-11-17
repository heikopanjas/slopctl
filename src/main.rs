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
        /// Programming language or framework (e.g., rust, python, typescript)
        #[arg(long)]
        lang: Option<String>,

        /// AI coding agent (e.g., claude, copilot, codex)
        #[arg(long)]
        agent: Option<String>,

        /// Force overwrite of local files without confirmation
        #[arg(long, default_value = "false")]
        force: bool,

        /// Path or URL to copy/download templates from
        #[arg(long)]
        from: Option<String>
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
    /// Clear local templates from current directory
    Clear
    {
        /// Force clear without confirmation
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
        | Commands::Init { lang, agent, force, from } =>
        {
            // Always update global templates for init command
            let source = from.as_deref().unwrap_or("https://github.com/heikopanjas/vibe-check/tree/develop/templates");
            println!("{} Updating global templates from {}", "→".blue(), source.yellow());

            if let Err(e) = manager.download_or_copy_templates(source)
            {
                eprintln!("{} Failed to update global templates: {}", "✗".red(), e.to_string().red());
                std::process::exit(1);
            }

            // If lang and agent are provided, update local templates
            match (lang, agent)
            {
                | (Some(l), Some(a)) =>
                {
                    println!("{} Installing templates for {} with {}", "→".blue(), l.green(), a.green());
                    manager.update(&l, &a, force)
                }
                | (Some(_), None) =>
                {
                    println!("{} Language specified without agent. Use both --lang and --agent to install templates.", "!".yellow());
                    Ok(())
                }
                | (None, Some(_)) =>
                {
                    println!("{} Agent specified without language. Use both --lang and --agent to install templates.", "!".yellow());
                    Ok(())
                }
                | (None, None) =>
                {
                    println!("{} Global templates downloaded successfully", "✓".green());
                    println!("{} Run with --lang and --agent to install templates to your project", "→".blue());
                    Ok(())
                }
            }
        }
        | Commands::Update { lang, agent, force } => manager.update(&lang, &agent, force),
        | Commands::Clear { force } => manager.clear(force),
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
            else if all == true
            {
                manager.remove_all(force)
            }
            else
            {
                manager.remove(agent.as_ref().unwrap(), force)
            }
        }
    };

    if let Err(e) = result
    {
        eprintln!("{} {}", "✗".red(), e.to_string().red());
        std::process::exit(1);
    }
}
