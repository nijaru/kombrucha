//! Utility and helper commands
//!
//! Miscellaneous utility commands including listing available commands,
//! finding which formula provides a command, managing aliases, and
//! accessing documentation.

use crate::api::BrewApi;
use crate::cellar;
use crate::error::Result;
use colored::Colorize;
use std::collections::HashMap;

/// List all available bru commands
///
/// Displays a formatted list of all commands with brief descriptions.
pub fn commands() -> Result<()> {
    println!("{}", "==> Available Commands".bold().green());
    println!();

    let commands_list = vec![
        ("search <query>", "Search for formulae and casks"),
        ("search <query> --formula", "Search only formulae"),
        ("search <query> --cask", "Search only casks"),
        ("info <formula>", "Show information about a formula or cask"),
        ("info <formula> --json", "Show formula info as JSON"),
        ("desc <formula>...", "Show formula descriptions"),
        ("deps <formula>", "Show dependencies for a formula"),
        (
            "deps <formula> --installed",
            "Show only installed dependencies",
        ),
        ("uses <formula>", "Show formulae that depend on a formula"),
        ("list", "List installed packages"),
        ("outdated", "Show outdated installed packages"),
        ("fetch <formula>...", "Download bottles for formulae"),
        ("install <formula>...", "Install formulae from bottles"),
        ("upgrade [formula...]", "Upgrade installed formulae"),
        ("reinstall <formula>...", "Reinstall formulae"),
        ("uninstall <formula>...", "Uninstall formulae"),
        ("autoremove", "Remove unused dependencies"),
        ("link <formula>...", "Link a formula"),
        ("unlink <formula>...", "Unlink a formula"),
        (
            "cleanup [formula...]",
            "Remove old versions of installed formulae",
        ),
        ("cache", "Manage download cache"),
        ("tap [user/repo]", "Add or list third-party repositories"),
        ("untap <user/repo>", "Remove a third-party repository"),
        ("update", "Update Homebrew and all taps"),
        ("config", "Show system configuration"),
        ("doctor", "Check system for potential problems"),
        ("home <formula>", "Open formula homepage in browser"),
        ("leaves", "List packages not required by others"),
        ("pin <formula>...", "Pin formulae to prevent upgrades"),
        ("unpin <formula>...", "Unpin formulae to allow upgrades"),
        ("missing [formula...]", "Check for missing dependencies"),
        ("analytics [on|off|state]", "Control analytics"),
        ("cat <formula>...", "Print formula source code"),
        ("shellenv [--shell <shell>]", "Print shell configuration"),
        ("gist-logs [formula]", "Generate diagnostic information"),
        ("alias [formula]", "Show formula aliases"),
        ("log <formula>", "Show install logs"),
        ("commands", "List all available commands"),
        ("completions <shell>", "Generate shell completion scripts"),
    ];

    for (cmd, desc) in &commands_list {
        println!("  {} {}", cmd.cyan().bold(), desc.dimmed());
    }

    println!();
    println!(
        "{} {} commands available",
        "ℹ".blue(),
        commands_list.len().to_string().bold()
    );
    println!("Run {} for help", "bru --help".cyan());

    Ok(())
}

/// Find which formula provides a given command
///
/// Searches installed formulae to find which one provides a specific
/// executable command in the bin directory.
///
/// # Arguments
/// * `command` - The command name to search for
pub fn which_formula(command: &str) -> Result<()> {
    let prefix = cellar::detect_prefix();
    let bin_dir = prefix.join("bin");
    let command_path = bin_dir.join(command);

    if !command_path.exists() {
        println!(
            "{} Command '{}' not found in {}",
            "⚠".yellow(),
            command.bold(),
            bin_dir.display()
        );
        return Ok(());
    }

    // Check if it's a symlink
    if command_path.is_symlink()
        && let Ok(target) = std::fs::read_link(&command_path)
    {
        // Resolve to absolute path
        let resolved = if target.is_absolute() {
            target
        } else {
            bin_dir.join(&target).canonicalize().unwrap_or(target)
        };

        // Extract formula name from Cellar path
        let cellar_path = cellar::cellar_path();
        if resolved.starts_with(&cellar_path)
            && let Ok(rel_path) = resolved.strip_prefix(&cellar_path)
            && let Some(formula_name) = rel_path.components().next()
        {
            println!(
                "{}",
                formula_name.as_os_str().to_string_lossy().green().bold()
            );
            return Ok(());
        }
    }

    println!(
        "{} Could not determine formula for '{}'",
        "⚠".yellow(),
        command.bold()
    );
    Ok(())
}

