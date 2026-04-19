use std::{fs, io};

use clap::Parser;
use clap_complete::generate;
use owo_colors::OwoColorize;
use slopctl::{
    Config, MergeOptions, Result, TemplateManager, UpdateOptions,
    cli::{Cli, Commands}
};

/// Default template source URL (V5 templates - agents.md standard)
const DEFAULT_SOURCE_URL: &str = "https://github.com/heikopanjas/slopctl/tree/develop/templates/v5";

/// Resolves template source URL from CLI argument, config, or default
///
/// Returns (source_url, is_configured, fallback_url).
/// Priority: CLI `from` argument > config `source.url` > default URL.
///
/// # Arguments
///
/// * `from` - Optional CLI-provided source URL
fn resolve_source(from: Option<String>) -> (String, bool, Option<String>)
{
    let config = Config::load().ok();
    let configured_source = config.as_ref().and_then(|c| c.get("source.url"));
    let fallback_source = config.as_ref().and_then(|c| c.get("source.fallback"));

    let (source, is_configured) = if let Some(from_url) = from
    {
        (from_url, false)
    }
    else if let Some(config_url) = configured_source
    {
        (config_url, true)
    }
    else
    {
        (DEFAULT_SOURCE_URL.to_string(), false)
    };

    (source, is_configured, fallback_source)
}

/// Downloads or copies templates with automatic fallback
///
/// Tries the primary source first. If it fails and a fallback is configured,
/// retries with the fallback source.
///
/// # Arguments
///
/// * `manager` - Template manager to use for download/copy
/// * `source` - Primary source URL or path
/// * `fallback` - Optional fallback source URL or path
///
/// # Errors
///
/// Returns an error if both primary and fallback sources fail
fn download_with_fallback(manager: &TemplateManager, source: &str, fallback: Option<String>) -> Result<()>
{
    match manager.download_or_copy_templates(source)
    {
        | Ok(()) => Ok(()),
        | Err(e) =>
        {
            if let Some(fallback_url) = fallback
            {
                println!("{} Primary source failed: {}", "!".yellow(), e);
                println!("{} Trying fallback source: {}", "→".blue(), fallback_url.yellow());
                manager.download_or_copy_templates(&fallback_url)
            }
            else
            {
                Err(e)
            }
        }
    }
}

/// Resolves mission content from CLI argument
///
/// If the value starts with `@`, reads content from the specified file path.
/// Otherwise, returns the value as-is.
///
/// # Arguments
///
/// * `value` - The mission argument value (inline text or @filepath)
///
/// # Errors
///
/// Returns an error if the file cannot be read
fn resolve_mission_content(value: &str) -> Result<String>
{
    if let Some(file_path) = value.strip_prefix('@')
    {
        // Read content from file
        fs::read_to_string(file_path).map_err(|e| anyhow::anyhow!("Failed to read mission file '{}': {}", file_path, e))
    }
    else
    {
        // Return inline content as-is
        Ok(value.to_string())
    }
}

