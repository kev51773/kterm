# kterm Codebase Reference

Per-file, per-function reference for every source file.

---

## Project Structure

```
kterm/
├── src-tauri/
│   ├── src/
│   │   ├── main.rs          Entry point, CLI dispatch, Tauri IPC & daemon setup
│   │   ├── cli.rs           Clap CLI argument definitions and help text
│   │   ├── client.rs        CLI named pipe client, IPC dispatch, daemon spawn
│   │   ├── ipc.rs           Tauri native IPC commands and event dispatch
│   │   ├── named_pipe.rs    Win32 Named Pipe server (\\.\pipe\kterm_daemon)
│   │   ├── remote.rs        Axum web server & WebSocket handler for Remote Access
│   │   ├── config.rs        JSON config persistence, window sizing
│   │   ├── yaml.rs          YAML parsing, validation, application
│   │   ├── exporter.rs      YAML export, Windows .lnk shortcut creation
│   │   ├── daemon/          Modular daemon endpoints (tabs, windows, splits, export, session, config)
│   │   └── pty/             Modular PTY manager (session, platform, elevated, layout, ring_buffer)
│   └── Cargo.toml
├── src/
│   ├── main.ts              App entry point, module bootstrap, global event listeners
│   ├── state.ts             Central state store, tab/pane interfaces
│   ├── config.ts            Config persistence & dynamic CSS theme applier
│   ├── daemon.ts            Tauri IPC client wrapper & state synchronizer
│   ├── terminal.ts          xterm.js instance lifecycle & pane DOM manager
│   ├── tabs.ts              Tab header strip renderer, focus & group switcher
│   ├── splits.ts            Split layout renderer, divider handlers, node operations
│   ├── findBar.ts           In-buffer search overlay & match highlighter
│   ├── highlights.ts        Word highlight overlay & flashing animation engine
│   ├── attentionBell.ts     Long-task duration monitor, audio chime & notification sender
│   ├── style.css            Complete CSS design system
│   ├── utils/
│   │   └── detector.ts      Path (Win/POSIX/WSL/Git Bash) & URL parser with tests
│   └── components/
│       ├── SplitGrid.ts     Split pane DOM renderer
│       ├── SettingsModal.ts Settings dialog (Appearance, General, Keybinds, Remote, Attention)
│       ├── ProfileDropdown.ts Profile launcher & Remote Access / Export trigger
│       ├── RemoteAccessModal.ts Remote Access configuration & QR code dialog
│       ├── ContextMenu.ts   Terminal pane & tab context menu with path/URL actions
│       ├── InputModal.ts    Text entry modal dialog
│       ├── HighlightsModal.ts Highlight manager dialog
│       └── TabBar.ts        Tab strip scroll buttons & drag spacer
├── web/
│   ├── index.html           Standalone web terminal interface with shell picker
│   ├── mobile-keyboard.js  Custom touch keyboard for mobile remote shell
│   └── mobile.html          Mobile touch terminal view
├── index.html
├── package.json
└── vite.config.ts
```

---

## Rust Backend

### `main.rs` — Entry Point

**Lines**: 167

| Line | Function | Input | Output | Responsibility |
|------|----------|-------|--------|---------------|
| 12 | `main()` | CLI args | process exit | Parses args, routes to daemon or client mode |
| 18 | (arg rewriting) | raw args | modified args | Detects positional `.yaml` args, inserts `--apply` |
| 44 | (elevated bridge) | pipe name, profile | process exit | If `--elevated-pty-bridge` set, runs bridge mode |
| 48 | (daemon check) | args | | If `--daemon`, runs host daemon |
| 56 | (client mode) | args | | Ensures daemon running, dispatches client command |
| 71 | `run_host_daemon(args)` | CliArgs | never returns | Creates PtyManager, AppState, Tauri app, Axum server |

**Key flow**: `main()` → arg rewrite → `CliArgs::parse_from()` → if elevated bridge → if daemon → else client mode.

