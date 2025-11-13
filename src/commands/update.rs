use crate::cache;
use crate::error::Result;
use crate::tap;
use colored::Colorize;

pub fn update() -> Result<()> {
    // Clear cached formula/cask data to ensure fresh results
    println!("Refreshing formula and cask cache...");
    if let Err(e) = crate::cache::clear_caches() {
        println!("  {} Failed to clear cache: {}", "⚠".yellow(), e);
    } else {
        println!("  {} Cache cleared", "✓".green());
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
                        let _ = tx.send((tap.clone(), Err("invalid tap directory".to_string())));
                        return;
                    }
                };

                if !tap_dir.exists() || !tap_dir.join(".git").exists() {
                    let _ = tx.send((tap.clone(), Err("not a git repository".to_string())));
                    return;
                }

                let tap_dir_str = match tap_dir.to_str() {
                    Some(s) => s,
                    None => {
                        let _ = tx.send((tap.clone(), Err("invalid path".to_string())));
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
                "✓".green().bold(),
                updated.to_string().bold(),
                unchanged.to_string().dimmed()
            );
        } else {
            println!("{} All taps are up to date", "✓".green().bold());
        }
    } else {
        println!(
            "{} {} succeeded, {} failed",
            "⚠".yellow(),
            (updated + unchanged).to_string().bold(),
            errors.to_string().bold()
        );
    }

    Ok(())
}
