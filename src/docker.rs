use crate::command::CommandRunner;
use crate::error::{AppError, AppResult};
use serde_json::Value;
use std::io::Write;

pub fn print_matching_containers(
    runner: &dyn CommandRunner,
    module_name: &str,
    out: &mut dyn Write,
) -> AppResult<()> {
    let output = runner.run_capture("docker", &crate::command_args(&["ps", "-a"]), None)?;
    if !output.success() {
        writeln!(out, "WARNING: docker ps -a failed:")?;
        writeln!(out, "{}", output.combined_output().trim())?;
        return Ok(());
    }

    writeln!(out, "[INFO] docker ps -a | grep {module_name}")?;
    let mut matched = false;
    for line in output
        .stdout
        .lines()
        .filter(|line| line.contains(module_name))
    {
        writeln!(out, "{line}")?;
        matched = true;
    }

    if !matched {
        writeln!(out, "[INFO] No docker ps -a rows matched: {module_name}")?;
    }
    Ok(())
}

pub fn stop_container(
    runner: &dyn CommandRunner,
    module_name: &str,
    out: &mut dyn Write,
) -> AppResult<()> {
    writeln!(out, "[1/2] Stopping container {module_name}...")?;
    let output = runner.run_capture(
        "docker",
        &crate::command_args(&["stop", module_name, "-t", "90"]),
        None,
    )?;
    if !output.success() {
        writeln!(
            out,
            "WARNING: container {module_name} may not be running or stop failed. Continuing..."
        )?;
    }
    Ok(())
}

pub fn start_container(
    runner: &dyn CommandRunner,
    module_name: &str,
    out: &mut dyn Write,
) -> AppResult<bool> {
    start_container_with_sleep(runner, module_name, out, std::thread::sleep)
}

pub fn start_container_with_sleep<S>(
    runner: &dyn CommandRunner,
    module_name: &str,
    out: &mut dyn Write,
    sleep: S,
) -> AppResult<bool>
where
    S: Fn(std::time::Duration),
{
    writeln!(out, "[2/2] Starting container {module_name}...")?;
    let output = runner.run_capture(
        "docker",
        &crate::command_args(&["start", module_name]),
        None,
    )?;
    if output.success() {
        writeln!(out, "Container {module_name} started successfully.")?;
        return Ok(true);
    }

    let combined = output.combined_output();
    if !combined.contains("attaching to network failed") {
        writeln!(out, "ERROR: failed to start container {module_name}.")?;
        writeln!(out, "Error output: {}", combined.trim())?;
        return Ok(false);
    }

    reconnect_network_and_retry(runner, module_name, &combined, out, sleep)
}

fn reconnect_network_and_retry<S>(
    runner: &dyn CommandRunner,
    module_name: &str,
    original_error: &str,
    out: &mut dyn Write,
    sleep: S,
) -> AppResult<bool>
where
    S: Fn(std::time::Duration),
{
    writeln!(
        out,
        "Detected network attach failure. Trying to reconnect the container network..."
    )?;
    sleep(std::time::Duration::from_secs(3));

    let Some(network_name) = first_network_name(&inspect(runner, module_name)?) else {
        writeln!(
            out,
            "ERROR: unable to find network name for container {module_name}."
        )?;
        writeln!(out, "Original error: {}", original_error.trim())?;
        return Ok(false);
    };

    writeln!(out, "Detected network name: {network_name}")?;
    let _ = runner.run_capture(
        "docker",
        &crate::command_args(&["network", "disconnect", &network_name, module_name]),
        None,
    );
    sleep(std::time::Duration::from_secs(1));

    writeln!(
        out,
        "Running: docker network connect {network_name} {module_name}"
    )?;
    let connect_output = runner.run_capture(
        "docker",
        &crate::command_args(&["network", "connect", &network_name, module_name]),
        None,
    )?;
    if !connect_output.success() {
        writeln!(
            out,
            "ERROR: unable to connect container {module_name} to network {network_name}."
        )?;
        writeln!(out, "Original error: {}", original_error.trim())?;
        return Ok(false);
    }

    writeln!(
        out,
        "Network connected successfully. Starting container again..."
    )?;
    let retry_output = runner.run_inherit(
        "docker",
        &crate::command_args(&["start", module_name]),
        None,
    )?;
    if !retry_output.success() {
        writeln!(
            out,
            "ERROR: container {module_name} failed to start after network reconnect."
        )?;
        return Ok(false);
    }

    writeln!(out, "Container {module_name} started successfully.")?;
    Ok(true)
}

pub fn inspect(runner: &dyn CommandRunner, module_name: &str) -> AppResult<Value> {
    let output = runner.run_capture(
        "docker",
        &crate::command_args(&["inspect", module_name]),
        None,
    )?;
    if !output.success() {
        return Err(AppError::CommandFailed {
            program: "docker inspect".to_string(),
            code: output.code,
            stderr: output.combined_output(),
        });
    }
    Ok(serde_json::from_str(&output.stdout)?)
}

pub fn first_network_name(inspect_payload: &Value) -> Option<String> {
    inspect_payload
        .get(0)?
        .get("NetworkSettings")?
        .get("Networks")?
        .as_object()?
        .keys()
        .next()
        .cloned()
}

pub fn extract_host_port(inspect_payload: &Value, container_port: &str) -> Option<String> {
    inspect_payload
        .get(0)?
        .get("NetworkSettings")?
        .get("Ports")?
        .get(container_port)?
        .as_array()?
        .iter()
        .find_map(|binding| binding.get("HostPort")?.as_str().map(str::to_string))
}
