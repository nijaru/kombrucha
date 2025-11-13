use crate::cellar;
use crate::error::Result;
use colored::Colorize;

pub fn log(formula_name: &str) -> Result<()> {
    println!("Checking logs for {}", formula_name.cyan());
    println!();

    // Check if formula is installed
    let installed_versions = cellar::get_installed_versions(formula_name)?;
    if installed_versions.is_empty() {
        println!("{} {} is not installed", "⚠".yellow(), formula_name.bold());
        println!(
            "Run {} to install it",
            format!("bru install {}", formula_name).cyan()
        );
        return Ok(());
    }

    let version = &installed_versions[0].version;
    let install_path = cellar::cellar_path().join(formula_name).join(version);

    println!(
        "{}",
        format!("==> {} {}", formula_name, version).bold().green()
    );
    println!();

    // Check for INSTALL_RECEIPT.json
    let receipt_path = install_path.join("INSTALL_RECEIPT.json");
    if receipt_path.exists() {
        println!("{}", "Install Receipt:".bold());
        let receipt_content = std::fs::read_to_string(&receipt_path)?;
        let receipt: serde_json::Value = serde_json::from_str(&receipt_content)?;

        if let Some(obj) = receipt.as_object() {
            let on_request = obj
                .get("installed_on_request")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let status_text = if on_request { "Yes" } else { "No (dependency)" };
            println!(
                "  {}: {}",
                "Installed on request".dimmed(),
                status_text.cyan()
            );

            if let Some(time) = obj.get("time").and_then(|v| v.as_i64()) {
                let datetime = chrono::DateTime::from_timestamp(time, 0)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_else(|| "Unknown".to_string());
                println!("  {}: {}", "Install time".dimmed(), datetime);
            }

            if let Some(built_from) = obj
                .get("source")
                .and_then(|v| v.get("spec"))
                .and_then(|v| v.as_str())
            {
                println!("  {}: {}", "Built from".dimmed(), built_from);
            }
        }
        println!();
    }

    // List installed files
    println!("{}", "Installed files:".bold());
    let prefix = cellar::detect_prefix();
    let bin_dir = prefix.join("bin");

    let mut file_count = 0;
    if let Ok(entries) = std::fs::read_dir(&bin_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_symlink()
                && let Ok(target) = std::fs::read_link(&path)
            {
                // Resolve relative symlinks to absolute paths
                let resolved_target = if target.is_absolute() {
                    target.clone()
                } else {
                    bin_dir
                        .join(&target)
                        .canonicalize()
                        .unwrap_or(target.clone())
                };

                if resolved_target.starts_with(&install_path)
                    && let Some(name) = path.file_name()
                {
                    println!(
                        "  {} {}",
                        name.to_string_lossy().cyan(),
                        format!("-> {}", target.display()).dimmed()
                    );
                    file_count += 1;
                    if file_count >= 10 {
                        println!("  {} (showing first 10)", "...".dimmed());
                        break;
                    }
                }
            }
        }
    }

    if file_count == 0 {
        println!("  {}", "No executables linked".dimmed());
    }

    println!();
    println!(
        "{}: {}",
        "Install directory".dimmed(),
        install_path.display().to_string().cyan()
    );

    Ok(())
}
