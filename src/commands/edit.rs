use crate::api::BrewApi;
use crate::cellar;
use crate::error::Result;
use colored::Colorize;

pub async fn edit(api: &BrewApi, formula_name: &str) -> Result<()> {
    println!(
        "{} Opening {} in editor...",
        "✏️".bold(),
        formula_name.cyan()
    );

    // First, verify formula exists
    match api.fetch_formula(formula_name).await {
        Ok(_) => {}
        Err(_) => {
            println!("{} Formula '{}' not found", "✗".red(), formula_name);
            return Ok(());
        }
    }

    // Try to find formula file in taps
    let prefix = cellar::detect_prefix();
    let taps_dir = prefix.join("Library/Taps");

    // Check homebrew-core first (try both flat and letter-organized structure)
    let first_letter = formula_name
        .chars()
        .next()
        .unwrap_or('a')
        .to_lowercase()
        .to_string();
    let core_formula_letter = taps_dir
        .join("homebrew/homebrew-core/Formula")
        .join(&first_letter)
        .join(format!("{}.rb", formula_name));
    let core_formula_flat = taps_dir
        .join("homebrew/homebrew-core/Formula")
        .join(format!("{}.rb", formula_name));

    let formula_path = if core_formula_letter.exists() {
        core_formula_letter
    } else if core_formula_flat.exists() {
        core_formula_flat
    } else {
        // Search all taps
        let mut found_path = None;
        if taps_dir.exists() {
            for tap_entry in std::fs::read_dir(&taps_dir)?.flatten() {
                let tap_path = tap_entry.path();
                if tap_path.is_dir() {
                    for repo_entry in std::fs::read_dir(&tap_path)?.flatten() {
                        let repo_path = repo_entry.path();
                        let formula_path = repo_path
                            .join("Formula")
                            .join(format!("{}.rb", formula_name));
                        if formula_path.exists() {
                            found_path = Some(formula_path);
                            break;
                        }
                    }
                }
                if found_path.is_some() {
                    break;
                }
            }
        }

        match found_path {
            Some(p) => p,
            None => {
                println!("{} Formula file not found locally", "⚠".yellow());
                println!("Formula exists in API but not in local taps");
                println!("Try: {}", "brew tap homebrew/core".to_string().cyan());
                return Ok(());
            }
        }
    };

    println!(
        "  {}: {}",
        "File".dimmed(),
        formula_path.display().to_string().cyan()
    );

    // Get editor from environment
    let editor = std::env::var("EDITOR")
        .or_else(|_| std::env::var("VISUAL"))
        .unwrap_or_else(|_| "vim".to_string());

    // Open in editor
    let status = std::process::Command::new(&editor)
        .arg(&formula_path)
        .status();

    match status {
        Ok(s) if s.success() => {
            println!("{} Finished editing {}", "✓".green(), formula_name.bold());
        }
        Ok(_) => {
            println!("{} Editor exited with error", "⚠".yellow());
        }
        Err(e) => {
            println!("{} Failed to open editor: {}", "✗".red(), e);
            println!("Set EDITOR environment variable to your preferred editor");
        }
    }

    Ok(())
}
