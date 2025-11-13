use crate::download;
use crate::error::Result;
use colored::Colorize;

pub fn cache(clean: bool) -> Result<()> {
    let cache_dir = download::cache_dir();

    if clean {
        println!("Cleaning download cache...");

        if !cache_dir.exists() {
            println!("{} Cache is already empty", "✓".green());
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
            "✓".green().bold(),
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
