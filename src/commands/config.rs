use anyhow::Result;
use clap::Parser;
use colored::Colorize;

use crate::{cask, cellar, download, tap};

/// Show system configuration
#[derive(Parser)]
pub struct ConfigCommand {}

pub fn config() -> Result<()> {
    println!("{}", "==> System Configuration".bold().green());
    println!();

    let prefix = cellar::detect_prefix();
    let cellar = cellar::cellar_path();
    let taps = tap::taps_path();

    println!("{}", "Paths:".bold());
    println!(
        "  {}: {}",
        "Prefix".dimmed(),
        prefix.display().to_string().cyan()
    );
    println!(
        "  {}: {}",
        "Cellar".dimmed(),
        cellar.display().to_string().cyan()
    );
    println!(
        "  {}: {}",
        "Taps".dimmed(),
        taps.display().to_string().cyan()
    );
    println!();

    let packages = cellar::list_installed()?;
    let installed_taps = tap::list_taps()?;

    println!("{}", "Statistics:".bold());
    println!(
        "  {}: {}",
        "Installed packages".dimmed(),
        packages.len().to_string().cyan()
    );
    println!(
        "  {}: {}",
        "Installed taps".dimmed(),
        installed_taps.len().to_string().cyan()
    );
    println!();

    println!("{}", "System:".bold());
    println!(
        "  {}: {}",
        "Version".dimmed(),
        env!("CARGO_PKG_VERSION").cyan()
    );
    println!(
        "  {}: {}",
        "Architecture".dimmed(),
        std::env::consts::ARCH.cyan()
    );
    println!("  {}: {}", "OS".dimmed(), std::env::consts::OS.cyan());

    Ok(())
}

/// Show Homebrew environment variables
#[derive(Parser)]
pub struct EnvCommand {}

pub fn env() -> Result<()> {
    let prefix = cellar::detect_prefix();
    let cellar = cellar::cellar_path();
    let cache = download::cache_dir();
    let taps = tap::taps_path();
    let logs = prefix.join("var/log");
    let caskroom = cask::caskroom_dir();

    println!("HOMEBREW_PREFIX=\"{}\"", prefix.display());
    println!("HOMEBREW_CELLAR=\"{}\"", cellar.display());
    println!("HOMEBREW_REPOSITORY=\"{}\"", prefix.display());
    println!("HOMEBREW_CACHE=\"{}\"", cache.display());
    println!("HOMEBREW_TAPS=\"{}\"", taps.display());
    println!("HOMEBREW_LOGS=\"{}\"", logs.display());
    println!("HOMEBREW_CASKROOM=\"{}\"", caskroom.display());
    println!("HOMEBREW_ARCH=\"{}\"", std::env::consts::ARCH);
    println!("HOMEBREW_OS=\"{}\"", std::env::consts::OS);
    println!("HOMEBREW_VERSION=\"{}\"", env!("CARGO_PKG_VERSION"));

    Ok(())
}

/// Check system for potential problems
#[derive(Parser)]
pub struct DoctorCommand {}

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
        println!(
            "  {} Homebrew prefix does not exist: {}",
            "✗".red(),
            prefix.display()
        );
        issues += 1;
    } else {
        println!(
            "  {} Homebrew prefix exists: {}",
            "✓".green(),
            prefix.display()
        );
    }

    // Check if Cellar exists and is writable
    if !cellar.exists() {
        println!(
            "  {} Cellar does not exist: {}",
            "⚠".yellow(),
            cellar.display()
        );
        warnings += 1;
    } else if std::fs::metadata(&cellar)?.permissions().readonly() {
        println!(
            "  {} Cellar is not writable: {}",
            "✗".red(),
            cellar.display()
        );
        issues += 1;
    } else {
        println!("  {} Cellar exists and is writable", "✓".green());
    }

    // Check if bin directory exists
    if !bin_dir.exists() {
        println!(
            "  {} Bin directory does not exist: {}",
            "⚠".yellow(),
            bin_dir.display()
        );
        warnings += 1;
    } else {
        println!(
            "  {} Bin directory exists: {}",
            "✓".green(),
            bin_dir.display()
        );
    }

    println!();
    println!("{}", "Checking dependencies...".bold());

    // Check for git
    match std::process::Command::new("git").arg("--version").output() {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout);
            println!(
                "  {} git is installed: {}",
                "✓".green(),
                version.trim().dimmed()
            );
        }
        _ => {
            println!("  {} git is not installed or not in PATH", "✗".red());
            println!("    git is required for tap management");
            println!(
                "    {} Install with: {}",
                "→".dimmed(),
                "brew install git".cyan()
            );
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
                && let Ok(target) = std::fs::read_link(&path)
            {
                let resolved = if target.is_absolute() {
                    target
                } else {
                    bin_dir.join(&target)
                };

                if !resolved.exists()
                    && let Some(name) = path.file_name()
                {
                    broken_links.push(name.to_string_lossy().to_string());
                }
            }
        }
    }

    if broken_links.is_empty() {
        println!("  {} No broken symlinks found", "✓".green());
    } else {
        println!(
            "  {} Found {} broken symlinks:",
            "⚠".yellow(),
            broken_links.len()
        );
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
    println!("  {} packages installed", packages.len());

    println!();
    println!("{}", "Summary:".bold());

    if issues == 0 && warnings == 0 {
        println!("  {} Your system is ready to brew!", "✓".green().bold());
    } else {
        if issues > 0 {
            println!(
                "  {} Found {} issue(s) that need attention",
                "✗".red(),
                issues
            );
        }
        if warnings > 0 {
            println!("  {} Found {} warning(s)", "⚠".yellow(), warnings);
        }
    }

    Ok(())
}
