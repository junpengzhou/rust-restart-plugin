# Teams Notification Retry Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Retry a Teams webhook once, after a one-second delay, only when the HTTP request fails with a network error.

**Architecture:** Preserve `send_notifications` and its per-webhook behavior. Add two constants and one inline two-attempt loop; the first network error is silent, a successful final result prints success, and two network errors print one final warning. Use local TCP integration tests to exercise real `reqwest` behavior without new dependencies.

**Tech Stack:** Rust 2021, `reqwest::blocking`, standard-library TCP test server, Cargo integration tests.

---

## File Structure

- Modify `tests/teams.rs`: add local TCP helpers and three retry behavior tests in the existing Teams integration-test file.
- Modify `src/notify/teams.rs`: add retry constants and the minimal inline retry loop.

### Task 1: Add failing retry behavior tests

**Files:**
- Test: `tests/teams.rs`

- [x] **Step 1: Import the notification API and TCP test support**

Replace the existing Teams and standard-library imports at the top of `tests/teams.rs` with:

```rust
use remote_restart_plugin::config::AppConfig;
use remote_restart_plugin::notify::teams::{
    build_payload, load_config, notify_module, send_notifications, TeamsConfig, WebhookConfig,
};
use std::fs;
use std::io::{Read, Write};
use std::net::{IpAddr, Ipv4Addr, TcpListener, TcpStream, UdpSocket};
use std::thread;
use std::time::{Duration, Instant};
```

- [x] **Step 2: Add focused local TCP helpers**

Add these helpers after the existing `expected_default_route_ipv4` helper:

```rust
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

    let response =
        format!("HTTP/1.1 {status}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
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
```

- [x] **Step 3: Add all three retry behavior tests**

Add these tests before `notify_module_does_not_fail_when_notify_config_is_invalid`:

```rust
#[test]
fn notification_retries_network_error_once_after_delay() {
    let (listener, url) = test_webhook_listener();
    let server = thread::spawn(move || {
        let first = accept_connection(&listener, Duration::from_secs(2))
            .expect("first webhook connection");
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
        let first = accept_connection(&listener, Duration::from_secs(2))
            .expect("first webhook connection");
        respond(first, "500 Internal Server Error");

        1 + usize::from(
            accept_connection(&listener, Duration::from_millis(1500)).is_some(),
        )
    });
    let config = teams_config_with_webhook(url);

    send_notifications(&config, &["frank".to_string()]).expect("notification should finish");

    assert_eq!(server.join().expect("test webhook should finish"), 1);
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
```

- [x] **Step 4: Run the new tests and verify the RED state**

Run:

```powershell
cargo test --test teams notification_ -- --nocapture
```

Expected: `notification_does_not_retry_http_error_response` passes because that is existing behavior. The other two tests fail because the current implementation makes only one network attempt.

### Task 2: Implement the minimal two-attempt loop

**Files:**
- Modify: `src/notify/teams.rs`
- Test: `tests/teams.rs`

- [x] **Step 1: Add the two retry constants**

Add these constants beside the existing Teams constants:

```rust
const SEND_ATTEMPTS: usize = 2;
const SEND_RETRY_DELAY: std::time::Duration = std::time::Duration::from_secs(1);
```

- [x] **Step 2: Replace the single send with the inline retry loop**

Replace the body of the existing `for webhook in &config.webhooks` loop with:

```rust
for attempt in 1..=SEND_ATTEMPTS {
    match client.post(&webhook.url).json(&payload).send() {
        Ok(_) => {
            output::print_info(format_args!(
                "Teams notification sent successfully to webhook: {}",
                webhook.url
            ));
            break;
        }
        Err(error) if attempt == SEND_ATTEMPTS => {
            output::print_warn(format_args!(
                "Microsoft Teams notification failed, webhook: {}, error: {error}",
                webhook.url
            ));
        }
        Err(_) => std::thread::sleep(SEND_RETRY_DELAY),
    }
}
```

This keeps the first network error silent. `Ok(_)` includes HTTP 4xx and 5xx responses, so those responses do not retry.

- [x] **Step 3: Run the new tests and verify the GREEN state**

Run:

```powershell
cargo test --test teams notification_ -- --nocapture
```

Expected: all three notification retry tests pass.

- [x] **Step 4: Run every Teams integration test**

Run:

```powershell
cargo test --test teams
```

Expected: all eight Teams tests pass.

- [x] **Step 5: Apply Rust formatting**

Run:

```powershell
cargo fmt --all
```

Expected: exit status 0.

### Task 3: Verify and commit the retry feature

**Files:**
- Verify: `src/notify/teams.rs`
- Verify: `tests/teams.rs`
- Include: `docs/superpowers/plans/2026-07-15-teams-notification-retry.md`

- [x] **Step 1: Verify formatting**

Run:

```powershell
cargo fmt --all -- --check
```

Expected: exit status 0 with no formatting diff.

- [x] **Step 2: Run Clippy across all targets**

Run:

```powershell
cargo clippy --all-targets --all-features -- -D warnings
```

Expected: exit status 0 with no warnings.

- [x] **Step 3: Run the complete test suite**

Run:

```powershell
cargo test
```

Expected: exit status 0 and zero failed tests.

- [x] **Step 4: Inspect the final diff**

Run:

```powershell
git diff --check
git diff -- src/notify/teams.rs tests/teams.rs docs/superpowers/plans/2026-07-15-teams-notification-retry.md
```

Expected: no whitespace errors; the production diff contains only two constants and the inline retry loop.

- [x] **Step 5: Commit the verified feature**

Run:

```powershell
git add src/notify/teams.rs tests/teams.rs docs/superpowers/plans/2026-07-15-teams-notification-retry.md
git commit -m "feat: retry Teams notification network errors"
```

Expected: one commit containing the retry implementation, integration tests, and implementation plan.

- [x] **Step 6: Confirm the worktree is clean**

Run:

```powershell
git status --short
```

Expected: no output.

### Task 4: Address code review findings

**Files:**
- Modify: `src/notify/teams.rs`
- Test: `tests/teams.rs`
- Update: `docs/superpowers/plans/2026-07-15-teams-notification-retry.md`

- [x] **Step 1: Add a failing invalid-URL regression test**

Add this test after the HTTP-error response test:

```rust
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
```

- [x] **Step 2: Verify the unrestricted retry fails the regression test**

Run:

```powershell
cargo test --test teams notification_does_not_retry_invalid_url -- --exact
```

Expected: FAIL after approximately one second because a reqwest builder error is incorrectly retried.

- [x] **Step 3: Restrict retry to reqwest request errors**

Use this final inner loop in `send_notifications`:

```rust
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
```

`reqwest::Error::is_request()` covers failures while sending the HTTP request, including connection, timeout, and connection-reset failures. Builder and redirect errors fail immediately without sleeping.

- [x] **Step 4: Make the local test server consume the complete request**

Add this helper and call it from `respond` before writing the response:

```rust
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
```

- [x] **Step 5: Run the focused regression tests and Clippy**

Run:

```powershell
cargo test --test teams notification_ -- --nocapture
cargo clippy --test teams -- -D warnings
```

Expected: all four notification tests pass and Clippy exits with status 0.

- [x] **Step 6: Re-run complete verification and amend the feature commit**

Run:

```powershell
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
git add src/notify/teams.rs tests/teams.rs docs/superpowers/plans/2026-07-15-teams-notification-retry.md
git commit --amend --no-edit
git status --short
```

Expected: formatting and Clippy succeed, all tests pass, the amended commit succeeds, and the final status is empty.
