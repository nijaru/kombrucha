use crate::api::{BrewApi, Formula};
use crate::cellar::{self, RuntimeDependency};
use crate::error::Result;
use crate::{download, extract, receipt, symlink};
use owo_colors::OwoColorize;
use std::collections::{HashMap, HashSet};

pub async fn search(api: &BrewApi, query: &str, formula_only: bool, cask_only: bool) -> Result<()> {
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

    // Determine what to display based on flags
    let show_formulae = !cask_only;
    let show_casks = !formula_only;

    // Count total results to show
    let total_to_show =
        (if show_formulae { results.formulae.len() } else { 0 }) +
        (if show_casks { results.casks.len() } else { 0 });

    if total_to_show == 0 {
        println!(
            "\n{} No results found with the specified filter",
            "❌".red()
        );
        return Ok(());
    }

    println!(
        "\n{} Found {} results\n",
        "✓".green(),
        total_to_show.to_string().bold()
    );

    // Display formulae
    if show_formulae && !results.formulae.is_empty() {
        println!("{}", "==> Formulae".bold().green());
        for formula in results.formulae.iter().take(20) {
            print!("{}", formula.name.bold());
            if let Some(desc) = &formula.desc
                && !desc.is_empty() {
                    print!(" {}", format!("({})", desc).dimmed());
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
    if show_casks && !results.casks.is_empty() {
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
            if let Some(desc) = &cask.desc
                && !desc.is_empty() {
                    print!(" {}", format!("- {}", desc).dimmed());
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

pub async fn info(api: &BrewApi, formula: &str, json: bool) -> Result<()> {
    if !json {
        println!("{} Fetching info for: {}", "📦".bold(), formula.cyan());
    }

    // Try formula first, then cask
    match api.fetch_formula(formula).await {
        Ok(formula) => {
            if json {
                // Output as JSON
                let json_str = serde_json::to_string_pretty(&formula)?;
                println!("{}", json_str);
            } else {
                // Pretty print format
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
        }
        Err(_) => {
            // Try as cask
            match api.fetch_cask(formula).await {
                Ok(cask) => {
                    if json {
                        let json_str = serde_json::to_string_pretty(&cask)?;
                        println!("{}", json_str);
                    } else {
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
                }
                Err(_) => {
                    if json {
                        println!("{{\"error\": \"No formula or cask found for '{}'\"}}", formula);
                    } else {
                        println!(
                            "\n{} No formula or cask found for '{}'",
                            "❌".red(),
                            formula
                        );
                    }
                }
            }
        }
    }

    Ok(())
}

pub async fn deps(api: &BrewApi, formula: &str, tree: bool, installed_only: bool) -> Result<()> {
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

    // If filtering by installed, get the list of installed packages
    let installed_names: HashSet<String> = if installed_only {
        cellar::list_installed()?
            .into_iter()
            .map(|p| p.name)
            .collect()
    } else {
        HashSet::new()
    };

    if !formula_data.dependencies.is_empty() {
        let mut deps: Vec<_> = formula_data.dependencies.iter().collect();

        if installed_only {
            deps.retain(|dep| installed_names.contains(*dep));
        }

        if !deps.is_empty() {
            println!("\n{}", "Runtime dependencies:".bold().green());
            for dep in deps {
                if tree {
                    println!("  └─ {}", dep.cyan());
                } else {
                    println!("  {}", dep.cyan());
                }
            }
        } else if installed_only {
            println!("\n{} No runtime dependencies installed", "ℹ".blue());
        }
    }

    if !formula_data.build_dependencies.is_empty() {
        let mut build_deps: Vec<_> = formula_data.build_dependencies.iter().collect();

        if installed_only {
            build_deps.retain(|dep| installed_names.contains(*dep));
        }

        if !build_deps.is_empty() {
            println!("\n{}", "Build dependencies:".bold().yellow());
            for dep in build_deps {
                if tree {
                    println!("  └─ {}", dep.cyan());
                } else {
                    println!("  {}", dep.cyan());
                }
            }
        } else if installed_only && !formula_data.build_dependencies.is_empty() {
            println!("\n{} No build dependencies installed", "ℹ".blue());
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
        if let Some(desc) = &f.desc
            && !desc.is_empty() {
                print!(" {}", format!("({})", desc).dimmed());
            }
        println!();
    }

    Ok(())
}

pub async fn list(_api: &BrewApi, show_versions: bool, json: bool) -> Result<()> {
    let packages = cellar::list_installed()?;

    if json {
        // Output as JSON
        #[derive(serde::Serialize)]
        struct PackageInfo {
            name: String,
            versions: Vec<String>,
        }

        // Group by formula name
        let mut by_name: std::collections::HashMap<String, Vec<_>> = std::collections::HashMap::new();
        for pkg in packages {
            by_name.entry(pkg.name.clone()).or_default().push(pkg.version.clone());
        }

        let mut package_list: Vec<PackageInfo> = by_name
            .into_iter()
            .map(|(name, versions)| PackageInfo { name, versions })
            .collect();

        package_list.sort_by(|a, b| a.name.cmp(&b.name));

        let json_str = serde_json::to_string_pretty(&package_list)?;
        println!("{}", json_str);
    } else {
        println!("{} Installed packages:", "📦".bold());

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
    }

    Ok(())
}

pub async fn outdated(api: &BrewApi) -> Result<()> {
    println!("{} Checking for outdated packages...", "🔍".bold());

    let packages = cellar::list_installed()?;

    if packages.is_empty() {
        println!("\n{} No packages installed", "ℹ".blue());
        return Ok(());
    }

    // Fetch all formula versions in parallel
    let fetch_futures: Vec<_> = packages
        .iter()
        .map(|pkg| async move {
            match api.fetch_formula(&pkg.name).await {
                Ok(formula) => {
                    if let Some(latest) = &formula.versions.stable {
                        if latest != &pkg.version {
                            return Some((pkg.clone(), latest.clone()));
                        }
                    }
                }
                Err(_) => {}
            }
            None
        })
        .collect();

    let results = futures::future::join_all(fetch_futures).await;
    let outdated_packages: Vec<_> = results.into_iter().flatten().collect();

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

        // Relocate bottle (fix install names)
        crate::relocate::relocate_bottle(&extracted_path, &crate::cellar::detect_prefix())?;

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

pub async fn upgrade(api: &BrewApi, formula_names: &[String]) -> Result<()> {
    // Determine which formulae to upgrade
    let to_upgrade = if formula_names.is_empty() {
        // Upgrade all outdated
        println!("{} Checking for outdated packages...", "🔍".bold());
        let packages = cellar::list_installed()?;
        let mut outdated = Vec::new();

        for pkg in packages {
            if let Ok(formula) = api.fetch_formula(&pkg.name).await
                && let Some(latest) = &formula.versions.stable
                && latest != &pkg.version {
                    outdated.push(pkg.name.clone());
                }
        }

        if outdated.is_empty() {
            println!("\n{} All packages are up to date", "✓".green());
            return Ok(());
        }

        println!(
            "{} {} packages to upgrade: {}",
            "→".bold(),
            outdated.len().to_string().bold(),
            outdated.join(", ").cyan()
        );
        outdated
    } else {
        formula_names.to_vec()
    };

    println!(
        "\n{} Upgrading {} packages...",
        "⬆".bold(),
        to_upgrade.len()
    );

    // Check for pinned formulae
    let pinned = read_pinned()?;

    for formula_name in &to_upgrade {
        // Skip pinned formulae
        if pinned.contains(formula_name) {
            println!(
                "  {} {} is pinned, skipping",
                "📌".bold(),
                formula_name.bold()
            );
            continue;
        }

        // Check if installed
        let installed_versions = cellar::get_installed_versions(formula_name)?;
        if installed_versions.is_empty() {
            println!(
                "  {} {} not installed, installing...",
                "ℹ".blue(),
                formula_name.bold()
            );
            install(api, std::slice::from_ref(formula_name), false).await?;
            continue;
        }

        let old_version = &installed_versions[0].version;

        // Fetch latest version
        let formula = api.fetch_formula(formula_name).await?;
        let new_version = formula
            .versions
            .stable
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No stable version for {}", formula_name))?;

        if old_version == new_version {
            println!(
                "  {} {} already at latest version {}",
                "✓".green(),
                formula_name.bold(),
                new_version.dimmed()
            );
            continue;
        }

        println!(
            "  {} Upgrading {} {} → {}",
            "⬆".bold(),
            formula_name.cyan(),
            old_version.dimmed(),
            new_version.cyan()
        );

        // Unlink old version
        symlink::unlink_formula(formula_name, old_version)?;

        // Download new version
        let bottle_path = download::download_bottle(&formula, None).await?;

        // Install new version
        let extracted_path = extract::extract_bottle(&bottle_path, formula_name, new_version)?;

        // Relocate bottle (fix install names)
        crate::relocate::relocate_bottle(&extracted_path, &crate::cellar::detect_prefix())?;

        let linked = symlink::link_formula(formula_name, new_version)?;

        // Generate receipt
        let runtime_deps = build_runtime_deps(&formula.dependencies, &{
            let mut map = HashMap::new();
            map.insert(formula.name.clone(), formula.clone());
            map
        });
        let receipt_data = receipt::InstallReceipt::new_bottle(&formula, runtime_deps, true);
        receipt_data.write(&extracted_path)?;

        println!(
            "    {} Linked {} files",
            "✓".green(),
            linked.len().to_string().dimmed()
        );

        // Remove old version
        let old_path = cellar::cellar_path().join(formula_name).join(old_version);
        if old_path.exists() {
            std::fs::remove_dir_all(&old_path)?;
            println!(
                "    {} Removed old version {}",
                "✓".green(),
                old_version.dimmed()
            );
        }

        println!(
            "    {} Upgraded {} to {}",
            "✓".green(),
            formula_name.bold().green(),
            new_version.dimmed()
        );
    }

    println!(
        "\n{} Upgraded {} packages",
        "✓".green().bold(),
        to_upgrade.len().to_string().bold()
    );

    Ok(())
}

pub async fn reinstall(api: &BrewApi, formula_names: &[String]) -> Result<()> {
    if formula_names.is_empty() {
        println!("{} No formulae specified", "❌".red());
        return Ok(());
    }

    println!(
        "{} Reinstalling {} formulae...",
        "🔄".bold(),
        formula_names.len().to_string().bold()
    );

    for formula_name in formula_names {
        // Check if installed
        let installed_versions = cellar::get_installed_versions(formula_name)?;
        if installed_versions.is_empty() {
            println!("  {} {} not installed", "⚠".yellow(), formula_name.bold());
            continue;
        }

        let version = &installed_versions[0].version;
        println!(
            "  {} Reinstalling {} {}",
            "🔄".bold(),
            formula_name.cyan(),
            version.dimmed()
        );

        // Unlink
        symlink::unlink_formula(formula_name, version)?;

        // Remove from Cellar
        let cellar_path = cellar::cellar_path().join(formula_name).join(version);
        if cellar_path.exists() {
            std::fs::remove_dir_all(&cellar_path)?;
        }

        // Fetch formula metadata
        let formula = api.fetch_formula(formula_name).await?;

        // Download bottle
        let bottle_path = download::download_bottle(&formula, None).await?;

        // Install
        let extracted_path = extract::extract_bottle(&bottle_path, formula_name, version)?;

        // Relocate bottle (fix install names)
        crate::relocate::relocate_bottle(&extracted_path, &crate::cellar::detect_prefix())?;

        let linked = symlink::link_formula(formula_name, version)?;

        // Generate receipt
        let runtime_deps = build_runtime_deps(&formula.dependencies, &{
            let mut map = HashMap::new();
            map.insert(formula.name.clone(), formula.clone());
            map
        });
        let receipt_data = receipt::InstallReceipt::new_bottle(&formula, runtime_deps, true);
        receipt_data.write(&extracted_path)?;

        println!(
            "    {} Linked {} files",
            "✓".green(),
            linked.len().to_string().dimmed()
        );
        println!(
            "    {} Reinstalled {} {}",
            "✓".green(),
            formula_name.bold().green(),
            version.dimmed()
        );
    }

    println!(
        "\n{} Reinstalled {} packages",
        "✓".green().bold(),
        formula_names.len().to_string().bold()
    );

    Ok(())
}

pub async fn uninstall(_api: &BrewApi, formula_names: &[String], force: bool) -> Result<()> {
    if formula_names.is_empty() {
        println!("{} No formulae specified", "❌".red());
        return Ok(());
    }

    println!(
        "{} Uninstalling {} formulae...",
        "🗑".bold(),
        formula_names.len().to_string().bold()
    );

    // Get all installed packages to check dependencies
    let all_installed = cellar::list_installed()?;

    for formula_name in formula_names {
        // Check if installed
        let installed_versions = cellar::get_installed_versions(formula_name)?;
        if installed_versions.is_empty() {
            println!("  {} {} not installed", "⚠".yellow(), formula_name.bold());
            continue;
        }

        // Check if other packages depend on this one (unless --force)
        if !force {
            let dependents: Vec<_> = all_installed
                .iter()
                .filter(|pkg| {
                    pkg.name != *formula_name
                        && pkg
                            .runtime_dependencies()
                            .iter()
                            .any(|dep| dep.full_name == *formula_name)
                })
                .map(|pkg| pkg.name.as_str())
                .collect();

            if !dependents.is_empty() {
                println!(
                    "  {} Cannot uninstall {} - required by: {}",
                    "⚠".yellow(),
                    formula_name.bold(),
                    dependents.join(", ").cyan()
                );
                println!("    Use {} to force uninstall", "--force".dimmed());
                continue;
            }
        }

        let version = &installed_versions[0].version;
        println!(
            "  {} Uninstalling {} {}",
            "🗑".bold(),
            formula_name.cyan(),
            version.dimmed()
        );

        // Unlink symlinks
        let unlinked = symlink::unlink_formula(formula_name, version)?;
        if !unlinked.is_empty() {
            println!(
                "    {} Unlinked {} files",
                "✓".green(),
                unlinked.len().to_string().dimmed()
            );
        }

        // Remove from Cellar
        let cellar_path = cellar::cellar_path().join(formula_name).join(version);
        if cellar_path.exists() {
            std::fs::remove_dir_all(&cellar_path)?;
        }

        // Remove formula directory if empty
        let formula_dir = cellar::cellar_path().join(formula_name);
        if formula_dir.exists() && formula_dir.read_dir()?.next().is_none() {
            std::fs::remove_dir(&formula_dir)?;
        }

        println!(
            "    {} Uninstalled {} {}",
            "✓".green(),
            formula_name.bold().green(),
            version.dimmed()
        );
    }

    println!(
        "\n{} Uninstalled {} packages",
        "✓".green().bold(),
        formula_names.len().to_string().bold()
    );

    Ok(())
}

pub fn autoremove(dry_run: bool) -> Result<()> {
    if dry_run {
        println!("{} Dry run - no packages will be removed", "ℹ".blue());
    } else {
        println!("{} Removing unused dependencies...", "🗑".bold());
    }

    let all_packages = cellar::list_installed()?;

    // Build a set of all packages installed on request
    let mut on_request: HashSet<String> = HashSet::new();
    for pkg in &all_packages {
        if pkg.installed_on_request() {
            on_request.insert(pkg.name.clone());
        }
    }

    // Build a set of all dependencies required by packages installed on request
    let mut required = HashSet::new();
    let mut to_check: Vec<String> = on_request.iter().cloned().collect();
    let mut checked = HashSet::new();

    while let Some(name) = to_check.pop() {
        if checked.contains(&name) {
            continue;
        }
        checked.insert(name.clone());

        // Find the package and get its dependencies
        if let Some(pkg) = all_packages.iter().find(|p| p.name == name) {
            for dep in pkg.runtime_dependencies() {
                required.insert(dep.full_name.clone());
                if !checked.contains(&dep.full_name) {
                    to_check.push(dep.full_name.clone());
                }
            }
        }
    }

    // Find packages that are:
    // 1. Installed as dependency (not on request)
    // 2. Not required by any package installed on request
    let mut to_remove: Vec<_> = all_packages
        .iter()
        .filter(|pkg| !pkg.installed_on_request() && !required.contains(&pkg.name))
        .collect();

    if to_remove.is_empty() {
        println!("\n{} No unused dependencies to remove", "✓".green());
        return Ok(());
    }

    to_remove.sort_by(|a, b| a.name.cmp(&b.name));

    println!(
        "\n{} Found {} unused dependencies:\n",
        "→".bold(),
        to_remove.len().to_string().bold()
    );

    for pkg in &to_remove {
        println!("  {} {}", pkg.name.cyan(), pkg.version.dimmed());
    }

    if dry_run {
        println!(
            "\n{} Would remove {} packages",
            "ℹ".blue(),
            to_remove.len().to_string().bold()
        );
        println!("Run without {} to remove them", "--dry-run".dimmed());
        return Ok(());
    }

    println!();

    // Remove packages
    for pkg in &to_remove {
        println!("  {} Uninstalling {} {}", "🗑".bold(), pkg.name.cyan(), pkg.version.dimmed());

        // Unlink symlinks
        let unlinked = symlink::unlink_formula(&pkg.name, &pkg.version)?;
        if !unlinked.is_empty() {
            println!(
                "    {} Unlinked {} files",
                "✓".green(),
                unlinked.len().to_string().dimmed()
            );
        }

        // Remove from Cellar
        let cellar_path = cellar::cellar_path().join(&pkg.name).join(&pkg.version);
        if cellar_path.exists() {
            std::fs::remove_dir_all(&cellar_path)?;
        }

        // Remove formula directory if empty
        let formula_dir = cellar::cellar_path().join(&pkg.name);
        if formula_dir.exists()
            && formula_dir.read_dir()?.next().is_none() {
                std::fs::remove_dir(&formula_dir)?;
            }

        println!("    {} Removed {}", "✓".green(), pkg.name.bold().green());
    }

    println!(
        "\n{} Removed {} unused packages",
        "✓".green().bold(),
        to_remove.len().to_string().bold()
    );

    Ok(())
}

pub fn tap(tap_name: Option<&str>) -> Result<()> {
    match tap_name {
        None => {
            // List all taps
            let taps = crate::tap::list_taps()?;
            if taps.is_empty() {
                println!("{} No taps installed", "ℹ".blue());
            } else {
                for tap in taps {
                    println!("{}", tap.cyan());
                }
            }
        }
        Some(tap) => {
            // Add a tap
            println!("{} Tapping {}...", "🔗".bold(), tap.cyan());

            if crate::tap::is_tapped(tap)? {
                println!("  {} {} already tapped", "✓".green(), tap.bold());
                return Ok(());
            }

            crate::tap::tap(tap)?;

            println!(
                "  {} Tapped {} successfully",
                "✓".green(),
                tap.bold().green()
            );
        }
    }
    Ok(())
}

pub fn untap(tap_name: &str) -> Result<()> {
    println!("{} Untapping {}...", "🔗".bold(), tap_name.cyan());

    if !crate::tap::is_tapped(tap_name)? {
        println!("  {} {} is not tapped", "⚠".yellow(), tap_name.bold());
        return Ok(());
    }

    crate::tap::untap(tap_name)?;

    println!(
        "  {} Untapped {} successfully",
        "✓".green(),
        tap_name.bold().green()
    );

    Ok(())
}

pub fn update() -> Result<()> {
    println!("{} Updating Homebrew...", "⬇".bold());

    let taps = crate::tap::list_taps()?;

    if taps.is_empty() {
        println!("\n{} No taps installed", "ℹ".blue());
        return Ok(());
    }

    println!("\n{} Updating {} taps...", "→".bold(), taps.len().to_string().bold());

    let mut updated = 0;
    let mut unchanged = 0;
    let mut errors = 0;

    for tap in &taps {
        print!("  {} Updating {}... ", "⬇".bold(), tap.cyan());

        let tap_dir = crate::tap::tap_directory(tap)?;

        if !tap_dir.exists() || !tap_dir.join(".git").exists() {
            println!("{} (not a git repository)", "⚠".yellow());
            errors += 1;
            continue;
        }

        // Run git pull
        let output = std::process::Command::new("git")
            .args(["-C", tap_dir.to_str().unwrap(), "pull", "--ff-only"])
            .output();

        match output {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if stdout.contains("Already up to date") || stdout.contains("Already up-to-date") {
                    println!("{}", "already up to date".dimmed());
                    unchanged += 1;
                } else {
                    println!("{}", "updated".green());
                    updated += 1;
                }
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                println!("{} {}", "failed".red(), stderr.trim().dimmed());
                errors += 1;
            }
            Err(e) => {
                println!("{} {}", "failed".red(), e.to_string().dimmed());
                errors += 1;
            }
        }
    }

    println!();

    if errors == 0 {
        if updated > 0 {
            println!(
                "{} Updated {} taps, {} unchanged",
                "✓".green().bold(),
                updated.to_string().bold(),
                unchanged.to_string().dimmed()
            );
        } else {
            println!("{} All taps are up to date", "✓".green().bold());
        }
    } else {
        println!(
            "{} {} succeeded, {} failed",
            "⚠".yellow(),
            (updated + unchanged).to_string().bold(),
            errors.to_string().bold()
        );
    }

    Ok(())
}

pub fn cleanup(formula_names: &[String], dry_run: bool) -> Result<()> {
    let all_packages = cellar::list_installed()?;

    // Group packages by formula name
    let mut by_formula: HashMap<String, Vec<&cellar::InstalledPackage>> = HashMap::new();
    for pkg in &all_packages {
        by_formula.entry(pkg.name.clone()).or_default().push(pkg);
    }

    // Filter to specified formulae if provided
    let to_clean: Vec<_> = if formula_names.is_empty() {
        by_formula.keys().cloned().collect()
    } else {
        formula_names.to_vec()
    };

    let mut total_removed = 0;
    let mut total_space_freed = 0u64;

    if dry_run {
        println!("{} Dry run - no files will be removed", "ℹ".blue());
    } else {
        println!("{} Cleaning up old versions...", "🧹".bold());
    }

    for formula in &to_clean {
        let versions = match by_formula.get(formula) {
            Some(v) => v,
            None => {
                if !formula_names.is_empty() {
                    println!("  {} {} not installed", "⚠".yellow(), formula.bold());
                }
                continue;
            }
        };

        if versions.len() <= 1 {
            continue;
        }

        // Sort by version (keep the first one, which is typically the latest)
        let latest = versions[0];
        let old_versions = &versions[1..];

        for old in old_versions {
            let version_path = cellar::cellar_path().join(&old.name).join(&old.version);

            // Calculate directory size
            let size = calculate_dir_size(&version_path)?;
            total_space_freed += size;

            if dry_run {
                println!(
                    "  {} Would remove {} {} ({})",
                    "→".dimmed(),
                    old.name.cyan(),
                    old.version.dimmed(),
                    format_size(size).dimmed()
                );
            } else {
                println!(
                    "  {} Removing {} {} ({})",
                    "🗑".bold(),
                    old.name.cyan(),
                    old.version.dimmed(),
                    format_size(size).dimmed()
                );

                // Remove the old version directory
                if version_path.exists() {
                    std::fs::remove_dir_all(&version_path)?;
                }
            }

            total_removed += 1;
        }

        println!(
            "    {} Keeping {} {}",
            "✓".green(),
            latest.name.bold(),
            latest.version.dimmed()
        );
    }

    if total_removed == 0 {
        println!("\n{} No old versions to remove", "✓".green());
    } else if dry_run {
        println!(
            "\n{} Would remove {} old versions ({})",
            "ℹ".blue(),
            total_removed.to_string().bold(),
            format_size(total_space_freed).bold()
        );
    } else {
        println!(
            "\n{} Removed {} old versions, freed {}",
            "✓".green().bold(),
            total_removed.to_string().bold(),
            format_size(total_space_freed).bold()
        );
    }

    Ok(())
}

pub fn cache(clean: bool) -> Result<()> {
    let cache_dir = download::cache_dir();

    if clean {
        println!("{} Cleaning download cache...", "🧹".bold());

        if !cache_dir.exists() {
            println!("\n{} Cache is already empty", "✓".green());
            return Ok(());
        }

        // Calculate size before cleaning
        let total_size = calculate_dir_size(&cache_dir)?;

        // Remove all bottles from cache
        let mut removed_count = 0;
        for entry in std::fs::read_dir(&cache_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file()
                && path.extension().and_then(|s| s.to_str()) == Some("gz") {
                    std::fs::remove_file(&path)?;
                    removed_count += 1;
                }
        }

        println!(
            "\n{} Removed {} bottles, freed {}",
            "✓".green().bold(),
            removed_count.to_string().bold(),
            format_size(total_size).bold()
        );
    } else {
        // Show cache info
        println!("{}", "==> Download Cache".bold().green());
        println!();

        println!("{}: {}", "Location".bold(), cache_dir.display().to_string().cyan());

        if !cache_dir.exists() {
            println!("{}: {}", "Status".bold(), "Empty".dimmed());
            println!("{}: {}", "Size".bold(), "0 bytes".dimmed());
            return Ok(());
        }

        // Count bottles and calculate size
        let mut bottle_count = 0;
        let mut total_size = 0u64;

        for entry in std::fs::read_dir(&cache_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file()
                && path.extension().and_then(|s| s.to_str()) == Some("gz") {
                    bottle_count += 1;
                    total_size += std::fs::metadata(&path)?.len();
                }
        }

        println!("{}: {}", "Bottles".bold(), bottle_count.to_string().cyan());
        println!("{}: {}", "Size".bold(), format_size(total_size).cyan());

        if bottle_count > 0 {
            println!();
            println!("Run {} to clean the cache", "bru cache --clean".dimmed());
        }
    }

    Ok(())
}

fn calculate_dir_size(path: &std::path::Path) -> Result<u64> {
    let mut total = 0u64;

    if !path.exists() {
        return Ok(0);
    }

    for entry in walkdir::WalkDir::new(path) {
        let entry = entry.map_err(|e| anyhow::anyhow!("Failed to read directory: {}", e))?;
        if entry.file_type().is_file() {
            total += entry
                .metadata()
                .map_err(|e| anyhow::anyhow!("Failed to read metadata: {}", e))?
                .len();
        }
    }

    Ok(total)
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}

pub fn config() -> Result<()> {
    println!("{}", "==> System Configuration".bold().green());
    println!();

    let prefix = cellar::detect_prefix();
    let cellar = cellar::cellar_path();
    let taps = crate::tap::taps_path();

    println!("{}", "Paths:".bold());
    println!("  {}: {}", "Prefix".dimmed(), prefix.display().to_string().cyan());
    println!("  {}: {}", "Cellar".dimmed(), cellar.display().to_string().cyan());
    println!("  {}: {}", "Taps".dimmed(), taps.display().to_string().cyan());
    println!();

    let packages = cellar::list_installed()?;
    let installed_taps = crate::tap::list_taps()?;

    println!("{}", "Statistics:".bold());
    println!("  {}: {}", "Installed packages".dimmed(), packages.len().to_string().cyan());
    println!("  {}: {}", "Installed taps".dimmed(), installed_taps.len().to_string().cyan());
    println!();

    println!("{}", "System:".bold());
    println!("  {}: {}", "Version".dimmed(), env!("CARGO_PKG_VERSION").cyan());
    println!("  {}: {}", "Architecture".dimmed(), std::env::consts::ARCH.cyan());
    println!("  {}: {}", "OS".dimmed(), std::env::consts::OS.cyan());

    Ok(())
}

pub fn doctor() -> Result<()> {
    println!("{}", "==> System Health Check".bold().green());
    println!();

    let mut issues = 0;
    let mut warnings = 0;

    let prefix = cellar::detect_prefix();
    let cellar = cellar::cellar_path();
    let bin_dir = prefix.join("bin");

    println!("{}", "Checking system directories...".bold());

    // Check if prefix exists
    if !prefix.exists() {
        println!("  {} Homebrew prefix does not exist: {}", "✗".red(), prefix.display());
        issues += 1;
    } else {
        println!("  {} Homebrew prefix exists: {}", "✓".green(), prefix.display());
    }

    // Check if Cellar exists and is writable
    if !cellar.exists() {
        println!("  {} Cellar does not exist: {}", "⚠".yellow(), cellar.display());
        warnings += 1;
    } else if std::fs::metadata(&cellar)?.permissions().readonly() {
        println!("  {} Cellar is not writable: {}", "✗".red(), cellar.display());
        issues += 1;
    } else {
        println!("  {} Cellar exists and is writable", "✓".green());
    }

    // Check if bin directory exists
    if !bin_dir.exists() {
        println!("  {} Bin directory does not exist: {}", "⚠".yellow(), bin_dir.display());
        warnings += 1;
    } else {
        println!("  {} Bin directory exists: {}", "✓".green(), bin_dir.display());
    }

    println!();
    println!("{}", "Checking dependencies...".bold());

    // Check for git
    match std::process::Command::new("git").arg("--version").output() {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout);
            println!("  {} git is installed: {}", "✓".green(), version.trim().dimmed());
        }
        _ => {
            println!("  {} git is not installed or not in PATH", "✗".red());
            println!("    {} git is required for tap management", "ℹ".blue());
            issues += 1;
        }
    }

    println!();
    println!("{}", "Checking installed packages...".bold());

    // Check for broken symlinks
    let mut broken_links = Vec::new();
    if bin_dir.exists() {
        for entry in std::fs::read_dir(&bin_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_symlink()
                && let Ok(target) = std::fs::read_link(&path) {
                    let resolved = if target.is_absolute() {
                        target
                    } else {
                        bin_dir.join(&target)
                    };

                    if !resolved.exists() {
                        broken_links.push(path.file_name().unwrap().to_string_lossy().to_string());
                    }
                }
        }
    }

    if broken_links.is_empty() {
        println!("  {} No broken symlinks found", "✓".green());
    } else {
        println!("  {} Found {} broken symlinks:", "⚠".yellow(), broken_links.len());
        for link in broken_links.iter().take(5) {
            println!("    - {}", link.dimmed());
        }
        if broken_links.len() > 5 {
            println!("    ... and {} more", broken_links.len() - 5);
        }
        warnings += 1;
    }

    // Check for outdated packages
    let packages = cellar::list_installed()?;
    println!("  {} {} packages installed", "ℹ".blue(), packages.len());

    println!();
    println!("{}", "Summary:".bold());

    if issues == 0 && warnings == 0 {
        println!("  {} Your system is ready to brew!", "✓".green().bold());
    } else {
        if issues > 0 {
            println!("  {} Found {} issue(s) that need attention", "✗".red(), issues);
        }
        if warnings > 0 {
            println!("  {} Found {} warning(s)", "⚠".yellow(), warnings);
        }
    }

    Ok(())
}

pub async fn home(api: &BrewApi, formula_name: &str) -> Result<()> {
    println!("{} Opening homepage for {}...", "🌐".bold(), formula_name.cyan());

    let formula = api.fetch_formula(formula_name).await?;

    match &formula.homepage {
        Some(url) if !url.is_empty() => {
            println!("  {}: {}", "Homepage".dimmed(), url.cyan());

            // Open URL in default browser
            let status = std::process::Command::new("open")
                .arg(url)
                .status();

            match status {
                Ok(s) if s.success() => {
                    println!("  {} Opened in browser", "✓".green());
                }
                _ => {
                    println!("  {} Could not open browser automatically", "⚠".yellow());
                    println!("  {} Please visit: {}", "ℹ".blue(), url);
                }
            }
        }
        _ => {
            println!("  {} No homepage available for {}", "⚠".yellow(), formula_name.bold());
        }
    }

    Ok(())
}

pub fn leaves() -> Result<()> {
    println!("{}", "==> Leaf Packages".bold().green());
    println!("(Packages not required by other packages)");
    println!();

    let all_packages = cellar::list_installed()?;

    // Build a set of all packages that are dependencies of others
    let mut required_by_others = std::collections::HashSet::new();
    for pkg in &all_packages {
        for dep in pkg.runtime_dependencies() {
            required_by_others.insert(dep.full_name.clone());
        }
    }

    // Filter to packages that are NOT in the required set
    let mut leaves: Vec<_> = all_packages
        .iter()
        .filter(|pkg| !required_by_others.contains(&pkg.name))
        .collect();

    leaves.sort_by(|a, b| a.name.cmp(&b.name));

    if leaves.is_empty() {
        println!("{} No leaf packages found", "ℹ".blue());
    } else {
        for pkg in &leaves {
            println!("{}", pkg.name.cyan());
        }
        println!();
        println!("{} {} leaf packages", "ℹ".blue(), leaves.len().to_string().bold());
    }

    Ok(())
}

fn pinned_file_path() -> std::path::PathBuf {
    cellar::detect_prefix().join("var/homebrew/pinned_formulae")
}

fn read_pinned() -> Result<Vec<String>> {
    let path = pinned_file_path();
    if !path.exists() {
        return Ok(vec![]);
    }

    let content = std::fs::read_to_string(&path)?;
    Ok(content.lines().map(|s| s.to_string()).collect())
}

fn write_pinned(pinned: &[String]) -> Result<()> {
    let path = pinned_file_path();

    // Create parent directory if needed
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    std::fs::write(&path, pinned.join("\n"))?;
    Ok(())
}

pub fn pin(formula_names: &[String]) -> Result<()> {
    if formula_names.is_empty() {
        println!("{} No formulae specified", "❌".red());
        return Ok(());
    }

    println!("{} Pinning formulae...", "📌".bold());

    let mut pinned = read_pinned()?;

    for formula in formula_names {
        // Check if formula is installed
        let versions = cellar::get_installed_versions(formula)?;
        if versions.is_empty() {
            println!("  {} {} is not installed", "⚠".yellow(), formula.bold());
            continue;
        }

        if pinned.contains(formula) {
            println!("  {} {} is already pinned", "ℹ".blue(), formula.bold());
        } else {
            pinned.push(formula.clone());
            println!("  {} Pinned {}", "✓".green(), formula.bold().green());
        }
    }

    write_pinned(&pinned)?;

    Ok(())
}

pub fn unpin(formula_names: &[String]) -> Result<()> {
    if formula_names.is_empty() {
        println!("{} No formulae specified", "❌".red());
        return Ok(());
    }

    println!("{} Unpinning formulae...", "📌".bold());

    let mut pinned = read_pinned()?;

    for formula in formula_names {
        if let Some(pos) = pinned.iter().position(|x| x == formula) {
            pinned.remove(pos);
            println!("  {} Unpinned {}", "✓".green(), formula.bold().green());
        } else {
            println!("  {} {} is not pinned", "ℹ".blue(), formula.bold());
        }
    }

    write_pinned(&pinned)?;

    Ok(())
}

pub async fn desc(api: &BrewApi, formula_names: &[String]) -> Result<()> {
    if formula_names.is_empty() {
        println!("{} No formulae specified", "❌".red());
        return Ok(());
    }

    for formula_name in formula_names {
        match api.fetch_formula(formula_name).await {
            Ok(formula) => {
                print!("{}", formula.name.bold().cyan());
                if let Some(desc) = &formula.desc
                    && !desc.is_empty() {
                        println!(": {}", desc);
                    } else {
                        println!(": {}", "No description available".dimmed());
                    }
            }
            Err(_) => {
                println!("{}: {}", formula_name.bold().yellow(), "Not found".dimmed());
            }
        }
    }

    Ok(())
}

pub fn link(formula_names: &[String]) -> Result<()> {
    if formula_names.is_empty() {
        println!("{} No formulae specified", "❌".red());
        return Ok(());
    }

    println!("{} Linking formulae...", "🔗".bold());

    for formula_name in formula_names {
        let versions = cellar::get_installed_versions(formula_name)?;
        if versions.is_empty() {
            println!("  {} {} is not installed", "⚠".yellow(), formula_name.bold());
            continue;
        }

        let version = &versions[0].version;
        println!("  {} Linking {} {}", "🔗".bold(), formula_name.cyan(), version.dimmed());

        let linked = symlink::link_formula(formula_name, version)?;
        println!(
            "    {} Linked {} files",
            "✓".green(),
            linked.len().to_string().dimmed()
        );
    }

    Ok(())
}

pub fn unlink(formula_names: &[String]) -> Result<()> {
    if formula_names.is_empty() {
        println!("{} No formulae specified", "❌".red());
        return Ok(());
    }

    println!("{} Unlinking formulae...", "🔗".bold());

    for formula_name in formula_names {
        let versions = cellar::get_installed_versions(formula_name)?;
        if versions.is_empty() {
            println!("  {} {} is not installed", "⚠".yellow(), formula_name.bold());
            continue;
        }

        let version = &versions[0].version;
        println!("  {} Unlinking {} {}", "🔗".bold(), formula_name.cyan(), version.dimmed());

        let unlinked = symlink::unlink_formula(formula_name, version)?;
        println!(
            "    {} Unlinked {} files",
            "✓".green(),
            unlinked.len().to_string().dimmed()
        );
    }

    Ok(())
}

pub fn commands() -> Result<()> {
    println!("{}", "==> Available Commands".bold().green());
    println!();

    let commands_list = vec![
        ("search <query>", "Search for formulae and casks"),
        ("search <query> --formula", "Search only formulae"),
        ("search <query> --cask", "Search only casks"),
        ("info <formula>", "Show information about a formula or cask"),
        ("info <formula> --json", "Show formula info as JSON"),
        ("desc <formula>...", "Show formula descriptions"),
        ("deps <formula>", "Show dependencies for a formula"),
        ("deps <formula> --installed", "Show only installed dependencies"),
        ("uses <formula>", "Show formulae that depend on a formula"),
        ("list", "List installed packages"),
        ("outdated", "Show outdated installed packages"),
        ("fetch <formula>...", "Download bottles for formulae"),
        ("install <formula>...", "Install formulae from bottles"),
        ("upgrade [formula...]", "Upgrade installed formulae"),
        ("reinstall <formula>...", "Reinstall formulae"),
        ("uninstall <formula>...", "Uninstall formulae"),
        ("autoremove", "Remove unused dependencies"),
        ("link <formula>...", "Link a formula"),
        ("unlink <formula>...", "Unlink a formula"),
        ("cleanup [formula...]", "Remove old versions of installed formulae"),
        ("cache", "Manage download cache"),
        ("tap [user/repo]", "Add or list third-party repositories"),
        ("untap <user/repo>", "Remove a third-party repository"),
        ("update", "Update Homebrew and all taps"),
        ("config", "Show system configuration"),
        ("doctor", "Check system for potential problems"),
        ("home <formula>", "Open formula homepage in browser"),
        ("leaves", "List packages not required by others"),
        ("pin <formula>...", "Pin formulae to prevent upgrades"),
        ("unpin <formula>...", "Unpin formulae to allow upgrades"),
        ("missing [formula...]", "Check for missing dependencies"),
        ("analytics [on|off|state]", "Control analytics"),
        ("cat <formula>...", "Print formula source code"),
        ("shellenv [--shell <shell>]", "Print shell configuration"),
        ("gist-logs [formula]", "Generate diagnostic information"),
        ("alias [formula]", "Show formula aliases"),
        ("log <formula>", "Show install logs"),
        ("commands", "List all available commands"),
        ("completions <shell>", "Generate shell completion scripts"),
    ];

    for (cmd, desc) in &commands_list {
        println!("  {} {}", cmd.cyan().bold(), desc.dimmed());
    }

    println!();
    println!("{} {} commands available", "ℹ".blue(), commands_list.len().to_string().bold());
    println!("Run {} for help", "bru --help".cyan());

    Ok(())
}

pub fn missing(formula_names: &[String]) -> Result<()> {
    let to_check = if formula_names.is_empty() {
        // Check all installed packages
        cellar::list_installed()?
            .into_iter()
            .map(|p| p.name)
            .collect()
    } else {
        formula_names.to_vec()
    };

    if to_check.is_empty() {
        println!("{} No packages installed", "ℹ".blue());
        return Ok(());
    }

    println!("{} Checking for missing dependencies...", "🔍".bold());
    println!();

    let all_installed = cellar::list_installed()?;
    let installed_set: HashSet<_> = all_installed.iter().map(|p| p.name.as_str()).collect();

    let mut has_missing = false;

    for formula_name in &to_check {
        // Check if formula is installed
        let pkg = match all_installed.iter().find(|p| &p.name == formula_name) {
            Some(p) => p,
            None => {
                if !formula_names.is_empty() {
                    println!("{} {} is not installed", "⚠".yellow(), formula_name.bold());
                }
                continue;
            }
        };

        // Check each runtime dependency
        let runtime_deps = pkg.runtime_dependencies();
        let missing_deps: Vec<_> = runtime_deps
            .iter()
            .filter(|dep| !installed_set.contains(dep.full_name.as_str()))
            .collect();

        if !missing_deps.is_empty() {
            has_missing = true;
            println!("{} {} is missing dependencies:", "✗".red(), formula_name.bold());
            for dep in missing_deps {
                println!("  {} {} {}", "→".dimmed(), dep.full_name.cyan(), dep.version.dimmed());
            }
            println!();
        }
    }

    if !has_missing {
        println!("{} No missing dependencies found", "✓".green());
    }

    Ok(())
}

pub fn analytics(action: Option<&str>) -> Result<()> {
    let analytics_file = cellar::detect_prefix()
        .join("var/homebrew/analytics_disabled");

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
            println!();
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
            println!("{} Invalid action: {}", "❌".red(), other);
            println!("Valid actions: on, off, state");
            return Ok(());
        }
    }

    Ok(())
}

pub async fn cat(api: &BrewApi, formula_names: &[String]) -> Result<()> {
    if formula_names.is_empty() {
        println!("{} No formulae specified", "❌".red());
        return Ok(());
    }

    for (i, formula_name) in formula_names.iter().enumerate() {
        if i > 0 {
            println!(); // Blank line between formulae
        }

        println!("{} {}", "==>".bold().green(), formula_name.bold().cyan());
        println!();

        // Try to fetch formula from API
        match api.fetch_formula(formula_name).await {
            Ok(formula) => {
                // Print formula as JSON (since we don't have Ruby source)
                let json = serde_json::to_string_pretty(&formula)?;
                println!("{}", json);
            }
            Err(_) => {
                // Try as cask
                match api.fetch_cask(formula_name).await {
                    Ok(cask) => {
                        let json = serde_json::to_string_pretty(&cask)?;
                        println!("{}", json);
                    }
                    Err(_) => {
                        println!("{} No formula or cask found for '{}'", "❌".red(), formula_name);
                    }
                }
            }
        }
    }

    Ok(())
}

pub fn shellenv(shell: Option<&str>) -> Result<()> {
    let prefix = cellar::detect_prefix();

    // Detect shell if not provided
    let shell_type = match shell {
        Some(s) => s.to_string(),
        None => {
            // Try to detect from SHELL environment variable
            std::env::var("SHELL")
                .ok()
                .and_then(|s| {
                    let path = std::path::PathBuf::from(s);
                    path.file_name()
                        .and_then(|f| f.to_str())
                        .map(|s| s.to_string())
                })
                .unwrap_or_else(|| "bash".to_string())
        }
    };

    match shell_type.as_str() {
        "bash" | "sh" => {
            println!("export HOMEBREW_PREFIX=\"{}\";", prefix.display());
            println!("export HOMEBREW_CELLAR=\"{}/Cellar\";", prefix.display());
            println!("export HOMEBREW_REPOSITORY=\"{}\";", prefix.display());
            println!("export PATH=\"{}/bin:{}/sbin:$PATH\";", prefix.display(), prefix.display());
            println!("export MANPATH=\"{}/share/man:$MANPATH\";", prefix.display());
            println!("export INFOPATH=\"{}/share/info:$INFOPATH\";", prefix.display());
        }
        "zsh" => {
            println!("export HOMEBREW_PREFIX=\"{}\";", prefix.display());
            println!("export HOMEBREW_CELLAR=\"{}/Cellar\";", prefix.display());
            println!("export HOMEBREW_REPOSITORY=\"{}\";", prefix.display());
            println!("export PATH=\"{}/bin:{}/sbin${{PATH+:$PATH}}\";", prefix.display(), prefix.display());
            println!("export MANPATH=\"{}/share/man${{MANPATH+:$MANPATH}}:\";", prefix.display());
            println!("export INFOPATH=\"{}/share/info:${{INFOPATH:-}}\";", prefix.display());
        }
        "fish" => {
            println!("set -gx HOMEBREW_PREFIX \"{}\";", prefix.display());
            println!("set -gx HOMEBREW_CELLAR \"{}/Cellar\";", prefix.display());
            println!("set -gx HOMEBREW_REPOSITORY \"{}\";", prefix.display());
            println!("fish_add_path -gP \"{}/bin\" \"{}/sbin\";", prefix.display(), prefix.display());
            println!("set -gx MANPATH \"{}/share/man\" $MANPATH;", prefix.display());
            println!("set -gx INFOPATH \"{}/share/info\" $INFOPATH;", prefix.display());
        }
        other => {
            println!("{} Unsupported shell: {}", "❌".red(), other);
            println!("Supported shells: bash, zsh, fish");
            return Ok(());
        }
    }

    Ok(())
}

pub async fn gist_logs(api: &BrewApi, formula: Option<&str>) -> Result<()> {
    println!("{} Generating diagnostic information...", "📋".bold());
    println!();

    let mut output = String::new();

    // System information
    output.push_str("=== System Information ===\n");
    output.push_str(&format!("bru version: {}\n", env!("CARGO_PKG_VERSION")));
    output.push_str(&format!("OS: {}\n", std::env::consts::OS));
    output.push_str(&format!("Architecture: {}\n", std::env::consts::ARCH));
    output.push_str(&format!("Prefix: {}\n", cellar::detect_prefix().display()));
    output.push('\n');

    // Installed packages
    output.push_str("=== Installed Packages ===\n");
    let packages = cellar::list_installed()?;
    output.push_str(&format!("Total: {}\n", packages.len()));
    for pkg in packages.iter().take(20) {
        output.push_str(&format!("{} {}\n", pkg.name, pkg.version));
    }
    if packages.len() > 20 {
        output.push_str(&format!("... and {} more\n", packages.len() - 20));
    }
    output.push('\n');

    // Taps
    output.push_str("=== Taps ===\n");
    let taps = crate::tap::list_taps()?;
    for tap in &taps {
        output.push_str(&format!("{}\n", tap));
    }
    output.push('\n');

    // Formula-specific info if provided
    if let Some(formula_name) = formula {
        output.push_str(&format!("=== Formula: {} ===\n", formula_name));

        match api.fetch_formula(formula_name).await {
            Ok(formula) => {
                output.push_str(&format!("Name: {}\n", formula.name));
                if let Some(desc) = &formula.desc {
                    output.push_str(&format!("Description: {}\n", desc));
                }
                if let Some(version) = &formula.versions.stable {
                    output.push_str(&format!("Version: {}\n", version));
                }
                output.push_str(&format!("Dependencies: {}\n", formula.dependencies.join(", ")));

                // Check if installed
                let installed_versions = cellar::get_installed_versions(formula_name)?;
                if installed_versions.is_empty() {
                    output.push_str("Installed: No\n");
                } else {
                    output.push_str(&format!("Installed: Yes ({})\n",
                        installed_versions.iter()
                            .map(|v| v.version.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                }
            }
            Err(e) => {
                output.push_str(&format!("Error fetching formula: {}\n", e));
            }
        }
        output.push('\n');
    }

    // Config
    output.push_str("=== Configuration ===\n");
    output.push_str(&format!("Cellar: {}\n", cellar::cellar_path().display()));
    output.push_str(&format!("Cache: {}\n", crate::download::cache_dir().display()));

    // Check for pinned formulae
    let pinned = read_pinned()?;
    if !pinned.is_empty() {
        output.push_str(&format!("Pinned: {}\n", pinned.join(", ")));
    }

    // Doctor check summary
    output.push_str("\n=== Health Check ===\n");
    let prefix = cellar::detect_prefix();
    let cellar = cellar::cellar_path();
    output.push_str(&format!("Prefix exists: {}\n", prefix.exists()));
    output.push_str(&format!("Cellar exists: {}\n", cellar.exists()));
    output.push_str(&format!("Git available: {}\n",
        std::process::Command::new("git")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    ));

    println!("{}", output);
    println!();
    println!("{} Diagnostic information generated", "✓".green());
    println!("Copy the above output to share for debugging");

    Ok(())
}

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
                    format!("→ {}", formula_name).dimmed(),
                    format!("({})", desc).dimmed()
                );
            }

            println!();
            println!("Run {} to see aliases for a specific formula", "bru alias <formula>".cyan());
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
                    ].iter().cloned().collect();

                    if let Some(aliases) = common_aliases_map.get(formula.name.as_str()) {
                        println!();
                        println!("{}", "Common aliases:".bold());
                        for alias in aliases {
                            println!("  {}", alias.cyan());
                        }
                    } else {
                        println!();
                        println!("{} No known aliases", "ℹ".blue());
                    }
                }
                Err(_) => {
                    println!("{} Formula '{}' not found", "❌".red(), formula_name);
                }
            }
        }
    }

    Ok(())
}

