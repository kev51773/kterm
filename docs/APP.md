# kterm App — Complete UI Reference

This document describes every user-facing feature of the kterm Windows application. The refactored codebase must reproduce this behavior exactly.

---

## 1. Window Chrome

kterm uses a **custom title bar** (native OS decorations are disabled). The window is fully frameless.

### 1.1 Title Bar Layout (left to right)

```
[Scroll ◄] [Tab 1] [Tab 2] [...] [Scroll ►] [+] [▼] [──drag spacer──] [─ □ ×]
```

| Element | Behavior |
|---------|----------|
| Tab scroll left `◄` | Scrolls tab bar 150px left. Hidden when not overflowing. Disabled at scroll start. |
| Tab scroll right `►` | Scrolls tab bar 150px right. Hidden when not overflowing. Disabled at scroll end. |
| Tabs list | Horizontal flex container of tab items. Smooth scroll. Scrollbar hidden. |
| Add tab `+` | Spawns a new tab with the default profile. |
| Dropdown `▼` | Opens the profile dropdown menu (see §3). |
| Drag spacer | Fills remaining width. Dragging moves the window. Double-clicking toggles maximize. |
| Minimize `─` | Minimizes the window. |
| Maximize `□` | Toggles maximize/restore. |
| Close `×` | Closes the window via the daemon (which cleans up all PTY sessions). |

### 1.2 Tab Overflow

When tabs exceed the available width:
- Scroll buttons `◄` / `►` appear at the ends of the tab bar.
- Tabs scroll smoothly by 150px per click.
- The active tab is auto-scrolled into view.
- Overflow detection uses `ResizeObserver` on the scroll container.

### 1.3 Window Behavior

- **Default size**: Calculated from config `default_cols` × `default_rows` + padding + title bar height.
- **Double-click tab bar**: Toggles maximize (only on the empty drag area or tab bar background, not on tab items).
- **Window ID**: Passed via URL parameter `?window=win-1`. Defaults to `win-1`.
- **Title format**: `kterm.exe - A scriptable terminal - {window_id}`
- **Last tab closed**: Window auto-closes (daemon sends close signal).
- **Resize**: Fitting is automatic via `ResizeObserver` on the terminal container. Terminals refit when the container resizes.

---

## 2. Tabs

### 2.1 Tab Item

Each tab in the tab bar represents either a single pane or a split group.

| Visual Element | Condition | Description |
|---------------|-----------|-------------|
| Title text | Always | The tab's title (shell profile name by default, or custom). |
| Brackets `[title]` | When tab represents a split group (multiple panes) | Indicates the tab is a group, not a single pane. |
| Badge | When `badge` is set | Colored pill with badge text (e.g., "PROD", "DEV"). |
| Color accent | When `color` is set | Top border color of the tab. |
| Admin shield | When any pane in the group is elevated | Orange shield icon + "ADMIN" text. Orange top-border accent. |
| Close `×` | Always (appears on hover/active) | Closes all panes in the tab's group. |

### 2.2 Tab States

| State | Visual |
|-------|--------|
| Inactive | Dark background (`--inactive-tab-bg`), muted text color. |
| Active | darkest background (`--active-tab-bg`), white text, bold. |
| Hover | Slightly lighter background (`--tab-hover-bg`). |

### 2.3 Tab Context Menu (Right-click on tab header)

| Option | Shortcut | Action |
|--------|----------|--------|
| Rename | — | Opens input modal to change tab title. |
| Close Tab | `Ctrl+Shift+-` | Closes all panes in the group. |

### 2.4 Tab Creation

- Click `+` button or use `Ctrl+Shift++`.
- New tab spawns with the **default profile** (from config, defaults to `powershell`).
- Shell is spawned in the daemon; frontend receives `TabData` and creates the local tab instance.
- Tab is focused immediately after creation.

### 2.5 Tab Closure

- Click `×` on tab, or `Ctrl+Shift+-`, or use context menu.
- Sends close request to daemon for all pane IDs in the group.
- Daemon kills PTY processes and removes layout nodes.
- If it was the last tab in a window, the window closes.
- Local state is cleaned up (WebSocket closed, terminal disposed, DOM removed).

---

## 3. Profiles & Dropdown Menu

### 3.1 Profile Dropdown (Click `▼`)