/// Handle config command operations
fn handle_config(key: Option<String>, add: Vec<String>, list: bool, remove: Option<String>) -> Result<()>
{
    // Handle --list flag
    if list == true
    {
        let config = Config::load()?;
        let values = config.list();

        if values.is_empty() == true
        {
            println!("{} No configuration values set", "→".blue());
            println!("{} Use 'slopctl config --add <key> <value>' to set a value", "→".blue());
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

    // Handle --add flag
    if add.len() == 2
    {
        let mut config = Config::load()?;
        config.set(&add[0], &add[1])?;
        config.save()?;
        println!("{} Set {} = {}", "✓".green(), add[0].yellow(), add[1].green());
        return Ok(());
    }

    // Handle --remove flag
    if let Some(remove_key) = remove
    {
        let mut config = Config::load()?;
        config.unset(&remove_key)?;
        config.save()?;
        println!("{} Removed {}", "✓".green(), remove_key.yellow());
        return Ok(());
    }

    // Handle get by key
    if let Some(k) = key
    {
        let config = Config::load()?;
        if let Some(v) = config.get(&k)
        {
            println!("{}", v);
        }
        else
        {
            println!("{} Key '{}' is not set", "→".blue(), k.yellow());
        }
        return Ok(());
    }

    // No flags or args: show help
    println!("{}", "slopctl config".bold());
    println!();
    println!("Usage:");
    println!("  slopctl config --add <key> <value>  Set a configuration value");
    println!("  slopctl config <key>                Get a configuration value");
    println!("  slopctl config --list               List all configuration values");
    println!("  slopctl config --remove <key>       Remove a configuration value");
    println!();
    println!("Valid keys:");
    for key in Config::valid_keys()
    {
        println!("  • {}", key.yellow());
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
        | Commands::Init { lang, agent, mission, skill, force, dry_run } =>
        {
            if lang.is_none() == true && agent.is_none() == true && skill.is_empty() == true
            {
                eprintln!("{} Must specify at least one of --lang, --agent, or --skill", "✗".red());
                eprintln!("{} Examples: slopctl init --lang rust", "→".blue());
                eprintln!("{}          slopctl init --agent cursor", "→".blue());
                eprintln!("{}          slopctl init --lang rust --agent cursor", "→".blue());
                eprintln!("{}          slopctl init --skill user/my-skill", "→".blue());
                std::process::exit(1);
            }

            let skill_only = lang.is_none() == true && agent.is_none() == true;

            let resolved_mission = if let Some(ref mission_value) = mission
            {
                match resolve_mission_content(mission_value)
                {
                    | Ok(content) => Some(content),
                    | Err(e) =>
                    {
                        eprintln!("{} {}", "✗".red(), e.to_string().red());
                        std::process::exit(1);
                    }
                }
            }
            else
            {
                None
            };

            let options = UpdateOptions { lang: lang.as_deref(), agent: agent.as_deref(), mission: resolved_mission.as_deref(), skills: &skill, force, dry_run };

            if skill_only == true
            {
                let prefix = if dry_run == true
                {
                    "Dry run: previewing"
                }
                else
                {
                    "Installing"
                };
                println!("{} {} skills", "→".blue(), prefix);
                manager.install_skills(&options)
            }
            else
            {
                if manager.has_global_templates() == false
                {
                    if dry_run == true
                    {
                        println!("{} Global templates not found (would download in non-dry-run mode)", "→".yellow());
                        return;
                    }

                    let (source, is_configured, fallback) = resolve_source(None);

                    if is_configured == true
                    {
                        println!("{} Using configured source", "→".blue());
                    }
                    println!("{} Global templates not found, downloading from {}", "→".blue(), source.yellow());

                    if let Err(e) = download_with_fallback(&manager, &source, fallback)
                    {
                        eprintln!("{} Failed to download global templates: {}", "✗".red(), e);
                        std::process::exit(1);
                    }
                }

                let prefix = if dry_run == true
                {
                    "Dry run: previewing"
                }
                else
                {
                    "Installing"
                };
                match (lang.as_ref(), agent.as_ref())
                {
                    | (Some(l), Some(a)) => println!("{} {} {} with {}", "→".blue(), prefix, l.green(), a.green()),
                    | (Some(l), None) => println!("{} {} {}", "→".blue(), prefix, l.green()),
                    | (None, Some(a)) => println!("{} {} {}", "→".blue(), prefix, a.green()),
                    | (None, None) => println!("{} {} skills", "→".blue(), prefix)
                }

                manager.update(&options)
            }
        }
        | Commands::Templates { update, list, from, dry_run } =>
        {
            if update == false && list == false
            {
                eprintln!("{} Must specify --update or --list", "✗".red());
                eprintln!("{} Examples: slopctl templates --update", "→".blue());
                eprintln!("{}          slopctl templates --list", "→".blue());
                eprintln!("{}          slopctl templates --update --list", "→".blue());
                std::process::exit(1);
            }

            let update_result = if update == true
            {
                let (source, is_configured, fallback) = resolve_source(from);

                if dry_run == true
                {
                    if is_configured == true
                    {
                        println!("{} Using configured source", "→".blue());
                    }
                    println!("{} Dry run: would update global templates from {}", "→".blue(), source.yellow());
                    if let Some(ref fallback_url) = fallback
                    {
                        println!("{} Fallback source configured: {}", "→".blue(), fallback_url.yellow());
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

                    download_with_fallback(&manager, &source, fallback)
                }
            }
            else
            {
                Ok(())
            };

            if let Err(e) = update_result
            {
                Err(e)
            }
            else if list == true
            {
                manager.list_global()
            }
            else
            {
                Ok(())
            }
        }
        | Commands::Purge { force, dry_run } => manager.purge(force, dry_run),
        | Commands::Remove { agent, lang, all, skill, force, dry_run } =>
        {
            if all == true && (agent.is_some() == true || lang.is_some() == true)
            {
                Err(anyhow::anyhow!("Cannot specify --agent or --lang together with --all"))
            }
            else if all == false && agent.is_none() == true && lang.is_none() == true && skill.is_empty() == true
            {
                Err(anyhow::anyhow!("Must specify at least one of --agent, --lang, --all, or --skill"))
            }
            else
            {
                manager.remove(agent.as_deref(), lang.as_deref(), &skill, force, dry_run)
            }
        }
        | Commands::Merge { lang, agent, mission, skill, preview, dry_run, verbose } =>
        {
            let resolved_mission = if let Some(ref mission_value) = mission
            {
                match resolve_mission_content(mission_value)
                {
                    | Ok(content) => Some(content),
                    | Err(e) =>
                    {
                        eprintln!("{} {}", "✗".red(), e.to_string().red());
                        std::process::exit(1);
                    }
                }
            }
            else
            {
                None
            };

            let merge_options = MergeOptions { lang: lang.as_deref(), agent: agent.as_deref(), mission: resolved_mission.as_deref(), skills: &skill };

            if dry_run == true
            {
                println!("{} Dry run: previewing merge candidates", "→".blue());
            }
            else
            {
                println!("{} AI-assisted merge of customized files", "→".blue());
            }
            manager.merge(&merge_options, dry_run, preview, verbose)
        }
        | Commands::ListModels { provider } => manager.list_models(provider.as_deref()),
        | Commands::Completions { shell } =>
        {
            let shell: clap_complete::Shell = shell.into();
            generate(shell, &mut Cli::command(), "slopctl", &mut io::stdout());
            Ok(())
        }
        | Commands::Doctor { fix, dry_run, verbose, smart } => manager.doctor(fix, dry_run, verbose, smart),
        | Commands::Status { verbose } => manager.status(verbose),
        | Commands::Config { key, add, list, remove } => handle_config(key, add, list, remove)
    };

    if let Err(e) = result
    {
        eprintln!("{} {}", "✗".red(), e.to_string().red());
        std::process::exit(1);
    }
}
