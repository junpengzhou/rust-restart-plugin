# Tests Directory Convention Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move behavior tests out of `src/` into `tests/` and document the repository convention in a durable instruction file.

**Architecture:** Keep production logic in `src/lib.rs` focused on runtime code and move the `resolve_health_url` coverage into an integration test file under `tests/`. Add a root-level `AGENTS.md` with concise project instructions so future sessions know to place tests in `tests/`.

**Tech Stack:** Rust stable, Cargo test harness, Markdown.

---

### Task 1: Move health URL tests

**Files:**
- Modify: `src/lib.rs`
- Create: `tests/health_url.rs`

- [ ] **Step 1: Write the failing integration test**

Create `tests/health_url.rs` with the two existing `resolve_health_url` scenarios and run it before exposing the needed API.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test health_url`
Expected: FAIL because the function is not accessible from integration tests yet.

- [ ] **Step 3: Write minimal implementation**

Remove the inline test module from `src/lib.rs` and expose only the smallest callable surface needed for the integration test.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test health_url`
Expected: PASS

### Task 2: Persist the convention

**Files:**
- Create: `AGENTS.md`

- [ ] **Step 1: Add repository memory**

Document that behavior and regression tests belong in `tests/`, with `src/` reserved for production code.

- [ ] **Step 2: Run final verification**

Run: `cargo test`
Expected: PASS
