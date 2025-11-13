use crate::error::Result;
use colored::Colorize;

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
    println!("Untapping {}...", tap_name.cyan());

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

pub fn tap_info(tap_name: &str) -> Result<()> {
    println!(
        "{} Tap information for {}",
        "ℹ".bold(),
        tap_name.cyan().bold()
    );
    println!();

    if !crate::tap::is_tapped(tap_name)? {
        println!(
            "  {} Tap {} is not installed",
            "⚠".yellow(),
            tap_name.bold()
        );
        return Ok(());
    }

    let tap_dir = crate::tap::tap_directory(tap_name)?;

    println!("{}", "Location:".bold());
    println!("  {}", tap_dir.display().to_string().cyan());
    println!();

    // Count formulae in the tap (recursively for letter-organized directories)
    let formula_dir = tap_dir.join("Formula");
    let mut formula_count = 0;

    if formula_dir.exists() {
        fn count_rb_files(dir: &std::path::Path, depth: usize) -> usize {
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
        formula_count = count_rb_files(&formula_dir, 0);
    }

    // Count casks in the tap
    let casks_dir = tap_dir.join("Casks");
    let mut cask_count = 0;

    if casks_dir.exists() {
        fn count_rb_files(dir: &std::path::Path, depth: usize) -> usize {
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
        cask_count = count_rb_files(&casks_dir, 0);
    }

    println!(
        "{}: {}",
        "Formulae".bold(),
        formula_count.to_string().cyan()
    );
    println!("{}: {}", "Casks".bold(), cask_count.to_string().cyan());

    Ok(())
}
