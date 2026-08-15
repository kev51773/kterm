# kterm - Scriptable Windows-Native Terminal

`kterm` is a high-performance, scriptable Windows-native terminal application built with **Tauri v2**, **Rust** (`portable-pty` for ConPTY + Axum embedded daemon), and **TypeScript** (`xterm.js`).

It behaves like Windows Terminal with modern tabs, split panes, dark theme styling, and keybindings, while providing a powerful **dual-mode CLI interface** (`kterm.exe`) for automation, scripting, and remote control.

---

## Key Features

- ⚡ **Embedded API Daemon**: Runs a background HTTP/WebSocket server on `127.0.0.1:9999`.
- 🔀 **Dual-Mode Executable (`kterm.exe`)**:
  - **Host Mode**: Spawns GUI app + background daemon if no daemon is running.
  - **Client Mode**: Connects to active daemon, executes CLI commands, prints output to stdout, and exits instantly.
- 🪟 **Multi-Window & Multi-Tab**: Full control over windows, tabs, and profiles (`powershell`, `cmd`, `wsl`, `git-bash`).
- 🧩 **Incremental Target-Based Splits**: Split panes vertically/horizontally, move tabs into splits, or explode splits into standalone tabs.
- 📝 **Tail-Argument Text Injection**: Pass raw commands directly with `--send-text` without complex shell escaping.
- ⏱️ **Synchronization & Buffer Reading**: Read screen output (`--read-text`) or block scripts until text appears (`--wait-for`).
- 💾 **Script Export Engine**: Export current live window/tab/split layouts into executable `.ps1`, `.bat`, or `.sh` scripts (`--export-script`).
- 🎯 **Intelligent Path & URL Detection**: Right-click context menus automatically detect file paths (Windows, Git Bash, WSL, POSIX, relative) and URLs with live visual text highlighting, offering instant opening, Explorer navigation, shell spawning at target directory, and format conversion.

---

## System Architecture

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

## CLI Reference Guide

### 1. Window & Tab Spawning

```powershell
# Spawn tab in active/focused window with default profile -> returns tab handle
kterm.exe

# Spawn tab with profile 'git-bash' in active window -> returns tab handle
kterm.exe --profile git-bash

# Spawn tab in a specific window ID
kterm.exe --window win-2 --profile powershell

# Spawn tab in a BRAND NEW window -> returns tab handle
kterm.exe --new-window --profile wsl
```

### 2. Window & Tab Inspection

```powershell
# List active windows
kterm.exe --list-windows

# List tabs across windows (formatted table)
kterm.exe --list-tabs

# List tabs filtered by window or profile as JSON
kterm.exe --list-tabs --window win-1 --profile git-bash --json
```

*Formatted Table Output Example:*
```
WINDOW  TAB ID   TITLE              PROFILE     BADGE  SPLIT    FOCUSED
win-1   tab-101  Backend Server     powershell  DEV    -        Yes
win-1   tab-102  Git Repo           git-bash    -      split-1  No
win-2   tab-201  Production DB      powershell  PROD   -        Yes
```

### 3. Text Injection & Metadata Update

```powershell
# Target tab by ID or by exact/pattern Title
kterm.exe --select-tab tab-101 --send-text echo $HOME
kterm.exe --select-tab "Production Server" --send-text docker ps

# Target multiple tabs simultaneously
kterm.exe --select-tab tab-101 --select-tab "Git Repo" --send-text git status

# Set tab title, visual badge, and color accent
kterm.exe --select-tab tab-101 --send-title Production Server
kterm.exe --select-tab "Production Server" --set-badge PROD
kterm.exe --select-tab "Production Server" --set-color "#ff3333"
```

### 4. Target-Based Incremental Split Panes

```powershell
# Split off $t1 to the right with profile (returns new tab handle $t2)
$t2 = kterm.exe --select-tab $t1 --split-right --profile git-bash

# Split off $t1 to the left with profile
kterm.exe --select-tab $t1 --split-left --profile git-bash

# Split off $t2 downward with profile (returns new bottom-right tab handle $t3)
$t3 = kterm.exe --select-tab $t2 --split-down --profile wsl

# Split off $t2 upward with profile
kterm.exe --select-tab $t2 --split-up --profile wsl

# Move an existing standalone tab into a split pane below $t1
kterm.exe --select-tab $t1 --split-down --move-tab tab-999

# Detach a split pane back into a top-level tab
kterm.exe --select-tab $t3 --unsplit

# Explode all panes in a split view into standalone top-level tabs
kterm.exe --select-tab $t1 --explode-split
```

### 5. Buffer Reading & Synchronous Wait

```powershell
# Read screen buffer (last 50 lines)
kterm.exe --select-tab tab-101 --read-text --tail 50

# Block script execution until string appears in PTY stream (with 30s timeout)
kterm.exe --select-tab tab-101 --wait-for "Build Succeeded" --timeout 30s
```

