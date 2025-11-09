use clap::{Parser, Subcommand};
use owo_colors::OwoColorize;
use vibe_check::TemplateManager;

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
        agent: String,

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
        force: bool,

        /// Path or URL to copy/download templates from
        #[arg(long)]
        from: Option<String>
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
        | Commands::Init { lang, agent, from } => manager.update(&lang, &agent, false, from.as_deref()),
        | Commands::Update { lang, agent, force, from } => manager.update(&lang, &agent, force, from.as_deref()),
        | Commands::Clear { force } => manager.clear(force)
    };

    if let Err(e) = result
    {
        eprintln!("{} {}", "✗".red(), e.to_string().red());
        std::process::exit(1);
    }
}