| Item | Action |
|------|--------|
| PowerShell | Left-click: spawn new tab with `powershell` profile. |
| Command Prompt | Left-click: spawn new tab with `cmd` profile. |
| WSL | Left-click: spawn new tab with `wsl` profile. |
| Git Bash | Left-click: spawn new tab with `git-bash` profile. |
| *(divider)* | — |
| Export Layout... | Opens save dialog, exports current window layout as YAML. |
| Settings `Ctrl+,` | Opens the settings modal. |

### 3.2 Admin Spawn (Right-click on profile)

Right-clicking any profile item in the dropdown opens a submenu:

| Option | Action |
|--------|--------|
| Run as Admin | Spawns a new tab with that profile elevated (Administrator privileges). |

Elevated tabs are launched via a separate process using `ShellExecuteW` with `runas` verb, connected back to the daemon through Win32 named pipes.

---

## 4. Terminal Pane

### 4.1 Terminal Rendering

- Uses **xterm.js** (v5.5) with fit and search addons.
- Font family and size are configurable via settings.
- Default font: `Consolas, "Courier New", monospace` at 14px.
- Terminal padding is configurable (default 8px).
- Theme follows the One Dark / Campbell color scheme.
- Cursor blinks.
- Bold text drawn in bright colors.
- Minimum contrast ratio: 1.2.

### 4.2 Pane Focus

- Clicking inside a pane focuses it.
- Focused pane gets a **glowing border** (configurable highlight color, default `#61afef`).
- Single-pane mode: no border or glow (clean look).
- Focus is tracked as `activePaneId` in the frontend.

### 4.3 Smart Ctrl+C

| Condition | Behavior |
|-----------|----------|
| Text is selected | Copies selection to clipboard, clears selection. Does NOT send SIGINT. |
| No selection | Sends `SIGINT` (`\x03`) to the shell. |

### 4.4 Paste

| Trigger | Behavior |
|---------|----------|
| `Ctrl+V` or `Ctrl+Shift+V` | Paste from clipboard. Tries daemon clipboard endpoint first, falls back to `navigator.clipboard`. |
| Middle-click (button 1) | Same paste behavior. |

Paste sends the text directly to the PTY via WebSocket if open, otherwise uses xterm's built-in paste.

### 4.5 Split Pane Divider

- Horizontal divider: `col-resize` cursor, 4px wide.
- Vertical divider: `row-resize` cursor, 4px tall.
- Divider turns highlight color on hover.
- Dragging resizes the split ratio (clamped 10%–90%).
- Ratio changes are persisted to the daemon via `/layout/ratio`.

---

## 5. Split Panes

### 5.1 Creating Splits

| Method | Action |
|--------|--------|
| `Ctrl+Shift+Right` | Split right (new pane to the right of active). |
| `Ctrl+Shift+Left` | Split left. |
| `Ctrl+Shift+Down` | Split down (new pane below active). |
| `Ctrl+Shift+Up` | Split up. |
| Context menu → Split → direction | Same as above. |

Each split creates a new shell tab in the new pane (same profile as the source, or default).

### 5.2 Unsplitting

| Method | Action |
|--------|--------|
| `Ctrl+Shift+Delete` or `Ctrl+Shift+W` | Remove the active pane from its split, promoting its sibling. |
| Context menu → Un-split Pane | Same. |

When unsplitting, the removed pane's shell is killed. The remaining pane takes the full space. The tab that contained the split group becomes a single-pane tab.

### 5.3 Layout Tree

The layout is a recursive tree of `Pane` and `Split` nodes:

```
Split (horizontal, 0.5)
├── Pane (tab-1)
└── Split (vertical, 0.6)
    ├── Pane (tab-2)
    └── Pane (tab-3)
```

- Each window has an array of layout trees (one per top-level tab group).
- Layouts are stored server-side and synced to the client every 2 seconds + on-demand.
- Split ratios are persisted when the user drags a divider.

---

## 6. Find Bar (Ctrl+F)

### 6.1 Opening

- `Ctrl+F` on any terminal pane opens the find bar for that pane.
- The find bar appears at the top-right of the pane.
- Selection color changes to cyan (`#00e5ff`) while find is active.

### 6.2 UI

```
[ Search input... ] [ 3 of 12 ] [▲] [▼] [✕]
```

| Element | Behavior |
|---------|----------|
| Input | Live search as you type (incremental). |
| Count | Shows current match index and total matches. "No results" when 0. |
| ▲ (Previous) | Jump to previous match. |
| ▼ (Next) | Jump to next match. |
| ✕ (Close) | Close find bar, restore selection colors. |

