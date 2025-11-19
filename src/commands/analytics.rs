//! Analytics management commands
//!
//! Handles enabling, disabling, and checking the status of Homebrew
//! analytics collection.

use crate::cellar;
use crate::error::Result;
use colored::Colorize;

/// Control analytics collection (on/off) or show current state
///
/// Analytics help Homebrew maintainers understand which packages are
/// most popular and prioritize maintenance efforts.
///
/// # Arguments
/// * `action` - The analytics action: on, off, or state (None defaults to state)
pub fn analytics(action: Option<&str>) -> Result<()> {
    let analytics_file = cellar::detect_prefix().join("var/homebrew/analytics_disabled");

    match action {
        Some("off") => {
            // Create the file to disable analytics
            if let Some(parent) = analytics_file.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&analytics_file, "")?;
            println!("{} Analytics disabled", "".green());
        }
        Some("on") => {
            // Remove the file to enable analytics
            if analytics_file.exists() {
                std::fs::remove_file(&analytics_file)?;
            }
            println!("{} Analytics enabled", "".green());
        }
        Some("state") | None => {
            // Show current state
            let enabled = !analytics_file.exists();
            println!("{}", "==> Analytics Status".bold().green());
            if enabled {
                println!("{}: {}", "Status".bold(), "Enabled".green());
                println!();
                println!("Analytics help bru improve by tracking usage patterns.");
                println!("Run {} to disable", "bru analytics off".cyan());
            } else {
                println!("{}: {}", "Status".bold(), "Disabled".red());
                println!();
                println!("Run {} to enable", "bru analytics on".cyan());
            }
        }
        Some(other) => {
            println!("{} Invalid action: {}", "".red(), other);
            println!("Valid actions: on, off, state");
            return Ok(());
        }
    }

    Ok(())
}

/// Show the current analytics state
///
/// Displays whether analytics collection is currently enabled or disabled.
pub fn analytics_state() -> anyhow::Result<()> {
    let prefix = cellar::detect_prefix();
    let analytics_disabled = prefix.join(".homebrew_analytics_disabled").exists();

    println!("Analytics state:");

    if analytics_disabled {
        println!("  Status: {}", "disabled".dimmed());
        println!("  Analytics are currently turned off");
        println!("  {} No usage data is being collected", "".green());
    } else {
        println!("  Status: {}", "enabled".green());
        println!("  Analytics are currently enabled");
        println!("  Anonymous usage data is collected");
    }

    println!(
        "\n  {} To change: {} [on|off]",
        "".dimmed(),
        "bru analytics".cyan()
    );

    Ok(())
}
