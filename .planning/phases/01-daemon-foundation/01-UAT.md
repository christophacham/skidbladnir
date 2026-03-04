---
status: testing
phase: 01-daemon-foundation
source: 01-01-SUMMARY.md, 01-02-SUMMARY.md
started: 2026-03-04T08:30:00Z
updated: 2026-03-04T08:30:00Z
---

## Current Test

number: 1
name: Cold Start Smoke Test
expected: |
  Kill any running agtxd process. Build from scratch with `cargo build -p agtxd`. Start the daemon with `./target/debug/agtxd`. Server boots without errors, colored log output appears on stderr, and `curl http://127.0.0.1:3742/health` returns JSON with status, uptime_secs, and version fields.
awaiting: user response

## Tests

### 1. Cold Start Smoke Test
expected: Kill any running agtxd process. Build from scratch with `cargo build -p agtxd`. Start the daemon with `./target/debug/agtxd`. Server boots without errors, colored log output appears on stderr, and `curl http://127.0.0.1:3742/health` returns JSON with status, uptime_secs, and version fields.
result: [pending]

### 2. Health Endpoint
expected: With the daemon running, `curl http://127.0.0.1:3742/health` returns a JSON object containing at minimum: `status` (string like "ok"), `uptime_secs` (number that increases over time), and `version` (string matching Cargo.toml version).
result: [pending]

### 3. Task CRUD via REST API
expected: Using curl or similar: POST to `/api/v1/tasks` with a JSON body creates a task and returns it with an ID. GET `/api/v1/tasks` lists tasks including the one just created. GET `/api/v1/tasks/{id}` returns the specific task. DELETE `/api/v1/tasks/{id}` removes it. Subsequent GET returns 404.
result: [pending]

### 4. Graceful Shutdown
expected: With the daemon running, send SIGTERM (`kill <pid>`) or press Ctrl+C. The daemon logs a shutdown message and exits cleanly (exit code 0) without hanging or leaving orphaned processes.
result: [pending]

### 5. Structured JSON Log Files
expected: After running the daemon for a few seconds, check `~/.local/share/agtx/logs/`. A daily log file exists containing JSON-formatted log entries (one JSON object per line) with fields like timestamp, level, message, and target.
result: [pending]

### 6. Config Hot-Reload
expected: While the daemon is running, edit `~/.config/agtx/config.toml` and change the `[daemon] log_level` value (e.g., from "info" to "debug"). Within 1 second, the daemon logs a message indicating the log level changed. New log output reflects the updated level. No restart required.
result: [pending]

## Summary

total: 6
passed: 0
issues: 0
pending: 6
skipped: 0

## Gaps

[none yet]
