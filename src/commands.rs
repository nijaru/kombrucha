use crate::api::BrewApi;
use crate::cellar;
use crate::error::Result;
use owo_colors::OwoColorize;

pub async fn search(api: &BrewApi, query: &str) -> Result<()> {
    println!("{} Searching for: {}", "🔍".bold(), query.cyan());

    let results = api.search(query).await?;

    if results.is_empty() {
        println!(
            "\n{} No formulae or casks found matching '{}'",
            "❌".red(),
            query
        );
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

pub async fn uses(api: &BrewApi, formula: &str) -> Result<()> {
    println!(
        "{} Finding formulae that depend on: {}",
        "🔍".bold(),
        formula.cyan()
    );

    // Fetch all formulae
    let all_formulae = api.fetch_all_formulae().await?;

    // Find formulae that depend on the target
    let dependent_formulae: Vec<_> = all_formulae
        .into_iter()
        .filter(|f| {
            f.dependencies.contains(&formula.to_string())
                || f.build_dependencies.contains(&formula.to_string())
        })
        .collect();

    if dependent_formulae.is_empty() {
        println!("\n{} No formulae depend on '{}'", "✓".green(), formula);
        return Ok(());
    }

    println!(
        "\n{} Found {} formulae that depend on {}:\n",
        "✓".green(),
        dependent_formulae.len().to_string().bold(),
        formula.cyan()
    );

    for f in dependent_formulae {
        print!("{}", f.name.bold());
        if let Some(desc) = &f.desc {
            if !desc.is_empty() {
                print!(" {}", format!("({})", desc).dimmed());
            }
        }
        println!();
    }

    Ok(())
}

pub async fn list(_api: &BrewApi, show_versions: bool) -> Result<()> {
    println!("{} Installed packages:", "📦".bold());

    let packages = cellar::list_installed()?;

    if packages.is_empty() {
        println!("\n{} No packages installed", "ℹ".blue());
        return Ok(());
    }

    // Group by formula name
    let mut by_name: std::collections::HashMap<String, Vec<_>> = std::collections::HashMap::new();
    for pkg in packages {
        by_name.entry(pkg.name.clone()).or_default().push(pkg);
    }

    let mut names: Vec<_> = by_name.keys().cloned().collect();
    names.sort();

    println!();
    for name in names {
        let versions = &by_name[&name];

        if show_versions && versions.len() > 1 {
            println!("{}", name.bold().green());
            for pkg in versions {
                println!("  {}", pkg.version);
            }
        } else {
            // Just show the first version (usually only one)
            let pkg = &versions[0];
            print!("{}", name.bold().green());
            println!(" {}", pkg.version.dimmed());
        }
    }

    println!(
        "\n{} {} packages installed",
        "✓".green(),
        by_name.len().to_string().bold()
    );

    Ok(())
}

pub async fn outdated(api: &BrewApi) -> Result<()> {
    println!("{} Checking for outdated packages...", "🔍".bold());

    let packages = cellar::list_installed()?;

    if packages.is_empty() {
        println!("\n{} No packages installed", "ℹ".blue());
        return Ok(());
    }

    let mut outdated_packages = Vec::new();

    // Check each package against API
    for pkg in packages {
        // Fetch current version from API
        if let Ok(formula) = api.fetch_formula(&pkg.name).await {
            if let Some(latest_version) = &formula.versions.stable {
                if latest_version != &pkg.version {
                    outdated_packages.push((pkg, latest_version.clone()));
                }
            }
        }
    }

    if outdated_packages.is_empty() {
        println!("\n{} All packages are up to date", "✓".green());
        return Ok(());
    }

    println!(
        "\n{} Found {} outdated packages:\n",
        "⚠".yellow(),
        outdated_packages.len().to_string().bold()
    );

    for (pkg, latest) in outdated_packages {
        println!(
            "{} {} {}",
            pkg.name.bold().yellow(),
            pkg.version.dimmed(),
            format!("→ {}", latest).cyan()
        );
    }

    Ok(())
}
