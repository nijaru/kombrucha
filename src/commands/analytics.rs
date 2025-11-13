use crate::error::Result;
use colored::Colorize;

pub fn analytics(action: Option<&str>) -> Result<()> {
    let analytics_file = crate::cellar::detect_prefix().join("var/homebrew/analytics_disabled");

    match action {
        Some("off") => {
            // Create the file to disable analytics
            if let Some(parent) = analytics_file.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&analytics_file, "")?;
            println!("{} Analytics disabled", "✓".green());
        }
        Some("on") => {
            // Remove the file to enable analytics
            if analytics_file.exists() {
                std::fs::remove_file(&analytics_file)?;
            }
            println!("{} Analytics enabled", "✓".green());
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
            println!("{} Invalid action: {}", "✗".red(), other);
            println!("Valid actions: on, off, state");
            return Ok(());
        }
    }

    Ok(())
}
