use crate::error::Result;
use colored::Colorize;

pub fn services(action: Option<&str>, formula: Option<&str>) -> Result<()> {
    match action {
        None | Some("list") => {
            // List all services
            println!("{}", "==> Services".bold().green());
            println!();

            let services = crate::services::list_all_services()?;

            if services.is_empty() {
                println!("No services found");
                println!("Services are background processes like databases and web servers.");
                println!("Common services: postgresql, mysql, redis, nginx");
                return Ok(());
            }

            // Print header
            println!(
                "{:<20} {:<12} {:<8} {}",
                "Name".bold(),
                "Status".bold(),
                "User".bold(),
                "File".bold()
            );

            // Print services
            for service in &services {
                let status_str = match &service.status {
                    crate::services::ServiceStatus::None => "none".dimmed().to_string(),
                    crate::services::ServiceStatus::Started => "started".green().to_string(),
                    crate::services::ServiceStatus::Error(code) => {
                        format!("error  {}", code).red().to_string()
                    }
                };

                let user_str = service.user.as_deref().unwrap_or("");
                let file_str = service
                    .plist_path
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default();

                println!(
                    "{:<20} {:<20} {:<8} {}",
                    service.name.cyan(),
                    status_str,
                    user_str,
                    file_str.dimmed()
                );
            }

            println!();
            println!(
                "{} {} services",
                "ℹ".blue(),
                services.len().to_string().bold()
            );
        }
        Some("start") => {
            let formula = formula.ok_or_else(|| anyhow::anyhow!("Formula name required"))?;
            println!("Starting service: {}", formula.cyan());

            if !crate::services::service_exists(formula) {
                println!(
                    "{} Service file not found for {}",
                    "⚠".yellow(),
                    formula.bold()
                );
                println!("To create a service, the formula must support it.");
                println!(
                    "Run {} to check if service is available",
                    "bru services list".to_string().cyan()
                );
                return Ok(());
            }

            match crate::services::start_service(formula) {
                Ok(_) => {
                    println!("  {} Started {}", "✓".green(), formula.bold().green());
                }
                Err(e) => {
                    println!("  {} Failed to start: {}", "✗".red(), e);
                }
            }
        }
        Some("stop") => {
            let formula = formula.ok_or_else(|| anyhow::anyhow!("Formula name required"))?;
            println!("Stopping service: {}", formula.cyan());

            if !crate::services::service_exists(formula) {
                println!(
                    "{} Service file not found for {}",
                    "⚠".yellow(),
                    formula.bold()
                );
                return Ok(());
            }

            match crate::services::stop_service(formula) {
                Ok(_) => {
                    println!("  {} Stopped {}", "✓".green(), formula.bold().green());
                }
                Err(e) => {
                    println!("  {} Failed to stop: {}", "✗".red(), e);
                }
            }
        }
        Some("restart") => {
            let formula = formula.ok_or_else(|| anyhow::anyhow!("Formula name required"))?;
            println!("Restarting service: {}", formula.cyan());

            if !crate::services::service_exists(formula) {
                println!(
                    "{} Service file not found for {}",
                    "⚠".yellow(),
                    formula.bold()
                );
                return Ok(());
            }

            match crate::services::restart_service(formula) {
                Ok(_) => {
                    println!("  {} Restarted {}", "✓".green(), formula.bold().green());
                }
                Err(e) => {
                    println!("  {} Failed to restart: {}", "✗".red(), e);
                }
            }
        }
        Some(other) => {
            println!("{} Unknown action: {}", "✗".red(), other);
            println!("Available actions:");
            println!("  {} - List all services", "list".cyan());
            println!("  {} - Start a service", "start <formula>".cyan());
            println!("  {} - Stop a service", "stop <formula>".cyan());
            println!("  {} - Restart a service", "restart <formula>".cyan());
        }
    }

    Ok(())
}
