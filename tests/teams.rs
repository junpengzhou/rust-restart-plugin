use remote_restart_plugin::config::AppConfig;
use remote_restart_plugin::notify::teams::{build_payload, load_config, notify_module};
use std::fs;
use std::net::{IpAddr, Ipv4Addr, UdpSocket};

const ROUTE_PROBE_ADDRESS: &str = "192.0.2.1:80";
const UNKNOWN_HOST_INFO: &str = "未知";

fn expected_host_name() -> String {
    hostname::get()
        .ok()
        .map(|name| name.to_string_lossy().trim().to_owned())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| UNKNOWN_HOST_INFO.to_owned())
}

fn expected_default_route_ipv4() -> String {
    let address: Option<Ipv4Addr> = (|| {
        let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
        socket.connect(ROUTE_PROBE_ADDRESS).ok()?;

        match socket.local_addr().ok()?.ip() {
            IpAddr::V4(address) if !address.is_unspecified() && !address.is_loopback() => {
                Some(address)
            }
            _ => None,
        }
    })();

    address
        .map(|address| address.to_string())
        .unwrap_or_else(|| UNKNOWN_HOST_INFO.to_owned())
}

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
fn builds_host_identity_facts_from_current_machine() {
    let payload = build_payload("DEMO", &["frank".to_string()]).expect("payload should build");
    let facts = payload["attachments"][0]["content"]["body"][2]["facts"]
        .as_array()
        .expect("facts should be an array");

    assert_eq!(facts.len(), 5);
    assert_eq!(facts[3]["title"], "主机名称");
    let host_name = facts[3]["value"]
        .as_str()
        .expect("host name should be a string");
    let expected_host_name = expected_host_name();
    assert_eq!(host_name, expected_host_name.as_str());

    assert_eq!(facts[4]["title"], "主机地址");
    let host_address = facts[4]["value"]
        .as_str()
        .expect("host address should be a string");
    let expected_host_address = expected_default_route_ipv4();
    assert_eq!(host_address, expected_host_address.as_str());
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
