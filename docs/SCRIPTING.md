# kterm Scripting — CLI Reference

This document describes every CLI command available in `kterm.exe` for scripting, automation, and external control.

---

## 1. Architecture

`kterm.exe` is a **dual-mode executable**:

1. **Host Mode** (`--daemon`): Runs the Axum HTTP daemon on `127.0.0.1:9999`, launches the Tauri GUI window. This mode is started automatically when no daemon is running.
2. **Client Mode** (all other invocations): Connects to an existing daemon via HTTP, executes the requested command, prints output to stdout, and exits.

When you run any `kterm.exe` command:
1. Client checks if `127.0.0.1:9999` is reachable.
2. If not — spawns a detached daemon process (waits up to 5 seconds for it to start).
3. If the running daemon is a different version — shuts it down and restarts.
4. Executes the CLI command via HTTP API.
5. Prints result to stdout and exits.

Every command below works whether or not kterm is already running — the daemon starts on-demand.

---

## 2. Shell Profiles

| Profile ID | Shell | Notes |
|-----------|-------|-------|
| `powershell` | `powershell.exe -NoExit` | Default |
| `cmd` | `cmd.exe /K` | |
| `wsl` | `wsl.exe` | Windows Subsystem for Linux |
| `git-bash` | Git Bash (`--login -i`) | Uses `C:\Program Files\Git\bin\bash.exe` or LOCALAPPDATA fallback |

---

## 3. Command Reference

### 3.1 Spawn Tab

Spawn a new tab (default action when no flags are given).

```bash
kterm.exe                                    # Default profile in active/default window
kterm.exe --profile git-bash                 # Specific profile
kterm.exe --profile cmd --window win-1       # Specific window
kterm.exe --new-window --profile wsl         # New window
kterm.exe --profile powershell --admin       # Elevated (admin)
```

| Flag | Description |
|------|-------------|
| `--profile <PROFILE>` | Shell profile. Default: `powershell`. |
| `--window <WINDOW>` | Target window ID. Default: `win-1`. |
| `--new-window` | Force create a new GUI window. |
| `--admin` | Run shell with Administrator privileges (triggers UAC). |

**Output**: The new tab's ID (e.g., `tab-101`).

---

### 3.2 List Windows

```bash
kterm.exe --list-windows
kterm.exe --list-windows --json
```

**Default output**:
```
Window ID: win-1, Label: win-1
Window ID: win-2, Label: win-2
```

**JSON output**:
```json
[
  { "id": "win-1", "label": "win-1", "title": "kterm.exe - A scriptable terminal - win-1" }
]
```

---

### 3.3 List Tabs

```bash
kterm.exe --list-tabs
kterm.exe --list-tabs --window win-1
kterm.exe --list-tabs --json
```

| Flag | Description |
|------|-------------|
| `--window <WINDOW>` | Filter by window ID. |
| `--json` | Output as JSON array. |

---

### 3.4 Select Target Tabs

Most action flags operate on target tabs selected by `--select-tab`.

```bash
kterm.exe --select-tab tab-101 --send-text git status          # By ID
kterm.exe --select-tab "Production Server" --send-text docker ps  # By title
kterm.exe --select-tab tab-101 --select-tab tab-102 --send-text git status  # Multiple
kterm.exe --send-text echo hello                                # Active tab (default)
```

| Resolution | Behavior |
|-----------|----------|
| By ID | Exact match on tab ID. |
| By title | Case-insensitive exact match on tab title. |
| `active` or empty | First tab in the daemon's session list. |
| Ambiguous (title matches in multiple windows) | Error: specify `--window`. |
| Not found | Error: "Tab 'X' not found." |

---

### 3.5 Send Text

Send text/commands to one or more terminal panes.

```bash
kterm.exe --select-tab tab-101 --send-text echo hello world
kterm.exe --select-tab tab-101 --send-text git status
kterm.exe --select-tab tab-101 --select-tab tab-102 --send-text git status
```

