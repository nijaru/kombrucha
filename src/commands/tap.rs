//! Tap management commands for adding, removing, and managing Homebrew taps.
//!
//! Taps are third-party repositories that extend Homebrew with additional formulae
//! and casks. This module provides commands for managing taps and their lifecycle.

use crate::cellar;
use crate::error::Result;
use colored::Colorize;
use std::path::Path;

/// List all installed taps, or add a new tap
pub fn tap(tap_name: Option<&str>) -> Result<()> {
    match tap_name {
        None => {
            // List all taps
            let taps = crate::tap::list_taps()?;
            if taps.is_empty() {
                println!("No taps installed");
            } else {
                for tap in taps {
                    println!("{}", tap.cyan());
                }
            }
        }
        Some(tap) => {
            // Add a tap
            println!("Tapping {}...", tap.cyan());

            if crate::tap::is_tapped(tap)? {
                println!("  {} {} already tapped", "".green(), tap.bold());
                return Ok(());
            }

            crate::tap::tap(tap)?;

            println!(
                "  {} Tapped {} successfully",
                "".green(),
                tap.bold().green()
            );
        }
    }
    Ok(())
}

/// Remove an installed tap
pub fn untap(tap_name: &str) -> Result<()> {
    println!("Untapping {}...", tap_name.cyan());

    if !crate::tap::is_tapped(tap_name)? {
        println!("  {} {} is not tapped", "".yellow(), tap_name.bold());
        return Ok(());
    }

    crate::tap::untap(tap_name)?;

    println!(
        "  {} Untapped {} successfully",
        "".green(),
        tap_name.bold().green()
    );

    Ok(())
}

/// Display information about an installed tap including formulae and cask counts
pub fn tap_info(tap_name: &str) -> Result<()> {
    println!(
        "{} Tap information for {}",
        "".bold(),
        tap_name.cyan().bold()
    );
    println!();

    if !crate::tap::is_tapped(tap_name)? {
        println!("  {} Tap {} is not installed", "".yellow(), tap_name.bold());
        return Ok(());
    }

    let tap_dir = crate::tap::tap_directory(tap_name)?;

    println!("{}", "Location:".bold());
    println!("  {}", tap_dir.display().to_string().cyan());
    println!();

    // Count formulae in the tap (recursively for letter-organized directories)
    let formula_dir = tap_dir.join("Formula");
    let formula_count = if formula_dir.exists() {
        count_rb_files(&formula_dir, 0)
    } else {
        0
    };

    // Count casks in the tap
    let casks_dir = tap_dir.join("Casks");
    let cask_count = if casks_dir.exists() {
        count_rb_files(&casks_dir, 0)
    } else {
        0
    };

    println!("{}", "Contents:".bold());
    println!(
        "  {}: {}",
        "Formulae".dimmed(),
        formula_count.to_string().cyan()
    );
    println!("  {}: {}", "Casks".dimmed(), cask_count.to_string().cyan());

    Ok(())
}

/// Recursively count .rb files in a directory (helper for tap_info)
/// Limits recursion depth to prevent infinite loops
fn count_rb_files(dir: &Path, depth: usize) -> usize {
    const MAX_DEPTH: usize = 10;
    if depth > MAX_DEPTH {
        return 0;
    }

    let mut count = 0;
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("rb") {
                count += 1;
            } else if path.is_dir() {
                count += count_rb_files(&path, depth + 1);
            }
        }
    }
    count
}

/// Create a new tap with standard directory structure
pub fn tap_new(tap_name: &str) -> Result<()> {
    // Validate tap name format (should be user/repo)
    let parts: Vec<&str> = tap_name.split('/').collect();
    if parts.len() != 2 {
        println!(
            "{} Invalid tap name. Format: {}",
            "".red(),
            "user/repo".cyan()
        );
        return Ok(());
    }

    let user = parts[0];
    let repo = parts[1];

    // Ensure repo name has "homebrew-" prefix
    let full_repo_name = if repo.starts_with("homebrew-") {
        String::from(repo)
    } else {
        format!("homebrew-{}", repo)
    };

    let tap_path = crate::tap::taps_path().join(user).join(&full_repo_name);

    if tap_path.exists() {
        println!(
            "{} Tap already exists: {}",
            "".yellow(),
            tap_path.display().to_string().cyan()
        );
        return Ok(());
    }

    println!("Creating new tap: {}", tap_name.cyan());

    // Create directory structure
    std::fs::create_dir_all(&tap_path)?;
    std::fs::create_dir_all(tap_path.join("Formula"))?;
    std::fs::create_dir_all(tap_path.join("Casks"))?;

    // Create README with usage instructions
    let readme_content = format!(
        "# {}/{}\n\nHomebrew tap for custom formulae and casks.\n\n## Usage\n\n```bash\nbrew tap {}\n```\n",
        user, full_repo_name, tap_name
    );
    std::fs::write(tap_path.join("README.md"), readme_content)?;

    // Initialize git repository
    let status = std::process::Command::new("git")
        .args(["init"])
        .current_dir(&tap_path)
        .status()?;

    if !status.success() {
        println!("  {} Failed to initialize git repository", "".yellow());
    }

    println!(
        "{} Tap created at: {}",
        "".green().bold(),
        tap_path.display().to_string().cyan()
    );
    println!(
        "\nAdd formulae to: {}",
        tap_path.join("Formula").display().to_string().dimmed()
    );
    println!(
        "Add casks to: {}",
        tap_path.join("Casks").display().to_string().dimmed()
    );

    Ok(())
}

