use clap::Parser;
use std::env;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "remote-restart-plugin")]
#[command(about = "Restart a Docker container, wait for health, and notify Teams.")]
pub struct Cli {
    #[arg(
        value_name = "module_name",
        help = "Name of the module/service to restart (used to determine container name and default log path)"
    )]
    module_name: String,

    #[arg(
        long,
        help = "Path to the log file to tail for startup verification.\nDefaults to /data/boot3/logs/sahara-{module_name}/catalina.out if not specified."
    )]
    pub log_file: Option<PathBuf>,

    #[arg(
        long,
        help = "Path to the notification settings JSON file (contains Teams webhook URL, etc.).\nDefaults to remote-restart-plugin-settings.json in the same directory as the executable if not specified."
    )]
    pub notify_config: Option<PathBuf>,

    #[arg(
        long,
        default_value_t = 120,
        help = "Maximum time in seconds to wait for the container to become healthy after restart.\n[default: 120]"
    )]
    pub health_timeout_seconds: u64,

    #[arg(
        long,
        default_value_t = 5,
        help = "Interval in seconds between consecutive health check probes.\n[default: 5]"
    )]
    pub health_interval_seconds: u64,

    #[arg(
        long,
        default_value = "8080/tcp",
        help = "Container port/protocol to check for health (format: \"port/protocol\", e.g. \"8080/tcp\").\n[default: 8080/tcp]"
    )]
    pub container_port: String,

    #[arg(
        long,
        help = "Explicit health check URL. When set to a non-empty value, Docker inspect and port mapping lookup are skipped."
    )]
    pub health_url: Option<String>,

    #[arg(
        long,
        help = "Skip sending Teams notification after restart completes.\n[default: false]"
    )]
    pub skip_notify: bool,
}

impl Cli {
    // 获取日志文件路径（自动根据模块名称填充默认值）
    pub fn log_file(&self) -> PathBuf {
        self.log_file.clone().unwrap_or_else(|| {
            PathBuf::from(format!(
                "/data/boot3/logs/sahara-{}/catalina.out",
                self.module_name
            ))
        })
    }

    // 模块名称
    pub fn module_name(&self) -> &str {
        &self.module_name
    }

    // 获取通知配置文件路径（自动根据可执行文件同级目录填充默认值）
    pub fn notify_config(&self) -> PathBuf {
        self.notify_config.clone().unwrap_or_else(|| {
            // 获取当前可执行文件所在目录，若获取失败则使用当前工作目录 "."
            let exe_dir = env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|dir| dir.to_path_buf()))
                .unwrap_or_else(|| PathBuf::from("."));
            // 拼接默认的通知配置文件路径
            exe_dir.join("remote-restart-plugin-settings.json")
        })
    }
}