pub fn log(formula_name: &str) -> Result<()> {
    println!("{} Checking logs for {}", "📋".bold(), formula_name.cyan());
    println!();

    // Check if formula is installed
    let installed_versions = cellar::get_installed_versions(formula_name)?;
    if installed_versions.is_empty() {
        println!("{} {} is not installed", "⚠".yellow(), formula_name.bold());
        println!();
        println!("Run {} to install it", format!("bru install {}", formula_name).cyan());
        return Ok(());
    }

    let version = &installed_versions[0].version;
    let install_path = cellar::cellar_path()
        .join(formula_name)
        .join(version);

    println!("{}", format!("==> {} {}", formula_name, version).bold().green());
    println!();

    // Check for INSTALL_RECEIPT.json
    let receipt_path = install_path.join("INSTALL_RECEIPT.json");
    if receipt_path.exists() {
        println!("{}", "Install Receipt:".bold());
        let receipt_content = std::fs::read_to_string(&receipt_path)?;
        let receipt: serde_json::Value = serde_json::from_str(&receipt_content)?;

        if let Some(obj) = receipt.as_object() {
            let on_request = obj.get("installed_on_request")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let status_text = if on_request { "Yes" } else { "No (dependency)" };
            println!("  {}: {}",
                "Installed on request".dimmed(),
                status_text.cyan()
            );

            if let Some(time) = obj.get("time").and_then(|v| v.as_i64()) {
                let datetime = chrono::DateTime::from_timestamp(time, 0)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_else(|| "Unknown".to_string());
                println!("  {}: {}", "Install time".dimmed(), datetime);
            }

            if let Some(built_from) = obj.get("source").and_then(|v| v.get("spec")).and_then(|v| v.as_str()) {
                println!("  {}: {}", "Built from".dimmed(), built_from);
            }
        }
        println!();
    }

    // List installed files
    println!("{}", "Installed files:".bold());
    let prefix = cellar::detect_prefix();
    let bin_dir = prefix.join("bin");

    let mut file_count = 0;
    if let Ok(entries) = std::fs::read_dir(&bin_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_symlink() {
                if let Ok(target) = std::fs::read_link(&path) {
                    // Resolve relative symlinks to absolute paths
                    let resolved_target = if target.is_absolute() {
                        target.clone()
                    } else {
                        bin_dir.join(&target).canonicalize().unwrap_or(target.clone())
                    };

                    if resolved_target.starts_with(&install_path) {
                        println!("  {} {}", path.file_name().unwrap().to_string_lossy().cyan(), format!("→ {}", target.display()).dimmed());
                        file_count += 1;
                        if file_count >= 10 {
                            println!("  {} (showing first 10)", "...".dimmed());
                            break;
                        }
                    }
                }
            }
        }
    }

    if file_count == 0 {
        println!("  {}", "No executables linked".dimmed());
    }

    println!();
    println!("{}: {}", "Install directory".dimmed(), install_path.display().to_string().cyan());

    Ok(())
}