/// Show or manage formula aliases
///
/// Aliases provide alternative names for formulae (e.g., "python" -> "python@3.13").
/// Without arguments, shows common aliases. With a formula name, shows aliases
/// for that specific formula.
///
/// # Arguments
/// * `api` - The Homebrew API client
/// * `formula` - Optional formula name to show aliases for
pub async fn alias(api: &BrewApi, formula: Option<&str>) -> Result<()> {
    match formula {
        None => {
            // Show all common aliases
            println!("{}", "==> Common Formula Aliases".bold().green());
            println!();

            let common_aliases = vec![
                ("python", "python@3.13", "Latest Python 3"),
                ("python3", "python@3.13", "Latest Python 3"),
                ("node", "node", "Node.js"),
                ("nodejs", "node", "Node.js"),
                ("postgres", "postgresql@17", "Latest PostgreSQL"),
                ("postgresql", "postgresql@17", "Latest PostgreSQL"),
                ("mysql", "mysql", "MySQL server"),
                ("mariadb", "mariadb", "MariaDB server"),
                ("redis", "redis", "Redis server"),
            ];

            for (alias_name, formula_name, desc) in &common_aliases {
                println!(
                    "{} {} {}",
                    alias_name.cyan().bold(),
                    format!("-> {}", formula_name).dimmed(),
                    format!("({})", desc).dimmed()
                );
            }

            println!();
            println!(
                "Run {} to see aliases for a specific formula",
                "bru alias <formula>".cyan()
            );
        }
        Some(formula_name) => {
            // Check if formula exists and show its aliases
            match api.fetch_formula(formula_name).await {
                Ok(formula) => {
                    println!("{} {}", "==>".bold().green(), formula.name.bold().cyan());
                    if let Some(desc) = &formula.desc {
                        println!("{}", desc);
                    }
                    println!();

                    // In real Homebrew, aliases are stored separately
                    // For now, show the formula name itself
                    println!("{}: {}", "Name".bold(), formula.name.cyan());
                    println!("{}: {}", "Full name".bold(), formula.full_name.dimmed());

                    // Check if this is commonly aliased
                    let common_aliases_map: HashMap<&str, Vec<&str>> = [
                        ("python@3.13", vec!["python", "python3"]),
                        ("node", vec!["nodejs"]),
                        ("postgresql@17", vec!["postgres", "postgresql"]),
                    ]
                    .iter()
                    .cloned()
                    .collect();

                    if let Some(aliases) = common_aliases_map.get(formula.name.as_str()) {
                        println!();
                        println!("{}", "Common aliases:".bold());
                        for alias in aliases {
                            println!("  {}", alias.cyan());
                        }
                    } else {
                        println!();
                        println!("No known aliases");
                    }
                }
                Err(_) => {
                    println!("{} Formula '{}' not found", "✗".red(), formula_name);
                }
            }
        }
    }

    Ok(())
}

/// Remove a command alias
///
/// Deletes an alias that was previously created.
///
/// # Arguments
/// * `alias_name` - The alias to remove
pub fn unalias(alias_name: &str) -> anyhow::Result<()> {
    println!("{} Removing alias: {}", "🗑️".bold(), alias_name.cyan());

    let prefix = cellar::detect_prefix();
    let alias_file = prefix.join(format!(".brew_alias_{}", alias_name));

    println!(" Alias management");
    println!("  Removes command aliases");

    if alias_file.exists() {
        println!("  {} Alias found:", "✓".green());
        println!("    {}", alias_file.display().to_string().dimmed());

        std::fs::remove_file(&alias_file)?;
        println!(" {} Alias removed successfully", "✓".green().bold());
    } else {
        println!("  Alias not found: {}", alias_name);
        println!("    To see all aliases: {}", "bru alias".cyan());
    }

    Ok(())
}

