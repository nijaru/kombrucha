use crate::cellar;
use crate::error::Result;
use colored::Colorize;

pub fn pinned_file_path() -> std::path::PathBuf {
    cellar::detect_prefix().join("var/homebrew/pinned_formulae")
}

pub fn read_pinned() -> Result<Vec<String>> {
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
        println!("{} No formulae specified", "✗".red());
        return Ok(());
    }

    println!("Pinning formulae...");

    let mut pinned = read_pinned()?;

    for formula in formula_names {
        // Check if formula is installed
        let versions = cellar::get_installed_versions(formula)?;
        if versions.is_empty() {
            println!("  {} {} is not installed", "⚠".yellow(), formula.bold());
            continue;
        }

        if pinned.contains(formula) {
            println!("  {} is already pinned", formula.bold());
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
        println!("{} No formulae specified", "✗".red());
        return Ok(());
    }

    println!("Unpinning formulae...");

    let mut pinned = read_pinned()?;

    for formula in formula_names {
        if let Some(pos) = pinned.iter().position(|x| x == formula) {
            pinned.remove(pos);
            println!("  {} Unpinned {}", "✓".green(), formula.bold().green());
        } else {
            println!("  {} is not pinned", formula.bold());
        }
    }

    write_pinned(&pinned)?;

    Ok(())
}
