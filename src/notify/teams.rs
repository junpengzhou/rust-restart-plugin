use crate::config::AppConfig;
use crate::error::{AppError, AppResult};
use chrono::Local;
use serde::Deserialize;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

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
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub description: String,
    pub members: Vec<MemberConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MemberConfig {
    pub name: String,
    pub id: String,
}

pub fn notify_module(module_name: &str, config: &AppConfig) -> AppResult<()> {
    if config.skip_notify {
        println!("Skipping Teams notification: disabled by --skip-notify.");
        return Ok(());
    }

    if !config.notify_config.as_path().is_file() {
        println!("Skipping Teams notification: notify config does not exist.");
        return Ok(());
    }

    println!("Sending update notification to Teams...");
    let teams_config = load_config(config.notify_config.as_path())?;
    let modules = parse_modules(module_name)?;
    send_notifications(&teams_config, &modules)?;
    println!("Teams notification sent.");
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

pub fn build_payload(env_name: &str, modules: &[String]) -> AppResult<Value> {
    if modules.is_empty() {
        return Err(AppError::InvalidConfig(
            "--modules must contain at least one module".to_string(),
        ));
    }

    let update_time = Local::now().format("%Y-%m-%d %H:%M:%S%z").to_string();
    let module_text = modules.join(", ");

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
                                {"title": "更新时间", "value": format!("{update_time} (Server Time)")}
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
        println!(
            "Processing webhook: {}, description: {}, url: {}",
            webhook.name, webhook.description, webhook.url
        );
        match client.post(&webhook.url).json(&payload).send() {
            Ok(_) => {}
            Err(error) => {
                println!("WARNING: Microsoft Teams notification failed: {error}");
            }
        }
    }
    Ok(())
}

fn validate_config(config: &TeamsConfig) -> AppResult<()> {
    for webhook in &config.webhooks {
        if webhook.name.trim().is_empty() {
            return Err(AppError::InvalidConfig(
                "webhook is missing required field `name`".to_string(),
            ));
        }
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
