use clap::Parser;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "remote-restart-plugin")]
#[command(about = "Restart a Docker container, wait for health, and notify Teams.")]
pub struct Cli {
    #[arg(value_name = "module_name")]
    module_name: String,

    #[arg(long, default_value = "/data/boot3/logs/sahara-social/catalina.out")]
    pub log_file: PathBuf,

    #[arg(long, default_value = "/prosh/salt-agent/notify/settings.json")]
    pub notify_config: PathBuf,

    #[arg(long, default_value_t = 120)]
    pub health_timeout_seconds: u64,

    #[arg(long, default_value_t = 5)]
    pub health_interval_seconds: u64,

    #[arg(long, default_value = "8080/tcp")]
    pub container_port: String,

    #[arg(long)]
    pub skip_notify: bool,
}

impl Cli {
    pub fn module_name(&self) -> &str {
        &self.module_name
    }
}
