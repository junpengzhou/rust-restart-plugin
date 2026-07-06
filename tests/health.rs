use remote_restart_plugin::config::AppConfig;
use remote_restart_plugin::health::{wait_for_healthy_with, HealthStatus};
use std::cell::{Cell, RefCell};
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
    let tailed_lines = RefCell::new(Vec::new());
    let mut output = Vec::new();

    let healthy = wait_for_healthy_with(
        "http://localhost:9904/actuator/health",
        &config,
        |_| {
            calls.set(calls.get() + 1);
            HealthStatus::Unknown
        },
        |lines| {
            tail_calls.set(tail_calls.get() + 1);
            tailed_lines.borrow_mut().push(lines);
            Ok("line 1\nline 2\n".to_string())
        },
        |_| {},
        &mut output,
    )
    .expect("health wait should not return IO errors");

    assert!(!healthy);
    assert_eq!(tail_calls.get(), 1);
    assert_eq!(*tailed_lines.borrow(), vec![300]);
    let text = String::from_utf8(output).expect("output should be utf8");
    assert!(text.contains("\u{1b}[33m[WARNING]Startup health check is unknown."));
    assert!(text.contains("[INFO] tail -n 300 /data/boot3/logs/sahara-social/catalina.out"));
    assert!(text
        .contains("Suggested command: tail -fn 300 /data/boot3/logs/sahara-social/catalina.out"));
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
    let tailed_lines = RefCell::new(Vec::new());
    let mut output = Vec::new();

    let healthy = wait_for_healthy_with(
        "http://localhost:9904/actuator/health",
        &config,
        |_| HealthStatus::Code(200),
        |lines| {
            tail_calls.set(tail_calls.get() + 1);
            tailed_lines.borrow_mut().push(lines);
            Ok("started\n".to_string())
        },
        |_| {},
        &mut output,
    )
    .expect("health wait should not return IO errors");

    assert!(healthy);
    assert_eq!(tail_calls.get(), 1);
    assert_eq!(*tailed_lines.borrow(), vec![20]);
    let text = String::from_utf8(output).expect("output should be utf8");
    assert!(text.contains("[INFO] Health check passed"));
    assert!(text.contains("started"));
}

#[test]
fn health_check_fails_immediately_on_http_404() {
    let config = AppConfig {
        health_timeout: Duration::from_secs(10),
        health_interval: Duration::from_secs(1),
        ..AppConfig::default()
    };
    let calls = Cell::new(0);
    let sleeps = Cell::new(0);
    let tailed_lines = RefCell::new(Vec::new());
    let mut output = Vec::new();

    let healthy = wait_for_healthy_with(
        "http://localhost:9904/actuator/health",
        &config,
        |_| {
            calls.set(calls.get() + 1);
            HealthStatus::Code(404)
        },
        |lines| {
            tailed_lines.borrow_mut().push(lines);
            Ok("spring boot failed\nstack trace\n".to_string())
        },
        |_| {
            sleeps.set(sleeps.get() + 1);
        },
        &mut output,
    )
    .expect("health wait should not return IO errors");

    assert!(!healthy);
    assert_eq!(calls.get(), 1);
    assert_eq!(sleeps.get(), 0);
    assert_eq!(*tailed_lines.borrow(), vec![300]);
    let text = String::from_utf8(output).expect("output should be utf8");
    assert!(text.contains(
        "[ERROR] Health check failed: http://localhost:9904/actuator/health returned 404."
    ));
    assert!(text.contains("[INFO] tail -n 300 /data/boot3/logs/sahara-social/catalina.out"));
    assert!(text.contains("spring boot failed"));
}