**State created in `run_host_daemon()`**:
- `PtyManager`: session map, spawn/kill
- `window_titles: Arc<Mutex<HashMap<String, String>>>` — window label → formatted title
- `window_layouts: Arc<Mutex<HashMap<String, Vec<LayoutNode>>>>` — window label → layout trees
- `AppState` bundle passed to Axum server and Tauri event handlers

**Tauri lifecycle hooks**:
- `setup`: pre-flight purge win-1, apply YAML file if `--apply` passed, configure default window
- `on_window_event(Destroyed)`: cleanup PTY sessions, exit if last window

---

### `cli.rs` — Argument Definitions

**Lines**: 268

| Item | Responsibility |
|------|---------------|
| `CliArgs` struct (line 24) | All CLI flags as Clap `#[arg]` fields |
| `print_help()` (line 132) | Manual help text with examples |

**Fields** (all `Option`/`bool`):
- Shell: `profile`, `admin`, `elevated_pty_bridge`
- Window: `window`, `new_window`, `list_windows`, `list_tabs`, `json`
- Tab selection: `select_tab` (Vec)
- Tab actions: `send_text`, `send_title`, `set_window_title`, `set_badge`, `set_color`, `focus`, `close`, `close_window`, `force`
- Split: `split_right`, `split_left`, `split_down`, `split_up`, `move_tab`, `unsplit`, `explode_split`
- YAML: `apply`, `export_layout`, `suffix`, `suffix_auto`, `dry_run`
- Read/wait: `read_text`, `tail`, `raw`, `wait_for`, `from_history`, `wait_for_prompt`, `timeout`
- Mode: `daemon`

---

### `client.rs` — CLI Named Pipe Client

| Function | Input | Output | Responsibility |
|---|---|---|---|
| `is_daemon_running()` | — | bool | Checks if `\\.\pipe\kterm_daemon` named pipe exists |
| `safe_println(msg)` | &str | stdout | Writes line to stdout (handles broken pipe gracefully) |
| `spawn_daemon_detached(exe_path)` | Path | Result | Win32 `CreateProcessW` with `DETACHED_PROCESS` flag |
| `ensure_daemon_running()` | — | Result | Checks pipe → spawns daemon if missing → retries with `ERROR_PIPE_BUSY` wait |
| `handle_client_mode(args)` | &CliArgs | Result | Main CLI dispatch: serialize args to JSON, send over named pipe, output result |

**`handle_client_mode` dispatch order**:
1. `--apply` → POST `/apply`
2. `--export-layout` → GET `/export-layout` + write file + create shortcut
3. `--new-window` → POST `/windows`
4. `--set-window-title` → POST `/windows/title`
5. `--close-window` → POST `/windows/close`
6. `--list-windows` → GET `/windows`
7. `--list-tabs` → GET `/tabs`
8. Split/unsplit/explode → POST `/tabs/:id/split|unsplit|explode`
9. Tab actions (send/title/badge/color/focus/close) → POST `/tabs/send|title|badge|color|focus|close`
10. `--read-text` → GET `/tabs/:id/read`
11. `--wait-for` / `--wait-for-prompt` → POST `/tabs/:id/wait`
12. Default → POST `/tabs` (spawn tab)

---

### `ipc.rs` — Tauri Native IPC Bridge

| Function / Handler | Input | Output | Responsibility |
|---|---|---|---|
| `invoke_cmd` / commands | IPC payload | JSON Result | Handles desktop GUI commands (`list_tabs`, `create_tab`, `send_text`, `close_tabs`, `split_tab`, etc.) via native Tauri IPC |
| `emit_layout_change` | Window label | Event | Emits event updates to frontend UI on state changes |

---

### `named_pipe.rs` — Win32 Named Pipe Listener

| Function | Input | Output | Responsibility |
|---|---|---|---|
| `start_named_pipe_server` | AppState | Task | Listens on `\\.\pipe\kterm_daemon` with `ERROR_PIPE_BUSY` handle retries for CLI client commands |
| `handle_client_connection` | Pipe stream, AppState | Result | Deserializes CLI request JSON, dispatches action to `PtyManager`/`LayoutNode`, returns response |

