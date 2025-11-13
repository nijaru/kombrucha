use anyhow::Result;
use clap::Parser;
use colored::Colorize;

use crate::api::BrewApi;

/// Show formula descriptions
#[derive(Parser)]
pub struct DescCommand {
    /// Formula names
    #[arg(required = true)]
    formula_names: Vec<String>,
}

pub async fn desc(api: &BrewApi, formula_names: &[String]) -> Result<()> {
    if formula_names.is_empty() {
        println!("{} No formulae specified", "✗".red());
        return Ok(());
    }

    for formula_name in formula_names {
        match api.fetch_formula(formula_name).await {
            Ok(formula) => {
                print!("{}", formula.name.bold().cyan());
                if let Some(desc) = &formula.desc
                    && !desc.is_empty()
                {
                    println!(": {}", desc);
                } else {
                    println!(": {}", "No description available".dimmed());
                }
            }
            Err(_) => {
                println!("{}: {}", formula_name.bold().yellow(), "Not found".dimmed());
            }
        }
    }

    Ok(())
}
