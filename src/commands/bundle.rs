//! Brewfile management commands
//!
//! Handles generating and installing from Brewfiles - declarative package
//! lists that specify which formulae, casks, and taps should be installed.

use crate::api::BrewApi;
use crate::cellar;
use crate::error::Result;
use colored::Colorize;

use super::install::install;

/// Generate or install from a Brewfile
///
/// Brewfiles are declarative package lists that specify which formulae,
/// casks, and taps should be installed. This is useful for setting up
/// new machines or maintaining consistent package sets.
///
/// # Arguments
/// * `api` - The Homebrew API client
/// * `dump` - If true, generate a Brewfile from installed packages
/// * `file` - The Brewfile path (defaults to "Brewfile")
pub async fn bundle(api: &BrewApi, dump: bool, file: Option<&str>) -> Result<()> {
    let brewfile_path = file.unwrap_or("Brewfile");

    if dump {
        // Generate Brewfile from installed packages
        println!("Generating Brewfile...");

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
        let mut formulae_names: Vec<_> = packages
            .iter()
            .filter(|p| p.installed_on_request())
            .map(|p| p.name.as_str())
            .collect();
        formulae_names.sort();

        for name in &formulae_names {
            content.push_str(&format!("brew \"{}\"\n", name));
        }

        // Get all installed casks
        let casks = crate::cask::list_installed_casks()?;
        let mut cask_tokens: Vec<_> = casks.iter().map(|(token, _)| token).collect();
        cask_tokens.sort();

        if !cask_tokens.is_empty() {
            content.push('\n');
            for token in &cask_tokens {
                content.push_str(&format!("cask \"{}\"\n", token));
            }
        }

        // Write to file
        std::fs::write(brewfile_path, &content)?;

        println!(
            "{} Generated {} with {} formulae and {} casks",
            "".green(),
            brewfile_path.cyan(),
            formulae_names.len().to_string().bold(),
            cask_tokens.len().to_string().bold()
        );
    } else {
        // Install from Brewfile
        println!("Reading {}...", brewfile_path.cyan());

        if !std::path::Path::new(brewfile_path).exists() {
            println!("{} {} not found", "".red(), brewfile_path.bold());
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
            "{} Found: {} taps, {} formulae, {} casks",
            "".green(),
            taps_to_add.len().to_string().bold(),
            formulae_to_install.len().to_string().bold(),
            casks_to_install.len().to_string().bold()
        );

        // Install taps first
        if !taps_to_add.is_empty() {
            println!("Adding taps...");
            for tap_name in &taps_to_add {
                if crate::tap::is_tapped(tap_name)? {
                    println!("  {} {} already tapped", "".green(), tap_name.dimmed());
                } else {
                    println!("  Tapping {}...", tap_name.cyan());
                    match crate::tap::tap(tap_name) {
                        Ok(_) => println!("    {} Tapped {}", "".green(), tap_name.bold()),
                        Err(e) => println!("    {} Failed: {}", "".red(), e),
                    }
                }
            }
        }

        // Install formulae
        if !formulae_to_install.is_empty() {
            println!("Installing formulae...");
            match install(api, &formulae_to_install, false, false, false).await {
                Ok(_) => {}
                Err(e) => {
                    println!("{} Failed to install some formulae: {}", "".yellow(), e);
                }
            }
        }

        // Install casks
        if !casks_to_install.is_empty() {
            println!("Installing casks...");
            match super::cask::install_cask(api, &casks_to_install).await {
                Ok(_) => {}
                Err(e) => {
                    println!("{} Failed to install some casks: {}", "".yellow(), e);
                }
            }
        }

        println!("{} Bundle install complete", "".green().bold());
    }

    Ok(())
}

/// Extract string from quotes: "string" or 'string'
fn extract_quoted_string(s: &str) -> Option<&str> {
    let s = s.trim();
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        Some(&s[1..s.len() - 1])
    } else {
        None
    }
}