### 6.3 Keyboard Behavior in Find Bar

| Key | Action |
|-----|--------|
| `Enter` | Next match. |
| `Shift+Enter` | Previous match. |
| `Escape` | Close find bar. |
| `Ctrl+F` | Refocus the input and select all text. |

### 6.4 Match Highlighting

- Non-active matches: pink background (`#ff007f`).
- Active match: cyan background (`#00e5ff`).
- Uses xterm's SearchAddon decoration system.

---

## 7. Word Highlights

### 7.1 Highlights Modal

Opened from the terminal context menu → "Highlights..."

| Element | Behavior |
|---------|----------|
| Input + "Add" button | Add a word/phrase to highlight in this pane. |
| List | Shows all active highlights for this pane. |
| Remove `×` per item | Remove a highlight. |
| Done | Close the modal. |

### 7.2 Highlight Rendering

- Highlights are rendered as xterm decorations overlaying the terminal buffer.
- **Flashing**: Alternates between two color schemes every 600ms:
  - Phase A: Yellow background (`#f1c40f`), dark blue text (`#0033cc`).
  - Phase B: Blue background (`#0044ff`), yellow text (`#ffff00`).
- Highlights scan the entire visible buffer on each flash.
- Multiple words can be highlighted simultaneously.
- Highlights are per-pane (stored in `paneHighlightsMap`).
- Regex special characters in highlight words are escaped (literal matching).

---

## 8. Context Menus

### 8.1 Terminal Pane Context Menu (Right-click on terminal)

| Option | Shortcut | Action |
|--------|----------|--------|
| Copy | `Ctrl+Shift+C` | Copy selection to clipboard. |
| Paste | `Ctrl+Shift+V` | Paste from clipboard. |
| Find | `Ctrl+Shift+F` | Open find bar. |
| *Split →* | | Submenu: |
| → Split Right | `Ctrl+Shift+Right` | Split pane right. |
| → Split Left | `Ctrl+Shift+Left` | Split pane left. |
| → Split Down | `Ctrl+Shift+Down` | Split pane down. |
| → Split Up | `Ctrl+Shift+Up` | Split pane up. |
| → Un-split Pane | `Ctrl+Shift+W` | Remove pane from split. |
| Highlights... | — | Open highlights modal. |
| Export buffer | — | Save terminal content to `.txt` file. |

### 8.2 Context Menu Positioning

- Menus are positioned at the mouse click coordinates.
- Auto-repositioned if they would overflow the window.
- The "open-left" class is applied when the menu would overflow the right edge (for submenus).
- Clicking outside any context menu closes it.

---

## 9. Settings Modal (Ctrl+,)

### 9.1 Structure

```
┌─────────────────────────────────────────┐
│ Settings                            [×] │
├──────────┬──────────────────────────────┤
│ Sidebar  │ Content                      │
│          │                              │
│ Appear.  │  Font & Layout               │
│ General  │  ─────────────               │
│ Keybind  │  Font Family: [dropdown]     │
│          │  Font Size: [14]             │
│          │  Terminal Padding: [8]        │
│          │                              │
│          │  Title & Tab Colors          │
│          │  ─────────────────           │
│          │  Title Bar: [■] [===] [#hex] │
│          │  Active Tab: [■] [===] [#hex]│
│          │  ... (9 color inputs)        │
│          │                              │
│          │  Terminal Palette            │
│          │  ────────────────            │
│          │  Background: [■] [===] [#hex]│
│          │  Foreground: [■] [===] [#hex]│
│          │  Highlight:  [■] [===] [#hex]│
├──────────┴──────────────────────────────┤
│                      [Cancel] [Save]    │
└─────────────────────────────────────────┘
```

### 9.2 Appearance Tab

| Setting | Control | Range | Default |
|---------|---------|-------|---------|
| Font Family | Dropdown + custom text input | 9 presets + custom | Consolas |
| Font Size | Number input | 8–36 px | 14 |
| Terminal Padding | Number input | 0–32 px | 8 |
| Title Bar Background | Color picker + brightness slider + hex input | — | `#21252b` |
| Active Tab Background | Same | — | `#0d0e11` |
| Active Tab Font Color | Same | — | `#ffffff` |
| Inactive Tab Background | Same | — | `#181a1f` |
| Inactive Tab Font Color | Same | — | `#abb2bf` |
| Tab Hover Background | Same | — | `#282c34` |
| Terminal Background | Same | — | `#0d0e11` |
| Terminal Foreground | Same | — | `#cccccc` |
| Highlight Color | Same | — | `#61afef` |

