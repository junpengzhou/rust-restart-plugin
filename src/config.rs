use crate::cli::Cli;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub log_file: PathBuf,
    pub notify_config: PathBuf,
    pub health_timeout: Duration,
    pub health_interval: Duration,
    pub container_port: String,
    pub skip_notify: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            log_file: PathBuf::from("/data/logs/xxx/catalina.out"),
            notify_config: PathBuf::from("./remote-restart-plugin-settings.json"),
            health_timeout: Duration::from_secs(120),
            health_interval: Duration::from_secs(5),
            container_port: "8080/tcp".to_string(),
            skip_notify: false,
        }
    }
}

impl AppConfig {
    pub fn from_cli(cli: &Cli) -> Self {
        Self {
            log_file: cli.log_file(),
            notify_config: cli.notify_config(),
            health_timeout: Duration::from_secs(cli.health_timeout_seconds),
            health_interval: Duration::from_secs(cli.health_interval_seconds),
            container_port: cli.container_port.clone(),
            skip_notify: cli.skip_notify,
        }
    }
}
