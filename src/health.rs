use crate::config::AppConfig;
use crate::error::AppResult;
use crate::logs;
use std::io::Write;
use std::time::Duration;

const YELLOW: &str = "\x1b[33m";
const RESET: &str = "\x1b[0m";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    Code(u16),
    Unknown,
}

pub fn get_health_status(url: &str) -> HealthStatus {
    let client = match reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
    {
        Ok(client) => client,
        Err(_) => return HealthStatus::Unknown,
    };

    match client.get(url).send() {
        Ok(response) => HealthStatus::Code(response.status().as_u16()),
        Err(_) => HealthStatus::Unknown,
    }
}

pub fn wait_for_healthy(
    health_url: &str,
    config: &AppConfig,
    out: &mut dyn Write,
) -> AppResult<bool> {
    wait_for_healthy_with(
        health_url,
        config,
        get_health_status,
        || logs::tail_log_lines(config.log_file.as_path(), 20),
        std::thread::sleep,
        out,
    )
}

pub fn wait_for_healthy_with<H, T, S>(
    health_url: &str,
    config: &AppConfig,
    health_getter: H,
    tail_log: T,
    sleep: S,
    out: &mut dyn Write,
) -> AppResult<bool>
where
    H: Fn(&str) -> HealthStatus,
    T: Fn() -> AppResult<String>,
    S: Fn(Duration),
{
    writeln!(
        out,
        "Restart completed. Waiting for application startup health check..."
    )?;

    let attempts = attempts_for(config.health_timeout, config.health_interval);
    for attempt in 0..attempts {
        match health_getter(health_url) {
            HealthStatus::Code(200) => {
                writeln!(
                    out,
                    "[INFO] Health check passed: {health_url} returned 200."
                )?;
                print_tail(config, &tail_log, out)?;
                return Ok(true);
            }
            HealthStatus::Code(code) => {
                writeln!(out, "[INFO] Health check returned HTTP {code}. Waiting...")?;
            }
            HealthStatus::Unknown => {
                writeln!(out, "[INFO] Health check pending...")?;
            }
        }

        if attempt + 1 < attempts {
            sleep(config.health_interval);
        }
    }

    print_tail(config, &tail_log, out)?;
    writeln!(
        out,
        "{YELLOW}[WARNING]Startup health check is unknown. Please log in to the server and check the application startup status manually. Suggested command: tail -fn 300 {}{RESET}",
        config.log_file.display()
    )?;
    Ok(false)
}

fn attempts_for(timeout: Duration, interval: Duration) -> u32 {
    if timeout.is_zero() || interval.is_zero() {
        return 1;
    }

    let timeout_ms = timeout.as_millis();
    let interval_ms = interval.as_millis();
    (timeout_ms.div_ceil(interval_ms) + 1)
        .try_into()
        .unwrap_or(u32::MAX)
}

fn print_tail<T>(config: &AppConfig, tail_log: &T, out: &mut dyn Write) -> AppResult<()>
where
    T: Fn() -> AppResult<String>,
{
    writeln!(out, "[INFO] tail -n 20 {}", config.log_file.display())?;
    let output = tail_log()?;
    if !output.trim().is_empty() {
        writeln!(out, "{}", output.trim_end())?;
    }
    Ok(())
}
