//! Example: Query formula information from the Homebrew API
//!
//! This example demonstrates how to fetch formula metadata, explore dependencies,
//! and check available versions.
//!
//! Usage: cargo run --example query_formula [formula_name]

use kombrucha::BrewApi;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let formula_name = if args.len() > 1 {
        args[1].clone()
    } else {
        "ripgrep".to_string()
    };

    println!("Querying formula: {}\n", formula_name);

    let api = BrewApi::new()?;

    match api.fetch_formula(&formula_name).await {
        Ok(formula) => {
            // Display basic info
            println!("Name:        {}", formula.name);
            println!("Full name:   {}", formula.full_name);

            if let Some(desc) = &formula.desc {
                println!("Description: {}", desc);
            }

            if let Some(homepage) = &formula.homepage {
                println!("Homepage:    {}", homepage);
            }

            // Display versions
            println!("\nVersions:");
            if let Some(stable) = &formula.versions.stable {
                println!("  Stable: {}", stable);
            }
            if let Some(head) = &formula.versions.head {
                println!("  Head:   {}", head);
            }
            println!("  Bottle available: {}", formula.versions.bottle);

            // Display dependencies
            if !formula.dependencies.is_empty() {
                println!("\nDependencies ({}):", formula.dependencies.len());
                for dep in &formula.dependencies {
                    println!("  - {}", dep);
                }
            }

            // Display build dependencies
            if !formula.build_dependencies.is_empty() {
                println!(
                    "\nBuild dependencies ({}):",
                    formula.build_dependencies.len()
                );
                for dep in &formula.build_dependencies {
                    println!("  - {}", dep);
                }
            }

            // Keg-only info
            if formula.keg_only {
                println!("\nKeg-only: true");
                if let Some(reason) = &formula.keg_only_reason {
                    println!("Reason: {}", reason.reason);
                    if !reason.explanation.is_empty() {
                        println!("Explanation: {}", reason.explanation);
                    }
                }
            }

            // Bottle info
            if let Some(bottle) = &formula.bottle
                && let Some(stable) = &bottle.stable
            {
                println!("\nBottle rebuild: {}", stable.rebuild);
                println!("Available platforms:");
                for platform in stable.files.keys() {
                    println!("  - {}", platform);
                }
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}