### 6. Focus, Move, and Close

```powershell
# Bring window to front and activate tab
kterm.exe --select-tab tab-101 --focus

# Move tab to another window
kterm.exe --select-tab tab-101 --move-to-window win-2

# Close tab gracefully (or force kill PTY process)
kterm.exe --select-tab tab-101 --close
kterm.exe --select-tab tab-101 --close --force

# Close specific window
kterm.exe --close-window win-2
```

### 7. Layout Script Exporter

Exports active live windows, tabs, split hierarchies, titles, and badges into an executable script:

```powershell
kterm.exe --export-script setup-env.ps1               # PowerShell (.ps1)
kterm.exe --export-script setup-env.bat               # Windows Batch (.bat)
kterm.exe --export-script setup-env.sh --format bash  # Bash (.sh)
```

*Sample Generated `setup-env.ps1`:*
```powershell
$w1_t1 = & kterm.exe --new-window --profile powershell
& kterm.exe --select-tab $w1_t1 --send-title "Backend Service"
& kterm.exe --select-tab $w1_t1 --set-badge "DEV"

$w1_t2 = & kterm.exe --select-tab $w1_t1 --split-right --profile git-bash
& kterm.exe --select-tab $w1_t2 --send-title "Git Repo"

$w1_t3 = & kterm.exe --select-tab $w1_t2 --split-down --profile wsl
& kterm.exe --select-tab $w1_t3 --send-title "Container Logs"
```

---

## HTTP REST & WebSocket API Specification (`127.0.0.1:9999`)

| Method | Endpoint | Description | Body / Query Payload |
| :--- | :--- | :--- | :--- |
| `GET` | `/health` | Daemon health check | `N/A` |
| `GET` | `/windows` | List active windows | `N/A` |
| `GET` | `/layout` | Get window layout tree | `?window=win-1` |
| `POST` | `/layout/ratio` | Update split ratio | `{"split_id": "...", "ratio": 0.5}` |
| `GET` | `/tabs` | List active tabs | `?window=win-1` |
| `POST` | `/tabs` | Spawn tab | `{"profile": "powershell", "window": "win-1", "cols": 120, "rows": 30, "elevated": false}` |
| `POST` | `/tabs/send` | Write text to 1+ tabs | `{"targets": ["tab-101"], "text": "dir\r", "window": "win-1"}` |
| `POST` | `/tabs/title` | Set tab title | `{"targets": ["tab-101"], "title": "New Title", "window": "win-1"}` |
| `POST` | `/tabs/badge` | Set tab badge | `{"targets": ["tab-101"], "badge": "PROD", "window": "win-1"}` |
| `POST` | `/tabs/color` | Set tab color accent | `{"targets": ["tab-101"], "color": "#1e88e5", "window": "win-1"}` |
| `POST` | `/tabs/close` | Close 1+ tabs | `{"targets": ["tab-101"], "window": "win-1"}` |
| `POST` | `/tabs/:id/split` | Split pane | `{"direction": "right"}` |
| `POST` | `/tabs/:id/unsplit` | Un-split pane | `N/A` |
| `GET` | `/tabs/:id/read` | Read screen buffer | `?tail=50` |
| `POST` | `/tabs/:id/wait` | Wait for text pattern | `{"pattern": "Build Succeeded", "timeout_sec": 30}` |
| `GET` | `/export-layout` | Export window layout YAML | `?window=win-1` |
| `POST` | `/export-shortcut` | Create launcher shortcut | `{"path": "..."}` |
| `GET` / `POST` | `/config` | Get or update app config | `AppConfig` JSON |
| `GET` | `/tabs/:id/ws` | PTY WebSocket | Bi-directional PTY I/O stream (`?window=win-1`) |

---

## Interactive UI & Keybindings

| Keybinding | Action |
| :--- | :--- |
| `Ctrl+Shift++` | Spawn New Tab (Default Profile) |
| `Ctrl+Shift+-` | Close Active Tab |
| `Ctrl+Tab` / `Ctrl+Shift+Tab` | Cycle Next / Previous Tab Group |
| `Ctrl+Shift+1..9` | Switch to Tab Group N |
| `Ctrl+Shift+Right` / `Alt+Shift+Right` | Split Right |
| `Ctrl+Shift+Left` / `Alt+Shift+Left` | Split Left |
| `Ctrl+Shift+Down` / `Alt+Shift+Down` | Split Down |
| `Ctrl+Shift+Up` / `Alt+Shift+Up` | Split Up |
| `Ctrl+Shift+Delete` / `Ctrl+Shift+W` | Un-split Active Pane |
| `Ctrl+F` | Open Terminal Find Bar |
| `Ctrl+,` | Open Settings Modal |