**Font presets**: Consolas, Cascadia Code, Cascadia Mono, Courier New, Lucida Console, Fira Code, JetBrains Mono, Source Code Pro, Custom.

**Color picker behavior**:
- The swatch (`<input type="color">`) sets the base color.
- The brightness slider (0–200, default 100) adjusts brightness by a factor.
- The text input shows/accepts the hex value.
- All three controls are synced; changing one updates the others.
- **Live preview**: All appearance changes apply in real-time as you adjust (no save required to see the effect).

### 9.3 General Tab

| Setting | Control | Range | Default |
|---------|---------|-------|---------|
| Default Profile | Dropdown | powershell, cmd, wsl, git-bash | powershell |
| Default Grid Columns | Number input | 40–300 | 120 |
| Default Grid Rows | Number input | 10–150 | 30 |
| Ring Buffer Capacity | Number input | 64–4096 KB (step 64) | 256 |

### 9.4 Keybindings Tab

Read-only reference list:

| Action | Keybinding |
|--------|-----------|
| New Tab (Default Shell) | `Ctrl+Shift++` |
| Close Current Tab | `Ctrl+Shift+-` |
| Cycle Tabs Forward | `Ctrl+Tab` |
| Cycle Tabs Backward | `Ctrl+Shift+Tab` |
| Jump to Tab 1..9 | `Ctrl+Shift+1..9` |
| Open Settings | `Ctrl+,` |
| Smart Copy / Cancel | `Ctrl+C` |
| Split Right | `Ctrl+Shift+Right` |
| Split Down | `Ctrl+Shift+Down` |

### 9.5 Save / Cancel

| Button | Behavior |
|--------|----------|
| Save Settings | Sends config to daemon via `POST /config`. Applies config locally. Closes modal. |
| Cancel | Reverts to the config that was active before the modal opened. Closes modal. |
| Close `×` | Same as Cancel. |
| `Escape` | Same as Cancel. |

Config is persisted by the daemon to `%APPDATA%/kterm/config.json`.

---

## 10. Keyboard Shortcuts (Complete Reference)

### 10.1 Global Shortcuts (always active)

| Shortcut | Action |
|----------|--------|
| `Ctrl+F` | Open find bar on active pane. |
| `Ctrl+,` | Open settings modal. |
| `Ctrl+Shift++` | New tab (default profile). |
| `Ctrl+Shift+-` | Close active tab. |
| `Ctrl+Tab` | Cycle to next tab group. |
| `Ctrl+Shift+Tab` | Cycle to previous tab group. |
| `Ctrl+Shift+1..9` | Jump to tab group by index (1-indexed). |

### 10.2 Pane Shortcuts (active when a pane is focused)

| Shortcut | Action |
|----------|--------|
| `Ctrl+Shift+Right` or `Alt+Shift+Right` | Split right. |
| `Ctrl+Shift+Left` or `Alt+Shift+Left` | Split left. |
| `Ctrl+Shift+Down` or `Alt+Shift+Down` | Split down. |
| `Ctrl+Shift+Up` or `Alt+Shift+Up` | Split up. |
| `Ctrl+Shift+Delete` or `Ctrl+Shift+W` | Unsplit active pane. |

### 10.3 Terminal-Level Shortcuts (handled by xterm key handler)

| Shortcut | Action |
|----------|--------|
| `Ctrl+C` (with selection) | Copy selection, clear selection. |
| `Ctrl+C` (no selection) | Send SIGINT. |
| `Ctrl+V` / `Ctrl+Shift+V` | Paste. |
| `Ctrl+Shift++` | New tab. |
| `Ctrl+Shift+-` | Close tab. |
| `Ctrl+Tab` / `Ctrl+Shift+Tab` | Cycle tabs. |
| `Ctrl+,` | Open settings. |
| `Ctrl+F` | Open find bar. |
| Arrow splits (as above) | Split/unsplit. |

### 10.4 Shortcut Conflict Resolution

- Global `window.addEventListener('keydown')` handles most shortcuts.
- The terminal also has `attachCustomKeyEventHandler` which handles the same shortcuts.
- The terminal handler calls `e.preventDefault()` and `e.stopImmediatePropagation()` to prevent the global handler from also firing.
- `e.repeat` is checked to avoid repeated actions during key hold.

---

## 11. Export

### 11.1 Export Layout (YAML)