---

### `remote.rs` — Remote Access Web Server & Mobile Interface

| Endpoint / Function | Input | Output | Responsibility |
|---|---|---|---|
| `start_remote_server` | Port, Password, AppState | Axum Server | Starts on-demand password-protected HTTP & WebSocket server |
| `GET /` & `/web/*` | HTTP Request | HTML/JS/CSS | Serves mobile web UI (`web/index.html`, `web/mobile-keyboard.js`) |
| `POST /api/auth` | Password | Auth Token / Cookie | Authenticates remote web sessions |
| `GET /tabs/:id/ws` | WebSocket | PTY I/O Stream | Bidirectional terminal stream for web client with active resizing |

---

### `config.rs` — Configuration Persistence

**Lines**: 167

| Line | Item | Input | Output | Responsibility |
|------|------|-------|--------|---------------|
| 4 | `FontConfig` | — | struct | `{ family: String, size: u32 }` |
| 15 | `ThemeConfig` | — | struct | 9 color fields with serde defaults |
| 78 | `AppConfig` | — | struct | All config fields: profile, cols/rows, ring_buffer_kb, padding, font, theme |
| 102 | `AppConfig::get_config_path()` | — | PathBuf | `%APPDATA%/kterm/config.json` or `~/.config/kterm/config.json` |
| 110 | `AppConfig::load()` | — | Self | Read JSON from disk, fallback to defaults, save |
| 122 | `AppConfig::get_default_window_size()` | — | (f64, f64) | cols×cell_w + padding → logical pixel dimensions |
| 134 | `AppConfig::save()` | — | Result | Write JSON to config path |

**Window size formula**: `w = cols * 8.42 + 16 + padding*2`, `h = rows * 17 + 41 + padding*2` (ceil).

---

### `yaml.rs` — YAML Session Engine

**Lines**: 343

| Line | Function | Input | Output | Responsibility |
|------|----------|-------|--------|---------------|
| 10 | `parse_yaml(content)` | &str | Result<YamlSessionSpec> | `serde_yaml::from_str` |
| 14 | `validate_yaml(spec)` | &YamlSessionSpec | Result | Check tabs non-empty, validate each tab/split |
| 20 | `validate_tab(tab)` | &YamlTabSpec | Result | Validate profile, recurse splits |
| 27 | `validate_split(split)` | &YamlSplitSpec | Result | Validate profile, direction, recurse |
| 35 | `validate_profile(profile)` | &str | Result | Must be powershell/cmd/wsl/git-bash/bash |
| 41 | `generate_next_window_id(state)` | &AppState | String | Find next unused `win-N` |
| 50 | `is_window_untouched_initial(state, id)` | (&AppState, &str) | bool | True if no tabs in window |
| 54 | `resolve_window_id(state, base, suffix, auto)` | (&AppState, &str, Option<&str>, bool) | String | Apply suffix/auto-increment logic |
| 74 | `apply_yaml_spec(state, spec, override, suffix, auto)` | (&AppState, &YamlSessionSpec, ...) | Result<String> | Validate → resolve window → create/clear → spawn tabs → build layout tree |
| 176 | `apply_split_recursive(state, win_id, parent, root_id, split)` | ... | Result | Recursively spawn split panes, attach to layout tree |
| 220 | `send_text_to_session(sess, text)` | (&PtySession, &str) | | Write bytes to PTY stdin |

**YAML structs**:
- `YamlSessionSpec { window: YamlWindowSpec, tabs: Vec<YamlTabSpec> }`
- `YamlWindowSpec { id: Option<String>, title: Option<String> }`
- `YamlTabSpec { id, profile, title, badge, color, cwd, admin, elevated, send_text, splits: Option<Vec<YamlSplitSpec>> }`
- `YamlSplitSpec { direction, profile, id, title, badge, color, cwd, admin, elevated, send_text, splits }`

