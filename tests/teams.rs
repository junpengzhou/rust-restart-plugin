use remote_restart_plugin::config::AppConfig;
use remote_restart_plugin::notify::teams::{
    build_payload, load_config, notify_module, send_notifications, TeamsConfig, WebhookConfig,
};
use std::fs;
use std::io::{Read, Write};
use std::net::{IpAddr, Ipv4Addr, TcpListener, TcpStream, UdpSocket};
use std::thread;
use std::time::{Duration, Instant};

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

fn test_webhook_listener() -> (TcpListener, String) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test webhook");
    let address = listener.local_addr().expect("test webhook address");
    listener
        .set_nonblocking(true)
        .expect("set test webhook nonblocking");
    (listener, format!("http://{address}"))
}

fn accept_connection(listener: &TcpListener, timeout: Duration) -> Option<TcpStream> {
    let deadline = Instant::now() + timeout;

    loop {
        match listener.accept() {
            Ok((stream, _)) => {
                stream
                    .set_nonblocking(false)
                    .expect("set test webhook connection blocking");
                return Some(stream);
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                if Instant::now() >= deadline {
                    return None;
                }
                thread::sleep(Duration::from_millis(10));
            }
            Err(error) => panic!("accept test webhook connection: {error}"),
        }
    }
}

fn read_request(stream: &mut TcpStream) {
    let mut request = Vec::new();
    let mut chunk = [0_u8; 1024];

    loop {
        let bytes_read = stream.read(&mut chunk).expect("read webhook request");
        assert!(bytes_read > 0, "webhook request should not be empty");
        request.extend_from_slice(&chunk[..bytes_read]);

        let Some(header_end) = request.windows(4).position(|bytes| bytes == b"\r\n\r\n") else {
            continue;
        };
        let body_start = header_end + 4;
        let headers = std::str::from_utf8(&request[..header_end]).expect("valid request headers");
        let content_length = headers
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().expect("valid content length"))
            })
            .unwrap_or(0);

        if request.len() >= body_start + content_length {
            return;
        }
    }
}

fn respond(mut stream: TcpStream, status: &str) {
    stream
        .set_read_timeout(Some(Duration::from_secs(1)))
        .expect("set request read timeout");
    read_request(&mut stream);

    let response = format!("HTTP/1.1 {status}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
    stream
        .write_all(response.as_bytes())
        .expect("write webhook response");
    stream.flush().expect("flush webhook response");
}

fn teams_config_with_webhook(url: String) -> TeamsConfig {
    TeamsConfig {
        role: "tester".to_string(),
        env_name: "DEMO".to_string(),
        webhooks: vec![WebhookConfig { url }],
    }
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
fn notification_retries_network_error_once_after_delay() {
    let (listener, url) = test_webhook_listener();
    let server = thread::spawn(move || {
        let first =
            accept_connection(&listener, Duration::from_secs(2)).expect("first webhook connection");
        let first_attempt = Instant::now();
        drop(first);

        let second = accept_connection(&listener, Duration::from_secs(3))
            .expect("second webhook connection");
        let retry_delay = first_attempt.elapsed();
        respond(second, "200 OK");
        retry_delay
    });
    let config = teams_config_with_webhook(url);

    send_notifications(&config, &["frank".to_string()]).expect("notification should finish");

    let retry_delay = server.join().expect("test webhook should finish");
    assert!(
        retry_delay >= Duration::from_millis(900),
        "retry occurred too soon: {retry_delay:?}"
    );
}

#[test]
fn notification_does_not_retry_http_error_response() {
    let (listener, url) = test_webhook_listener();
    let server = thread::spawn(move || {
        let first =
            accept_connection(&listener, Duration::from_secs(2)).expect("first webhook connection");
        respond(first, "500 Internal Server Error");

        1 + usize::from(accept_connection(&listener, Duration::from_millis(1500)).is_some())
    });
    let config = teams_config_with_webhook(url);

    send_notifications(&config, &["frank".to_string()]).expect("notification should finish");

    assert_eq!(server.join().expect("test webhook should finish"), 1);
}

#[test]
fn notification_does_not_retry_invalid_url() {
    let config = teams_config_with_webhook("not a valid URL".to_string());
    let started = Instant::now();

    send_notifications(&config, &["frank".to_string()]).expect("notification should finish");

    let elapsed = started.elapsed();
    assert!(
        elapsed < Duration::from_millis(900),
        "invalid URL should not wait before failing: {elapsed:?}"
    );
}

#[test]
fn notification_stops_after_two_network_errors() {
    let (listener, url) = test_webhook_listener();
    let server = thread::spawn(move || {
        let mut attempts = 0;
        for timeout in [
            Duration::from_secs(2),
            Duration::from_secs(3),
            Duration::from_millis(1500),
        ] {
            let Some(connection) = accept_connection(&listener, timeout) else {
                break;
            };
            attempts += 1;
            drop(connection);
        }
        attempts
    });
    let config = teams_config_with_webhook(url);

    send_notifications(&config, &["frank".to_string()]).expect("notification should finish");

    assert_eq!(server.join().expect("test webhook should finish"), 2);
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