From profile dropdown → "Export Layout..."

1. Save dialog opens (default filename: `kterm-layout.yaml`, filter: YAML).
2. Daemon exports the current window's layout as YAML.
3. File is written to the selected path.
4. A Windows `.lnk` shortcut is created next to the YAML file (same name, `.lnk` extension).
5. The shortcut launches `kterm.exe --apply "<path>" --suffix-auto`.

### 11.2 Export Buffer

From terminal context menu → "Export buffer"

1. Save dialog opens (default filename: `terminal-buffer-{paneId}.txt`, filter: text).
2. Terminal buffer content is extracted (all visible lines, ANSI codes stripped).
3. File is written to the selected path.

---

## 12. Multi-Window

### 12.1 Window Creation

- New windows can be created via CLI (`kterm --new-window`).
- Each window has its own set of tabs, layouts, and PTY sessions.
- Windows are identified by labels: `win-1`, `win-2`, etc.
- The window ID is passed to the frontend via URL parameter: `index.html?window=win-2`.

### 12.2 Window Lifecycle

- Closing a window destroys all its PTY sessions.
- If all windows are closed, the daemon process exits.
- The daemon tracks window titles in `window_titles` and layouts in `window_layouts`.

### 12.3 Tab Syncing

- The frontend polls `GET /tabs?window={id}` and `GET /layout?window={id}` every 2 seconds.
- Sync handles: new tabs, removed tabs, updated metadata (title/badge/color), layout changes.
- If the active tab is removed, focus shifts to a sibling in the same group, or the first available tab.
- If all tabs are removed and the window previously had tabs, the window auto-closes.

---

## 13. Theming

### 13.1 CSS Custom Properties

| Variable | Default | Used For |
|----------|---------|----------|
| `--highlight-color` | `#61afef` | Focus border, badges, active settings nav, find focus |
| `--title-bar-bg` | `#21252b` | Title bar, settings header/footer, dividers |
| `--active-tab-bg` | `#0d0e11` | Active tab, terminal container, settings body |
| `--active-tab-fg` | `#ffffff` | Active tab text, window controls, settings text |
| `--inactive-tab-bg` | `#181a1f` | Inactive tabs, settings sidebar, context menus |
| `--inactive-tab-fg` | `#abb2bf` | Inactive tab text, muted text, context menu items |
| `--tab-hover-bg` | `#282c34` | Tab hover, button hover, settings nav hover |
| `--terminal-bg` | `#0d0e11` | Body background, scrollbar track |

### 13.2 Theme Application

When config is loaded or saved:
1. CSS custom properties are set on `document.documentElement`.
2. Each xterm.js terminal instance gets its theme updated (Campbell base + user foreground/background).
3. Terminals are refreshed to apply the new theme.

### 13.3 Campbell Base Colors

The terminal uses the Windows Terminal Campbell color scheme as its base:

| Color | Hex |
|-------|-----|
| Black | `#0C0C0C` |
| Red | `#C50F1F` |
| Green | `#13A10E` |
| Yellow | `#C19C00` |
| Blue | `#0037DA` |
| Magenta | `#881798` |
| Cyan | `#3A96DD` |
| White | `#CCCCCC` |
| Bright variants | Standard bright counterparts |

User background/foreground overrides are applied on top of this base.

---

## 14. Daemon Communication

### 14.1 Connection Lifecycle

1. On startup, the frontend retries connecting to `http://127.0.0.1:9999` up to 30 times (100ms intervals).
2. Once connected, it loads config (`GET /config`) and syncs tabs.
3. A polling interval of 2 seconds keeps tabs and layouts in sync.

### 14.2 WebSocket Connections

- Each tab has a WebSocket at `ws://127.0.0.1:9999/tabs/{id}/ws?window={window_id}`.
- The WebSocket carries raw terminal I/O (user input → PTY, PTY output → xterm).
- Resize events are sent as JSON: `{"type": "resize", "cols": N, "rows": N}`.
- On WebSocket close, the frontend triggers a tab sync after 100ms (to detect if the shell exited).
- On initial connect, the backend replays the output history buffer to avoid missing startup output.

### 14.3 Error Handling

- All `fetch()` calls are wrapped in try/catch.
- Failed API calls log to console but don't crash the app.
- If the daemon is unreachable, the frontend continues to retry on the next sync cycle.

---

## 15. Codebase Architecture & Refactor Notes