---

### `exporter.rs` — Layout Export

**Lines**: 246

| Line | Function | Input | Output | Responsibility |
|------|----------|-------|--------|---------------|
| 8 | `export_yaml_spec(state, window_id)` | (&AppState, &str) | YamlSessionSpec | Convert live layout tree → YAML spec |
| 43 | `export_yaml_layout(state, window_id)` | (&AppState, &str) | String | Serialize spec to YAML string |
| 48 | `convert_node_to_tab(node, sessions, processed)` | (&LayoutNode, &[Arc<PtySession>], &mut Vec<String>) | Option<YamlTabSpec> | Recursive layout node → tab spec |
| 98 | `convert_node_to_split(node, dir, sessions, processed)` | (...) | Option<YamlSplitSpec> | Recursive layout node → split spec |
| 139 | `export_shortcut_for_yaml(yaml_path_str)` | &str | Result<String> | Create Windows `.lnk` shortcut via PowerShell COM |

---

### `daemon/mod.rs` — HTTP Server

**Lines**: 1247

**State structs** (lines 23-146):
- `AppState { pty_manager, app_handle, window_titles, window_layouts }`
- `TabInfo` — JSON response for tab listing
- Request types: `CreateTabRequest`, `SendTextRequest`, `SetTitleRequest`, `SetBadgeRequest`, `SetColorRequest`, `TargetTabRequest`, `ApplySessionRequest`, `CloseWindowRequest`, `ResizeRequest`, `SplitTabRequest`, `UpdateRatioRequest`

**Route table** (lines 148-194):

| Method | Path | Handler | Description |
|--------|------|---------|-------------|
| GET | `/health` | `health_check` | 200 OK |
| GET | `/layout` | `get_window_layout` | Get layout tree for window |
| POST | `/layout/ratio` | `update_layout_ratio` | Update split ratio |
| GET | `/tabs` | `list_tabs` | List all tabs (optional `?window=`) |
| POST | `/tabs` | `create_tab` | Spawn new tab |
| POST | `/tabs/send` | `send_text` | Send text to tabs |
| POST | `/tabs/title` | `set_title` | Set tab title |
| POST | `/tabs/badge` | `set_badge` | Set tab badge |
| POST | `/tabs/color` | `set_color` | Set tab color |
| POST | `/tabs/close` | `close_tabs` | Close tabs |
| POST | `/tabs/focus` | `focus_tabs` | Focus (no-op) |
| POST | `/tabs/:id/resize` | `resize_tab` | Resize PTY |
| POST | `/tabs/:id/split` | `split_tab` | Split pane |
| POST | `/tabs/:id/unsplit` | `unsplit_tab` | Remove from split |
| POST | `/tabs/:id/explode` | `explode_tab` | Flatten split tree |
| GET | `/tabs/:id/read` | `read_tab_buffer` | Read ring buffer tail |
| POST | `/tabs/:id/wait` | `wait_tab_output` | Block until pattern match |
| GET | `/windows` | `list_windows` | List all windows |
| POST | `/windows` | `create_window` | Create new Tauri window |
| POST | `/windows/title` | `set_window_title` | Set window title |
| POST | `/windows/close` | `close_window` | Close window + cleanup |
| POST | `/windows/show` | `show_window` | Show + focus window |
| POST | `/windows/size` | `resize_window` | Resize window |
| GET | `/tabs/:id/ws` | `ws_handler` | WebSocket upgrade for terminal I/O |
| POST | `/apply` | `apply_session` | Apply YAML spec |
| GET | `/export-layout` | `export_layout_endpoint` | Export layout as YAML |
| POST | `/export-shortcut` | `export_shortcut_endpoint` | Create .lnk shortcut |
| GET | `/config` | `get_config` | Get app config |
| POST | `/config` | `update_config` | Save app config |
| GET | `/build_id` | `get_build_id` | Return cargo version |
| POST | `/shutdown` | `shutdown_daemon` | Exit process |
| GET | `/clipboard` | `get_clipboard` | Read Windows clipboard |

