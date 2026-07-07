# Health URL Override Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Allow callers to pass an explicit `health_url` so restart health checks can skip Docker port inspection when an external URL is already known.

**Architecture:** Keep the health polling module unchanged and add the override at the configuration and orchestration layers. `src/lib.rs` will resolve the effective health URL by preferring `health_url` when present and only falling back to `docker inspect` plus `container_port` mapping when it is absent.

**Tech Stack:** Rust stable, Cargo, clap, serde_json, reqwest blocking client, thiserror.

---

### Task 1: Add regression coverage

**Files:**
- Modify: `src/lib.rs`
- Create: `tests/run.rs`

- [ ] **Step 1: Write failing tests**

Add a testable `run_with_runner` entry point and a regression test that passes `health_url`, makes container start succeed, and verifies the flow completes without requiring a Docker inspect result.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test run`
Expected: FAIL because there is no override-aware orchestration entry point yet.

### Task 2: Implement override support

**Files:**
- Modify: `src/cli.rs`
- Modify: `src/config.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Add optional CLI/config field**

Introduce `--health-url` on the CLI and store it as `Option<String>` in `AppConfig`.

- [ ] **Step 2: Resolve effective health URL**

Update orchestration to prefer `config.health_url` and only execute Docker inspect plus host port extraction when the override is absent.

- [ ] **Step 3: Run targeted tests**

Run: `cargo test --test run --test health`
Expected: PASS

### Task 3: Verify CLI/config behavior

**Files:**
- Modify: `tests/run.rs`

- [ ] **Step 1: Add focused assertions if needed**

Cover the override branch clearly enough that future regressions in the inspect fallback are visible.

- [ ] **Step 2: Run final verification**

Run: `cargo test`
Expected: PASS
