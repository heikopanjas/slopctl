use std::{env, fs, path::PathBuf};

use clap::{CommandFactory, Parser, Subcommand, ValueEnum};

/// Supported shells for completion generation
#[derive(Clone, Copy, ValueEnum)]
enum ShellType
{
    Bash,
    Fish,
    Powershell,
    Zsh
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
    },
    /// Show current project status
    Status,
    /// List available agents and languages
    List
}

fn main()
{
    // Only generate man pages for release builds
    let profile = env::var("PROFILE").unwrap_or_default();
    if profile != "release"
    {
        return;
    }

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let man_dir = out_dir.join("man");
    fs::create_dir_all(&man_dir).unwrap();

    let cmd = Cli::command();
    let man = clap_mangen::Man::new(cmd);
    let mut buffer: Vec<u8> = Vec::new();
    man.render(&mut buffer).unwrap();

    let man_path = man_dir.join("vibe-check.1");
    fs::write(&man_path, buffer).unwrap();

    println!("cargo:warning=Man page generated at: {}", man_path.display());
}
