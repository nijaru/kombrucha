use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::io::IsTerminal;

use crate::{api::BrewApi, download};

/// Fetch formulae bottles without installing them
#[derive(Parser)]
pub struct FetchCommand {
    /// Formula names to fetch
    #[arg(required = true)]
    formula_names: Vec<String>,
}

pub async fn fetch(api: &BrewApi, formula_names: &[String]) -> Result<()> {
    let is_tty = std::io::IsTerminal::is_terminal(&std::io::stdout());

    let spinner = if is_tty {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} {msg}")
                .unwrap(),
        );
        pb.set_message(format!("Fetching {} formulae...", formula_names.len()));
        pb.enable_steady_tick(std::time::Duration::from_millis(100));
        pb
    } else {
        ProgressBar::hidden()
    };

    // Fetch formula metadata in parallel
    let fetch_futures: Vec<_> = formula_names
        .iter()
        .map(|name| async move {
            match api.fetch_formula(name).await {
                Ok(formula) => {
                    // Check if bottle exists
                    if formula.bottle.is_none()
                        || formula
                            .bottle
                            .as_ref()
                            .and_then(|b| b.stable.as_ref())
                            .is_none()
                    {
                        println!("{} No bottle available for {}", "⚠".yellow(), name.bold());
                        return None;
                    }
                    Some(formula)
                }
                Err(e) => {
                    println!(
                        "{} Failed to fetch formula {}: {}",
                        "✗".red(),
                        name.bold(),
                        e
                    );
                    None
                }
            }
        })
        .collect();

    let results = futures::future::join_all(fetch_futures).await;
    let formulae: Vec<_> = results.into_iter().flatten().collect();

    spinner.finish_and_clear();

    if formulae.is_empty() {
        println!("No formulae to download");
        return Ok(());
    }

    // Download bottles in parallel
    match download::download_bottles(api, &formulae).await {
        Ok(results) => {
            println!(
                "{} Downloaded {} bottles to {}",
                "✓".green(),
                results.len().to_string().bold(),
                download::cache_dir().display().to_string().dimmed()
            );
            for (name, path) in results {
                println!(
                    "  {} {}",
                    name.bold().green(),
                    path.display().to_string().dimmed()
                );
            }
        }
        Err(e) => {
            println!("{} Download failed: {}", "✗".red(), e);
            return Err(e.into());
        }
    }

    Ok(())
}