/// Pin a tap to prevent it from being updated
pub fn tap_pin(tap_name: &str) -> anyhow::Result<()> {
    println!("Pinning tap: {}", tap_name.cyan());

    let tap_dir = crate::tap::tap_directory(tap_name)?;

    if !tap_dir.exists() {
        println!("{} Tap not found: {}", "".red(), tap_name);
        return Ok(());
    }

    let prefix = cellar::detect_prefix();
    let pinned_dir = prefix.join("Library/PinnedTaps");

    std::fs::create_dir_all(&pinned_dir)?;

    // Use normalized filename for pinned taps (replace / with --)
    let pin_file = pinned_dir.join(tap_name.replace('/', "--"));

    if pin_file.exists() {
        println!("Tap already pinned");
        return Ok(());
    }

    std::fs::write(&pin_file, "")?;

    println!(" {} Tap pinned: {}", "".green().bold(), tap_name.bold());
    println!(
        "  This tap will not be updated by {} or {}",
        "bru update".cyan(),
        "bru upgrade".cyan()
    );

    Ok(())
}

/// Unpin a tap to allow it to be updated again
pub fn tap_unpin(tap_name: &str) -> anyhow::Result<()> {
    println!("Unpinning tap: {}", tap_name.cyan());

    let prefix = cellar::detect_prefix();
    let pinned_dir = prefix.join("Library/PinnedTaps");
    let pin_file = pinned_dir.join(tap_name.replace('/', "--"));

    if !pin_file.exists() {
        println!("Tap is not pinned");
        return Ok(());
    }

    std::fs::remove_file(&pin_file)?;

    println!("\n {} Tap unpinned: {}", "".green().bold(), tap_name.bold());
    println!(
        "  This tap will now be updated by {} and {}",
        "bru update".cyan(),
        "bru upgrade".cyan()
    );

    Ok(())
}

/// Generate or display README for a tap
pub fn tap_readme(tap_name: &str) -> anyhow::Result<()> {
    println!("Generating README for tap: {}", tap_name.cyan());

    let tap_dir = crate::tap::tap_directory(tap_name)?;

    if !tap_dir.exists() {
        println!("{} Tap not found: {}", "".red(), tap_name);
        return Ok(());
    }

    let readme_path = tap_dir.join("README.md");

    if readme_path.exists() {
        println!(" README.md already exists");
        println!("  Location: {}", readme_path.display().to_string().dimmed());
    } else {
        println!(" Would generate README.md with:");
        println!("  - Tap name and description");
        println!("  - Installation instructions");
        println!("  - List of formulae/casks");
        println!("  - Contributing guidelines");
        println!("  Location: {}", readme_path.display().to_string().cyan());
    }

    Ok(())
}

/// Execute a custom tap command from the tap's cmd/ directory
pub fn tap_cmd(tap_name: &str, command: &str, args: &[String]) -> anyhow::Result<()> {
    println!(
        "{} Running tap command: {} {}",
        "⚙️".bold(),
        tap_name.cyan(),
        command.cyan()
    );

    if !args.is_empty() {
        println!("  Arguments: {}", args.join(" ").dimmed());
    }

    println!(" External tap commands");
    println!("  Taps can provide custom commands");
    println!("  These are scripts in the tap's cmd/ directory");

    let tap_dir = crate::tap::tap_directory(tap_name)?;
    let cmd_dir = tap_dir.join("cmd");

    if cmd_dir.exists() {
        println!("  {} Tap has cmd/ directory", "".green());
        println!("  Would execute: {}/{}", tap_name, command.cyan());
    } else {
        println!("  {} Tap has no cmd/ directory", "".yellow());
    }

    Ok(())
}
