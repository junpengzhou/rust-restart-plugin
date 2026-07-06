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
    let mut stdout = io::stdout();
    docker::print_matching_containers(&runner, module_name, &mut stdout)?;
    docker::stop_container(&runner, module_name, &mut stdout)?;

    if !docker::start_container(&runner, module_name, &mut stdout)? {
        return Err(error::AppError::RestartFailed(module_name.to_string()));
    }

    println!();
    println!("=========================================");
    println!("Container {module_name} restart completed.");
    println!("=========================================");

    let inspect_payload = docker::inspect(&runner, module_name)?;
    let host_port = docker::extract_host_port(&inspect_payload, &config.container_port)
        .ok_or_else(|| error::AppError::MissingPortMapping {
            module: module_name.to_string(),
            port: config.container_port.clone(),
        })?;

    let health_url = format!("http://localhost:{host_port}/actuator/health");
    match health::wait_for_healthy(&health_url, &config, &mut stdout)? {
        StartupStatus::Started => {}
        StartupStatus::Failed => return Err(error::AppError::StartupFailed),
        StartupStatus::Unknown => return Err(error::AppError::HealthCheckUnknown),
    }

    notify::teams::notify_module(module_name, &config)?;
    Ok(())
}

pub(crate) fn command_args(args: &[&str]) -> Vec<String> {
    args.iter().map(|arg| (*arg).to_string()).collect()
}
