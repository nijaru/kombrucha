//! Example: Search for packages in Homebrew
//!
//! This example demonstrates how to search across all formulae and casks
//! using flexible query matching. Results are presented separately for
//! filtering by type.
//!
//! Usage: cargo run --example search_packages [query]

use kombrucha::BrewApi;
use std::io::{self, Write};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let query = if args.len() > 1 {
        args[1].clone()
    } else {
        // Interactive prompt
        print!("Enter search query: ");
        io::stdout().flush()?;
        let mut query = String::new();
        io::stdin().read_line(&mut query)?;
        query.trim().to_string()
    };

    if query.is_empty() {
        eprintln!("Error: search query cannot be empty");
        std::process::exit(1);
    }

    println!("Searching for: '{}'\n", query);

    let api = BrewApi::new()?;
    println!("Fetching from API (this may take a moment on first run)...\n");

    match api.search(&query).await {
        Ok(results) => {
            let total = results.formulae.len() + results.casks.len();

            if total == 0 {
                println!("No packages found matching '{}'", query);
                return Ok(());
            }

            println!("Found {} matches:\n", total);

            // Display formulae
            if !results.formulae.is_empty() {
                println!("FORMULAE ({}):", results.formulae.len());
                for formula in &results.formulae {
                    print!("  {} ", formula.name);

                    if let Some(version) = &formula.versions.stable {
                        print!("({})", version);
                    }

                    if let Some(desc) = &formula.desc {
                        println!(" - {}", desc);
                    } else {
                        println!();
                    }
                }
                println!();
            }

            // Display casks
            if !results.casks.is_empty() {
                println!("CASKS ({}):", results.casks.len());
                for cask in &results.casks {
                    let names = if cask.name.is_empty() {
                        cask.token.clone()
                    } else {
                        cask.name.join(", ")
                    };

                    print!("  {} ", cask.token);

                    if let Some(version) = &cask.version {
                        print!("({})", version);
                    }

                    if let Some(desc) = &cask.desc {
                        println!(" - {}", desc);
                    } else {
                        println!();
                    }

                    if !names.is_empty() && names != cask.token {
                        println!("    Names: {}", names);
                    }
                }
                println!();
            }

            // Display summary
            println!(
                "Summary: {} formulae + {} casks = {} matches",
                results.formulae.len(),
                results.casks.len(),
                total
            );
        }
        Err(e) => {
            eprintln!("Search error: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}
