//! Cask operations for GUI application management
//!
//! This module provides commands for installing, upgrading, and managing macOS GUI applications
//! (casks) through Homebrew's cask infrastructure. Casks are applications like Chrome, VSCode,
//! Slack, etc. that are typically installed to /Applications.
//!
//! Key operations:
//! - **install_cask**: Download and install GUI applications from .dmg, .pkg, or .zip files
//! - **upgrade_cask**: Upgrade outdated casks to their latest versions
//! - **reinstall_cask**: Uninstall and reinstall casks (useful for fixing corrupted installations)
//! - **uninstall_cask**: Remove casks and clean up their files
//! - **cleanup_cask**: Remove old versions of casks to free up disk space
//! - **uses_cask**: Show information about cask dependencies (typically none)
//! - **abv_cask**: Display abbreviated cask information

use crate::api::BrewApi;
use crate::error::Result;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;

/// Install one or more casks from Homebrew
///
/// Downloads cask metadata in parallel, then sequentially downloads and installs each cask.
/// Supports .dmg, .pkg, and .zip file formats. Creates tracking metadata in Caskroom.
pub async fn install_cask(api: &BrewApi, cask_names: &[String]) -> Result<()> {
    if cask_names.is_empty() {
        println!("{} No casks specified", "".red());
        return Ok(());
    }

    println!(
        "Installing {} casks...",
        cask_names.len().to_string().bold()
    );

    // Fetch all cask metadata in parallel first
    let fetch_futures: Vec<_> = cask_names
        .iter()
        .map(|name| async move {
            // Check if already installed
            if crate::cask::is_cask_installed(name)
                && let Some(version) = crate::cask::get_installed_cask_version(name)
            {
                return (name.clone(), Err(format!("Already installed: {}", version)));
            }

            match api.fetch_cask(name).await {
                Ok(c) => (name.clone(), Ok(c)),
                Err(e) => (name.clone(), Err(format!("Failed to fetch: {}", e))),
            }
        })
        .collect();

    let metadata_results = futures::future::join_all(fetch_futures).await;

    // Process each cask sequentially (downloads and installs must be sequential)
    for (cask_name, result) in metadata_results {
        println!("Installing cask: {}", cask_name.cyan());

        let cask = match result {
            Ok(c) => c,
            Err(msg) => {
                if msg.starts_with("Already installed") {
                    println!("  {} {}", "".green(), msg);
                } else {
                    println!("  {} {}", "".red(), msg);
                }
                continue;
            }
        };

        let version = cask
            .version
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No version"))?;
        let url = cask
            .url
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No download URL"))?;

        println!("  {}: {}", "Version".dimmed(), version.cyan());
        println!("  {}: {}", "URL".dimmed(), url.dimmed());

        // Extract app artifacts from cask metadata
        let apps = crate::cask::extract_app_artifacts(&cask.artifacts);
        if apps.is_empty() {
            println!("  {} No app artifacts found", "".yellow());
            continue;
        }

        println!("  {}: {}", "Apps".dimmed(), apps.join(", ").cyan());

        // Download cask to cache directory
        println!("  Downloading...");
        let download_path = match crate::cask::download_cask(url, &cask_name).await {
            Ok(p) => p,
            Err(e) => {
                println!("  {} Failed to download: {}", "".red(), e);
                continue;
            }
        };

        println!(
            "    {} Downloaded to {}",
            "".green(),
            download_path.display().to_string().dimmed()
        );

        // Handle different file types - DMG, PKG, or ZIP
        let filename = download_path
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("Invalid download path: no filename"))?
            .to_string_lossy()
            .to_lowercase();

        if filename.ends_with(".dmg") {
            // Mount DMG and extract apps
            println!("  Mounting DMG...");
            let mount_point = match crate::cask::mount_dmg(&download_path) {
                Ok(p) => p,
                Err(e) => {
                    println!("  {} Failed to mount: {}", "".red(), e);
                    continue;
                }
            };

            println!(
                "    {} Mounted at {}",
                "".green(),
                mount_point.display().to_string().dimmed()
            );

            // Install each app from the mounted DMG
            for app_name in &apps {
                let app_path = mount_point.join(app_name);

                if !app_path.exists() {
                    println!("    {} App not found: {}", "".yellow(), app_name);
                    continue;
                }

                println!("  Installing {}...", app_name.cyan());
                match crate::cask::install_app(&app_path, app_name) {
                    Ok(_) => {
                        println!(
                            "    └ {} Installed to /Applications/{}",
                            "".green(),
                            app_name.bold()
                        );
                    }
                    Err(e) => {
                        println!("    └ {} Failed to install: {}", "".red(), e);
                    }
                }
            }

            // Unmount DMG after installation
            println!("  Unmounting DMG...");
            if let Err(e) = crate::cask::unmount_dmg(&mount_point) {
                println!("    {} Failed to unmount: {}", "".yellow(), e);
            }
        } else if filename.ends_with(".pkg") {
            // Install PKG directly using system installer
            println!("  Installing PKG...");
            match crate::cask::install_pkg(&download_path) {
                Ok(_) => {
                    println!("    └ {} Installed successfully", "".green());
                }
                Err(e) => {
                    println!("  {} Failed to install: {}", "".red(), e);
                    continue;
                }
            }
        } else if filename.ends_with(".zip") {
            // Extract ZIP and install apps
            println!("  Extracting ZIP...");
            let extract_dir = match crate::cask::extract_zip(&download_path) {
                Ok(dir) => {
                    println!(
                        "    └ {} Extracted to {}",
                        "".green(),
                        dir.display().to_string().dimmed()
                    );
                    dir
                }
                Err(e) => {
                    println!("  {} Failed to extract: {}", "".red(), e);
                    continue;
                }
            };

            // Install apps from extracted directory
            for app in &apps {
                println!("  Installing {}...", app.cyan());
                let app_path = extract_dir.join(app);

                if !app_path.exists() {
                    println!("    └ {} App not found in ZIP: {}", "".yellow(), app);
                    continue;
                }

                match crate::cask::install_app(&app_path, app) {
                    Ok(target) => {
                        println!(
                            "    └ {} Installed to {}",
                            "".green(),
                            target.display().to_string().bold()
                        );
                    }
                    Err(e) => {
                        println!("  {} Failed to install: {}", "".red(), e);
                        continue;
                    }
                }
            }
        } else {
            println!("  {} Unsupported file type: {}", "".yellow(), filename);
            continue;
        }

        // Create Caskroom directory to track installation
        let cask_dir = crate::cask::cask_install_dir(&cask_name, version);
        std::fs::create_dir_all(&cask_dir)?;

        // Write metadata file for future reference
        let metadata = serde_json::json!({
            "token": cask_name,
            "version": version,
            "installed_apps": apps,
            "install_time": chrono::Utc::now().timestamp(),
        });
        let metadata_path = cask_dir.join(".metadata.json");
        std::fs::write(&metadata_path, serde_json::to_string_pretty(&metadata)?)?;

        println!(
            "\n  {} Installed {} {}",
            "".green().bold(),
            cask_name.bold().green(),
            version.dimmed()
        );
    }

    println!("{} Cask installation complete", "".green().bold());
    Ok(())
}