pub fn which_formula(command: &str) -> Result<()> {
    println!("{} Finding formula for command: {}", "🔍".bold(), command.cyan());
    
    let prefix = cellar::detect_prefix();
    let bin_dir = prefix.join("bin");
    let command_path = bin_dir.join(command);
    
    if !command_path.exists() {
        println!("\n{} Command '{}' not found in {}", "⚠".yellow(), command.bold(), bin_dir.display());
        return Ok(());
    }
    
    // Check if it's a symlink
    if command_path.is_symlink() {
        if let Ok(target) = std::fs::read_link(&command_path) {
            // Resolve to absolute path
            let resolved = if target.is_absolute() {
                target
            } else {
                bin_dir.join(&target).canonicalize().unwrap_or(target)
            };
            
            // Extract formula name from Cellar path
            let cellar_path = cellar::cellar_path();
            if resolved.starts_with(&cellar_path) {
                if let Ok(rel_path) = resolved.strip_prefix(&cellar_path) {
                    if let Some(formula_name) = rel_path.components().next() {
                        println!("\n{}", formula_name.as_os_str().to_string_lossy().green().bold());
                        return Ok(());
                    }
                }
            }
        }
    }
    
    println!("\n{} Could not determine formula for '{}'", "⚠".yellow(), command.bold());
    Ok(())
}

