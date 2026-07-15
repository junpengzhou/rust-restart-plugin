use crate::config::AppConfig;
use crate::error::{AppError, AppResult};
use crate::output;
use chrono::Local;
use serde::Deserialize;
use serde_json::{json, Value};
use std::fs;
use std::net::{IpAddr, Ipv4Addr, UdpSocket};
use std::path::Path;

const ROUTE_PROBE_ADDRESS: &str = "192.0.2.1:80";
const SEND_ATTEMPTS: usize = 2;
const SEND_RETRY_DELAY: std::time::Duration = std::time::Duration::from_secs(1);
const UNKNOWN_HOST_INFO: &str = "未知";

#[derive(Debug, Clone, Deserialize)]
pub struct TeamsConfig {
    #[serde(default = "default_role")]
    pub role: String,
    #[serde(default = "default_env_name")]
    pub env_name: String,
    #[serde(default)]
    pub webhooks: Vec<WebhookConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WebhookConfig {
    pub url: String,
}

pub fn notify_module(module_name: &str, config: &AppConfig) -> AppResult<()> {
    if config.skip_notify {
        output::print_info(format_args!(
            "Skipping Teams notification: disabled by --skip-notify."
        ));
        return Ok(());
    }

    if !config.notify_config.as_path().is_file() {
        output::print_warn(format_args!(
            "Skipping Teams notification: notify config does not exist."
        ));
        return Ok(());
    }

    if let Err(error) = notify_module_inner(module_name, config) {
        output::print_warn(format_args!("Microsoft Teams notification failed: {error}"));
    }
    Ok(())
}

fn notify_module_inner(module_name: &str, config: &AppConfig) -> AppResult<()> {
    let teams_config = load_config(config.notify_config.as_path())?;
    let modules = parse_modules(module_name)?;
    send_notifications(&teams_config, &modules)?;
    Ok(())
}

pub fn load_config(path: &Path) -> AppResult<TeamsConfig> {
    let content = fs::read_to_string(path).map_err(|source| AppError::ReadFile {
        path: path.to_path_buf(),
        source,
    })?;
    let config: TeamsConfig =
        serde_json::from_str(&content).map_err(|source| AppError::ParseJson {
            path: path.to_path_buf(),
            source,
        })?;

    validate_config(&config)?;
    Ok(config)
}

fn current_host_name() -> String {
    hostname::get()
        .ok()
        .map(|name| name.to_string_lossy().trim().to_owned())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| UNKNOWN_HOST_INFO.to_owned())
}

fn default_route_ipv4() -> Option<Ipv4Addr> {
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect(ROUTE_PROBE_ADDRESS).ok()?;

    match socket.local_addr().ok()?.ip() {
        IpAddr::V4(address) if !address.is_unspecified() && !address.is_loopback() => Some(address),
        _ => None,
    }
}

pub fn build_payload(env_name: &str, modules: &[String]) -> AppResult<Value> {
    if modules.is_empty() {
        return Err(AppError::InvalidConfig(
            "--modules must contain at least one module".to_string(),
        ));
    }

    let update_time = Local::now().format("%Y-%m-%d %H:%M:%S%z").to_string();
    let module_text = modules.join(", ");
    let host_name = current_host_name();
    let host_address = default_route_ipv4()
        .map(|address| address.to_string())
        .unwrap_or_else(|| UNKNOWN_HOST_INFO.to_owned());

    Ok(json!({
        "type": "message",
        "attachments": [
            {
                "contentType": "application/vnd.microsoft.card.adaptive",
                "contentUrl": "",
                "content": {
                    "$schema": "http://adaptivecards.io/schemas/adaptive-card.json",
                    "type": "AdaptiveCard",
                    "version": "1.5",
                    "body": [
                        {
                            "type": "TextBlock",
                            "text": "SPUG 更新完成通知",
                            "wrap": true,
                            "weight": "bolder",
                            "size": "medium",
                            "color": "Attention",
                            "style": "heading"
                        },
                        {
                            "type": "TextBlock",
                            "text": "所有人, 请知悉.",
                            "wrap": true
                        },
                        {
                            "type": "FactSet",
                            "facts": [
                                {"title": "更新环境", "value": env_name},
                                {"title": "更新模块", "value": module_text},
                                {"title": "更新时间", "value": format!("{update_time} (Server Time)")},
                                {"title": "主机名称", "value": host_name},
                                {"title": "主机地址", "value": host_address}
                            ]
                        }
                    ],
                    "msteams": {
                        "entities": []
                    }
                }
            }
        ]
    }))
}

pub fn send_notifications(config: &TeamsConfig, modules: &[String]) -> AppResult<()> {
    let payload = build_payload(&config.env_name, modules)?;
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    for webhook in &config.webhooks {
        for attempt in 1..=SEND_ATTEMPTS {
            match client.post(&webhook.url).json(&payload).send() {
                Ok(_) => {
                    output::print_info(format_args!(
                        "Teams notification sent successfully to webhook: {}",
                        webhook.url
                    ));
                    break;
                }
                Err(error) if error.is_request() && attempt < SEND_ATTEMPTS => {
                    std::thread::sleep(SEND_RETRY_DELAY);
                }
                Err(error) => {
                    output::print_warn(format_args!(
                        "Microsoft Teams notification failed, webhook: {}, error: {error}",
                        webhook.url
                    ));
                    break;
                }
            }
        }
    }
    Ok(())
}

fn validate_config(config: &TeamsConfig) -> AppResult<()> {
    for webhook in &config.webhooks {
        if webhook.url.trim().is_empty() {
            return Err(AppError::InvalidConfig(
                "webhook is missing required field `url`".to_string(),
            ));
        }
    }
    Ok(())
}

fn parse_modules(modules: &str) -> AppResult<Vec<String>> {
    let parsed: Vec<String> = modules
        .split(',')
        .map(str::trim)
        .filter(|module| !module.is_empty())
        .map(str::to_string)
        .collect();

    if parsed.is_empty() {
        return Err(AppError::InvalidConfig(
            "--modules must contain at least one module".to_string(),
        ));
    }
    Ok(parsed)
}

fn default_role() -> String {
    "tester".to_string()
}

fn default_env_name() -> String {
    "DEMO".to_string()
}
