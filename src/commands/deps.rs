use crate::api::BrewApi;
use crate::cellar;
use crate::error::Result;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashSet;

pub async fn deps(
    api: &BrewApi,
    formula: &str,
    tree: bool,
    installed_only: bool,
    direct: bool,
) -> Result<()> {
    let is_tty = std::io::IsTerminal::is_terminal(&std::io::stdout());

    let installed_names: HashSet<String> = if installed_only {
        cellar::list_installed()?
            .into_iter()
            .map(|p| p.name)
            .collect()
    } else {
        HashSet::new()
    };

    if direct {
        let spinner = if is_tty {
            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.cyan} {msg}")
                    .unwrap(),
            );
            pb.set_message(format!("Fetching dependencies for {}...", formula));
            pb.enable_steady_tick(std::time::Duration::from_millis(100));
            pb
        } else {
            ProgressBar::hidden()
        };

        let formula_data = api.fetch_formula(formula).await?;
        spinner.finish_and_clear();

        if formula_data.dependencies.is_empty() && formula_data.build_dependencies.is_empty() {
            if is_tty {
                println!("{} No dependencies", "✓".green());
            }
            return Ok(());
        }

        if !formula_data.dependencies.is_empty() {
            let mut deps: Vec<_> = formula_data.dependencies.iter().collect();

            if installed_only {
                deps.retain(|dep| installed_names.contains(*dep));
            }

            if !deps.is_empty() {
                if is_tty {
                    println!("{}", "Runtime dependencies:".bold().green());
                }
                let len = deps.len();
                for (i, dep) in deps.iter().enumerate() {
                    if is_tty {
                        if tree {
                            let prefix = if i == len - 1 { "└─" } else { "├─" };
                            println!("  {} {}", prefix, dep.cyan());
                        } else {
                            println!("  {}", dep.cyan());
                        }
                    } else {
                        println!("{}", dep);
                    }
                }
            } else if installed_only && is_tty {
                println!("No runtime dependencies installed");
            }
        }
    } else {
        unsafe {
            std::env::set_var("BRU_QUIET", "1");
        }
        let (_all_formulae, dep_order) =
            crate::commands::resolve_dependencies(api, &[formula.to_string()]).await?;
        unsafe {
            std::env::remove_var("BRU_QUIET");
        }

        let mut deps: Vec<_> = dep_order
            .into_iter()
            .filter(|name| name != formula)
            .collect();

        if deps.is_empty() {
            if is_tty {
                println!("{} No dependencies", "✓".green());
            }
            return Ok(());
        }

        if installed_only {
            deps.retain(|dep| installed_names.contains(dep));
        }

        if deps.is_empty() && installed_only {
            if is_tty {
                println!("No dependencies installed");
            }
            return Ok(());
        }

        let len = deps.len();
        for (i, dep) in deps.iter().enumerate() {
            if is_tty {
                if tree {
                    let prefix = if i == len - 1 { "└─" } else { "├─" };
                    println!("{} {}", prefix, dep.cyan());
                } else {
                    println!("{}", dep.cyan());
                }
            } else {
                println!("{}", dep);
            }
        }
    }

    Ok(())
}