**Behavior**: A `\r` is appended automatically if the text doesn't end with `\r` or `\n`. Text is written directly to the PTY's stdin.

---

### 3.6 Set Tab Title

```bash
kterm.exe --select-tab tab-101 --send-title "Backend Server"
```

Multiple words are joined with spaces.

---

### 3.7 Set Tab Badge

```bash
kterm.exe --select-tab tab-101 --set-badge PROD
```

---

### 3.8 Set Tab Color

```bash
kterm.exe --select-tab tab-101 --set-color "#E53935"
```

Accent color hex for the tab's top border.

---

### 3.9 Set Window Title

```bash
kterm.exe --window win-1 --set-window-title "Main Workspace"
```

Formatted as `kterm.exe - {title} - {window_id}`.

---

### 3.10 Focus Window/Tab

```bash
kterm.exe --select-tab tab-101 --focus
```

Brings the window to the front and activates the tab.

---

### 3.11 Close Tab

```bash
kterm.exe --select-tab tab-101 --close
kterm.exe --select-tab tab-101 --close --force
```

If closing the last tab in a window, the window closes too.

---

### 3.12 Close Window

```bash
kterm.exe --close-window win-1
```

Closes the window and all its PTY sessions.

---

### 3.13 Split Panes

```bash
$t2 = kterm.exe --select-tab $t1 --split-right --profile git-bash   # Split right
kterm.exe --select-tab $t1 --split-left --profile wsl               # Split left
$t3 = kterm.exe --select-tab $t2 --split-down --profile cmd         # Split down
kterm.exe --select-tab $t1 --split-up --profile powershell          # Split up
kterm.exe --select-tab $t1 --split-down --move-tab tab-999          # Move existing tab into split
```

| Flag | Description |
|------|-------------|
| `--split-right` | Split right (horizontal, new pane right). |
| `--split-left` | Split left (horizontal, new pane left). |
| `--split-down` | Split down (vertical, new pane below). |
| `--split-up` | Split up (vertical, new pane above). |
| `--move-tab <TAB_ID>` | Move an existing tab into the split instead of creating a new shell. |
| `--profile <PROFILE>` | Profile for the new pane (ignored when `--move-tab` is used). |

**Output**: The new tab's ID (the pane created by the split).

---

### 3.14 Unsplit Pane

```bash
kterm.exe --select-tab $t2 --unsplit
```

Removes the pane from its split layout. The sibling pane takes the full space. The removed pane's shell is killed.

---

### 3.15 Explode Split

```bash
kterm.exe --select-tab $t1 --explode-split
```

Separates all panes in a split group into standalone top-level tabs. The split tree is flattened.

---

### 3.16 Read Tab Output

Read recent output from a tab's ring buffer.

```bash
kterm.exe --select-tab tab-101 --read-text
kterm.exe --select-tab tab-101 --read-text --tail 100
kterm.exe --select-tab tab-101 --read-text --raw
```

| Flag | Description |
|------|-------------|
| `--read-text [TAB_ID]` | Read output. Optional tab ID (defaults to `--select-tab` or `tab-101`). |
| `--tail <N>` | Number of lines to read from the end. Default: 50. |
| `--raw` | Keep raw ANSI escape codes (default strips them). |

---

### 3.17 Wait for Output

Block script execution until specific text appears in a tab's output.

```bash
kterm.exe --select-tab tab-101 --wait-for "Build Succeeded" --timeout 30
kterm.exe --select-tab tab-101 --wait-for "ready on port \d+" --timeout 60
kterm.exe --select-tab tab-101 --wait-for-prompt --timeout 30
kterm.exe --select-tab tab-101 --wait-for "SUCCESS" --from-history
```

