use crate::api::{BrewApi, Formula};
use crate::cellar::{self, RuntimeDependency};
use crate::error::Result;
use crate::{download, extract, receipt, symlink};
use owo_colors::OwoColorize;
use std::collections::{HashMap, HashSet};

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

pub async fn fetch(api: &BrewApi, formula_names: &[String]) -> Result<()> {
    println!(
        "{} Fetching {} formulae...",
        "⬇".bold(),
        formula_names.len().to_string().bold()
    );

    // Fetch formula metadata in parallel
    let mut formulae = Vec::new();
    for name in formula_names {
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
                    continue;
                }
                formulae.push(formula);
            }
            Err(e) => {
                println!(
                    "{} Failed to fetch formula {}: {}",
                    "❌".red(),
                    name.bold(),
                    e
                );
                continue;
            }
        }
    }

    if formulae.is_empty() {
        println!("\n{} No formulae to download", "ℹ".blue());
        return Ok(());
    }

    // Download bottles in parallel
    match download::download_bottles(api, &formulae).await {
        Ok(results) => {
            println!(
                "\n{} Downloaded {} bottles to {}",
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
            println!("\n{} Download failed: {}", "❌".red(), e);
            return Err(e.into());
        }
    }

    Ok(())
}

pub async fn install(
    api: &BrewApi,
    formula_names: &[String],
    _only_dependencies: bool,
) -> Result<()> {
    println!(
        "{} Installing {} formulae...",
        "📦".bold(),
        formula_names.len().to_string().bold()
    );

    // Step 1: Resolve all dependencies
    println!("\n{} Resolving dependencies...", "🔍".bold());
    let (all_formulae, dep_order) = resolve_dependencies(api, formula_names).await?;

    // Filter installed packages
    let installed = cellar::list_installed()?;
    let installed_names: HashSet<_> = installed.iter().map(|p| p.name.as_str()).collect();

    let to_install: Vec<_> = all_formulae
        .values()
        .filter(|f| !installed_names.contains(f.name.as_str()))
        .cloned()
        .collect();

    if to_install.is_empty() {
        println!("\n{} All formulae already installed", "✓".green());
        return Ok(());
    }

    println!(
        "{} {} formulae to install: {}",
        "→".bold(),
        to_install.len().to_string().bold(),
        to_install
            .iter()
            .map(|f| f.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
            .cyan()
    );

    // Step 2: Download all bottles in parallel
    println!("\n{} Downloading bottles...", "⬇".bold());
    let downloaded = download::download_bottles(api, &to_install).await?;
    let download_map: HashMap<_, _> = downloaded.into_iter().collect();

    // Step 3: Install in dependency order
    println!("\n{} Installing packages...", "🔧".bold());
    let requested_set: HashSet<_> = formula_names.iter().map(|s| s.as_str()).collect();

    for formula_name in &dep_order {
        let formula = match all_formulae.get(formula_name.as_str()) {
            Some(f) => f,
            None => continue,
        };

        // Skip if already installed
        if installed_names.contains(formula.name.as_str()) {
            continue;
        }

        // Get downloaded bottle path
        let bottle_path = match download_map.get(&formula.name) {
            Some(path) => path,
            None => {
                println!(
                    "  {} Skipping {} (no bottle)",
                    "⚠".yellow(),
                    formula.name.bold()
                );
                continue;
            }
        };

        // Determine version
        let version = formula
            .versions
            .stable
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No stable version for {}", formula.name))?;

        println!("  {} Installing {}...", "→".bold(), formula.name.cyan());

        // Extract bottle
        let extracted_path = extract::extract_bottle(bottle_path, &formula.name, version)?;

        // Create symlinks
        let linked = symlink::link_formula(&formula.name, version)?;
        println!(
            "    {} Linked {} files",
            "✓".green(),
            linked.len().to_string().dimmed()
        );

        // Generate install receipt
        let runtime_deps = build_runtime_deps(&formula.dependencies, &all_formulae);
        let is_requested = requested_set.contains(formula.name.as_str());
        let receipt_data = receipt::InstallReceipt::new_bottle(formula, runtime_deps, is_requested);
        receipt_data.write(&extracted_path)?;

        println!(
            "    {} Installed {} {}",
            "✓".green(),
            formula.name.bold().green(),
            version.dimmed()
        );
    }

    // Summary
    let installed_count = to_install.len();
    println!(
        "\n{} Installed {} packages",
        "✓".green().bold(),
        installed_count.to_string().bold()
    );

    Ok(())
}

/// Resolve all dependencies recursively
async fn resolve_dependencies(
    api: &BrewApi,
    root_formulae: &[String],
) -> Result<(HashMap<String, Formula>, Vec<String>)> {
    let mut all_formulae = HashMap::new();
    let mut to_process = root_formulae.to_vec();
    let mut processed = HashSet::new();

    // Recursively fetch all dependencies
    while let Some(name) = to_process.pop() {
        if processed.contains(&name) {
            continue;
        }

        let formula = api.fetch_formula(&name).await?;

        // Add dependencies to process queue
        for dep in &formula.dependencies {
            if !processed.contains(dep) {
                to_process.push(dep.clone());
            }
        }

        processed.insert(name.clone());
        all_formulae.insert(formula.name.clone(), formula);
    }

    // Build dependency order (topological sort)
    let dep_order = topological_sort(&all_formulae)?;

    Ok((all_formulae, dep_order))
}

/// Topological sort for dependency order
fn topological_sort(formulae: &HashMap<String, Formula>) -> anyhow::Result<Vec<String>> {
    let mut in_degree: HashMap<String, usize> = HashMap::new();
    let mut graph: HashMap<String, Vec<String>> = HashMap::new();

    // Build dependency graph
    for (name, formula) in formulae {
        in_degree.entry(name.clone()).or_insert(0);
        for dep in &formula.dependencies {
            graph.entry(dep.clone()).or_default().push(name.clone());
            *in_degree.entry(name.clone()).or_insert(0) += 1;
        }
    }

    // Kahn's algorithm
    let mut queue: Vec<_> = in_degree
        .iter()
        .filter(|(_, count)| **count == 0)
        .map(|(name, _)| name.clone())
        .collect();
    let mut result = Vec::new();

    while let Some(node) = queue.pop() {
        result.push(node.clone());

        if let Some(dependents) = graph.get(&node) {
            for dependent in dependents {
                if let Some(count) = in_degree.get_mut(dependent) {
                    *count -= 1;
                    if *count == 0 {
                        queue.push(dependent.clone());
                    }
                }
            }
        }
    }

    if result.len() != formulae.len() {
        anyhow::bail!("Circular dependency detected");
    }

    Ok(result)
}

/// Build runtime dependencies list for receipt
fn build_runtime_deps(
    dep_names: &[String],
    all_formulae: &HashMap<String, Formula>,
) -> Vec<RuntimeDependency> {
    dep_names
        .iter()
        .filter_map(|name| {
            all_formulae.get(name).and_then(|f| {
                f.versions.stable.as_ref().map(|v| RuntimeDependency {
                    full_name: f.name.clone(),
                    version: v.clone(),
                    revision: 0,
                    pkg_version: v.clone(),
                    declared_directly: true,
                })
            })
        })
        .collect()
}
