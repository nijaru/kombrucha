//! Maintenance commands for Homebrew package management
//!
//! This module provides commands for maintaining a healthy Homebrew installation:
//! - `autoremove`: Remove unused dependencies
//! - `cleanup`: Remove old package versions
//! - `cache`: Manage download cache
//! - `doctor`: Check system health
//! - `update`: Update tap repositories
//! - `update_reset`: Reset taps to origin
//! - `update_report`: Show recent tap changes
//! - `update_if_needed`: Conditionally update if stale

use crate::cellar;
use crate::download;
use crate::error::Result;
use crate::symlink;
use colored::Colorize;
use std::collections::{HashMap, HashSet, VecDeque};

/// Remove unused dependencies that were installed automatically
///
/// Performs a breadth-first traversal of the dependency graph to identify
/// packages that were installed as dependencies but are no longer needed.
pub fn autoremove(dry_run: bool) -> Result<()> {
    if dry_run {
        println!("Dry run - no packages will be removed");
    } else {
        println!("Removing unused dependencies...");
    }

    let all_packages = cellar::list_installed()?;

    // Build a set of all packages installed on request
    let on_request: HashSet<String> = all_packages
        .iter()
        .filter(|p| p.installed_on_request())
        .map(|p| p.name.clone())
        .collect();

    // Build a set of all dependencies required by packages installed on request
    // This uses a breadth-first traversal of the dependency graph from receipts
    let mut required = HashSet::new();
    let mut to_check: VecDeque<String> = on_request.iter().cloned().collect();
    let mut checked = HashSet::new();

    // Traverse dependency graph using receipts only (matches Homebrew behavior)
    // NO network calls - instant operation
    while let Some(name) = to_check.pop_front() {
        if !checked.insert(name.clone()) {
            continue; // Already processed
        }

        // Find package and add its runtime dependencies from receipt
        if let Some(pkg) = all_packages.iter().find(|p| p.name == name) {
            for dep in pkg.runtime_dependencies() {
                required.insert(dep.full_name.clone());
                to_check.push_back(dep.full_name.clone());
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
        println!("{} No unused dependencies to remove", "".green());
        return Ok(());
    }

    to_remove.sort_by(|a, b| a.name.cmp(&b.name));

    println!(
        "Found {} unused dependencies:",
        to_remove.len().to_string().bold()
    );

    for pkg in &to_remove {
        println!("  {} {}", pkg.name.cyan(), pkg.version.dimmed());
    }

    if dry_run {
        println!(
            "{} Would remove {} packages",
            "".dimmed(),
            to_remove.len().to_string().bold()
        );
        println!("Run without {} to remove them", "--dry-run".dimmed());
        return Ok(());
    }

    // Remove packages
    use indicatif::{ProgressBar, ProgressStyle};
    let total = to_remove.len();

    for (idx, pkg) in to_remove.iter().enumerate() {
        println!(
            "  Uninstalling {} {} [{}/{}]",
            pkg.name.cyan(),
            pkg.version.dimmed(),
            idx + 1,
            total
        );

        // Unlink symlinks
        let unlinked = symlink::unlink_formula(&pkg.name, &pkg.version)?;
        if !unlinked.is_empty() {
            println!(
                "    {} Unlinked {} files",
                "".green(),
                unlinked.len().to_string().dimmed()
            );
        }

        // Remove version-agnostic symlinks (opt/ and var/homebrew/linked/)
        symlink::unoptlink(&pkg.name)?;

        // Calculate size for progress indication
        let cellar_path = cellar::cellar_path().join(&pkg.name).join(&pkg.version);
        let size = if cellar_path.exists() {
            calculate_dir_size(&cellar_path).unwrap_or(0)
        } else {
            0
        };

        // Show spinner for large deletions (> 10 MB)
        let show_spinner = size > 10 * 1024 * 1024;
        let spinner = if show_spinner {
            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("    {spinner:.cyan} Removing files...")
                    .unwrap(),
            );
            pb.enable_steady_tick(std::time::Duration::from_millis(100));
            pb
        } else {
            ProgressBar::hidden()
        };

        // Remove from Cellar
        if cellar_path.exists() {
            std::fs::remove_dir_all(&cellar_path)?;
        }

        if show_spinner {
            spinner.finish_and_clear();
        }

        // Remove formula directory if empty
        let formula_dir = cellar::cellar_path().join(&pkg.name);
        if formula_dir.exists() && formula_dir.read_dir()?.next().is_none() {
            std::fs::remove_dir(&formula_dir)?;
        }

        println!("    {} Removed {}", "".green(), pkg.name.bold().green());
    }

    println!(
        "{} Removed {} unused packages",
        "".green().bold(),
        to_remove.len().to_string().bold()
    );

    Ok(())
}

/// Remove old versions of installed packages
///
/// Keeps the linked version and the newest version, removes everything else.
/// This matches Homebrew's cleanup behavior.
pub fn cleanup(formula_names: &[String], dry_run: bool, cask: bool) -> Result<()> {
    if cask {
        return super::cask::cleanup_cask(formula_names, dry_run);
    }

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
        println!("Dry run - no files will be removed");
    } else {
        println!("Cleaning up old versions...");
    }

    for formula in &to_clean {
        let versions = match by_formula.get(formula) {
            Some(v) => v,
            None => {
                if !formula_names.is_empty() {
                    println!("  {} {} not installed", "".yellow(), formula.bold());
                }
                continue;
            }
        };

        if versions.len() <= 1 {
            continue;
        }

        // Determine which versions to keep:
        // 1. Always keep the linked version (active installation)
        // 2. Keep the newest version (may be same as linked)
        // This matches Homebrew's behavior of preserving the installed version

        let linked_version = symlink::get_linked_version(formula).ok().flatten();

        // Sort by version to find the newest
        let mut sorted_versions = versions.to_vec();
        sorted_versions.sort_by(|a, b| {
            // Try to parse as semantic version numbers
            let a_parts: Vec<u32> = a
                .version
                .split('.')
                .filter_map(|s| s.parse::<u32>().ok())
                .collect();
            let b_parts: Vec<u32> = b
                .version
                .split('.')
                .filter_map(|s| s.parse::<u32>().ok())
                .collect();

            // Compare version parts numerically
            for i in 0..a_parts.len().max(b_parts.len()) {
                let a_part = a_parts.get(i).unwrap_or(&0);
                let b_part = b_parts.get(i).unwrap_or(&0);
                match a_part.cmp(b_part) {
                    std::cmp::Ordering::Equal => continue,
                    other => return other,
                }
            }

            // If numeric comparison fails, fall back to lexicographic
            a.version.cmp(&b.version)
        });
        sorted_versions.reverse(); // Highest version first

        let newest_version = sorted_versions[0];

        // Collect versions to keep
        let mut versions_to_keep = vec![newest_version];
        if let Some(ref linked_ver) = linked_version
            && let Some(linked_pkg) = sorted_versions.iter().find(|v| &v.version == linked_ver)
            && linked_pkg.version != newest_version.version
        {
            versions_to_keep.push(linked_pkg);
        }

        // Everything else can be removed
        let old_versions: Vec<_> = sorted_versions
            .iter()
            .filter(|v| {
                !versions_to_keep
                    .iter()
                    .any(|keep| keep.version == v.version)
            })
            .copied()
            .collect();

        // Skip if no old versions to remove
        if old_versions.is_empty() {
            continue;
        }

        // Show which versions we're keeping
        if dry_run {
            for keep in &versions_to_keep {
                let marker = if Some(&keep.version) == linked_version.as_ref() {
                    "(linked)"
                } else {
                    "(newest)"
                };
                println!(
                    "  Keeping {} {} {}",
                    keep.name.cyan().bold(),
                    keep.version.green(),
                    marker.dimmed()
                );
            }
        }

        // Collect metadata for all versions to remove
        let mut removal_tasks: Vec<(String, String, std::path::PathBuf, u64)> = Vec::new();

        for old in old_versions {
            let version_path = cellar::cellar_path().join(&old.name).join(&old.version);
            let size = calculate_dir_size(&version_path)?;
            total_space_freed += size;

            removal_tasks.push((old.name.clone(), old.version.clone(), version_path, size));
        }

        if dry_run {
            // Dry run: just show what would be removed
            for (name, version, _, size) in &removal_tasks {
                println!(
                    "  Would remove {} {} ({})",
                    name.cyan(),
                    version.dimmed(),
                    format_size(*size).dimmed()
                );
                total_removed += 1;
            }
        } else {
            // Phase 1: Unlink all symlinks sequentially (touches shared directories)
            for (name, version, _, _) in &removal_tasks {
                let unlinked = symlink::unlink_formula(name, version)?;
                if !unlinked.is_empty() {
                    println!(
                        "  {} Unlinked {} symlinks from {} {}",
                        "".green(),
                        unlinked.len().to_string().dimmed(),
                        name.cyan(),
                        version.dimmed()
                    );
                }
            }

            // Phase 2: Delete directories in parallel
            use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
            use std::sync::Arc;
            use std::sync::atomic::{AtomicUsize, Ordering};

            let multi_progress = MultiProgress::new();
            let completed = Arc::new(AtomicUsize::new(0));
            let total = removal_tasks.len();

            // Create deletion tasks
            let handles: Vec<_> = removal_tasks
                .into_iter()
                .map(|(name, version, version_path, size)| {
                    let multi_progress = multi_progress.clone();
                    let completed = Arc::clone(&completed);

                    std::thread::spawn(move || {
                        // Show spinner for deletions > 10 MB
                        let show_spinner = size > 10 * 1024 * 1024;
                        let spinner = if show_spinner {
                            let pb = multi_progress.add(ProgressBar::new_spinner());
                            pb.set_style(
                                ProgressStyle::default_spinner()
                                    .template(&format!(
                                        "  {{spinner:.cyan}} Removing {} {} ({})",
                                        name,
                                        version,
                                        format_size(size)
                                    ))
                                    .unwrap(),
                            );
                            pb.enable_steady_tick(std::time::Duration::from_millis(100));
                            pb
                        } else {
                            ProgressBar::hidden()
                        };

                        // Remove directory
                        let result = if version_path.exists() {
                            std::fs::remove_dir_all(&version_path)
                        } else {
                            Ok(())
                        };

                        if show_spinner {
                            spinner.finish_and_clear();
                        }

                        let count = completed.fetch_add(1, Ordering::Relaxed) + 1;
                        println!(
                            "  {} Removed {} {} ({}) [{}/{}]",
                            "".green(),
                            name.cyan(),
                            version.dimmed(),
                            format_size(size).dimmed(),
                            count,
                            total
                        );

                        result
                    })
                })
                .collect();

            // Wait for all deletions to complete
            for handle in handles {
                handle.join().unwrap()?;
            }

            total_removed = total;
        }
    }

    if total_removed == 0 {
        println!("{} No old versions to remove", "".green());
    } else if dry_run {
        println!(
            "{} Would remove {} old versions ({})",
            "".dimmed(),
            total_removed.to_string().bold(),
            format_size(total_space_freed).bold()
        );
    } else {
        println!(
            "{} Removed {} old versions, freed {}",
            "".green().bold(),
            total_removed.to_string().bold(),
            format_size(total_space_freed).bold()
        );
    }

    Ok(())
}

