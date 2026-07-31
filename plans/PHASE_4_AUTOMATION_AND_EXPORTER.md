# Phase 4 Implementation Plan - Automation Primitives & Script Exporter

## Objective
Implement buffer reading (`--read-text`), pattern-matching sync waiting (`--wait-for`), layout script exporter engine (`--export-script`), visual badges & color accents, and full integration testing.

---

## 1. Buffer Tracking & Sync Engine (`src-tauri/src/pty/ring_buffer.rs`)

### Circular Ring Buffer
- Every `TabState` maintains a circular ring buffer (default 1,000 lines).
- As PTY output bytes arrive from master reader, clean ASCII/UTF-8 strings are appended to ring buffer.

### `GET /tabs/:id/read?tail=N`
- Returns recent `N` output lines from ring buffer as plain text or JSON.

### `POST /tabs/:id/wait`
```json
{
  "pattern": "Build Succeeded",
  "timeout_sec": 30
}
```
- Subscribes to tab output channel.
- Blocks HTTP request handler until regex/substring `pattern` matches incoming output line or `timeout_sec` elapses.
- Returns `200 OK` on match or `408 Request Timeout` on timeout.

---

## 2. Layout Script Exporter (`src-tauri/src/exporter.rs`)

Iterates over all windows, tabs, and split hierarchies in `DaemonState` and generates an executable script.

```rust
pub fn export_layout(state: &DaemonState, format: ScriptFormat) -> String {
    let mut script = String::new();
    match format {
        ScriptFormat::PowerShell => {
            // Emit $w1_t1 = & kterm.exe --new-window --profile powershell ...
        }
        ScriptFormat::Batch => {
            // Emit FOR /F "tokens=*" %%a IN ('kterm.exe ...') DO ...
        }
        ScriptFormat::Bash => {
            // Emit w1_t1=$(kterm.exe --new-window --profile powershell) ...
        }
    }
    script
}
```

---

## 3. Visual Badges & Accent Colors

- **API**: `PATCH /tabs/metadata` `{ "target": ["tab-101"], "badge": "PROD", "color": "#ff3333" }`.
- **UI**:
  - `badge`: Renders a high-contrast pill badge next to tab title (e.g. `[PROD]`).
  - `color`: Applies custom accent border color around tab header or pane outline.

---

## 4. Full Scripting Verification Workflow

Run a complete automated integration test script:

```powershell
# 1. Spawn base window
$t1 = .\kterm.exe --new-window --profile powershell
.\kterm.exe --select-tab $t1 --send-title "Main App"
.\kterm.exe --select-tab $t1 --set-badge "DEV"

# 2. Split right
$t2 = .\kterm.exe --select-tab $t1 --split-right --profile git-bash
.\kterm.exe --select-tab $t2 --send-title "Git Monitor"

# 3. Send text and wait for output
.\kterm.exe --select-tab $t1 --send-text "echo Hello_World_123`r"
.\kterm.exe --select-tab $t1 --wait-for "Hello_World_123" --timeout 10s

# 4. Export environment layout
.\kterm.exe --export-script setup-test.ps1

# 5. Verify exported script content
Get-Content setup-test.ps1
```