### Stage 1: Backend Daemon Split
`src-tauri/src/daemon/` structure:
- `mod.rs`: `AppState` container, Axum router, `run_server` entry point.
- `types.rs`: Data structures and serde request/response payloads (`TabInfo`, `WindowInfo`, `CreateTabRequest`, etc.).
- `tabs.rs`: Tab operations (`list_tabs`, `create_tab`, `send_text`, `set_title`, `set_badge`, `set_color`, `close_tabs`, `resize_tab`, `read_tab_buffer`, `wait_tab_output`).
- `windows.rs`: Window management (`list_windows`, `create_window`, `close_window`, `show_window`, `resize_window`).
- `splits.rs`: Layout and split operations (`split_tab`, `unsplit_tab`, `explode_tab`, `update_layout_ratio`, `get_window_layout`).
- `session.rs`: Session lifecycle (`auto_close_tab`, `apply_session`).
- `export.rs`: Export endpoints (`export_layout_endpoint`, `export_shortcut_endpoint`).
- `websocket.rs`: WebSocket upgrade and binary/text message handling (`ws_handler`, `handle_websocket`).
- `config.rs`: Application configuration endpoints (`get_config`, `update_config`).
- `system.rs`: System & utility endpoints (`health_check`, `get_build_id`, `shutdown_daemon`, `get_clipboard`).

### Stage 2: Backend PTY Split
`src-tauri/src/pty/` structure:
- `session.rs`: `PtySession` struct, spawn/close/read/write methods, `ExitCallback`.
- `platform.rs`: Win32 named-pipe helpers (`create_win32_named_pipe_handle`, `connect_win32_named_pipe`), process inspection (`is_process_alive`, `is_app_elevated`), and Windows job object assignment (`assign_pid_to_job`).
- `elevated.rs`: UAC elevation bridge (`run_elevated_pty_bridge`).
- `manager.rs`: `PtyManager` lifecycle coordinator and tab resolution.
- `layout.rs`: Split layout tree structures (`LayoutNode`, `SplitDirection`).
- `ring_buffer.rs`: ANSI-aware PTY output ring buffer.
- `mod.rs`: Re-exports and submodule declarations.

### Stage 3: Frontend Extraction & Hardening
`src/` structure:
- `state.ts`: Window/tab/pane state store, type definitions (`TabData`, `PaneInstance`, `TabInstance`), state getters/setters, and fit flags.
- `config.ts`: Config loading/application, theme application (`applyAppConfig`), grid dimension calculations.
- `daemon.ts`: Daemon connection initialization, tab/layout state sync (`syncTabs`), WebSocket stream handler.
- `terminal.ts`: xterm.js instance lifecycle per pane (`createTabLocal`, `removeTabLocal`), window sizing (`adjustWindowForGrid`), clipboard paste.
- `tabs.ts`: Tab model, tab header rendering (`renderTabBarHeaders`), tab focus, tab switching/cycling, tab spawn handlers.
- `splits.ts`: Layout tree queries (`containsTab`, `getTabIdsInNode`), split pane rendering (`renderActiveLayout`), split/unsplit commands.
- `findBar.ts`: Search find-bar UI (`showFindBar`, `closeFindBar`), match count tracking, terminal buffer text extraction.
- `highlights.ts`: Word highlighting overlay and flashing interval animation (`updatePaneHighlights`).
- `components/InputModal.ts`: Modal component for text/rename inputs.
- `components/HighlightsModal.ts`: Modal component for managing pane highlight terms.
- `components/ContextMenu.ts`: Terminal pane and tab header context menus (`showTerminalContextMenu`, `showTabHeaderContextMenu`).
- `components/ProfileDropdown.ts`: Profile switcher and administrator execution menu.
- `components/TabBar.ts`: Tab strip scroll controls, overflow detector, window maximize trigger.
- `main.ts`: Main entry point wiring module initialization, global keyboard shortcuts, window controls, and Tauri event handlers.

### Stage 4: Security & Polish
- **Elevated Spawn Surface**: Validated `pipe_id` in `elevated.rs` to enforce alphanumeric/hyphen/underscore naming, eliminating named pipe path traversal/injection risks.
- **WebSocket IPC Clamping**: Enforced parameter boundary clamping on incoming WebSocket `resize` events (`cols` 1–1000, `rows` 1–500) in `daemon/websocket.rs`.
- **Dead-Code & Type Safety**: Cleaned unused imports, typecheck (`tsc --noEmit`) 100% green, cargo tests 111/111 passing.