| Flag | Description |
|------|-------------|
| `--wait-for <PATTERN>` | String or regex pattern to wait for. |
| `--wait-for-prompt` | Wait until shell prompt detected (ends with `>`, `$`, `#`, or contains `PS `). |
| `--timeout <SECONDS>` | Timeout in seconds. Default: 30. |
| `--from-history` | Check from the start of the buffer, not just new output. |
| `--json` | Output result as JSON. |

**Default output**: `Matched output successfully.`

**JSON output**: `{"status":"ok","matched":true,"elapsed_sec":2.34}`

**Error conditions**:
- Timeout returns HTTP 408
- PTY closed returns HTTP 410

---

### 3.18 Apply YAML Layout

Apply a declarative YAML session specification. See [YAML.md](YAML.md) for the full schema.

```bash
kterm.exe --apply my-layout.yaml                    # Apply from file
kterm.exe --apply my-layout.yaml --suffix -backend  # Apply with suffix
kterm.exe --apply my-layout.yaml --suffix-auto      # Apply with auto-suffix
kterm.exe --apply my-layout.yaml --dry-run           # Validate only
kterm.exe --apply my-layout.yaml --window win-3      # Target specific window
```

| Flag | Description |
|------|-------------|
| `--apply <FILE>` | Path to YAML file. |
| `--suffix <TEXT>` | Append text to the window ID. |
| `--suffix-auto` | Auto-increment window ID suffix. |
| `--dry-run` | Validate YAML without making changes. |
| `--window <WINDOW>` | Override the window ID from the YAML. |

**Implicit apply**: A positional argument ending in `.yaml` or `.yml` is treated as `--apply <file>`.

**Output**: The resolved window ID.

---

### 3.19 Export Layout

Export the current window layout to a YAML file.

```bash
kterm.exe --export-layout my-layout.yaml
kterm.exe --export-layout my-layout.yaml --window win-1
```

| Flag | Description |
|------|-------------|
| `--export-layout <FILE>` | Output file path. |
| `--window <WINDOW>` | Source window. Default: `win-1`. |

**Output**: `Layout written to: {path}` and `Shortcut written to: {path}.lnk`

A Windows `.lnk` shortcut is automatically created alongside the YAML file.

---

## 4. Scripting Patterns

### 4.1 Multi-Pane Dev Environment (PowerShell)

```powershell
$w1 = kterm.exe --new-window --profile powershell
kterm.exe --select-tab $w1 --send-title "Backend"
kterm.exe --select-tab $w1 --set-badge "DEV"

$w1_t2 = kterm.exe --select-tab $w1 --split-right --profile git-bash
kterm.exe --select-tab $w1_t2 --send-title "Frontend"

$w1_t3 = kterm.exe --select-tab $w1_t2 --split-down --profile wsl
kterm.exe --select-tab $w1_t3 --send-title "Docker"
kterm.exe --select-tab $w1_t3 --send-text docker compose up

kterm.exe --select-tab $w1 --send-text npm run dev
```

### 4.2 Wait-and-React Script

```powershell
$w = kterm.exe --new-window --profile powershell
kterm.exe --select-tab $w --send-text "npm run build"
kterm.exe --select-tab $w --wait-for "Build complete" --timeout 120
if ($LASTEXITCODE -eq 0) {
    kterm.exe --select-tab $w --send-text "npm test"
} else {
    kterm.exe --select-tab $w --send-title "BUILD FAILED"
    kterm.exe --select-tab $w --set-badge "FAIL"
    kterm.exe --select-tab $w --set-color "#ff0000"
}
```

### 4.3 YAML-Based Setup

```powershell
# Create environment from YAML definition
kterm.exe --apply dev-stack.yaml --suffix-auto
```

### 4.4 Read Output in Script

```powershell
$w = kterm.exe --new-window --profile powershell
kterm.exe --select-tab $w --send-text "Get-Process | Select-Object Name"
kterm.exe --select-tab $w --wait-for-prompt --timeout 10
$output = kterm.exe --select-tab $w --read-text --tail 20
Write-Host $output
```
