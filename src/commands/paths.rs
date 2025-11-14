//! Path and environment variable operations.
//!
//! This module provides commands for displaying and configuring Homebrew paths
//! and environment variables, including:
//! - Homebrew installation prefix and cellar locations
//! - Repository paths for taps
//! - Environment variable configuration
//! - Shell integration setup

use crate::cellar;
use crate::error::Result;
use colored::Colorize;

/// Display the Homebrew installation prefix.
///
/// Without arguments, shows the Homebrew prefix directory.
/// With a formula name, shows the installation path for that specific formula.
pub fn prefix(formula_name: Option<&str>) -> anyhow::Result<()> {
    let prefix = cellar::detect_prefix();

    if let Some(name) = formula_name {
        // Show formula-specific installation prefix
        let versions = cellar::get_installed_versions(name)?;
        if versions.is_empty() {
            anyhow::bail!("Formula '{}' is not installed", name);
        }

        let version = &versions[0].version;
        let formula_prefix = cellar::cellar_path().join(name).join(version);

        println!("{}", formula_prefix.display());
    } else {
        // Show Homebrew prefix directory
        println!("{}", prefix.display());
    }

    Ok(())
}

/// Display the Homebrew Cellar path.
///
/// Without arguments, shows the Cellar directory where all formulae are installed.
/// With a formula name, shows the Cellar subdirectory for that specific formula.
pub fn cellar_cmd(formula_name: Option<&str>) -> anyhow::Result<()> {
    let cellar = cellar::cellar_path();

    if let Some(name) = formula_name {
        // Show formula-specific cellar directory
        println!("{}", cellar.join(name).display());
    } else {
        // Show main Cellar path
        println!("{}", cellar.display());
    }

    Ok(())
}

/// Display the path to a Homebrew tap repository.
///
/// Without arguments, shows the path to homebrew-core (the main tap).
/// With a tap name, shows the path to that specific tap's repository.
pub fn repository(tap_name: Option<&str>) -> anyhow::Result<()> {
    let prefix = cellar::detect_prefix();

    if let Some(tap) = tap_name {
        // Show path to specific tap repository
        let tap_path = crate::tap::tap_directory(tap)?;
        println!("{}", tap_path.display());
    } else {
        // Show path to main homebrew-core repository
        let repo = prefix.join("Library/Taps/homebrew/homebrew-core");
        println!("{}", repo.display());
    }

    Ok(())
}

/// Display comprehensive system configuration information.
///
/// Shows installation paths, statistics about installed packages and taps,
/// and system information like version and architecture.
pub fn config() -> Result<()> {
    println!("{}", "==> System Configuration".bold().green());
    println!();

    let prefix = cellar::detect_prefix();
    let cellar = cellar::cellar_path();
    let taps = crate::tap::taps_path();

    // Display key installation paths
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

    // Get installation statistics
    let packages = cellar::list_installed()?;
    let installed_taps = crate::tap::list_taps()?;

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

    // Display system information
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

/// Display Homebrew environment variables.
///
/// Outputs environment variables that can be sourced by the shell or used
/// by other tools. This includes paths to prefix, cellar, cache, and logs.
pub fn env() -> Result<()> {
    let prefix = cellar::detect_prefix();
    let cellar = cellar::cellar_path();
    let cache = crate::download::cache_dir();
    let taps = crate::tap::taps_path();
    let logs = prefix.join("var/log");
    let caskroom = crate::cask::caskroom_dir();

    // Output environment variables in shell-compatible format
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

/// Generate shell configuration for Homebrew integration.
///
/// Outputs shell-specific commands to set up PATH, MANPATH, INFOPATH, and
/// Homebrew environment variables. The output is designed to be evaluated
/// by the shell (e.g., `eval "$(bru shellenv)"`).
///
/// Supports bash, zsh, and fish shells. Auto-detects from $SHELL if not specified.
pub fn shellenv(shell: Option<&str>) -> Result<()> {
    let prefix = cellar::detect_prefix();

    // Auto-detect shell from $SHELL environment variable if not explicitly provided
    let shell_type = match shell {
        Some(s) => String::from(s),
        None => std::env::var("SHELL")
            .ok()
            .and_then(|s| {
                let path = std::path::PathBuf::from(s);
                path.file_name().and_then(|f| f.to_str()).map(String::from)
            })
            .unwrap_or_else(|| String::from("bash")),
    };

    // Generate shell-specific configuration
    match shell_type.as_str() {
        "bash" | "sh" => {
            // POSIX-compatible shell configuration
            println!("export HOMEBREW_PREFIX=\"{}\";", prefix.display());
            println!("export HOMEBREW_CELLAR=\"{}/Cellar\";", prefix.display());
            println!("export HOMEBREW_REPOSITORY=\"{}\";", prefix.display());
            println!(
                "export PATH=\"{}/bin:{}/sbin:$PATH\";",
                prefix.display(),
                prefix.display()
            );
            println!(
                "export MANPATH=\"{}/share/man:$MANPATH\";",
                prefix.display()
            );
            println!(
                "export INFOPATH=\"{}/share/info:$INFOPATH\";",
                prefix.display()
            );
        }
        "zsh" => {
            // Zsh-specific configuration with parameter expansion
            println!("export HOMEBREW_PREFIX=\"{}\";", prefix.display());
            println!("export HOMEBREW_CELLAR=\"{}/Cellar\";", prefix.display());
            println!("export HOMEBREW_REPOSITORY=\"{}\";", prefix.display());
            println!(
                "export PATH=\"{}/bin:{}/sbin${{PATH+:$PATH}}\";",
                prefix.display(),
                prefix.display()
            );
            println!(
                "export MANPATH=\"{}/share/man${{MANPATH+:$MANPATH}}:\";",
                prefix.display()
            );
            println!(
                "export INFOPATH=\"{}/share/info:${{INFOPATH:-}}\";",
                prefix.display()
            );
        }
        "fish" => {
            // Fish shell configuration using fish-specific commands
            println!("set -gx HOMEBREW_PREFIX \"{}\";", prefix.display());
            println!("set -gx HOMEBREW_CELLAR \"{}/Cellar\";", prefix.display());
            println!("set -gx HOMEBREW_REPOSITORY \"{}\";", prefix.display());
            println!(
                "fish_add_path -gP \"{}/bin\" \"{}/sbin\";",
                prefix.display(),
                prefix.display()
            );
            println!(
                "set -gx MANPATH \"{}/share/man\" $MANPATH;",
                prefix.display()
            );
            println!(
                "set -gx INFOPATH \"{}/share/info\" $INFOPATH;",
                prefix.display()
            );
        }
        other => {
            // Unsupported shell - display error and supported options
            println!("{} Unsupported shell: {}", "✗".red(), other);
            println!("Supported shells: bash, zsh, fish");
            return Ok(());
        }
    }

    Ok(())
}
