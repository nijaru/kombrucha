use crate::api::BrewApi;
use crate::error::Result;
use colored::Colorize;

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
                    let common_aliases_map: std::collections::HashMap<&str, Vec<&str>> = [
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
