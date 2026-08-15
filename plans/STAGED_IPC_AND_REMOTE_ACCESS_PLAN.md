# Staged Implementation Plan: Tauri IPC Migration & Remote Access

> **Goal:** Migrate `kterm` from a permanent Axum HTTP/WebSocket listener on TCP port `9999` to native Tauri IPC for the desktop frontend and a Windows Named Pipe (`\\\\.\\pipe\\kterm_daemon`) for CLI commands (0 open TCP ports in default state). In Stage 2, add an on-demand, password-protected Remote Access web server for mobile phone shell management.

---

## Workspace Context & Technical Architecture

### Existing Codebase Layout
- **Frontend (`src/`):** TypeScript + Vite web app rendered inside Tauri WebView2.
- **Backend (`src-tauri/`):** Tauri v2 + Rust application using `portable-pty` for ConPTY terminal process management.
- **CLI & Client (`src-tauri/src/client.rs`):** `kterm.exe` runs in host daemon mode (`--daemon`) or client mode (`kterm list-tabs`, `kterm new-tab`, etc.).
- **Autotest (`autotest/`):** WebdriverIO + Tauri WebDriver autotest suite testing the packaged release executable.

### Core Architecture Shift
1. **Desktop App (Stage 1):** 
   - **Frontend UI (`src/`):** Calls Tauri native `invoke()` commands for RPC and Tauri `Channel` / events for PTY stdout/stdin streams.
   - **CLI Mode (`kterm.exe`):** Connects to daemon via Windows Named Pipe (`\\\\.\\pipe\\kterm_daemon`).
   - **TCP Port 9999:** **Completely removed** from default startup. Zero listening network ports.
2. **Remote Access (Stage 2):**
   - **On-Demand Web Server:** Disabled by default. When enabled in Settings with a password, spawns an Axum server on `0.0.0.0:<port>`.
   - **Mobile Web UI (`src/remote_mobile/`):** Flattens all active desktop tabs & split panes into a touch-friendly list of selectable shells.
   - **Active Resizing (Option B):** PTY resizes to mobile screen width when active downstairs, and seamlessly restores full desktop dimensions (`120x30`) when desk regains focus.

---

## Mandatory Repository Workflow Rules (`AGENTS.md`)

Whenever modifying app code in `src/` or `src-tauri/`:
1. Make code changes in `src/` or `src-tauri/src/`.
2. **Rebuild Release Binary:** Run `npm run build` inside `autotest/` (tests exercise the packaged exe, never dev server).
3. **Run Test Suite:** Run `npm run test:gui:all` inside `autotest/` in an **elevated (Admin) PowerShell prompt**.
4. **Visual Review Rule:** If screenshots differ, DO NOT auto-promote baselines. Run `npm run review` for human verdict.

---

## Stage 1: Core IPC & Named Pipe Migration

