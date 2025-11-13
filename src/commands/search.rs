use crate::api::BrewApi;
use crate::error::Result;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};

pub async fn search(api: &BrewApi, query: &str, formula_only: bool, cask_only: bool) -> Result<()> {
    let is_tty = std::io::IsTerminal::is_terminal(&std::io::stdout());

    let spinner = if is_tty {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} {msg}")
                .unwrap(),
        );
        pb.set_message(format!("Searching for '{}'...", query));
        pb.enable_steady_tick(std::time::Duration::from_millis(100));
        pb
    } else {
        ProgressBar::hidden()
    };

    let results = api.search(query).await?;
    spinner.finish_and_clear();

    if results.is_empty() {
        if is_tty {
            println!(
                "{} No formulae or casks found matching '{}'",
                "✗".red(),
                query
            );
        }
        return Ok(());
    }

    let show_formulae = !cask_only;
    let show_casks = !formula_only;

    let total_to_show = (if show_formulae {
        results.formulae.len()
    } else {
        0
    }) + (if show_casks { results.casks.len() } else { 0 });

    if total_to_show == 0 {
        if is_tty {
            println!("{} No results found with the specified filter", "✗".red());
        }
        return Ok(());
    }

    if show_formulae && !results.formulae.is_empty() {
        if is_tty {
            println!("{}", "==> Formulae".bold().green());
        }

        for formula in &results.formulae {
            if is_tty {
                println!("{}", formula.name.bold().green());
            } else {
                println!("{}", formula.name);
            }
        }

        if is_tty && !results.casks.is_empty() {
            println!();
        }
    }

    if show_casks && !results.casks.is_empty() {
        if is_tty {
            println!("{}", "==> Casks".bold().cyan());
        }

        for cask in &results.casks {
            if is_tty {
                println!("{}", cask.token.bold().cyan());
            } else {
                println!("{}", cask.token);
            }
        }
    }

    Ok(())
}
