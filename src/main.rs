#![allow(clippy::bool_comparison)]

use std::{fs, io};

use clap::Parser;
use clap_complete::generate;
use owo_colors::OwoColorize;
use slopctl::{
    Config, EffectiveConfig, MergeOptions, Result, TemplateManager, UpdateOptions,
    agent_defaults::AGENT_DEFAULTS_FILE,
    cli::{Cli, Commands},
    model_defaults::MODEL_DEFAULTS_FILE
};

/// Default template source URL (V5 templates - agents.md standard)
const DEFAULT_SOURCE_URL: &str = "https://github.com/heikopanjas/slopctl/tree/develop/templates/v5";

/// Default agent defaults source URL
const DEFAULT_AGENTS_SOURCE_URL: &str = "https://github.com/heikopanjas/slopctl/tree/develop/templates/v5";

/// Default model defaults source URL
const DEFAULT_MODELS_SOURCE_URL: &str = "https://github.com/heikopanjas/slopctl/tree/develop/templates/v5";

/// Resolves template source URL from CLI argument, config, or default
///
/// Returns (source_url, is_configured, fallback_url).
/// Priority: CLI `from` argument > effective config `templates.uri` > default URL.
///
/// The effective config merges workspace + global with per-key precedence.
fn resolve_source(from: Option<String>) -> (String, bool, Option<String>)
{
    let cwd = std::env::current_dir().ok();
    let effective = cwd.as_deref().and_then(|w| EffectiveConfig::load(w).ok());
    let configured_source = effective.as_ref().and_then(|c| c.get("templates.uri"));
    let fallback_source = effective.as_ref().and_then(|c| c.get("templates.fallbackUri"));

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

/// Resolves agent defaults source URL from CLI argument, config, or default
///
/// Returns (source_url, is_configured, fallback_url).
fn resolve_agents_source(from: Option<String>) -> (String, bool, Option<String>)
{
    let cwd = std::env::current_dir().ok();
    let effective = cwd.as_deref().and_then(|w| EffectiveConfig::load(w).ok());
    let configured_source = effective.as_ref().and_then(|c| c.get("agents.uri"));
    let fallback_source = effective.as_ref().and_then(|c| c.get("agents.fallbackUri"));

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
        (DEFAULT_AGENTS_SOURCE_URL.to_string(), false)
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

/// Downloads or copies agent defaults with automatic fallback
///
/// # Errors
///
/// Returns an error if both primary and fallback sources fail.
fn download_agent_defaults_with_fallback(manager: &TemplateManager, source: &str, fallback: Option<String>) -> Result<()>
{
    match manager.download_or_copy_agent_defaults(source)
    {
        | Ok(()) => Ok(()),
        | Err(e) =>
        {
            if let Some(fallback_url) = fallback
            {
                println!("{} Primary agent defaults source failed: {}", "!".yellow(), e);
                println!("{} Trying fallback source: {}", "→".blue(), fallback_url.yellow());
                manager.download_or_copy_agent_defaults(&fallback_url)
            }
            else
            {
                Err(e)
            }
        }
    }
}

/// Bootstrap agent defaults after template download when the catalog is missing
///
/// # Errors
///
/// Returns an error if all bootstrap sources fail.
fn bootstrap_agent_defaults_if_missing(manager: &TemplateManager, template_source: &str, dry_run: bool) -> Result<()>
{
    if manager.has_agent_defaults() == true
    {
        return Ok(());
    }

    let cwd = std::env::current_dir().ok();
    let effective = cwd.as_deref().and_then(|w| EffectiveConfig::load(w).ok());
    let configured_agent_source = effective.as_ref().and_then(|c| c.get("agents.uri"));
    let configured_agent_fallback = effective.as_ref().and_then(|c| c.get("agents.fallbackUri"));

    let mut candidates: Vec<String> = Vec::new();
    for candidate in
        [configured_agent_source, configured_agent_fallback, Some(template_source.to_string()), Some(DEFAULT_AGENTS_SOURCE_URL.to_string())].into_iter().flatten()
    {
        if candidates.contains(&candidate) == false
        {
            candidates.push(candidate);
        }
    }

    if dry_run == true
    {
        println!("{} Missing {}; would bootstrap from {}", "→".blue(), AGENT_DEFAULTS_FILE.yellow(), candidates.join(", ").yellow());
        return Ok(());
    }

    for source in candidates
    {
        match manager.download_or_copy_agent_defaults(&source)
        {
            | Ok(()) => return Ok(()),
            | Err(e) => println!("{} Could not bootstrap {} from {}: {}", "!".yellow(), AGENT_DEFAULTS_FILE.yellow(), source.yellow(), e)
        }
    }

    Err(anyhow::anyhow!("Failed to bootstrap {}; run slopctl agents --update", AGENT_DEFAULTS_FILE))
}

/// Resolves model defaults source URL from CLI argument, config, or default
///
/// Returns (source_url, is_configured, fallback_url).
fn resolve_models_source(from: Option<String>) -> (String, bool, Option<String>)
{
    let cwd = std::env::current_dir().ok();
    let effective = cwd.as_deref().and_then(|w| EffectiveConfig::load(w).ok());
    let configured_source = effective.as_ref().and_then(|c| c.get("models.uri"));
    let fallback_source = effective.as_ref().and_then(|c| c.get("models.fallbackUri"));

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
        (DEFAULT_MODELS_SOURCE_URL.to_string(), false)
    };

    (source, is_configured, fallback_source)
}

/// Downloads or copies model defaults with automatic fallback
///
/// # Errors
///
/// Returns an error if both primary and fallback sources fail.
fn download_model_defaults_with_fallback(manager: &TemplateManager, source: &str, fallback: Option<String>) -> Result<()>
{
    match manager.download_or_copy_model_defaults(source)
    {
        | Ok(()) => Ok(()),
        | Err(e) =>
        {
            if let Some(fallback_url) = fallback
            {
                println!("{} Primary model defaults source failed: {}", "!".yellow(), e);
                println!("{} Trying fallback source: {}", "→".blue(), fallback_url.yellow());
                manager.download_or_copy_model_defaults(&fallback_url)
            }
            else
            {
                Err(e)
            }
        }
    }
}

/// Bootstrap model defaults after template download when the catalog is missing
///
/// # Errors
///
/// Returns an error if all bootstrap sources fail.
fn bootstrap_model_defaults_if_missing(manager: &TemplateManager, template_source: &str, dry_run: bool) -> Result<()>
{
    if manager.has_model_defaults() == true
    {
        return Ok(());
    }

    let cwd = std::env::current_dir().ok();
    let effective = cwd.as_deref().and_then(|w| EffectiveConfig::load(w).ok());
    let configured_model_source = effective.as_ref().and_then(|c| c.get("models.uri"));
    let configured_model_fallback = effective.as_ref().and_then(|c| c.get("models.fallbackUri"));

    let mut candidates: Vec<String> = Vec::new();
    for candidate in
        [configured_model_source, configured_model_fallback, Some(template_source.to_string()), Some(DEFAULT_MODELS_SOURCE_URL.to_string())].into_iter().flatten()
    {
        if candidates.contains(&candidate) == false
        {
            candidates.push(candidate);
        }
    }

    if dry_run == true
    {
        println!("{} Missing {}; would bootstrap from {}", "→".blue(), MODEL_DEFAULTS_FILE.yellow(), candidates.join(", ").yellow());
        return Ok(());
    }

    for source in candidates
    {
        match manager.download_or_copy_model_defaults(&source)
        {
            | Ok(()) => return Ok(()),
            | Err(e) => println!("{} Could not bootstrap {} from {}: {}", "!".yellow(), MODEL_DEFAULTS_FILE.yellow(), source.yellow(), e)
        }
    }

    Err(anyhow::anyhow!("Failed to bootstrap {}; run slopctl models --update", MODEL_DEFAULTS_FILE))
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
///
/// Without `--global`, writes target the workspace config (`.slopctl/config.yml`)
/// and reads show the merged effective view (workspace wins over global).
/// With `--global`, all operations target the global config only.
fn handle_config(key: Option<String>, set: Vec<String>, list: bool, delete: Option<String>, global: bool) -> Result<()>
{
    let cwd = std::env::current_dir()?;

    if list == true
    {
        if global == true
        {
            let config = Config::load_global()?;
            let values = config.list();

            if values.is_empty() == true
            {
                println!("{} No global configuration values set", "→".blue());
                println!("{} Use 'slopctl config --global --set <key> <value>' to set a value", "→".blue());
                println!("{} Valid keys: {}", "→".blue(), Config::valid_keys().join(", ").yellow());
            }
            else
            {
                println!("{}", "Global configuration:".bold());
                for (k, v) in &values
                {
                    println!("  {} = {}", k.green(), v.yellow());
                }
            }
        }
        else
        {
            let effective = EffectiveConfig::load(&cwd)?;
            let values = effective.list_with_origin();

            if values.is_empty() == true
            {
                println!("{} No configuration values set", "→".blue());
                println!("{} Use 'slopctl config --set <key> <value>' to set a workspace value", "→".blue());
                println!("{} Use 'slopctl config --global --set <key> <value>' to set a global value", "→".blue());
                println!("{} Valid keys: {}", "→".blue(), Config::valid_keys().join(", ").yellow());
            }
            else
            {
                println!("{}", "Configuration:".bold());
                for (k, (v, scope)) in &values
                {
                    println!("  {} = {} {}", k.green(), v.yellow(), format!("[{}]", scope).dimmed());
                }
            }
        }
        return Ok(());
    }

    if set.len() == 2
    {
        if global == true
        {
            let mut config = Config::load_global()?;
            config.set(&set[0], &set[1])?;
            config.save_global()?;
            println!("{} Set {} = {} {}", "✓".green(), set[0].yellow(), set[1].green(), "[global]".dimmed());
        }
        else
        {
            let mut config = Config::load_workspace(&cwd)?;
            config.set(&set[0], &set[1])?;
            config.save_workspace(&cwd)?;
            println!("{} Set {} = {} {}", "✓".green(), set[0].yellow(), set[1].green(), "[workspace]".dimmed());
        }
        return Ok(());
    }

    if let Some(delete_key) = delete
    {
        if global == true
        {
            let mut config = Config::load_global()?;
            if config.get(&delete_key).is_none() == true
            {
                let ws = Config::load_workspace(&cwd)?;
                if ws.get(&delete_key).is_some() == true
                {
                    println!("{} Key '{}' is not set in global config; it exists in workspace config — try without --global", "→".blue(), delete_key.yellow());
                }
                else
                {
                    println!("{} Key '{}' is not set in global config", "→".blue(), delete_key.yellow());
                }
            }
            else
            {
                config.unset(&delete_key)?;
                config.save_global()?;
                println!("{} Deleted {} {}", "✓".green(), delete_key.yellow(), "[global]".dimmed());
            }
        }
        else
        {
            let mut config = Config::load_workspace(&cwd)?;
            if config.get(&delete_key).is_none() == true
            {
                let gl = Config::load_global()?;
                if gl.get(&delete_key).is_some() == true
                {
                    println!("{} Key '{}' is not set in workspace config; it exists in global config — try --global", "→".blue(), delete_key.yellow());
                }
                else
                {
                    println!("{} Key '{}' is not set in workspace config", "→".blue(), delete_key.yellow());
                }
            }
            else
            {
                config.unset(&delete_key)?;
                config.save_workspace(&cwd)?;
                println!("{} Deleted {} {}", "✓".green(), delete_key.yellow(), "[workspace]".dimmed());
            }
        }
        return Ok(());
    }

    if let Some(k) = key
    {
        if global == true
        {
            let config = Config::load_global()?;
            if let Some(v) = config.get(&k)
            {
                println!("{}", v);
            }
            else
            {
                println!("{} Key '{}' is not set in global config", "→".blue(), k.yellow());
            }
        }
        else
        {
            let effective = EffectiveConfig::load(&cwd)?;
            if let Some(v) = effective.get(&k)
            {
                println!("{}", v);
            }
            else
            {
                println!("{} Key '{}' is not set", "→".blue(), k.yellow());
            }
        }
        return Ok(());
    }

    // No flags or args: show help
    println!("{}", "slopctl config".bold());
    println!();
    println!("Usage:");
    println!("  slopctl config --set <key> <value>    Set a workspace configuration value");
    println!("  slopctl config --global --set <k> <v> Set a global configuration value");
    println!("  slopctl config <key>                  Get effective value (workspace > global)");
    println!("  slopctl config --list                 List effective configuration");
    println!("  slopctl config --global --list        List global configuration only");
    println!("  slopctl config --delete <key>         Delete from workspace configuration");
    println!("  slopctl config --global --delete <k>  Delete from global configuration");
    println!();
    println!("Workspace config: {}", Config::get_workspace_path(&cwd).display().to_string().yellow());
    if let Ok(gp) = Config::get_global_path()
    {
        println!("Global config:    {}", gp.display().to_string().yellow());
    }
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
        | Commands::Init { lang, agent, mission, force, dry_run } =>
        {
            if lang.is_none() == true && agent.is_none() == true
            {
                eprintln!("{} Must specify at least one of --lang or --agent", "✗".red());
                eprintln!("{} Examples: slopctl init --lang <language>", "→".blue());
                eprintln!("{}          slopctl init --agent <agent>", "→".blue());
                eprintln!("{}          slopctl init --lang <language> --agent <agent>", "→".blue());
                std::process::exit(1);
            }

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

            let options = UpdateOptions { lang: lang.as_deref(), agent: agent.as_deref(), mission: resolved_mission.as_deref(), force, dry_run };

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
                    if let Err(e) = bootstrap_agent_defaults_if_missing(&manager, &source, false)
                    {
                        eprintln!("{} Failed to bootstrap agent defaults: {}", "✗".red(), e);
                        std::process::exit(1);
                    }
                    if let Err(e) = bootstrap_model_defaults_if_missing(&manager, &source, false)
                    {
                        eprintln!("{} Failed to bootstrap model defaults: {}", "✗".red(), e);
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
                    | (None, None) => unreachable!("validated at least one init target")
                }

                manager.update(&options)
            }
        }
        | Commands::Templates { update, list, verify, from, dry_run } =>
        {
            if update == false && list == false && verify == false
            {
                eprintln!("{} Must specify --update, --list, or --verify", "✗".red());
                eprintln!("{} Examples: slopctl templates --update", "→".blue());
                eprintln!("{}          slopctl templates --list", "→".blue());
                eprintln!("{}          slopctl templates --verify", "→".blue());
                eprintln!("{}          slopctl templates --update --list", "→".blue());
                std::process::exit(1);
            }

            if from.is_some() == true && update == false && verify == false
            {
                eprintln!("{} --from requires --update or --verify", "✗".red());
                std::process::exit(1);
            }

            let (source, is_configured, fallback) = resolve_source(from);

            let update_result = if update == true
            {
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
                    if let Err(e) = bootstrap_agent_defaults_if_missing(&manager, &source, true)
                    {
                        Err(e)
                    }
                    else if let Err(e) = bootstrap_model_defaults_if_missing(&manager, &source, true)
                    {
                        Err(e)
                    }
                    else
                    {
                        println!("\n{} Dry run complete. No files were modified.", "✓".green());
                        Ok(())
                    }
                }
                else
                {
                    if is_configured == true
                    {
                        println!("{} Using configured source", "→".blue());
                    }
                    println!("{} Updating global templates from {}", "→".blue(), source.yellow());
                    download_with_fallback(&manager, &source, fallback)
                        .and_then(|()| bootstrap_agent_defaults_if_missing(&manager, &source, false))
                        .and_then(|()| bootstrap_model_defaults_if_missing(&manager, &source, false))
                }
            }
            else
            {
                Ok(())
            };

            update_result
                .and_then(|()| {
                    if verify == true
                    {
                        manager.verify(&source)
                    }
                    else
                    {
                        Ok(())
                    }
                })
                .and_then(|()| {
                    if list == true
                    {
                        manager.list_global()
                    }
                    else
                    {
                        Ok(())
                    }
                })
        }
        | Commands::Agents { update, list, verify, from, dry_run } =>
        {
            if update == false && list == false && verify == false
            {
                eprintln!("{} Must specify --update, --list, or --verify", "✗".red());
                eprintln!("{} Examples: slopctl agents --update", "→".blue());
                eprintln!("{}          slopctl agents --list", "→".blue());
                eprintln!("{}          slopctl agents --verify", "→".blue());
                eprintln!("{}          slopctl agents --update --list", "→".blue());
                std::process::exit(1);
            }

            if from.is_some() == true && update == false && verify == false
            {
                eprintln!("{} --from requires --update or --verify", "✗".red());
                std::process::exit(1);
            }

            let (source, is_configured, fallback) = resolve_agents_source(from);

            let update_result = if update == true
            {
                if dry_run == true
                {
                    if is_configured == true
                    {
                        println!("{} Using configured agent defaults source", "→".blue());
                    }
                    println!("{} Dry run: would update agent defaults from {}", "→".blue(), source.yellow());
                    if let Some(ref fallback_url) = fallback
                    {
                        println!("{} Fallback source configured: {}", "→".blue(), fallback_url.yellow());
                    }
                    println!(
                        "{} Agent defaults would be downloaded to: {}",
                        "→".blue(),
                        manager.get_config_dir().join(AGENT_DEFAULTS_FILE).display().to_string().yellow()
                    );
                    println!("\n{} Dry run complete. No files were modified.", "✓".green());
                    Ok(())
                }
                else
                {
                    if is_configured == true
                    {
                        println!("{} Using configured agent defaults source", "→".blue());
                    }
                    println!("{} Updating agent defaults from {}", "→".blue(), source.yellow());
                    download_agent_defaults_with_fallback(&manager, &source, fallback)
                }
            }
            else
            {
                Ok(())
            };

            update_result
                .and_then(|()| {
                    if verify == true
                    {
                        manager.verify_agents(&source)
                    }
                    else
                    {
                        Ok(())
                    }
                })
                .and_then(|()| {
                    if list == true
                    {
                        manager.list_agents()
                    }
                    else
                    {
                        Ok(())
                    }
                })
        }
        | Commands::Remove { agent, lang, all, purge, force, dry_run } =>
        {
            if purge == true
            {
                manager.remove_purge(force, dry_run)
            }
            else if all == true && (agent.is_some() == true || lang.is_some() == true)
            {
                Err(anyhow::anyhow!("Cannot specify --agent or --lang together with --all"))
            }
            else if all == false && agent.is_none() == true && lang.is_none() == true
            {
                Err(anyhow::anyhow!("Must specify at least one of --agent, --lang, --all, or --purge"))
            }
            else
            {
                manager.remove(agent.as_deref(), lang.as_deref(), force, dry_run)
            }
        }
        | Commands::Merge { lang, agent, mission, preview, dry_run, verbose } =>
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

            let merge_options = MergeOptions { lang: lang.as_deref(), agent: agent.as_deref(), mission: resolved_mission.as_deref() };

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
        | Commands::Models { update, list, verify, from, dry_run } =>
        {
            if update == false && list == false && verify == false
            {
                eprintln!("{} Must specify --update, --list, or --verify", "✗".red());
                eprintln!("{} Examples: slopctl models --update", "→".blue());
                eprintln!("{}          slopctl models --list", "→".blue());
                eprintln!("{}          slopctl models --verify", "→".blue());
                eprintln!("{}          slopctl models --update --list", "→".blue());
                std::process::exit(1);
            }

            if from.is_some() == true && update == false && verify == false
            {
                eprintln!("{} --from requires --update or --verify", "✗".red());
                std::process::exit(1);
            }

            let (source, is_configured, fallback) = resolve_models_source(from);

            let update_result = if update == true
            {
                if dry_run == true
                {
                    if is_configured == true
                    {
                        println!("{} Using configured model defaults source", "→".blue());
                    }
                    println!("{} Dry run: would update model defaults from {}", "→".blue(), source.yellow());
                    if let Some(ref fallback_url) = fallback
                    {
                        println!("{} Fallback source configured: {}", "→".blue(), fallback_url.yellow());
                    }
                    println!(
                        "{} Model defaults would be downloaded to: {}",
                        "→".blue(),
                        manager.get_config_dir().join(MODEL_DEFAULTS_FILE).display().to_string().yellow()
                    );
                    println!("\n{} Dry run complete. No files were modified.", "✓".green());
                    Ok(())
                }
                else
                {
                    if is_configured == true
                    {
                        println!("{} Using configured model defaults source", "→".blue());
                    }
                    println!("{} Updating model defaults from {}", "→".blue(), source.yellow());
                    download_model_defaults_with_fallback(&manager, &source, fallback)
                }
            }
            else
            {
                Ok(())
            };

            update_result
                .and_then(|()| {
                    if verify == true
                    {
                        manager.verify_models(&source)
                    }
                    else
                    {
                        Ok(())
                    }
                })
                .and_then(|()| {
                    if list == true
                    {
                        manager.list_models_catalog()
                    }
                    else
                    {
                        Ok(())
                    }
                })
        }
        | Commands::Completions { shell } =>
        {
            let shell: clap_complete::Shell = shell.into();
            generate(shell, &mut Cli::command(), "slopctl", &mut io::stdout());
            Ok(())
        }
        | Commands::Doctor { fix, dry_run, verbose, smart } => manager.doctor(fix, dry_run, verbose, smart),
        | Commands::Status { verbose } => manager.status(verbose),
        | Commands::Config { key, set, list, delete, global } => handle_config(key, set, list, delete, global)
    };

    if let Err(e) = result
    {
        eprintln!("{} {}", "✗".red(), e.to_string().red());
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    static CWD_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn test_resolve_mission_content_inline()
    {
        let result = resolve_mission_content("Build a CLI tool").unwrap();
        assert_eq!(result, "Build a CLI tool");
    }

    #[test]
    fn test_resolve_mission_content_from_file()
    {
        let dir = tempfile::TempDir::new().unwrap();
        let file = dir.path().join("mission.md");
        fs::write(&file, "# My Mission\nBuild great software.\n").unwrap();

        let result = resolve_mission_content(&format!("@{}", file.display())).unwrap();
        assert!(result.contains("My Mission") == true);
    }

    #[test]
    fn test_resolve_mission_content_missing_file()
    {
        let result = resolve_mission_content("@/nonexistent/path/mission.md");
        assert!(result.is_err() == true);
        assert!(result.unwrap_err().to_string().contains("Failed to read mission file") == true);
    }

    #[test]
    fn test_resolve_source_returns_default_when_no_args()
    {
        let (source, is_configured, _fallback) = resolve_source(None);
        assert!(source.is_empty() == false);
        assert!(is_configured == false);
    }

    #[test]
    fn test_resolve_source_returns_from_arg()
    {
        let (source, _is_configured, _) = resolve_source(Some("/tmp/my-templates".to_string()));
        assert_eq!(source, "/tmp/my-templates");
    }

    #[test]
    fn test_resolve_agents_source_returns_default()
    {
        let (source, is_configured, _) = resolve_agents_source(None);
        assert!(source.is_empty() == false);
        assert!(is_configured == false);
    }

    #[test]
    fn test_resolve_models_source_returns_default()
    {
        let (source, is_configured, _) = resolve_models_source(None);
        assert!(source.is_empty() == false);
        assert!(is_configured == false);
    }

    #[test]
    fn test_handle_config_list_empty_workspace() -> Result<()>
    {
        let _lock = CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let workspace = tempfile::TempDir::new()?;
        let original = std::env::current_dir()?;
        std::env::set_current_dir(workspace.path())?;

        let result = handle_config(None, vec![], true, None, false);
        let _ = std::env::set_current_dir(&original);
        result
    }

    #[test]
    fn test_handle_config_set_and_get() -> Result<()>
    {
        let _lock = CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let workspace = tempfile::TempDir::new()?;
        let original = std::env::current_dir()?;
        std::env::set_current_dir(workspace.path())?;

        handle_config(None, vec!["merge.provider".into(), "ollama".into()], false, None, false)?;
        let result = handle_config(Some("merge.provider".into()), vec![], false, None, false);
        let _ = std::env::set_current_dir(&original);
        result
    }

    #[test]
    fn test_handle_config_delete() -> Result<()>
    {
        let _lock = CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let workspace = tempfile::TempDir::new()?;
        let original = std::env::current_dir()?;
        std::env::set_current_dir(workspace.path())?;

        handle_config(None, vec!["merge.provider".into(), "ollama".into()], false, None, false)?;
        let result = handle_config(None, vec![], false, Some("merge.provider".into()), false);
        let _ = std::env::set_current_dir(&original);
        result
    }

    #[test]
    fn test_handle_config_global_list() -> Result<()>
    {
        handle_config(None, vec![], true, None, true)?;
        Ok(())
    }
}
