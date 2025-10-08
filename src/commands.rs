use crate::api::BrewApi;
use crate::error::Result;
use owo_colors::OwoColorize;

pub async fn search(api: &BrewApi, query: &str) -> Result<()> {
    println!("{} Searching for: {}", "🔍".bold(), query.cyan());

    let results = api.search(query).await?;

    if results.is_empty() {
        println!("\n{} No formulae or casks found matching '{}'", "❌".red(), query);
        return Ok(());
    }

    println!(
        "\n{} Found {} results\n",
        "✓".green(),
        results.total_count().to_string().bold()
    );

    // Display formulae
    if !results.formulae.is_empty() {
        println!("{}", "==> Formulae".bold().green());
        for formula in results.formulae.iter().take(20) {
            print!("{}", formula.name.bold());
            if let Some(desc) = &formula.desc {
                if !desc.is_empty() {
                    print!(" {}", format!("({})", desc).dimmed());
                }
            }
            println!();
        }

        if results.formulae.len() > 20 {
            println!(
                "{}",
                format!("... and {} more", results.formulae.len() - 20).dimmed()
            );
        }
        println!();
    }

    // Display casks
    if !results.casks.is_empty() {
        println!("{}", "==> Casks".bold().cyan());
        for cask in results.casks.iter().take(20) {
            let display_name = if !cask.name.is_empty() {
                cask.name.join(", ")
            } else {
                cask.token.clone()
            };
            print!("{}", cask.token.bold());
            if cask.token != display_name {
                print!(" {}", format!("({})", display_name).dimmed());
            }
            if let Some(desc) = &cask.desc {
                if !desc.is_empty() {
                    print!(" {}", format!("- {}", desc).dimmed());
                }
            }
            println!();
        }

        if results.casks.len() > 20 {
            println!(
                "{}",
                format!("... and {} more", results.casks.len() - 20).dimmed()
            );
        }
    }

    Ok(())
}

pub async fn info(api: &BrewApi, formula: &str) -> Result<()> {
    println!("{} Fetching info for: {}", "📦".bold(), formula.cyan());

    // Try formula first, then cask
    match api.fetch_formula(formula).await {
        Ok(formula) => {
            println!("\n{}", format!("==> {}", formula.name).bold().green());
            if let Some(desc) = &formula.desc {
                println!("{}", desc);
            }
            if let Some(homepage) = &formula.homepage {
                println!("{}: {}", "Homepage".bold(), homepage);
            }
            if let Some(version) = &formula.versions.stable {
                println!("{}: {}", "Version".bold(), version);
            }

            if !formula.dependencies.is_empty() {
                println!(
                    "{}: {}",
                    "Dependencies".bold(),
                    formula.dependencies.join(", ")
                );
            }

            if !formula.build_dependencies.is_empty() {
                println!(
                    "{}: {}",
                    "Build dependencies".bold(),
                    formula.build_dependencies.join(", ")
                );
            }
        }
        Err(_) => {
            // Try as cask
            match api.fetch_cask(formula).await {
                Ok(cask) => {
                    println!("\n{}", format!("==> {}", cask.token).bold().cyan());
                    if !cask.name.is_empty() {
                        println!("{}: {}", "Name".bold(), cask.name.join(", "));
                    }
                    if let Some(desc) = &cask.desc {
                        println!("{}", desc);
                    }
                    if let Some(homepage) = &cask.homepage {
                        println!("{}: {}", "Homepage".bold(), homepage);
                    }
                    if let Some(version) = &cask.version {
                        println!("{}: {}", "Version".bold(), version);
                    }
                }
                Err(_) => {
                    println!(
                        "\n{} No formula or cask found for '{}'",
                        "❌".red(),
                        formula
                    );
                }
            }
        }
    }

    Ok(())
}

pub async fn deps(api: &BrewApi, formula: &str, tree: bool) -> Result<()> {
    if tree {
        println!("{} Dependency tree for: {}", "🌳".bold(), formula.cyan());
    } else {
        println!("{} Dependencies for: {}", "📊".bold(), formula.cyan());
    }

    let formula_data = api.fetch_formula(formula).await?;

    if formula_data.dependencies.is_empty() && formula_data.build_dependencies.is_empty() {
        println!("\n{} No dependencies", "✓".green());
        return Ok(());
    }

    if !formula_data.dependencies.is_empty() {
        println!("\n{}", "Runtime dependencies:".bold().green());
        for dep in &formula_data.dependencies {
            if tree {
                println!("  └─ {}", dep);
            } else {
                println!("  {}", dep);
            }
        }
    }

    if !formula_data.build_dependencies.is_empty() {
        println!("\n{}", "Build dependencies:".bold().yellow());
        for dep in &formula_data.build_dependencies {
            if tree {
                println!("  └─ {}", dep);
            } else {
                println!("  {}", dep);
            }
        }
    }

    Ok(())
}
