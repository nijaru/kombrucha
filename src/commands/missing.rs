use crate::cellar;
use crate::error::Result;
use colored::Colorize;
use std::collections::HashSet;

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
        println!("No packages installed");
        return Ok(());
    }

    println!("Checking for missing dependencies...");
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
            println!(
                "{} {} is missing dependencies:",
                "✗".red(),
                formula_name.bold()
            );
            for dep in missing_deps {
                println!("  {} {}", dep.full_name.cyan(), dep.version.dimmed());
            }
            println!();
        }
    }

    if !has_missing {
        println!("{} No missing dependencies found", "✓".green());
    }

    Ok(())
}
