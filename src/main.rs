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
        lang: String,

        /// AI coding agent (e.g., claude, copilot, codex)
        #[arg(long)]
        agent: String,

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

            // Now update local templates with the force flag
            manager.update(&lang, &agent, force)
        }
        | Commands::Update { lang, agent, force } => manager.update(&lang, &agent, force),
        | Commands::Clear { force } => manager.clear(force)
    };

    if let Err(e) = result
    {
        eprintln!("{} {}", "✗".red(), e.to_string().red());
        std::process::exit(1);
    }
}