/// Open Homebrew documentation in browser
///
/// Opens the Homebrew documentation website in the default browser.
pub fn docs() -> Result<()> {
    let docs_url = "https://docs.brew.sh";
    println!("Opening documentation: {}", docs_url.cyan());

    // Try to open URL in browser
    let status = std::process::Command::new("open").arg(docs_url).status()?;

    if !status.success() {
        println!(
            "{} Failed to open browser. Visit: {}",
            "⚠".yellow(),
            docs_url
        );
    }

    Ok(())
}

/// Open Homebrew man page
///
/// Opens the Homebrew manual page using the system's man command.
pub fn man() -> anyhow::Result<()> {
    println!("Opening Homebrew man page...");

    let status = std::process::Command::new("man").arg("brew").status();

    match status {
        Ok(exit_status) if exit_status.success() => Ok(()),
        Ok(_) => {
            println!(" {} Man page not found", "⚠".yellow());
            println!("  Try: {}", "brew install man-db".cyan());
            Ok(())
        }
        Err(e) => {
            println!("{} Failed to open man page: {}", "✗".red(), e);
            println!(
                "\n  Documentation available at: {}",
                "https://docs.brew.sh".cyan()
            );
            Ok(())
        }
    }
}

/// Generate shell integration for command-not-found handler
///
/// Outputs shell-specific configuration for integrating Homebrew's
/// command-not-found handler, which suggests packages to install
/// when a command is not found.
///
/// # Arguments
/// * `shell` - Optional shell name (defaults to $SHELL)
pub fn command_not_found_init(shell: Option<&str>) -> Result<()> {
    let detected_shell = shell.map(String::from).unwrap_or_else(|| {
        std::env::var("SHELL")
            .ok()
            .and_then(|s| s.split('/').next_back().map(String::from))
            .unwrap_or_else(|| "bash".to_string())
    });

    println!("# bru command-not-found hook for {}", detected_shell);
    println!();

    match detected_shell.as_str() {
        "bash" => {
            println!("# Add this to your ~/.bashrc:");
            println!(
                "HB_CNF_HANDLER=\"$(brew --repository)/Library/Taps/homebrew/homebrew-command-not-found/handler.sh\""
            );
            println!("if [ -f \"$HB_CNF_HANDLER\" ]; then");
            println!("  source \"$HB_CNF_HANDLER\"");
            println!("fi");
        }
        "zsh" => {
            println!("# Add this to your ~/.zshrc:");
            println!(
                "HB_CNF_HANDLER=\"$(brew --repository)/Library/Taps/homebrew/homebrew-command-not-found/handler.sh\""
            );
            println!("if [ -f \"$HB_CNF_HANDLER\" ]; then");
            println!("  source \"$HB_CNF_HANDLER\"");
            println!("fi");
        }
        "fish" => {
            println!("# Add this to your ~/.config/fish/config.fish:");
            println!(
                "set HB_CNF_HANDLER (brew --repository)/Library/Taps/homebrew/homebrew-command-not-found/handler.fish"
            );
            println!("if test -f $HB_CNF_HANDLER");
            println!("  source $HB_CNF_HANDLER");
            println!("end");
        }
        _ => {
            println!("# Shell '{}' not directly supported", detected_shell);
            println!("# Use bash or zsh configuration as a starting point");
        }
    }

    Ok(())
}

/// Execute an external Homebrew command
///
/// Runs external Homebrew commands or scripts (typically those prefixed
/// with "brew-" in the PATH).
///
/// # Arguments
/// * `subcommand` - The command name to execute
/// * `args` - Arguments to pass to the command
pub fn command(subcommand: &str, args: &[String]) -> anyhow::Result<()> {
    println!("Running Homebrew sub-command: {}", subcommand.cyan());

    if !args.is_empty() {
        println!("  Arguments: {}", args.join(" ").dimmed());
    }

    println!(" Sub-command execution");
    println!("  Runs external Homebrew commands or scripts");

    println!("  Would execute:");
    if args.is_empty() {
        println!("    brew-{}", subcommand);
    } else {
        println!("    brew-{} {}", subcommand, args.join(" "));
    }

    println!(" This is an internal command");
    println!("  Used by Homebrew to run external commands");

    Ok(())
}
