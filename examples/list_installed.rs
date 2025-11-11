//! Example: List all installed Homebrew packages
//!
//! This example demonstrates how to inspect the Cellar directory and list all
//! installed packages with their versions.
//!
//! Usage: cargo run --example list_installed

use kombrucha::cellar;

fn main() -> anyhow::Result<()> {
    println!(
        "Homebrew Cellar Location: {}\n",
        cellar::cellar_path().display()
    );

    match cellar::list_installed() {
        Ok(packages) => {
            if packages.is_empty() {
                println!("No packages installed.");
                return Ok(());
            }

            println!("Installed packages ({}):\n", packages.len());

            // Group by formula name to show all versions
            use std::collections::BTreeMap;
            let mut by_name: BTreeMap<String, Vec<_>> = BTreeMap::new();

            for pkg in packages {
                by_name
                    .entry(pkg.name.clone())
                    .or_insert_with(Vec::new)
                    .push(pkg);
            }

            for (name, mut versions) in by_name {
                // Sort versions to show newest first
                versions.sort_by(|a, b| {
                    // Compare versions semantically
                    let a_parts: Vec<u32> = a
                        .version
                        .split('.')
                        .filter_map(|s| s.parse().ok())
                        .collect();
                    let b_parts: Vec<u32> = b
                        .version
                        .split('.')
                        .filter_map(|s| s.parse().ok())
                        .collect();

                    for i in 0..a_parts.len().max(b_parts.len()) {
                        let a_part = a_parts.get(i).unwrap_or(&0);
                        let b_part = b_parts.get(i).unwrap_or(&0);
                        match b_part.cmp(a_part) {
                            std::cmp::Ordering::Equal => continue,
                            other => return other,
                        }
                    }
                    std::cmp::Ordering::Equal
                });

                // Print formula with all its versions
                print!("{}", name);
                if versions.len() > 1 {
                    print!(" ({})", versions.len());
                }
                println!();

                for (idx, pkg) in versions.iter().enumerate() {
                    let marker = if idx == 0 { "→" } else { " " };
                    println!("  {} {}", marker, pkg.version);

                    // Show installation details if receipt is available
                    if let Some(receipt) = &pkg.receipt {
                        if receipt.installed_on_request {
                            print!(" (installed on request)");
                        }
                        if !receipt.runtime_dependencies.is_empty() {
                            print!(" [deps: {}]", receipt.runtime_dependencies.len());
                        }
                    }
                    println!();
                }
                println!();
            }
        }
        Err(e) => {
            eprintln!("Error reading Cellar: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}