pub async fn options(api: &BrewApi, formula_name: &str) -> Result<()> {
    println!("{} Checking options for: {}", "🔍".bold(), formula_name.cyan());
    
    // Verify formula exists
    match api.fetch_formula(formula_name).await {
        Ok(formula) => {
            println!("\n{}", format!("==> {}", formula.name).bold().green());
            if let Some(desc) = &formula.desc {
                println!("{}", desc);
            }
            println!();
            println!("{} No options available", "ℹ".blue());
            println!();
            println!("{}", "Bottles are pre-built binaries with fixed options.".dimmed());
            println!("{}", "For custom builds with options, use `brew install --build-from-source`.".dimmed());
        }
        Err(_) => {
            println!("\n{} Formula '{}' not found", "❌".red(), formula_name);
        }
    }
    
    Ok(())
}

pub async fn bundle(api: &BrewApi, dump: bool, file: Option<&str>) -> Result<()> {
    let brewfile_path = file.unwrap_or("Brewfile");
    
    if dump {
        // Generate Brewfile from installed packages
        println!("{} Generating Brewfile...", "📝".bold());
        
        let mut content = String::new();
        
        // Get all taps
        let taps = crate::tap::list_taps()?;
        if !taps.is_empty() {
            for tap in &taps {
                content.push_str(&format!("tap \"{}\"\n", tap));
            }
            content.push('\n');
        }
        
        // Get all installed packages
        let packages = cellar::list_installed()?;
        let mut formulae_names: Vec<_> = packages.iter()
            .filter(|p| p.installed_on_request())
            .map(|p| p.name.as_str())
            .collect();
        formulae_names.sort();
        
        for name in &formulae_names {
            content.push_str(&format!("brew \"{}\"\n", name));
        }
        
        // Write to file
        std::fs::write(brewfile_path, &content)?;
        
        println!("{} Generated {} with {} formulae", 
            "✓".green(), 
            brewfile_path.cyan(), 
            formulae_names.len().to_string().bold()
        );
    } else {
        // Install from Brewfile
        println!("{} Reading {}...", "📖".bold(), brewfile_path.cyan());
        
        if !std::path::Path::new(brewfile_path).exists() {
            println!("\n{} {} not found", "❌".red(), brewfile_path.bold());
            println!("Run {} to generate one", "bru bundle dump".cyan());
            return Ok(());
        }
        
        let content = std::fs::read_to_string(brewfile_path)?;
        
        let mut taps_to_add = Vec::new();
        let mut formulae_to_install = Vec::new();
        let mut casks_to_install = Vec::new();
        
        // Parse Brewfile
        for line in content.lines() {
            let line = line.trim();
            
            // Skip comments and empty lines
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            
            // Parse tap lines: tap "user/repo"
            if let Some(tap_line) = line.strip_prefix("tap") {
                let tap_line = tap_line.trim();
                if let Some(tap_name) = extract_quoted_string(tap_line) {
                    taps_to_add.push(tap_name.to_string());
                }
            }
            
            // Parse brew lines: brew "formula"
            if let Some(brew_line) = line.strip_prefix("brew") {
                let brew_line = brew_line.trim();
                if let Some(formula_name) = extract_quoted_string(brew_line) {
                    formulae_to_install.push(formula_name.to_string());
                }
            }
            
            // Parse cask lines: cask "app"
            if let Some(cask_line) = line.strip_prefix("cask") {
                let cask_line = cask_line.trim();
                if let Some(cask_name) = extract_quoted_string(cask_line) {
                    casks_to_install.push(cask_name.to_string());
                }
            }
            
            // Skip mas lines for now
        }
        
        println!(
            "\n{} Found: {} taps, {} formulae, {} casks",
            "✓".green(),
            taps_to_add.len().to_string().bold(),
            formulae_to_install.len().to_string().bold(),
            casks_to_install.len().to_string().bold()
        );
        
        // Install taps first
        if !taps_to_add.is_empty() {
            println!("\n{} Adding taps...", "🔗".bold());
            for tap_name in &taps_to_add {
                if crate::tap::is_tapped(tap_name)? {
                    println!("  {} {} already tapped", "✓".green(), tap_name.dimmed());
                } else {
                    println!("  {} Tapping {}...", "→".bold(), tap_name.cyan());
                    match crate::tap::tap(tap_name) {
                        Ok(_) => println!("    {} Tapped {}", "✓".green(), tap_name.bold()),
                        Err(e) => println!("    {} Failed: {}", "❌".red(), e),
                    }
                }
            }
        }
        
        // Install formulae
        if !formulae_to_install.is_empty() {
            println!("\n{} Installing formulae...", "📦".bold());
            match install(api, &formulae_to_install, false).await {
                Ok(_) => {}
                Err(e) => {
                    println!("{} Failed to install some formulae: {}", "⚠".yellow(), e);
                }
            }
        }
        
        // Casks - for now, just notify
        if !casks_to_install.is_empty() {
            println!("\n{} Cask installation not yet implemented", "ℹ".blue());
            println!("  Casks to install: {}", casks_to_install.join(", ").dimmed());
        }
        
        println!("\n{} Bundle install complete", "✓".green().bold());
    }
    
    Ok(())
}

