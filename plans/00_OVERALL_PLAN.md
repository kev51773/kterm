# kterm - Master Architectural Plan & Blueprint

`kterm` is a scriptable Windows-native terminal application using **Tauri v2**, **Rust** (`portable-pty` for ConPTY + Axum embedded daemon), and **TypeScript** (`xterm.js`).

---

## 1. System Architecture Overview

```
                      CLI Invocation (kterm.exe --select-tab ID --send-text ...)
                                        │
                                        ▼
                            ┌───────────────────────┐
                            │ Check 127.0.0.1:9999  │
                            └───────────┬───────────┘
                                        │
                  ┌─────────────────────┴─────────────────────┐
                  │ Daemon Running?                           │
                  ▼                                           ▼
                [YES]                                       [NO]
                  │                                           │
 ┌─────────────────────────────────┐        ┌─────────────────────────────────┐
 │ HTTP Client Mode                │        │ GUI Host Mode                   │
 │ 1. Parse CLI flags              │        │ 1. Start Axum Daemon (port 9999)│
 │ 2. Call HTTP REST API           │        │ 2. Launch Tauri Window + UI     │
 │ 3. Print output to stdout & exit│        │ 3. Execute initial CLI args     │
 └─────────────────────────────────┘        └─────────────────────────────────┘
```

---

## 2. Core Data Structures (Rust Backend)

```rust
// Shared Global State (Arc<RwLock<DaemonState>>)
pub struct DaemonState {
    pub windows: HashMap<String, WindowState>,
    pub tabs: HashMap<String, TabState>,
    pub splits: HashMap<String, SplitState>,
    pub active_window_id: String,
}

pub struct WindowState {
    pub id: String,                  // e.g. "win-1"
    pub tauri_window_label: String,  // Tauri window reference
    pub active_tab_id: Option<String>,
    pub layout_tree: LayoutNode,     // Root layout node
}

pub struct TabState {
    pub id: String,                  // e.g. "tab-101"
    pub window_id: String,
    pub title: String,
    pub profile: String,             // "powershell", "cmd", "wsl", "git-bash"
    pub badge: Option<String>,
    pub color: Option<String>,
    pub pty_pair: PtyPair,           // portable-pty Master/Slave pair
    pub pty_writer: Box<dyn std::io::Write + Send>,
    pub child_pid: u32,
    pub ring_buffer: Arc<RwLock<RingBuffer>>, // Recent output lines for --read-text
}

pub enum LayoutNode {
    Tab(String),
    Split {
        id: String,                  // e.g. "split-1"
        direction: SplitDirection,   // Horizontal | Vertical
        ratio: f32,                  // e.g. 0.5
        first: Box<LayoutNode>,
        second: Box<LayoutNode>,
    },
}
```

---

## 3. Complete CLI Syntax Matrix

| Command Category | Syntax | Description |
| :--- | :--- | :--- |
| **Spawning** | `kterm.exe [--profile P] [--window WIN] [--new-window]` | Spawn new tab |
| **Inspection** | `kterm.exe --list-windows` | List windows |
| | `kterm.exe --list-tabs [--window WIN] [--profile P] [--json]` | List tabs |
| **Input / Meta** | `kterm.exe --select-tab <ID/Title> --send-text <TEXT...>` | Inject raw input |
| | `kterm.exe --select-tab <ID/Title> --send-title <TITLE...>` | Set tab title |
| | `kterm.exe --select-tab <ID/Title> --set-badge <BADGE>` | Set visual badge |
| | `kterm.exe --select-tab <ID/Title> --set-color <COLOR>` | Set tab accent color |
| **Splits** | `kterm.exe --select-tab <ID> --split-right [--profile P]` | Split right |
| | `kterm.exe --select-tab <ID> --split-down [--profile P]` | Split down |
| | `kterm.exe --select-tab <ID> --split-down --move-tab <TAB_ID>` | Move tab into split |
| | `kterm.exe --select-tab <ID> --unsplit` | Detach pane to tab |
| | `kterm.exe --select-tab <ID> --explode-split` | Separate all split panes |
| **Sync / Read** | `kterm.exe --select-tab <ID> --read-text [--tail N]` | Read buffer |
| | `kterm.exe --select-tab <ID> --wait-for <TEXT...> [--timeout 30s]` | Block until match |
| **Focus / Close** | `kterm.exe --select-tab <ID> --focus` | Bring window to front |
| | `kterm.exe --select-tab <ID> --close [--force]` | Close tab |
| | `kterm.exe --close-window <WIN_ID>` | Close window |
| **Exporter** | `kterm.exe --export-script <FILE> [--format ps1|bat|bash]` | Export layout script |

---

## 4. Phase Execution Roadmap

1. **`plans/PHASE_1_POC.md`**: Core Tauri + Rust `portable-pty` setup, Axum daemon on port 9999, WebSocket PTY bridge, xterm.js rendering.
2. **`plans/PHASE_2_CLI_AND_MULTITAB.md`**: Clap CLI parser with tail-arg capture, single-instance client mode, HTTP client IPC dispatcher, multi-window & multi-tab state map, title targeting.
3. **`plans/PHASE_3_SPLITS_AND_UI.md`**: Target-based split engine (`/tabs/:id/split`), frontend split grid layout, split CLI commands (`--split-right`, `--split-down`, `--unsplit`, `--move-tab`).
4. **`plans/PHASE_4_AUTOMATION_AND_EXPORTER.md`**: Ring-buffer output tracker, `--read-text`, `--wait-for`, script exporter engine (`--export-script`), badges, colors.
5. **`plans/PHASE_5_SETTINGS_AND_POLISH.md`**: User `config.json` profile definitions, Settings UI modal, custom context menus, smart `Ctrl+C` copy, middle-click paste, shell padding, and extensible backlog.
