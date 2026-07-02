use remote_restart_plugin::config::AppConfig;
use remote_restart_plugin::health::{HealthStatus, wait_for_healthy_with};
use std::cell::Cell;
use std::time::Duration;

#[test]
fn health_check_unknown_prints_warning_and_tail_command() {
    let config = AppConfig {
        health_timeout: Duration::from_secs(2),
        health_interval: Duration::from_secs(1),
        ..AppConfig::default()
    };
    let calls = Cell::new(0);
    let tail_calls = Cell::new(0);
    let mut output = Vec::new();

    let healthy = wait_for_healthy_with(
        "http://localhost:9904/actuator/health",
        &config,
        |_| {
            calls.set(calls.get() + 1);
            HealthStatus::Unknown
        },
        || {
            tail_calls.set(tail_calls.get() + 1);
            Ok("line 1\nline 2\n".to_string())
        },
        |_| {},
        &mut output,
    )
    .expect("health wait should not return IO errors");

    assert!(!healthy);
    assert_eq!(tail_calls.get(), 1);
    let text = String::from_utf8(output).expect("output should be utf8");
    assert!(text.contains("\u{1b}[33m[WARNING]Startup health check is unknown."));
    assert!(
        text.contains(
            "Suggested command: tail -fn 300 /data/boot3/logs/sahara-social/catalina.out"
        )
    );
    assert!(text.contains("line 1"));
}

#[test]
fn health_check_succeeds_on_http_200() {
    let config = AppConfig {
        health_timeout: Duration::from_secs(10),
        health_interval: Duration::from_secs(1),
        ..AppConfig::default()
    };
    let tail_calls = Cell::new(0);
    let mut output = Vec::new();

    let healthy = wait_for_healthy_with(
        "http://localhost:9904/actuator/health",
        &config,
        |_| HealthStatus::Code(200),
        || {
            tail_calls.set(tail_calls.get() + 1);
            Ok("started\n".to_string())
        },
        |_| {},
        &mut output,
    )
    .expect("health wait should not return IO errors");

    assert!(healthy);
    assert_eq!(tail_calls.get(), 1);
    let text = String::from_utf8(output).expect("output should be utf8");
    assert!(text.contains("[INFO] Health check passed"));
    assert!(text.contains("started"));
}
