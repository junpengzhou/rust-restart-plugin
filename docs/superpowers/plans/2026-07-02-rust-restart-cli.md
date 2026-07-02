# Rust Restart CLI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Rust CLI that fully replaces the Python remote restart and Teams notification plugins.

**Architecture:** The CLI is split into focused modules: CLI parsing, command execution, Docker orchestration, health polling, log tailing, Teams notification, configuration, and errors. External process calls go through a small command runner boundary so parsing and orchestration can be tested without Docker.

**Tech Stack:** Rust stable, Cargo, clap, serde, serde_json, reqwest blocking client, thiserror, assert_cmd, predicates, tempfile.

---

### File Structure

- Create: `Cargo.toml` for package metadata, dependencies, and dev-dependencies.
- Create: `src/main.rs` for process entry and exit code mapping.
- Create: `src/lib.rs` to expose testable modules.
- Create: `src/cli.rs` for clap argument parsing.
- Create: `src/config.rs` for default paths and runtime settings.
- Create: `src/error.rs` for typed application errors.
- Create: `src/command.rs` for process execution and testable command output types.
- Create: `src/docker.rs` for Docker ps/stop/start/inspect/network and JSON parsing helpers.
- Create: `src/health.rs` for HTTP health checks and polling.
- Create: `src/logs.rs` for tailing startup logs.
- Create: `src/notify/mod.rs` and `src/notify/teams.rs` for Teams config, payload generation, and webhook POST.
- Create: `tests/docker.rs`, `tests/health.rs`, and `tests/teams.rs` for behavior tests ported from Python plus notification coverage.

### Task 1: Install Rust Toolchain

- [ ] Download rustup-init for Windows x86_64.
- [ ] Set machine/user environment variables so `RUST_HOME=D:\Environment\Rust`, `RUSTUP_HOME=%RUST_HOME%\rustup`, `CARGO_HOME=%RUST_HOME%\cargo`, and `Path` references `%CARGO_HOME%\bin`.
- [ ] Install the latest stable Rust toolchain with rustup.
- [ ] Verify with `rustc --version` and `cargo --version`.

### Task 2: Cargo Skeleton

- [ ] Create `Cargo.toml` and `src` module files.
- [ ] Add a minimal `main` that parses CLI args and calls the library.
- [ ] Run `cargo test` to verify the skeleton compiles.

### Task 3: Docker Inspect Parsing

- [ ] Write failing tests for extracting `8080/tcp` host port and first Docker network name from inspect JSON.
- [ ] Implement serde-backed parsing helpers in `src/docker.rs`.
- [ ] Run the Docker parsing tests.

### Task 4: Health Polling

- [ ] Write failing tests for successful health check and timeout warning behavior.
- [ ] Implement health status abstraction and polling loop in `src/health.rs`.
- [ ] Preserve the Python warning text and ANSI yellow output.
- [ ] Run health tests.

### Task 5: Teams Notification

- [ ] Write failing tests for config validation and Adaptive Card payload generation.
- [ ] Implement `settings.json` loading, webhook validation, payload generation, and POST sender.
- [ ] Remove unused mention generation from the Rust design.
- [ ] Run Teams tests.

### Task 6: Restart Orchestration

- [ ] Write tests for command sequence behavior where practical without Docker.
- [ ] Implement `print_matching_containers`, stop/start, network reconnect on `attaching to network failed`, inspect, health, log tail, and notification flow.
- [ ] Keep default paths compatible with the Python plugin.
- [ ] Run the full test suite.

### Task 7: Polish and Verification

- [ ] Run `cargo fmt`.
- [ ] Run `cargo clippy -- -D warnings` if clippy is installed.
- [ ] Run `cargo test`.
- [ ] Review `git diff` for unrelated changes and implementation completeness.