fn extract_quoted_string(s: &str) -> Option<&str> {
    // Extract string from quotes: "string" or 'string'
    let s = s.trim();
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        Some(&s[1..s.len()-1])
    } else {
        None
    }
}

pub fn services(action: Option<&str>, formula: Option<&str>) -> Result<()> {
    match action {
        None | Some("list") => {
            // List all services
            println!("{}", "==> Services".bold().green());
            println!();

            let services = crate::services::list_all_services()?;

            if services.is_empty() {
                println!("{} No services found", "ℹ".blue());
                println!();
                println!("Services are background processes like databases and web servers.");
                println!("Common services: postgresql, mysql, redis, nginx");
                return Ok(());
            }

            // Print header
            println!(
                "{:<20} {:<12} {:<8} {}",
                "Name".bold(),
                "Status".bold(),
                "User".bold(),
                "File".bold()
            );

            // Print services
            for service in &services {
                let status_str = match &service.status {
                    crate::services::ServiceStatus::None => "none".dimmed().to_string(),
                    crate::services::ServiceStatus::Started => "started".green().to_string(),
                    crate::services::ServiceStatus::Error(code) => {
                        format!("error  {}", code).red().to_string()
                    }
                };

                let user_str = service.user.as_deref().unwrap_or("");
                let file_str = service
                    .plist_path
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default();

                println!(
                    "{:<20} {:<20} {:<8} {}",
                    service.name.cyan(),
                    status_str,
                    user_str,
                    file_str.dimmed()
                );
            }

            println!();
            println!("{} {} services", "ℹ".blue(), services.len().to_string().bold());
        }
        Some("start") => {
            let formula = formula.ok_or_else(|| anyhow::anyhow!("Formula name required"))?;
            println!("{} Starting service: {}", "▶".bold(), formula.cyan());

            if !crate::services::service_exists(formula) {
                println!("\n{} Service file not found for {}", "⚠".yellow(), formula.bold());
                println!();
                println!("To create a service, the formula must support it.");
                println!("Run {} to check if service is available", format!("bru services list").cyan());
                return Ok(());
            }

            match crate::services::start_service(formula) {
                Ok(_) => {
                    println!("  {} Started {}", "✓".green(), formula.bold().green());
                }
                Err(e) => {
                    println!("  {} Failed to start: {}", "❌".red(), e);
                }
            }
        }
        Some("stop") => {
            let formula = formula.ok_or_else(|| anyhow::anyhow!("Formula name required"))?;
            println!("{} Stopping service: {}", "■".bold(), formula.cyan());

            if !crate::services::service_exists(formula) {
                println!("\n{} Service file not found for {}", "⚠".yellow(), formula.bold());
                return Ok(());
            }

            match crate::services::stop_service(formula) {
                Ok(_) => {
                    println!("  {} Stopped {}", "✓".green(), formula.bold().green());
                }
                Err(e) => {
                    println!("  {} Failed to stop: {}", "❌".red(), e);
                }
            }
        }
        Some("restart") => {
            let formula = formula.ok_or_else(|| anyhow::anyhow!("Formula name required"))?;
            println!("{} Restarting service: {}", "🔄".bold(), formula.cyan());

            if !crate::services::service_exists(formula) {
                println!("\n{} Service file not found for {}", "⚠".yellow(), formula.bold());
                return Ok(());
            }

            match crate::services::restart_service(formula) {
                Ok(_) => {
                    println!("  {} Restarted {}", "✓".green(), formula.bold().green());
                }
                Err(e) => {
                    println!("  {} Failed to restart: {}", "❌".red(), e);
                }
            }
        }
        Some(other) => {
            println!("{} Unknown action: {}", "❌".red(), other);
            println!();
            println!("Available actions:");
            println!("  {} - List all services", "list".cyan());
            println!("  {} - Start a service", "start <formula>".cyan());
            println!("  {} - Stop a service", "stop <formula>".cyan());
            println!("  {} - Restart a service", "restart <formula>".cyan());
        }
    }

    Ok(())
}
