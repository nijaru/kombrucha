use anyhow::Result;
use clap::Parser;
use colored::Colorize;

/// List all available commands
#[derive(Parser)]
pub struct CommandsCommand {}

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