/// Manage the download cache (view or clean)
pub fn cache(clean: bool) -> Result<()> {
    let cache_dir = download::cache_dir();

    if clean {
        println!("Cleaning download cache...");

        if !cache_dir.exists() {
            println!("{} Cache is already empty", "".green());
            return Ok(());
        }

        // Calculate size before cleaning
        let total_size = calculate_dir_size(&cache_dir)?;

        // Remove all bottles from cache
        let mut removed_count = 0;
        for entry in std::fs::read_dir(&cache_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("gz") {
                std::fs::remove_file(&path)?;
                removed_count += 1;
            }
        }

        println!(
            "{} Removed {} bottles, freed {}",
            "".green().bold(),
            removed_count.to_string().bold(),
            format_size(total_size).bold()
        );
    } else {
        // Show cache info
        println!("{}", "==> Download Cache".bold().green());
        println!();

        println!(
            "{}: {}",
            "Location".bold(),
            cache_dir.display().to_string().cyan()
        );

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

            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("gz") {
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

/// Check system health and configuration
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
            "".red(),
            prefix.display()
        );
        issues += 1;
    } else {
        println!(
            "  {} Homebrew prefix exists: {}",
            "".green(),
            prefix.display()
        );
    }

    // Check if Cellar exists and is writable
    if !cellar.exists() {
        println!(
            "  {} Cellar does not exist: {}",
            "".yellow(),
            cellar.display()
        );
        warnings += 1;
    } else if std::fs::metadata(&cellar)?.permissions().readonly() {
        println!(
            "  {} Cellar is not writable: {}",
            "".red(),
            cellar.display()
        );
        issues += 1;
    } else {
        println!("  {} Cellar exists and is writable", "".green());
    }

    // Check if bin directory exists
    if !bin_dir.exists() {
        println!(
            "  {} Bin directory does not exist: {}",
            "".yellow(),
            bin_dir.display()
        );
        warnings += 1;
    } else {
        println!(
            "  {} Bin directory exists: {}",
            "".green(),
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
                "".green(),
                version.trim().dimmed()
            );
        }
        _ => {
            println!("  {} git is not installed or not in PATH", "".red());
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
        println!("  {} No broken symlinks found", "".green());
    } else {
        println!(
            "  {} Found {} broken symlinks:",
            "".yellow(),
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
        println!("  {} Your system is ready to brew!", "".green().bold());
    } else {
        if issues > 0 {
            println!(
                "  {} Found {} issue(s) that need attention",
                "".red(),
                issues
            );
        }
        if warnings > 0 {
            println!("  {} Found {} warning(s)", "".yellow(), warnings);
        }
    }

    Ok(())
}

/// Update all taps in parallel
pub fn update() -> Result<()> {
    // Clear cached formula/cask data to ensure fresh results
    println!("Refreshing formula and cask cache...");
    if let Err(e) = crate::cache::clear_caches() {
        println!("  {} Failed to clear cache: {}", "".yellow(), e);
    } else {
        println!("  {} Cache cleared", "".green());
    }

    let taps = crate::tap::list_taps()?;

    if taps.is_empty() {
        println!("No taps installed");
        return Ok(());
    }

    println!("Updating {} taps...", taps.len().to_string().bold());

    // Parallel tap updates with live progress
    use std::sync::mpsc;
    let (tx, rx) = mpsc::channel();

    let handles: Vec<_> = taps
        .iter()
        .map(|tap| {
            let tap = tap.clone();
            let tx = tx.clone();
            std::thread::spawn(move || {
                let tap_dir = match crate::tap::tap_directory(&tap) {
                    Ok(dir) => dir,
                    Err(_) => {
                        let _ = tx.send((tap.clone(), Err(String::from("invalid tap directory"))));
                        return;
                    }
                };

                if !tap_dir.exists() || !tap_dir.join(".git").exists() {
                    let _ = tx.send((tap.clone(), Err(String::from("not a git repository"))));
                    return;
                }

                let tap_dir_str = match tap_dir.to_str() {
                    Some(s) => s,
                    None => {
                        let _ = tx.send((tap.clone(), Err(String::from("invalid path"))));
                        return;
                    }
                };

                let output = std::process::Command::new("git")
                    .args(["-C", tap_dir_str, "pull", "--ff-only"])
                    .output();

                let result = match output {
                    Ok(output) if output.status.success() => {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        if stdout.contains("Already up to date")
                            || stdout.contains("Already up-to-date")
                        {
                            Ok("unchanged")
                        } else {
                            Ok("updated")
                        }
                    }
                    Ok(output) => {
                        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                        Err(stderr)
                    }
                    Err(e) => Err(e.to_string()),
                };

                let _ = tx.send((tap, result));
            })
        })
        .collect();

    drop(tx); // Close sender so receiver knows when done

    let mut updated = 0;
    let mut unchanged = 0;
    let mut errors = 0;

    // Display results as they complete
    for (tap, result) in rx {
        print!("  Updating {}... ", tap.cyan());

        match result {
            Ok("updated") => {
                println!("{}", "updated".green());
                updated += 1;
            }
            Ok("unchanged") => {
                println!("{}", "already up to date".dimmed());
                unchanged += 1;
            }
            Ok(_) => {
                println!("{}", "unknown status".yellow());
                errors += 1;
            }
            Err(msg) => {
                println!("{} {}", "failed".red(), msg.trim().to_string().dimmed());
                errors += 1;
            }
        }
    }

    // Wait for all threads
    for handle in handles {
        let _ = handle.join();
    }

    if errors == 0 {
        if updated > 0 {
            println!(
                "{} Updated {} taps, {} unchanged",
                "".green().bold(),
                updated.to_string().bold(),
                unchanged.to_string().dimmed()
            );
        } else {
            println!("{} All taps are up to date", "".green().bold());
        }
    } else {
        println!(
            "{} {} succeeded, {} failed",
            "".yellow(),
            (updated + unchanged).to_string().bold(),
            errors.to_string().bold()
        );
    }

    Ok(())
}

/// Reset a tap to origin/master or origin/main
pub fn update_reset(tap_name: Option<&str>) -> anyhow::Result<()> {
    let tap = tap_name.unwrap_or("homebrew/core");

    println!("Resetting tap: {}", tap.cyan());

    let tap_dir = if tap == "homebrew/core" {
        cellar::detect_prefix().join("Library/Taps/homebrew/homebrew-core")
    } else {
        crate::tap::tap_directory(tap)?
    };

    if !tap_dir.exists() {
        println!("{} Tap not found: {}", "".red(), tap);
        return Ok(());
    }

    let git_dir = tap_dir.join(".git");
    if !git_dir.exists() {
        println!("{} Not a git repository: {}", "".yellow(), tap);
        return Ok(());
    }

    println!("  Fetching latest changes...");

    let fetch_status = std::process::Command::new("git")
        .current_dir(&tap_dir)
        .args(["fetch", "origin"])
        .status()?;

    if !fetch_status.success() {
        println!("{} Failed to fetch", "".red());
        return Ok(());
    }

    println!("  Resetting to origin/master...");

    let reset_status = std::process::Command::new("git")
        .current_dir(&tap_dir)
        .args(["reset", "--hard", "origin/master"])
        .status()?;

    if !reset_status.success() {
        let reset_main_status = std::process::Command::new("git")
            .current_dir(&tap_dir)
            .args(["reset", "--hard", "origin/main"])
            .status()?;

        if !reset_main_status.success() {
            println!("{} Failed to reset", "".red());
            return Ok(());
        }
    }

    println!("{} Tap reset complete: {}", "".green().bold(), tap.bold());

    Ok(())
}

/// Show recent changes in homebrew/core tap
pub fn update_report() -> anyhow::Result<()> {
    println!("Generating update report...");

    let prefix = cellar::detect_prefix();
    let repository_path = prefix.join("Library/Taps/homebrew/homebrew-core");

    if !repository_path.exists() {
        println!("{} homebrew/core tap not found", "".red());
        return Ok(());
    }

    println!(" Checking git log for recent changes...");

    let output = std::process::Command::new("git")
        .current_dir(&repository_path)
        .args(["log", "--oneline", "--since=24.hours.ago"])
        .output()?;

    if output.status.success() {
        let log = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<&str> = log.lines().collect();

        if lines.is_empty() {
            println!(" No updates in the last 24 hours");
        } else {
            println!(
                "{} {} commits in the last 24 hours:",
                "".green(),
                lines.len().to_string().bold()
            );
            for line in lines.iter().take(10) {
                println!("  {}", line.dimmed());
            }
            if lines.len() > 10 {
                println!("  ... and {} more", (lines.len() - 10).to_string().dimmed());
            }
        }
    }

    Ok(())
}

/// Update taps if last update was more than 24 hours ago
pub fn update_if_needed() -> anyhow::Result<()> {
    println!("Checking if update is needed...");

    let prefix = cellar::detect_prefix();
    let repository_path = prefix.join("Library/Taps/homebrew/homebrew-core");

    if !repository_path.exists() {
        println!("{} homebrew/core tap not found", "".red());
        return Ok(());
    }

    println!(" Conditional update");
    println!("  Only updates if last update was more than 24 hours ago");

    let last_update_file = prefix.join(".homebrew_last_update");

    let needs_update = if last_update_file.exists() {
        if let Ok(metadata) = std::fs::metadata(&last_update_file) {
            if let Ok(modified) = metadata.modified() {
                if let Ok(elapsed) = modified.elapsed() {
                    elapsed.as_secs() > 86400
                } else {
                    true
                }
            } else {
                true
            }
        } else {
            true
        }
    } else {
        true
    };

    if needs_update {
        println!("  Update needed (>24 hours since last update)");
        println!("    Running: {}", "bru update".cyan());

        update()?;

        std::fs::write(&last_update_file, "")?;
    } else {
        println!("  {} Update not needed (updated recently)", "".green());
        println!("    Last update: within 24 hours");
    }

    Ok(())
}

// Helper functions

/// Calculate the total size of a directory recursively
fn calculate_dir_size(path: &std::path::Path) -> Result<u64> {
    let mut total = 0u64;

    if !path.exists() {
        return Ok(0);
    }

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
