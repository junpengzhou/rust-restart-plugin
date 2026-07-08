use crate::config::AppConfig;
use crate::error::AppResult;
use crate::logs;
use crate::output;
use std::io::Write;
use std::time::Duration;

const SUCCESS_LOG_LINES: usize = 20;
const FAILURE_LOG_LINES: usize = 300;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    Code(u16),
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupStatus {
    Started,
    Failed,
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
) -> AppResult<StartupStatus> {
    wait_for_healthy_with(
        health_url,
        config,
        get_health_status,
        |lines| logs::tail_log_lines(config.log_file.as_path(), lines),
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
) -> AppResult<StartupStatus>
where
    H: Fn(&str) -> HealthStatus,
    T: Fn(usize) -> AppResult<String>,
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
                output::info(
                    out,
                    format_args!("Health check passed: {health_url} returned ok."),
                )?;
                print_tail(config, &tail_log, SUCCESS_LOG_LINES, out)?;
                return Ok(StartupStatus::Started);
            }
            HealthStatus::Code(404) => {
                output::error(
                    out,
                    format_args!("Health check failed: {health_url} returned an explicit error."),
                )?;
                print_tail(config, &tail_log, FAILURE_LOG_LINES, out)?;
                return Ok(StartupStatus::Failed);
            }
            HealthStatus::Code(code) => {
                output::info(
                    out,
                    format_args!("Health check returned HTTP {code}. Waiting..."),
                )?;
            }
            HealthStatus::Unknown => {
                output::info(out, format_args!("Health check pending..."))?;
            }
        }

        if attempt + 1 < attempts {
            sleep(config.health_interval);
        }
    }

    print_tail(config, &tail_log, FAILURE_LOG_LINES, out)?;
    output::warn(
        out,
        format_args!(
            "Startup health check is unknown. Please log in to the server and check the application startup status manually. Suggested command: tail -fn 300 {}",
            config.log_file.display()
        ),
    )?;
    Ok(StartupStatus::Unknown)
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

fn print_tail<T>(
    config: &AppConfig,
    tail_log: &T,
    lines: usize,
    out: &mut dyn Write,
) -> AppResult<()>
where
    T: Fn(usize) -> AppResult<String>,
{
    output::info(
        out,
        format_args!("tail -n {lines} {}", config.log_file.display()),
    )?;
    let output = tail_log(lines)?;
    if !output.trim().is_empty() {
        writeln!(out, "{}", output.trim_end())?;
    }
    Ok(())
}
