use crate::cellar;
use crate::error::Result;
use colored::Colorize;

pub fn which_formula(command: &str) -> Result<()> {
    let prefix = cellar::detect_prefix();
    let bin_dir = prefix.join("bin");
    let command_path = bin_dir.join(command);

    if !command_path.exists() {
        println!(
            "{} Command '{}' not found in {}",
            "⚠".yellow(),
            command.bold(),
            bin_dir.display()
        );
        return Ok(());
    }

    // Check if it's a symlink
    if command_path.is_symlink()
        && let Ok(target) = std::fs::read_link(&command_path)
    {
        // Resolve to absolute path
        let resolved = if target.is_absolute() {
            target
        } else {
            bin_dir.join(&target).canonicalize().unwrap_or(target)
        };

        // Extract formula name from Cellar path
        let cellar_path = cellar::cellar_path();
        if resolved.starts_with(&cellar_path)
            && let Ok(rel_path) = resolved.strip_prefix(&cellar_path)
            && let Some(formula_name) = rel_path.components().next()
        {
            println!(
                "{}",
                formula_name.as_os_str().to_string_lossy().green().bold()
            );
            return Ok(());
        }
    }

    println!(
        "{} Could not determine formula for '{}'",
        "⚠".yellow(),
        command.bold()
    );
    Ok(())
}
