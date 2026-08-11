# kterm Codebase Reference

Per-file, per-function reference for every source file.

---

## Project Structure

```
kterm/
├── src-tauri/
│   ├── src/
│   │   ├── main.rs          (167 lines)  Entry point, CLI dispatch, Tauri setup
│   │   ├── cli.rs           (268 lines)  Clap argument definitions, help text
│   │   ├── client.rs        (670 lines)  CLI HTTP client, daemon lifecycle
│   │   ├── config.rs        (167 lines)  JSON config persistence, window sizing
│   │   ├── yaml.rs          (343 lines)  YAML parsing, validation, application
│   │   ├── exporter.rs      (246 lines)  YAML export, Windows .lnk shortcut creation
│   │   ├── daemon/
│   │   │   └── mod.rs       (1247 lines) Axum HTTP server, all REST endpoints
│   │   └── pty/
│   │       ├── mod.rs       (8 lines)    Re-exports
│   │       ├── manager.rs   (1080 lines) PTY spawn/kill, elevated bridge, session map
│   │       ├── layout.rs    (141 lines)  Recursive split tree (LayoutNode)
│   │       └── ring_buffer.rs (217 lines) Circular output buffer, pattern matching
│   └── Cargo.toml
├── src/
│   ├── main.ts              (2009 lines) Frontend monolith — everything
│   ├── style.css            (1008 lines) All CSS
│   └── components/
│       ├── SplitGrid.ts     (111 lines)  Split pane DOM renderer
│       └── SettingsModal.ts (495 lines)  Settings UI
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

### `client.rs` — CLI HTTP Client

**Lines**: 670

| Line | Function | Input | Output | Responsibility |
|------|----------|-------|--------|---------------|
| 10 | `is_daemon_running()` | — | bool | TCP connect test to `127.0.0.1:9999` |
| 17 | `safe_println(msg)` | &str | stdout | Write line to stdout (handles broken pipe) |
| 22 | `spawn_daemon_detached(exe_path)` | Path | Result | Win32 `CreateProcessW` with `DETACHED_PROCESS` flag |
| 127 | `ensure_daemon_running()` | — | Result | Check daemon → version check → spawn if needed → wait up to 5s |
| 172 | `handle_client_mode(args)` | &CliArgs | Result | Main CLI dispatch: apply, export, new-window, list, split, send, read, wait, spawn |
| 500 | `ensure_window_visible(win_id)` | &str | Result | Fallback: apply default YAML if no windows visible |

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

### `src/main.ts` — Frontend Monolith

**Lines**: 2009

Everything in one file. Major sections:

| Line Range | Section | Responsibility |
|-----------|---------|---------------|
| 1-30 | Imports | Tauri API, xterm, FitAddon, SplitGrid, SettingsModal |
| 32-80 | State variables | `currentWindowId`, `terminals` map, `tabs`, `settings`, WebSocket connections |
| 82-150 | `initWindow()` | Read URL params, load config, create terminal, connect WebSocket, sync tabs |
| 152-200 | `createTerminal(tabInfo)` | Instantiate xterm + FitAddon, attach to DOM, connect WebSocket |
| 202-280 | `connectToTab(tabId, term)` | WebSocket to `/tabs/{id}/ws`, handle resize messages, binary data |
| 282-350 | `syncTabs()` | GET `/tabs?window=X`, reconcile local state, update tab bar |
| 352-420 | `createTabBar()` | Render tab strip DOM (badges, colors, close buttons) |
| 422-500 | `handleTabClick/Close/ContextMenu()` | Tab interaction handlers |
| 502-600 | Keyboard shortcuts | Ctrl+T/N/W, Ctrl+Shift+T, Ctrl+Tab, Ctrl+Shift+], etc. |
| 602-700 | Split pane handlers | `splitPane(direction)`, `unsplitPane()`, `explodeSplit()` |
| 702-800 | Settings integration | Load/save config via `/config`, apply theme/fonts |
| 802-900 | Find bar | Ctrl+F overlay, regex search across terminal buffer |
| 902-1000 | Context menu | Right-click menu (copy, paste, split, close, etc.) |
| 1002-1100 | Highlight system | User-defined text highlights with colors |
| 1102-1200 | Multi-window support | Window creation, cross-window communication |
| 1202-1400 | Export/layout | Export button, YAML layout application |
| 1402-1600 | Tab metadata | Title, badge, color updates via API |
| 1602-1800 | Terminal resize | FitAddon + debounce + send resize to backend |
| 1802-1900 | Init on DOMContentLoaded | Bootstrap the app |
| 1902-2009 | Utility functions | Debounce, throttle, DOM helpers |

**Key state**:
- `terminals: Map<string, Terminal>` — tab ID → xterm instance
- `tabs: TabInfo[]` — current tab list from backend
- `settings: AppConfig` — cached config
- `currentWindowId: string` — from URL params

**WebSocket protocol**: Each tab gets its own WebSocket at `/tabs/{id}/ws?window={winId}`. Messages: text (PTY output) or JSON `{"type":"resize","cols":N,"rows":N}`.

---

### `src/components/SplitGrid.ts` — Split Pane Renderer

**Lines**: 111

| Line | Function | Input | Output | Responsibility |
|------|----------|-------|--------|---------------|
| 10 | `renderLayout(layoutNodes, container, callbacks)` | (LayoutNode[], HTMLElement, callbacks) | | Walk layout tree, create split DOM |
| 30 | `createSplitNode(node, callbacks)` | (LayoutNode, callbacks) | HTMLElement | Recursive split rendering with ratio-based sizing |
| 70 | `createResizeHandle(splitId, direction)` | (string, string) | HTMLElement | Draggable divider bar |
| 90 | (drag logic) | | | Update ratio via POST `/layout/ratio` |

---

### `src/components/SettingsModal.ts` — Settings UI

**Lines**: 495

| Line | Function | Input | Output | Responsibility |
|------|----------|-------|--------|---------------|
| 15 | `openSettings()` | — | | Build and show modal overlay |
| 30 | `buildFontSection()` | — | HTMLElement | Font family + size inputs |
| 60 | `buildThemeSection()` | — | HTMLElement | Color pickers for all theme fields |
| 120 | `buildTerminalSection()` | — | HTMLElement | Default profile, cols, rows, padding, ring buffer |
| 200 | `buildHighlightSection()` | — | HTMLElement | User-defined highlight rules |
| 300 | `applySettings()` | — | | POST `/config`, update xterm themes, resize |
| 350 | `loadSettings()` | — | AppConfig | GET `/config` |
| 400 | (highlight CRUD) | | | Add/edit/delete highlight patterns |

---

## Data Flow Summary

```
CLI (client.rs)
  ↓ HTTP
Axum daemon (daemon/mod.rs)
  ↓
PtyManager (pty/manager.rs) → PtySession → PTY (portable-pty)
  ↓ broadcast
WebSocket (daemon/mod.rs ws_handler)
  ↓
Frontend (main.ts) → xterm.js → DOM
```

**Config path**: `AppConfig::load()` → `%APPDATA%/kterm/config.json`
**YAML path**: CLI reads file → POST `/apply` → `yaml::apply_yaml_spec()` → spawn sessions + build layout tree
**Export path**: GET `/export-layout` → `exporter::export_yaml_layout()` → serialize layout tree → YAML string