### 1.1 Rust Backend: Tauri Commands & Streaming
- **[NEW] [`src-tauri/src/ipc.rs`](file:///c:/Users/Kev/Desktop/Terminal/src-tauri/src/ipc.rs)**
  - Implement `#[tauri::command]` functions:
    - `list_tabs(state)`
    - `spawn_tab(state, profile, window_id)`
    - `close_tab(state, targets)`
    - `split_tab(state, tab_id, direction, profile)`
    - `set_title(state, targets, title)`
    - `set_badge(state, targets, badge)`
    - `set_color(state, targets, color)`
    - `send_text(state, targets, command)`
    - `read_text(state, tab_id, tail)`
    - `attach_pty(state, tab_id, on_data_channel)`
  - Map PTY session reader thread output directly to `on_data_channel.send()`.
- **[MODIFY] [`src-tauri/src/main.rs`](file:///c:/Users/Kev/Desktop/Terminal/src-tauri/src/main.rs)**
  - Remove `daemon::run_server("127.0.0.1:9999", ...)` call from daemon setup.
  - Register `ipc` command handlers in `tauri::Builder::default().invoke_handler(tauri::generate_handler![...])`.

### 1.2 Rust Backend: Windows Named Pipe Server
- **[NEW] [`src-tauri/src/named_pipe.rs`](file:///c:/Users/Kev/Desktop/Terminal/src-tauri/src/named_pipe.rs)**
  - Implement async Tokio loop listening on `\\\\.\\pipe\\kterm_daemon`.
  - Parse CLI request frames (JSON RPC: `{ action: "list_tabs" | "spawn_tab" | ... }`).
  - Route to `AppState` handlers and write JSON response frame back to pipe handle.
- **[MODIFY] [`src-tauri/src/main.rs`](file:///c:/Users/Kev/Desktop/Terminal/src-tauri/src/main.rs)**
  - Spawn `named_pipe::start_server(daemon_state)` inside `tauri::async_runtime::spawn`.

### 1.3 Rust Backend: Client Mode Named Pipe Connection
- **[MODIFY] [`src-tauri/src/client.rs`](file:///c:/Users/Kev/Desktop/Terminal/src-tauri/src/client.rs)**
  - Update `is_daemon_running()` to check if `\\\\.\\pipe\\kterm_daemon` can be opened via `std::fs::OpenOptions`.
  - Update `handle_client_mode()` to send/receive JSON RPC frames over `\\\\.\\pipe\\kterm_daemon` instead of `reqwest` HTTP calls to `127.0.0.1:9999`.

### 1.4 Desktop Frontend Refactor
- **[MODIFY] [`src/config.ts`](file:///c:/Users/Kev/Desktop/Terminal/src/config.ts)**
  - Remove `DAEMON_URL` and `WS_URL` static constants.
  - Export typed `@tauri-apps/api/core` `invoke` helper wrappers.
- **[MODIFY] [`src/main.ts`](file:///c:/Users/Kev/Desktop/Terminal/src/main.ts)**
  - Replace `new WebSocket(...)` in PTY connection setup with Tauri `Channel` listener.
  - Bind xterm.js `onData` directly to `invoke("send_pty_input", { tabId, data })`.

### 1.5 Autotest Helper Update
- **[MODIFY] [`autotest/src/helpers/daemon.ts`](file:///c:/Users/Kev/Desktop/Terminal/autotest/src/helpers/daemon.ts)**
  - Replace direct HTTP `fetch('http://127.0.0.1:9999/...')` calls with CLI binary child process execution (`execFileSync(KTERM_EXE, ['list-tabs', '--json'])`) or named pipe IPC client.

### 1.6 Stage 1 Verification
```pwsh
# 1. Rebuild packaged binary
cd c:\Users\Kev\Desktop\Terminal\autotest
npm run build

# 2. Confirm no TCP port 9999 is open
netstat -ano | findstr 9999

# 3. Run full GUI & CLI test suite (elevated prompt)
npm run test:gui:all
```

---

## Stage 2: On-Demand Remote Access Web Server & Mobile Web UI

### 2.1 On-Demand Server & Auth
- **[NEW] [`src-tauri/src/remote.rs`](file:///c:/Users/Kev/Desktop/Terminal/src-tauri/src/remote.rs)**
  - Axum web server module starting/stopping on command.
  - Password hashing & token verification middleware (`/auth/login`).
  - Serves static assets for mobile client + WebSocket PTY multiplexer (`/ws/remote`).
  - Active Resizing Handler (Option B): Handles PTY resize to mobile dimensions on mobile focus, restores desktop dimensions (`120x30`) on mobile disconnect / desktop focus.

### 2.2 Mobile Web UI
- **[NEW] [`src/remote_mobile/`](file:///c:/Users/Kev/Desktop/Terminal/src/remote_mobile)**
  - **Flat Shell List View:** Displays all active tabs & split panes as simple selectable shell cards ("Shell 1: PowerShell", "Shell 2: git bash").
  - **Terminal View:** Full-screen xterm.js instance tuned for touch devices.
  - **Sticky Keybar:** Rendered above touch keyboard: `[Esc] [Tab] [Ctrl] [▲] [▼] [◄] [►]`.

### 2.3 Settings Integration
- **[MODIFY] [`src/settings.ts`](file:///c:/Users/Kev/Desktop/Terminal/src/settings.ts)**
  - Add Remote Access controls: Enable toggle switch, Port field (default 8080), Password setup input, and Active Connection status badge.

### 2.4 Stage 2 Verification
1. Enable Remote Access in Settings, set password.
2. Open `http://<PC-LAN-IP>:8080` from mobile phone / browser.
3. Verify password authentication succeeds.
4. Verify flat shell list allows selecting any active shell.
5. Verify Option B active resizing scales PTY output cleanly on phone and restores desktop size on disconnect.
6. Disable Remote Access in Settings and verify network port closes immediately (`netstat -ano | findstr 8080`).

---

## File Summary Table for Fresh Agent

| Action | Path | Purpose |
| :--- | :--- | :--- |
| **NEW** | [`src-tauri/src/ipc.rs`](file:///c:/Users/Kev/Desktop/Terminal/src-tauri/src/ipc.rs) | Tauri IPC RPC commands & PTY Channel streaming |
| **NEW** | [`src-tauri/src/named_pipe.rs`](file:///c:/Users/Kev/Desktop/Terminal/src-tauri/src/named_pipe.rs) | Windows Named Pipe listener for `kterm.exe` CLI |
| **NEW** | [`src-tauri/src/remote.rs`](file:///c:/Users/Kev/Desktop/Terminal/src-tauri/src/remote.rs) | On-demand password-protected Axum web server |
| **NEW** | [`src/remote_mobile/`](file:///c:/Users/Kev/Desktop/Terminal/src/remote_mobile) | Mobile HTML/JS Web UI (flat shell list + keybar) |
| **MODIFY** | [`src-tauri/src/main.rs`](file:///c:/Users/Kev/Desktop/Terminal/src-tauri/src/main.rs) | Register IPC commands, spawn named pipe listener |
| **MODIFY** | [`src-tauri/src/client.rs`](file:///c:/Users/Kev/Desktop/Terminal/src-tauri/src/client.rs) | Connect CLI mode to named pipe instead of HTTP 9999 |
| **MODIFY** | [`src/config.ts`](file:///c:/Users/Kev/Desktop/Terminal/src/config.ts) | Use Tauri `@tauri-apps/api` invoke/events |
| **MODIFY** | [`src/main.ts`](file:///c:/Users/Kev/Desktop/Terminal/src/main.ts) | Wire xterm.js PTY stream to Tauri IPC Channel |
| **MODIFY** | [`autotest/src/helpers/daemon.ts`](file:///c:/Users/Kev/Desktop/Terminal/autotest/src/helpers/daemon.ts) | Execute CLI test calls over Named Pipe |
