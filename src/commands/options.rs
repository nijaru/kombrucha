use crate::api::BrewApi;
use crate::error::Result;
use colored::Colorize;

pub async fn options(api: &BrewApi, formula_name: &str) -> Result<()> {
    println!("Checking options for: {}", formula_name.cyan());

    // Verify formula exists
    match api.fetch_formula(formula_name).await {
        Ok(formula) => {
            println!("{}", format!("==> {}", formula.name).bold().green());
            if let Some(desc) = &formula.desc {
                println!("{}", desc);
            }
            println!();
            println!("No options available");
            println!(
                "{}",
                "Bottles are pre-built binaries with fixed options.".dimmed()
            );
            println!(
                "{}",
                "For custom builds with options, use `brew install --build-from-source`.".dimmed()
            );
        }
        Err(_) => {
            println!("{} Formula '{}' not found", "✗".red(), formula_name);
        }
    }

    Ok(())
}