/// Reinstall one or more casks (uninstall then install)
///
/// Useful for fixing corrupted installations or forcing a clean install.
pub async fn reinstall_cask(api: &BrewApi, cask_names: &[String]) -> Result<()> {
    if cask_names.is_empty() {
        println!("{} No casks specified", "".red());
        return Ok(());
    }

    println!(
        "Reinstalling {} casks...",
        cask_names.len().to_string().bold()
    );

    for cask_name in cask_names {
        // Check if installed before attempting reinstall
        if !crate::cask::is_cask_installed(cask_name) {
            println!("  {} {} not installed", "".yellow(), cask_name.bold());
            continue;
        }

        println!("  Reinstalling {}...", cask_name.cyan());

        // Uninstall the existing version
        uninstall_cask(std::slice::from_ref(cask_name))?;

        // Install fresh copy
        install_cask(api, std::slice::from_ref(cask_name)).await?;

        println!("  {} Reinstalled {}", "".green(), cask_name.bold().green());
    }

    println!("{} Cask reinstall complete", "".green().bold());
    Ok(())
}

/// Clean up old versions of casks to free disk space
///
/// Removes all but the most recent version of each cask. Can be run in dry-run mode
/// to preview what would be removed without actually deleting files.
pub fn cleanup_cask(cask_names: &[String], dry_run: bool) -> Result<()> {
    let caskroom = crate::cask::caskroom_dir();

    if !caskroom.exists() {
        println!("No casks installed");
        return Ok(());
    }

    // Get list of casks to clean (all or specified)
    let to_clean: Vec<String> = if cask_names.is_empty() {
        // Clean all installed casks - collect directory names
        std::fs::read_dir(&caskroom)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .filter_map(|e| e.file_name().to_str().map(String::from))
            .collect()
    } else {
        cask_names.to_vec()
    };

    let mut total_removed = 0;
    let mut total_space_freed = 0u64;

    if dry_run {
        println!("Dry run - no files will be removed");
    } else {
        println!("Cleaning up old cask versions...");
    }

    for token in &to_clean {
        let cask_dir = caskroom.join(token);

        if !cask_dir.exists() {
            if !cask_names.is_empty() {
                println!("  {} {} not installed", "".yellow(), token.bold());
            }
            continue;
        }

        // Get all version directories for this cask
        let mut versions: Vec<_> = std::fs::read_dir(&cask_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .collect();

        // Need at least 2 versions to have something to clean
        if versions.len() <= 1 {
            continue;
        }

        // Sort by modification time (newest first)
        versions.sort_by_key(|e| {
            e.metadata()
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
        });
        versions.reverse();

        let latest = &versions[0];
        let old_versions = &versions[1..];

        // Remove all old versions
        for old in old_versions {
            let version_path = old.path();
            let version_name = old.file_name();

            // Calculate directory size for reporting
            let size = calculate_dir_size(&version_path)?;
            total_space_freed += size;

            if dry_run {
                println!(
                    "  Would remove {} {} ({})",
                    token.cyan(),
                    version_name.to_string_lossy().dimmed(),
                    format_size(size).dimmed()
                );
            } else {
                println!(
                    "  Removing {} {} ({})",
                    token.cyan(),
                    version_name.to_string_lossy().dimmed(),
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
            "".green(),
            token.bold(),
            latest.file_name().to_string_lossy().dimmed()
        );
    }

    // Print summary
    if total_removed == 0 {
        println!("{} No old cask versions to remove", "".green());
    } else if dry_run {
        println!(
            "{} Would remove {} old cask versions ({})",
            "".dimmed(),
            total_removed.to_string().bold(),
            format_size(total_space_freed).bold()
        );
    } else {
        println!(
            "{} Removed {} old cask versions, freed {}",
            "".green().bold(),
            total_removed.to_string().bold(),
            format_size(total_space_freed).bold()
        );
    }

    Ok(())
}

/// Upgrade outdated casks to their latest versions
///
/// If no casks are specified, checks all installed casks for updates and upgrades
/// any that are outdated. Otherwise upgrades only the specified casks.
pub async fn upgrade_cask(api: &BrewApi, cask_names: &[String]) -> Result<()> {
    // Determine which casks to upgrade
    let to_upgrade = if cask_names.is_empty() {
        // Upgrade all outdated casks - check in parallel
        let is_tty = std::io::IsTerminal::is_terminal(&std::io::stdout());

        let spinner = if is_tty {
            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.cyan} {msg}")
                    .unwrap(),
            );
            pb.set_message("Checking for outdated casks...");
            pb.enable_steady_tick(std::time::Duration::from_millis(100));
            pb
        } else {
            ProgressBar::hidden()
        };

        let installed_casks = crate::cask::list_installed_casks()?;

        // Fetch all cask metadata in parallel to check for updates
        let fetch_futures: Vec<_> = installed_casks
            .iter()
            .map(|(token, installed_version)| {
                let token = token.clone();
                let installed_version = installed_version.clone();
                async move {
                    // Check if a newer version is available
                    if let Ok(cask) = api.fetch_cask(&token).await
                        && let Some(latest) = &cask.version
                        && latest != &installed_version
                    {
                        return Some(token);
                    }
                    None
                }
            })
            .collect();

        let results = futures::future::join_all(fetch_futures).await;
        spinner.finish_and_clear();

        let outdated: Vec<_> = results.into_iter().flatten().collect();

        if outdated.is_empty() {
            println!("{} All casks are up to date", "".green());
            return Ok(());
        }

        println!(
            "{} casks to upgrade: {}",
            outdated.len().to_string().bold(),
            outdated.join(", ").cyan()
        );
        outdated
    } else {
        cask_names.to_vec()
    };

    println!("Upgrading {} casks...", to_upgrade.len());

    // Upgrade each cask sequentially
    for cask_name in &to_upgrade {
        println!("  Upgrading {}...", cask_name.cyan());

        // First uninstall the old version
        uninstall_cask(std::slice::from_ref(cask_name))?;

        // Then install the new version
        install_cask(api, std::slice::from_ref(cask_name)).await?;

        println!("  {} Upgraded {}", "".green(), cask_name.bold().green());
    }

    println!("{} Cask upgrade complete", "".green().bold());
    Ok(())
}

/// Uninstall one or more casks
///
/// Removes the application from /Applications and cleans up tracking metadata
/// from the Caskroom directory.
pub fn uninstall_cask(cask_names: &[String]) -> Result<()> {
    if cask_names.is_empty() {
        println!("{} No casks specified", "".red());
        return Ok(());
    }

    println!(
        "Uninstalling {} casks...",
        cask_names.len().to_string().bold()
    );

    for cask_name in cask_names {
        println!("Uninstalling cask: {}", cask_name.cyan());

        // Check if installed
        if !crate::cask::is_cask_installed(cask_name) {
            println!("  {} {} is not installed", "".yellow(), cask_name.bold());
            continue;
        }

        let version = crate::cask::get_installed_cask_version(cask_name)
            .ok_or_else(|| anyhow::anyhow!("Could not determine version"))?;

        // Read metadata to find installed apps
        let cask_dir = crate::cask::cask_install_dir(cask_name, &version);
        let metadata_path = cask_dir.join(".metadata.json");

        let apps = if metadata_path.exists() {
            let metadata_str = std::fs::read_to_string(&metadata_path)?;
            let metadata: serde_json::Value = serde_json::from_str(&metadata_str)?;

            if let Some(apps_array) = metadata.get("installed_apps").and_then(|v| v.as_array()) {
                apps_array
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            } else {
                Vec::new()
            }
        } else {
            // Fallback: guess app name from cask name (capitalize first letter)
            let mut chars = cask_name.chars();
            if let Some(first_char) = chars.next() {
                vec![format!(
                    "{}.app",
                    first_char.to_uppercase().to_string() + chars.as_str()
                )]
            } else {
                vec![]
            }
        };

        // Remove apps from /Applications
        for app_name in &apps {
            let app_path = PathBuf::from("/Applications").join(app_name);

            if app_path.exists() {
                println!("  Removing {}...", app_name.cyan());

                match std::fs::remove_dir_all(&app_path) {
                    Ok(_) => {
                        println!("    {} Removed {}", "".green(), app_name.bold());
                    }
                    Err(e) => {
                        println!("    {} Failed to remove: {}", "".red(), e);
                        println!(
                            "    Try: {}",
                            format!("sudo rm -rf {}", app_path.display()).cyan()
                        );
                    }
                }
            } else {
                println!("  {} App not found: {}", "".yellow(), app_name.dimmed());
            }
        }

        // Remove Caskroom directory
        let caskroom_path = crate::cask::caskroom_dir().join(cask_name);
        if caskroom_path.exists() {
            std::fs::remove_dir_all(&caskroom_path)?;
        }

        println!(
            "\n  {} Uninstalled {} {}",
            "".green().bold(),
            cask_name.bold().green(),
            version.dimmed()
        );
    }

    println!("{} Cask uninstallation complete", "".green().bold());
    Ok(())
}

/// Show what depends on a cask (typically nothing)
///
/// Unlike formulae which can have dependents, casks are GUI applications that
/// typically don't have other packages depending on them.
pub fn uses_cask(cask_name: &str) -> anyhow::Result<()> {
    println!("Checking what uses cask: {}", cask_name.cyan());

    println!(" Cask usage analysis");
    println!("  Unlike formulae, casks typically don't have dependents");
    println!("  Casks are GUI applications, not libraries");

    println!("  {} Casks are usually standalone", "".dimmed());
    println!(
        "  {} Safe to uninstall without affecting other software",
        "".green()
    );

    Ok(())
}

/// Display abbreviated information about a cask
///
/// Shows basic metadata like name, version, homepage, and description.
pub async fn abv_cask(api: &BrewApi, cask_name: &str) -> anyhow::Result<()> {
    println!("Cask info: {}", cask_name.cyan());

    let cask = api.fetch_cask(cask_name).await?;

    println!(" {}", cask.token.bold());
    if let Some(name) = &cask.name.first() {
        println!("{}", name.dimmed());
    }

    if let Some(version) = &cask.version {
        println!("Version: {}", version.cyan());
    }

    if let Some(homepage) = &cask.homepage {
        println!("Homepage: {}", homepage.dimmed());
    }

    if let Some(desc) = &cask.desc {
        println!(" {}", desc.dimmed());
    }

    Ok(())
}

// Helper functions

/// Calculate the total size of a directory recursively
///
/// Uses walkdir to traverse the directory tree and sum up all file sizes.
/// Returns 0 if the directory doesn't exist.
fn calculate_dir_size(path: &std::path::Path) -> Result<u64> {
    let mut total = 0u64;

    if !path.exists() {
        return Ok(0);
    }

    // Use walkdir with limits to prevent resource exhaustion
    for entry in walkdir::WalkDir::new(path).follow_links(false).max_open(64) {
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

/// Format byte size as human-readable string
///
/// Converts bytes to KB, MB, or GB as appropriate.
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
