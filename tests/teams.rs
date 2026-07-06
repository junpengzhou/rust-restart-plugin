use remote_restart_plugin::config::AppConfig;
use remote_restart_plugin::notify::teams::{build_payload, load_config, notify_module};
use std::fs;

#[test]
fn loads_valid_teams_config() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("settings.json");
    fs::write(
        &path,
        r#"{
          "role": "tester",
          "env_name": "DEMO",
          "webhooks": [
            {
              "name": "Team3",
              "url": "https://example.test/webhook",
              "description": "test group"
            }
          ]
        }"#,
    )
    .expect("write config");

    let config = load_config(path.as_path()).expect("config should load");
    assert_eq!(config.env_name, "DEMO");
    assert_eq!(config.webhooks.len(), 1);
}

#[test]
fn rejects_webhook_missing_required_fields() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("settings.json");
    fs::write(
        &path,
        r#"{
          "webhooks": [
            {
              "name": "Team3",
              "description": "missing url"
            }
          ]
        }"#,
    )
    .expect("write config");

    let error = load_config(path.as_path()).expect_err("missing url should fail");
    assert!(error.to_string().contains("url"));
}

#[test]
fn builds_adaptive_card_payload_for_modules() {
    let payload = build_payload("DEMO", &["frank".to_string(), "pm".to_string()])
        .expect("payload should build");

    assert_eq!(payload["type"], "message");
    let body = &payload["attachments"][0]["content"]["body"];
    assert_eq!(body[0]["type"], "TextBlock");
    assert!(body[2]["facts"][0]["value"]
        .as_str()
        .unwrap()
        .contains("DEMO"));
    assert!(body[2]["facts"][1]["value"]
        .as_str()
        .unwrap()
        .contains("frank, pm"));
    assert_eq!(
        payload["attachments"][0]["content"]["msteams"]["entities"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
}

#[test]
fn notify_module_does_not_fail_when_notify_config_is_invalid() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("settings.json");
    fs::write(&path, "{ invalid json").expect("write invalid config");
    let config = AppConfig {
        notify_config: path,
        ..AppConfig::default()
    };

    notify_module("frank", &config).expect("notification errors should not fail main flow");
}
