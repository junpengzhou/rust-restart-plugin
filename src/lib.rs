pub mod cli;
pub mod command;
pub mod config;
pub mod docker;
pub mod error;
pub mod health;
pub mod logs;
pub mod notify;
pub mod output;

use crate::command::SystemCommandRunner;
use crate::config::AppConfig;
use crate::error::AppResult;
use crate::health::StartupStatus;
use std::io;

pub fn run(module_name: &str, config: AppConfig) -> AppResult<()> {
    let runner = SystemCommandRunner;
    run_with_runner(&runner, module_name, config)
}

fn run_with_runner(
    runner: &dyn crate::command::CommandRunner,
    module_name: &str,
    config: AppConfig,
) -> AppResult<()> {
    let mut stdout = io::stdout();
    docker::print_matching_containers(runner, module_name, &mut stdout)?;
    docker::stop_container(runner, module_name, &mut stdout)?;

    if !docker::start_container(runner, module_name, &mut stdout)? {
        return Err(error::AppError::RestartFailed(module_name.to_string()));
    }

    println!();
    println!("=========================================");
    println!("Container {module_name} restart completed.");
    println!("=========================================");

    let health_url = resolve_health_url(runner, module_name, &config)?;

    output::print_info(format_args!(
        "Waiting for container {module_name} to become healthy..."
    ));

    match health::wait_for_healthy(&health_url, &config, &mut stdout)? {
        StartupStatus::Started => {}
        StartupStatus::Failed => return Err(error::AppError::StartupFailed),
        StartupStatus::Unknown => return Err(error::AppError::HealthCheckUnknown),
    }

    notify::teams::notify_module(module_name, &config)?;

    // Print success message
    println!();
    output::print_info(format_args!("========================================="));
    output::print_info(format_args!("Deploy {module_name} successful!"));
    output::print_info(format_args!("========================================="));
    Ok(())
}

pub fn resolve_health_url(
    runner: &dyn crate::command::CommandRunner,
    module_name: &str,
    config: &AppConfig,
) -> AppResult<String> {
    if let Some(health_url) = config
        .health_url
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return Ok(health_url.to_string());
    }

    let inspect_payload = docker::inspect(runner, module_name)?;
    let host_port = docker::extract_host_port(&inspect_payload, &config.container_port)
        .ok_or_else(|| error::AppError::MissingPortMapping {
            module: module_name.to_string(),
            port: config.container_port.clone(),
        })?;

    Ok(format!("http://localhost:{host_port}/actuator/health"))
}

pub(crate) fn command_args(args: &[&str]) -> Vec<String> {
    args.iter().map(|arg| (*arg).to_string()).collect()
}
