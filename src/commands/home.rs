use anyhow::Result;
use clap::Parser;
use colored::Colorize;

use crate::api::BrewApi;

/// Open formula homepage in browser
#[derive(Parser)]
pub struct HomeCommand {
    /// Formula name
    formula: String,
}

pub async fn home(api: &BrewApi, formula_name: &str) -> Result<()> {
    println!("Opening homepage for {}...", formula_name.cyan());

    let formula = api.fetch_formula(formula_name).await?;

    match &formula.homepage {
        Some(url) if !url.is_empty() => {
            println!("  {}: {}", "Homepage".dimmed(), url.cyan());

            // Open URL in default browser
            let status = std::process::Command::new("open").arg(url).status();

            match status {
                Ok(s) if s.success() => {
                    println!("  {} Opened in browser", "✓".green());
                }
                _ => {
                    println!("  {} Could not open browser automatically", "⚠".yellow());
                    println!("  Please visit: {}", url);
                }
            }
        }
        _ => {
            println!(
                "  {} No homepage available for {}",
                "⚠".yellow(),
                formula_name.bold()
            );
        }
    }

    Ok(())
}
