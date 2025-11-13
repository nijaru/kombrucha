use anyhow::Result;
use clap::Parser;
use colored::Colorize;

use crate::{api::BrewApi, cellar, symlink};

/// Link formulae
#[derive(Parser)]
pub struct LinkCommand {
    /// Formula names to link
    #[arg(required = true)]
    formula_names: Vec<String>,
}

/// Unlink formulae
#[derive(Parser)]
pub struct UnlinkCommand {
    /// Formula names to unlink
    #[arg(required = true)]
    formula_names: Vec<String>,
}

pub async fn link(api: &BrewApi, formula_names: &[String]) -> Result<()> {
    if formula_names.is_empty() {
        println!("{} No formulae specified", "✗".red());
        return Ok(());
    }

    println!("Linking formulae...");

    for formula_name in formula_names {
        let versions = cellar::get_installed_versions(formula_name)?;
        if versions.is_empty() {
            println!(
                "  {} {} is not installed",
                "⚠".yellow(),
                formula_name.bold()
            );
            continue;
        }

        // Fetch formula metadata to check if it's keg-only
        let formula = match api.fetch_formula(formula_name).await {
            Ok(f) => f,
            Err(_) => {
                println!(
                    "  {} Failed to fetch metadata for {}",
                    "✗".red(),
                    formula_name.bold()
                );
                continue;
            }
        };

        // Homebrew doesn't allow linking keg-only formulas
        if formula.keg_only {
            println!(
                "  {} {} is keg-only and cannot be linked",
                "⚠".yellow(),
                formula_name.bold()
            );
            if let Some(reason) = &formula.keg_only_reason {
                println!("    {} {}", "ℹ".cyan(), reason.explanation);
            }
            continue;
        }

        let version = &versions[0].version;
        println!("  Linking {} {}", formula_name.cyan(), version.dimmed());

        let linked = symlink::link_formula(formula_name, version)?;

        // Create version-agnostic symlinks (opt/ and var/homebrew/linked/)
        symlink::optlink(formula_name, version)?;

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
        println!("{} No formulae specified", "✗".red());
        return Ok(());
    }

    println!("Unlinking formulae...");

    for formula_name in formula_names {
        let versions = cellar::get_installed_versions(formula_name)?;
        if versions.is_empty() {
            println!(
                "  {} {} is not installed",
                "⚠".yellow(),
                formula_name.bold()
            );
            continue;
        }

        // Use linked version as version to unlink
        // This is correct even if multiple versions exist
        let version = if let Ok(Some(linked_ver)) = symlink::get_linked_version(formula_name) {
            linked_ver
        } else {
            println!("  {} {} is not linked", "⚠".yellow(), formula_name.bold());
            continue;
        };

        println!("  Unlinking {} {}", formula_name.cyan(), version.dimmed());

        let unlinked = symlink::unlink_formula(formula_name, &version)?;

        // Remove version-agnostic symlinks (opt/ and var/homebrew/linked/)
        symlink::unoptlink(formula_name)?;

        println!(
            "    {} Unlinked {} files",
            "✓".green(),
            unlinked.len().to_string().dimmed()
        );
    }

    Ok(())
}
