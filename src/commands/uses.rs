use crate::api::BrewApi;
use crate::error::Result;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};

pub async fn uses(api: &BrewApi, formula: &str) -> Result<()> {
    let is_tty = std::io::IsTerminal::is_terminal(&std::io::stdout());

    let spinner = if is_tty {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} {msg}")
                .unwrap(),
        );
        pb.set_message(format!("Finding formulae that depend on {}...", formula));
        pb.enable_steady_tick(std::time::Duration::from_millis(100));
        pb
    } else {
        ProgressBar::hidden()
    };

    let all_formulae = api.fetch_all_formulae().await?;
    spinner.finish_and_clear();

    let dependent_formulae: Vec<_> = all_formulae
        .into_iter()
        .filter(|f| {
            f.dependencies.contains(&formula.to_string())
                || f.build_dependencies.contains(&formula.to_string())
        })
        .collect();

    if dependent_formulae.is_empty() {
        if is_tty {
            println!("{} No formulae depend on '{}'", "✓".green(), formula);
        }
        return Ok(());
    }

    if is_tty {
        println!(
            "{} Found {} formulae that depend on {}:",
            "✓".green(),
            dependent_formulae.len().to_string().bold(),
            formula.cyan()
        );
    }

    for f in dependent_formulae {
        if is_tty {
            print!("{}", f.name.bold());
            if let Some(desc) = &f.desc
                && !desc.is_empty()
            {
                print!(" {}", format!("({})", desc).dimmed());
            }
            println!();
        } else {
            println!("{}", f.name);
        }
    }

    Ok(())
}
