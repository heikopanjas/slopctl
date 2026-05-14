//! CLI argument definitions shared between the binary and build script

use clap::{CommandFactory, Parser, Subcommand, ValueEnum};

/// Supported shells for completion generation
#[derive(Clone, Copy, ValueEnum)]
pub enum ShellType
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
#[command(name = "slopctl")]
#[command(about = "A manager for coding agent instruction files", long_about = None)]
#[command(version)]
pub struct Cli
{
    #[command(subcommand)]
    pub command: Commands
}

impl Cli
{
    /// Returns the clap `Command` for use in man page and completion generation
    pub fn command() -> clap::Command
    {
        <Self as CommandFactory>::command()
    }
}

#[derive(Subcommand)]
pub enum Commands
{
    /// Initialize agent instructions and skills for a project
    Init
    {
        /// Programming language or framework (single value; use merge to add another)
        #[arg(short, long)]
        lang: Option<String>,

        /// AI coding agent (e.g., claude, copilot, codex, cursor)
        #[arg(short, long)]
        agent: Option<String>,

        /// Custom mission statement (use @filename to read from file)
        #[arg(short, long)]
        mission: Option<String>,

        /// Force overwrite of local files without confirmation
        #[arg(short, long, default_value = "false")]
        force: bool,

        /// Preview changes without applying them
        #[arg(short = 'n', long, default_value = "false")]
        dry_run: bool
    },
    /// Manage global template catalog
    Templates
    {
        /// Download or update global templates from source
        #[arg(short, long, default_value = "false")]
        update: bool,

        /// Show available agents, languages, and skills
        #[arg(short, long, default_value = "false")]
        list: bool,

        /// Verify local templates: YAML validity, file integrity, and source freshness
        #[arg(short = 'V', long, default_value = "false")]
        verify: bool,

        /// Path or URL to use as source (applies to --update and --verify)
        #[arg(short, long)]
        from: Option<String>,

        /// Preview changes without applying them
        #[arg(short = 'n', long, default_value = "false", requires = "update")]
        dry_run: bool
    },
    /// Remove installed files from the current workspace
    Remove
    {
        /// AI coding agent (e.g., claude, copilot, codex, cursor)
        #[arg(short, long, conflicts_with = "purge")]
        agent: Option<String>,

        /// Programming language or framework (e.g., rust, c++, swift)
        #[arg(short, long, conflicts_with = "purge")]
        lang: Option<String>,

        /// Remove all agent files and skills; AGENTS.md is kept
        #[arg(long, default_value = "false", conflicts_with = "purge")]
        all: bool,

        /// Remove everything slopctl installed, including AGENTS.md.
        /// A customized AGENTS.md is preserved unless --force is also given.
        #[arg(long, default_value = "false")]
        purge: bool,

        /// Skip confirmation prompt.
        /// With --purge: also deletes a customized AGENTS.md.
        #[arg(short, long, default_value = "false")]
        force: bool,

        /// Preview what would be removed without making changes
        #[arg(short = 'n', long, default_value = "false")]
        dry_run: bool
    },
    /// Generate shell completions
    Completions
    {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: ShellType
    },
    /// Check workspace for stale or broken managed files
    Doctor
    {
        /// Automatically fix detected issues where possible
        #[arg(short, long, default_value = "false")]
        fix: bool,

        /// Preview changes without applying them
        #[arg(short = 'n', long, default_value = "false")]
        dry_run: bool,

        /// Print every checked file and its result
        #[arg(short, long, default_value = "false")]
        verbose: bool,

        /// Use an LLM to lint AGENTS.md for contradictions, stale references, and unclear instructions
        #[arg(long, default_value = "false")]
        smart: bool
    },
    /// Show workspace status
    Status
    {
        /// Show managed file list
        #[arg(short, long, default_value = "false")]
        verbose: bool
    },
    /// AI-assisted merge of customized files with updated templates
    Merge
    {
        /// Programming language or framework (e.g., rust, c++, swift)
        #[arg(short, long)]
        lang: Option<String>,

        /// AI coding agent (e.g., claude, copilot, codex, cursor)
        #[arg(short, long)]
        agent: Option<String>,

        /// Custom mission statement (use @filename to read from file)
        #[arg(short, long)]
        mission: Option<String>,

        /// Write merged output to .merged sidecar files instead of replacing originals
        #[arg(long, default_value = "false")]
        preview: bool,

        /// Show merge candidates without calling the LLM
        #[arg(short = 'n', long, default_value = "false")]
        dry_run: bool,

        /// Show token usage, list unchanged files, and print the outgoing/incoming chat messages for each merge
        #[arg(short, long, default_value = "false")]
        verbose: bool
    },
    /// List available models from an LLM provider
    ListModels
    {
        /// LLM provider to query (overrides config and auto-detected provider)
        #[arg(short, long)]
        provider: Option<String>
    },
    /// Manage configuration
    Config
    {
        /// Configuration key to get (e.g., templates.uri)
        key: Option<String>,

        /// Set a configuration value: --set <key> <value>
        #[arg(short, long, num_args = 2, value_names = ["KEY", "VALUE"])]
        set: Vec<String>,

        /// List all configuration values
        #[arg(short, long, default_value = "false")]
        list: bool,

        /// Delete a configuration key
        #[arg(short, long)]
        delete: Option<String>,

        /// Operate on the global config (~/.config/slopctl/config.yml) instead of the workspace config
        #[arg(short = 'g', long, default_value = "false")]
        global: bool
    }
}
