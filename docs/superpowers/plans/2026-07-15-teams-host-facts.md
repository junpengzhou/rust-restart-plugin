# Teams Host Facts Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the current hostname and the default-route IPv4 address to every Microsoft Teams update-complete notification.

**Architecture:** Keep the existing `build_payload(env_name, modules)` API and notification flow unchanged. Resolve the hostname with the cross-platform `hostname` crate, resolve the default-route IPv4 with a non-sending UDP route probe, and convert either lookup failure to the visible value `未知` before appending two facts to the existing Adaptive Card.

**Tech Stack:** Rust 2021, `hostname` 0.4.2, `std::net::UdpSocket`, `serde_json`, Cargo integration tests.

---

## File Structure

- Modify `Cargo.toml`: declare the cross-platform hostname dependency.
- Modify `Cargo.lock`: record the dependency version selected by Cargo.
- Modify `src/notify/teams.rs`: detect host identity and append the two Adaptive Card facts.
- Modify `tests/teams.rs`: cover the new public payload behavior in the existing Teams integration-test file.

### Task 1: Add the failing payload behavior test

**Files:**
- Test: `tests/teams.rs:1-87`

- [x] **Step 1: Import IPv4 parsing and add a focused integration test**

Add the network type import next to the existing standard-library import:

```rust
use std::fs;
use std::net::Ipv4Addr;
```

Add this test after `builds_adaptive_card_payload_for_modules`:

```rust
#[test]
fn builds_host_identity_facts_from_current_machine() {
    let payload =
        build_payload("DEMO", &["frank".to_string()]).expect("payload should build");
    let facts = payload["attachments"][0]["content"]["body"][2]["facts"]
        .as_array()
        .expect("facts should be an array");

    assert_eq!(facts.len(), 5);
    assert_eq!(facts[3]["title"], "主机名称");
    let host_name = facts[3]["value"]
        .as_str()
        .expect("host name should be a string");
    assert!(!host_name.trim().is_empty());

    assert_eq!(facts[4]["title"], "主机地址");
    let host_address = facts[4]["value"]
        .as_str()
        .expect("host address should be a string");
    if host_address != "未知" {
        let address = host_address
            .parse::<Ipv4Addr>()
            .expect("host address should be a valid IPv4 address");
        assert!(!address.is_unspecified());
        assert!(!address.is_loopback());
    }
}
```

- [x] **Step 2: Run the focused test and verify the RED state**

Run:

```powershell
cargo test --test teams builds_host_identity_facts_from_current_machine -- --exact
```

Expected: FAIL at `assert_eq!(facts.len(), 5)` because the current payload has only the three existing facts.

### Task 2: Detect host identity and append both facts

**Files:**
- Modify: `Cargo.toml:6-12`
- Modify: `Cargo.lock`
- Modify: `src/notify/teams.rs:1-119`
- Test: `tests/teams.rs`

- [x] **Step 1: Add the hostname dependency**

Add `hostname` to `[dependencies]` in `Cargo.toml`:

```toml
[dependencies]
chrono = { version = "0.4", default-features = false, features = ["clock"] }
clap = { version = "4.5", features = ["derive"] }
hostname = "0.4.2"
reqwest = { version = "0.13.4", features = ["blocking", "json", "rustls"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
```

- [x] **Step 2: Add the route-probe imports and constants**

Update the standard-library imports and add constants below them in `src/notify/teams.rs`:

```rust
use std::fs;
use std::net::{IpAddr, Ipv4Addr, UdpSocket};
use std::path::Path;

const ROUTE_PROBE_ADDRESS: &str = "192.0.2.1:80";
const UNKNOWN_HOST_INFO: &str = "未知";
```

- [x] **Step 3: Add the hostname and default-route IPv4 helpers**

Add these production helpers immediately before `build_payload`:

```rust
fn current_host_name() -> String {
    hostname::get()
        .ok()
        .map(|name| name.to_string_lossy().trim().to_owned())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| UNKNOWN_HOST_INFO.to_owned())
}

fn default_route_ipv4() -> Option<Ipv4Addr> {
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect(ROUTE_PROBE_ADDRESS).ok()?;

    match socket.local_addr().ok()?.ip() {
        IpAddr::V4(address) if !address.is_unspecified() && !address.is_loopback() => {
            Some(address)
        }
        _ => None,
    }
}
```

- [x] **Step 4: Resolve host values while building the payload and append the facts**

Add both values after the existing `module_text` calculation:

```rust
let update_time = Local::now().format("%Y-%m-%d %H:%M:%S%z").to_string();
let module_text = modules.join(", ");
let host_name = current_host_name();
let host_address = default_route_ipv4()
    .map(|address| address.to_string())
    .unwrap_or_else(|| UNKNOWN_HOST_INFO.to_owned());
```

Replace the `facts` array with the five-field version:

```rust
"facts": [
    {"title": "更新环境", "value": env_name},
    {"title": "更新模块", "value": module_text},
    {"title": "更新时间", "value": format!("{update_time} (Server Time)")},
    {"title": "主机名称", "value": host_name},
    {"title": "主机地址", "value": host_address}
]
```

- [x] **Step 5: Run the focused test and verify the GREEN state**

Run:

```powershell
cargo test --test teams builds_host_identity_facts_from_current_machine -- --exact
```

Expected: PASS. Cargo also updates `Cargo.lock` with `hostname` and its platform dependencies.

- [x] **Step 6: Run all Teams integration tests**

Run:

```powershell
cargo test --test teams
```

Expected: all five Teams tests pass.

- [x] **Step 7: Apply Rust formatting**

Run:

```powershell
cargo fmt --all
```

Expected: command exits with status 0.

### Task 3: Verify and commit the feature

**Files:**
- Verify: `Cargo.toml`
- Verify: `Cargo.lock`
- Verify: `src/notify/teams.rs`
- Verify: `tests/teams.rs`
- Include: `docs/superpowers/plans/2026-07-15-teams-host-facts.md`

- [x] **Step 1: Verify formatting without modifying files**

Run:

```powershell
cargo fmt --all -- --check
```

Expected: exit status 0 with no formatting diff.

- [x] **Step 2: Run Clippy across every target**

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

- [x] **Step 4: Inspect the final change set**

Run:

```powershell
git diff --check
git diff -- Cargo.toml Cargo.lock src/notify/teams.rs tests/teams.rs docs/superpowers/plans/2026-07-15-teams-host-facts.md
```

Expected: no whitespace errors; the diff contains only the hostname dependency, route-probe helpers, two facts, integration test, and this plan.

- [x] **Step 5: Commit the verified feature**

Run:

```powershell
git add Cargo.toml Cargo.lock src/notify/teams.rs tests/teams.rs docs/superpowers/plans/2026-07-15-teams-host-facts.md
git commit -m "feat: add host facts to Teams notifications"
```

Expected: one commit containing the implementation, regression test, lockfile update, and implementation plan.

- [x] **Step 6: Confirm the worktree is clean**

Run:

```powershell
git status --short
```

Expected: no output.
