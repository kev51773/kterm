# Phase 1 Implementation Plan - Minimal Working PoC

## Objective
Scaffold the Tauri v2 + Rust + TypeScript repository, implement the `portable-pty` ConPTY engine, spawn an embedded Axum background daemon on `127.0.0.1:9999`, and render a live interactive xterm.js terminal stream in the Web UI.

---

## 1. Environment & Dependencies

### `src-tauri/Cargo.toml`
```toml
[package]
name = "kterm"
version = "0.1.0"
edition = "2021"

[dependencies]
tauri = { version = "2.0", features = [] }
portable-pty = "0.8"
tokio = { version = "1.38", features = ["full"] }
axum = { version = "0.7", features = ["ws"] }
tower-http = { version = "0.5", features = ["cors"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tracing = "0.1"
tracing-subscriber = "0.3"
```

### `package.json`
```json
{
  "name": "kterm-ui",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "tauri": "tauri"
  },
  "dependencies": {
    "@tauri-apps/api": "^2.0.0",
    "@xterm/xterm": "^5.5.0",
    "@xterm/addon-fit": "^0.10.0"
  },
  "devDependencies": {
    "typescript": "^5.4.0",
    "vite": "^5.2.0"
  }
}
```

---

## 2. Component Implementation Details

### A. PTY Engine (`src-tauri/src/pty/manager.rs`)
- Use `portable_pty::native_pty_system()`.
- Spawn `CommandBuilder::new("powershell.exe")` or `cmd.exe`.
- Setup master reader loop: read bytes from PTY master and broadcast over tokio broadcast channel.
- Hold PTY writer handle in thread-safe state for writing incoming HTTP/WebSocket bytes.

### B. Axum Daemon (`src-tauri/src/daemon/mod.rs`)
- Spawn background tokio task in `tauri::Builder::setup` listening on `127.0.0.1:9999`.
- **Endpoints**:
  - `GET /tabs`: Returns active tab list JSON (`[{ "id": "tab-101", "pid": 1234, "profile": "powershell" }]`).
  - `POST /tabs`: Spawns new PTY session, returns `{ "id": "tab-101", "pid": 1234 }`.
  - `POST /tabs/:id/send`: Body `{ "command": "string" }` -> writes to PTY writer.
  - `GET /tabs/:id/ws`: Axum WebSocket endpoint. Receives incoming websocket frames and writes to PTY; pipes PTY broadcast channel chunks out to websocket.

### C. Web UI (`src/main.ts`)
- Initialize `Terminal` from `@xterm/xterm` with `@xterm/addon-fit`.
- Connect to `ws://127.0.0.1:9999/tabs/tab-101/ws`.
- Bind `term.onData(data => ws.send(data))` and `ws.onmessage = evt => term.write(evt.data)`.

---

## 3. Step-by-Step Execution Guide

1. Scaffold project directory:
   ```powershell
   npx -y create-vite ./ --template vanilla-ts
   cargo init src-tauri
   ```
2. Create `src-tauri/src/pty/manager.rs` and `src-tauri/src/daemon/mod.rs`.
3. Integrate Axum daemon in `src-tauri/src/main.rs`:
   ```rust
   #[tokio::main]
   async fn main() {
       let state = Arc::new(RwLock::new(DaemonState::default()));
       let daemon_state = state.clone();
       
       tokio::spawn(async move {
           daemon::run_server("127.0.0.1:9999", daemon_state).await;
       });

       tauri::Builder::default()
           .run(tauri::generate_context!())
           .expect("error while running tauri application");
   }
   ```

---

## 4. Verification Checkpoints

- **Build Check**: `cargo check` inside `src-tauri` completes with zero errors.
- **Daemon Verification**:
  ```powershell
  # Spawn session via REST
  $res = Invoke-RestMethod -Method Post -Uri http://localhost:9999/tabs -ContentType "application/json" -Body '{"profile":"powershell"}'
  # List sessions
  Invoke-RestMethod -Method Get -Uri http://localhost:9999/tabs
  # Send text sequence
  Invoke-RestMethod -Method Post -Uri "http://localhost:9999/tabs/$($res.id)/send" -ContentType "application/json" -Body '{"command":"dir\r"}'
  ```
- **UI Verification**: Terminal window renders PowerShell prompt, responds to typing, and renders `dir` command output dynamically.