**Key handlers**:

| Line | Handler | Notes |
|------|---------|-------|
| 215 | `auto_close_tab` | PTY exit callback: remove from layout, close session |
| 271 | `create_tab` | Spawn PTY, add to layout tree |
| 330 | `send_text` | Resolve tabs, append `\r`, write to PTY stdin |
| 473 | `close_tabs` | Remove from layout, close sessions, close window if empty |
| 613 | `ws_handler` | WebSocket: replay history, bidirectional relay with resize messages |
| 683 | `split_tab` | Create/move tab, update layout tree with `split_at()` |
| 767 | `unsplit_tab` | Remove pane from split, add as standalone |
| 794 | `explode_tab` | Flatten split tree into standalone tabs |
| 881 | `apply_session` | Parse YAML, validate, apply via `yaml::apply_yaml_spec` |
| 1025 | `wait_tab_output` | Subscribe to broadcast, poll pattern match with timeout |

---

### `pty/mod.rs` — Module Re-exports

**Lines**: 8

Re-exports: `LayoutNode`, `SplitDirection`, `PtyManager`, `PtySession`, `RingBuffer`.

---

### `pty/manager.rs` — PTY Management

**Lines**: 1080

| Line | Function | Input | Output | Responsibility |
|------|----------|-------|--------|---------------|
| 7 | `debug_log(msg)` | &str | file | Write to `admin_debug.log` |
| 52 | `PtySession::resize(rows, cols)` | (u16, u16) | | Resize PTY + update stored dimensions |
| 66 | `PtySession::get_output_history()` | — | Vec<u8> | Clone output_buffer (64KB cap) |
| 70 | `PtySession::kill_by_pid()` | — | | Win32 `TerminateProcess` or `SIGKILL` |
| 94 | `PtySession::is_alive()` | — | bool | Check `is_dead` flag + process status |
| 106 | `is_app_elevated()` | — | bool | Win32 token elevation check |
| 132 | `is_process_alive(pid)` | u32 | bool | Win32 `GetExitCodeProcess` == STILL_ACTIVE |
| 160 | `run_elevated_pty_bridge(id, profile)` | (&str, &str) | never | Named pipe relay: pipe_in → PTY stdin, PTY stdout → pipe_out |
| 314 | `PtyManager::new()` | — | Self | Empty session map |
| 328 | `PtyManager::set_exit_callback(cb)` | F | | Store PTY exit callback |
| 336 | `PtyManager::spawn(id, profile, window_id)` | (...) | Result<Arc<PtySession>> | Convenience wrapper for `spawn_with_cwd` |
| 345 | `PtyManager::spawn_with_cwd(id, profile, window_id, cwd, elevated)` | (...) | Result<Arc<PtySession>> | Load config for default cols/rows, delegate |
| 359 | `PtyManager::generate_next_tab_id_for_window(window_id)` | &str | String | Find next unused `tab-N` in window |
| 386 | `PtyManager::spawn_with_size_and_cwd(id, profile, window_id, cols, rows, cwd, elevated)` | (...) | Result<Arc<PtySession>> | **Core spawn**: open PTY → build command → spawn child → optional elevation via named pipes → reader thread → create session |
| 420 | `create_win32_named_pipe_handle(pipe_name)` | &str | Result<HANDLE> | Create named pipe with NULL DACL |
| 493 | `connect_win32_named_pipe(handle)` | HANDLE | Result<File> | `ConnectNamedPipe` with 30s timeout |
| 770 | `PtyManager::get(id)` | &str | Option<Arc<PtySession>> | Lookup by ID |
| 774 | `PtyManager::get_in_window(id, window_id)` | (&str, Option<&str>) | Option<Arc<PtySession>> | Lookup by ID + optional window |
| 796 | `PtyManager::prune_dead_sessions()` | — | Vec<String> | Remove dead sessions, return removed IDs |
| 813 | `PtyManager::list_by_window(window_id)` | Option<&str> | Vec<Arc<PtySession>> | List sessions, prune dead, sort by tab number |
| 837 | `PtyManager::close(id)` | &str | bool | Kill child, remove from map |
| 841 | `PtyManager::close_in_window(id, window_id)` | (&str, Option<&str>) | bool | Window-scoped close |
| 883 | `PtyManager::close_all_for_window(window_id)` | &str | | Close all sessions in window |
| 890 | `PtyManager::resolve_targets(targets)` | &[String] | Vec<Arc<PtySession>> | Resolve by ID, title, or "active" |
| 923 | `PtyManager::resolve_targets_strict(targets, window)` | (&[String], Option<&str>) | Result<Vec<...>> | Strict resolve with ambiguity errors |
| 997 | `assign_pid_to_job(pid)` | u32 | | Win32 job object with `KILL_ON_JOB_CLOSE` |

