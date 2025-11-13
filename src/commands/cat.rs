use crate::api::BrewApi;
use crate::error::Result;
use colored::Colorize;

pub async fn cat(api: &BrewApi, formula_names: &[String]) -> Result<()> {
    if formula_names.is_empty() {
        println!("{} No formulae specified", "✗".red());
        return Ok(());
    }

    for (i, formula_name) in formula_names.iter().enumerate() {
        if i > 0 {
            println!(); // Blank line between formulae
        }

        println!("{} {}", "==>".bold().green(), formula_name.bold().cyan());
        println!();

        // Try to fetch formula from API
        match api.fetch_formula(formula_name).await {
            Ok(formula) => {
                // Print formula as JSON (since we don't have Ruby source)
                let json = serde_json::to_string_pretty(&formula)?;
                println!("{}", json);
            }
            Err(_) => {
                // Try as cask
                match api.fetch_cask(formula_name).await {
                    Ok(cask) => {
                        let json = serde_json::to_string_pretty(&cask)?;
                        println!("{}", json);
                    }
                    Err(_) => {
                        println!(
                            "{} No formula or cask found for '{}'",
                            "✗".red(),
                            formula_name
                        );
                    }
                }
            }
        }
    }

    Ok(())
}