**Elevated PTY flow** (Windows):
1. Create two named pipes (`kterm_pipe_in_{uuid}`, `kterm_pipe_out_{uuid}`)
2. `ShellExecuteW("runas")` to launch self with `--elevated-pty-bridge`
3. `ConnectNamedPipe` waits for bridge process to connect
4. Bridge opens pipes, spawns shell, relays stdin/stdout between pipes and PTY

**Session map key**: `"{window_id}:{tab_id}"`

---

### `pty/layout.rs` — Split Tree

**Lines**: 141

| Line | Function | Input | Output | Responsibility |
|------|----------|-------|--------|---------------|
| 29 | `LayoutNode::contains_tab(target_id)` | &str | bool | Recursive tree search |
| 39 | `LayoutNode::collect_tabs()` | — | Vec<String> | Flatten tree to tab ID list |
| 55 | `LayoutNode::split_at(target_tab_id, direction, new_tab_id, insert_first)` | (&str, SplitDirection, &str, bool) | bool | Replace Pane with Split node at target |
| 95 | `LayoutNode::remove_tab(target_tab_id)` | &str | bool | Remove pane, promote sibling |
| 124 | `LayoutNode::unsplit_pane(target_tab_id)` | &str | bool | Alias for `remove_tab` |
| 128 | `LayoutNode::update_ratio(split_id, new_ratio)` | (&str, f32) | bool | Update split ratio (clamped 0.1-0.9) |

**Enums**:
- `SplitDirection { Horizontal, Vertical }` — serde `lowercase`
- `LayoutNode::Pane { tab_id }` | `LayoutNode::Split { id, direction, ratio, first, second }` — serde tagged `type`

---

### `pty/ring_buffer.rs` — Output Buffer

**Lines**: 217

| Line | Function | Input | Output | Responsibility |
|------|----------|-------|--------|---------------|
| 21 | `RingBuffer::new()` | — | Self | 256KB capacity |
| 25 | `RingBuffer::with_capacity(capacity)` | usize | Self | Custom capacity |
| 33 | `RingBuffer::append(data)` | &[u8] | | Push bytes, evict oldest when full |
| 45 | `RingBuffer::get_total_bytes_written()` | — | usize | Lifetime byte counter |
| 49 | `RingBuffer::get_raw_bytes()` | — | Vec<u8> | Clone buffer contents |
| 54 | `RingBuffer::get_text(strip_ansi)` | bool | String | UTF-8 lossy, optional ANSI strip |
| 64 | `RingBuffer::get_text_from_offset(start_offset, strip)` | (usize, bool) | String | Text since byte offset |
| 83 | `RingBuffer::read_tail_lines(n, strip)` | (usize, bool) | Vec<String> | Last N non-blank lines |
| 100 | `RingBuffer::contains_pattern(pattern)` | &str | bool | Full-buffer pattern check |
| 104 | `RingBuffer::contains_pattern_from_offset(offset, pattern)` | (usize, &str) | bool | Pattern check from byte offset |
| 127 | `RingBuffer::matches_prompt()` | — | bool | Last line ends with `>`, `$`, `#`, or contains `PS ` |
| 147 | `strip_ansi(input)` | &str | String | Remove CSI/OSC sequences, convert cursor positioning to newlines |

---

## TypeScript Frontend

### Core Modules (`src/`)

| Module | Responsibility | Key Functions / Exports |
|---|---|---|
| `main.ts` | Entry point & listener setup | App initialization, global keybindings wiring, Tauri IPC listener registration |
| `state.ts` | State store | `state` object, `getAppState()`, `setAppState()`, `TabData`, `PaneInstance` interfaces |
| `config.ts` | Config & Theme | `loadAppConfig()`, `saveAppConfig()`, `applyAppConfig()` dynamic CSS variable binding |
| `daemon.ts` | Tauri IPC & Sync | `syncTabs()`, `invokeTauri()`, tab state reconciliation loop |
| `terminal.ts` | xterm.js Pane Manager | `createTabLocal()`, `removeTabLocal()`, `adjustWindowForGrid()`, terminal paste handler |
| `tabs.ts` | Tab Header Strip | `renderTabBarHeaders()`, `selectTab()`, `closeTab()`, tab group navigation |
| `splits.ts` | Split Layout Operations | `renderActiveLayout()`, `splitPane()`, `unsplitPane()`, `explodeSplit()` |
| `findBar.ts` | Search Overlay | `showFindBar()`, `closeFindBar()`, search addon match navigation |
| `highlights.ts` | Highlight Overlay | `updatePaneHighlights()`, dual-phase flashing interval animation engine |
| `attentionBell.ts` | Task Reminder Bell | Long-task execution monitor, audio chime player, OS desktop notification trigger |
| `utils/detector.ts` | Path & URL Parser | `detectPathOrUrl()`, regex detection for Win/POSIX/WSL/Git Bash paths & URLs with tests |

### UI Components (`src/components/`)

| Component | Responsibility |
|---|---|
| `SplitGrid.ts` | Walks split tree, creates DOM nodes, divider handle dragging & ratio updates |
| `SettingsModal.ts` | Configuration dialog (Appearance, General, Keybindings, Remote Access, Attention Bell) |
| `ProfileDropdown.ts` | Shell launcher, admin elevation menu, Remote Access & layout exporter triggers |
| `RemoteAccessModal.ts` | Remote Access server controls, password generator, IP/QR code generator |
| `ContextMenu.ts` | Terminal pane & tab header context menus with intelligent path/URL context actions |
| `InputModal.ts` | Text prompt dialog (tab renaming) |
| `HighlightsModal.ts` | Search term highlight list manager |
| `TabBar.ts` | Tab strip overflow scroll controls (`◄`/`►`) & drag spacer window controls |

### Mobile Web Client (`web/`)

| File | Purpose |
|---|---|
| `web/index.html` | Touch-friendly web terminal UI with shell picker modal for PowerShell, CMD, WSL, Git Bash |
| `web/mobile-keyboard.js` | Virtual touch keyboard with Ctrl/Alt modifier toggles, arrow keys, and shell shortcuts |
| `web/mobile.html` | Mobile viewport layout container |

---

## Data Flow Summary

```
CLI Client (client.rs)
  ↓ Win32 Named Pipe (\\.\pipe\kterm_daemon)
Daemon Server (named_pipe.rs / ipc.rs)
  ↓
PtyManager (pty/manager.rs) → PtySession → PTY (portable-pty)
  ↓ Tauri Events / Channel
Frontend (main.ts / daemon.ts) → xterm.js → DOM
```

**Config path**: `AppConfig::load()` → `%APPDATA%/kterm/config.json`
**YAML path**: CLI reads file → Named Pipe → `yaml::apply_yaml_spec()` → spawn sessions + build layout tree
**Export path**: Named pipe / UI → `exporter::export_yaml_layout()` → serialize layout tree → YAML string
**Remote Access path**: Browser → Axum HTTP/WS (`remote.rs`) → PtySession WebSocket relay
