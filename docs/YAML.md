# kterm YAML Layout System

This document describes the declarative YAML session specification used by kterm to define, save, and restore window layouts.

---

## 1. Overview

kterm can apply a YAML file that declaratively describes a complete window layout — tabs, shells, splits, titles, badges, colors, and admin elevation. This enables:

- **Reproducible environments**: Save a layout, restore it later.
- **Scripting**: Shell scripts or CI pipelines can launch kterm with a specific layout.
- **Desktop shortcuts**: A `.lnk` shortcut can open a YAML layout directly.

### 1.1 Applying a YAML File

```bash
# From CLI
kterm --apply my-layout.yaml

# With window suffix (creates win-backend, win-frontend, etc.)
kterm my-layout.yaml --suffix -backend

# With auto-incrementing suffix (creates win-1, win-2, etc.)
kterm my-layout.yaml --suffix-auto

# Dry run (validate only, no changes)
kterm my-layout.yaml --dry-run

# Target a specific window
kterm my-layout.yaml --window win-3
```

### 1.2 Exporting a YAML File

```bash
# Export current layout
kterm --export-layout my-layout.yaml

# Or from the UI: Profile dropdown → Export Layout...
```

### 1.3 YAML Application via HTTP API

```bash
POST /apply
Content-Type: application/json

{
  "yaml": "<yaml-content>",
  "window": "win-1",       // optional: override window ID
  "suffix": "-backend",    // optional: append to window ID
  "suffix_auto": true,     // optional: auto-increment window ID
  "dry_run": false         // optional: validate only
}
```

---

## 2. YAML Schema

```yaml
window:
  id: string | null        # Window ID (null = auto-generate)
  title: string | null     # Window title (null = default format)

tabs:
  - id: string | null      # Tab ID (null = auto-generate)
    profile: string         # Shell profile (required)
    title: string | null    # Custom tab title
    badge: string | null    # Badge text (e.g., "PROD")
    color: string | null    # Accent color hex (e.g., "#ff3333")
    cwd: string | null      # Working directory
    admin: boolean | null   # Run as Administrator (deprecated, use elevated)
    elevated: boolean | null # Run as Administrator
    send_text: string | null # Command to send after shell starts
    splits:                 # Nested splits (optional)
      - direction: string   # "right" | "left" | "down" | "up"
        profile: string     # Shell profile for the new pane
        id: string | null   # Tab ID for the new pane
        title: string | null
        badge: string | null
        color: string | null
        cwd: string | null
        admin: boolean | null
        elevated: boolean | null
        send_text: string | null
        splits:             # Recursive (arbitrary depth)
          - ...
```

---

## 3. Field Reference

### 3.1 Window Spec

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | string \| null | No | Window identifier. `null` or omit → auto-generate (`win-1`, `win-2`, ...). If the window already exists, it is reused (existing tabs are closed first). |
| `title` | string \| null | No | Window title. `null` → `kterm.exe - A scriptable terminal - {window_id}`. If the string starts with `kterm.exe`, it's used as-is; otherwise formatted as `kterm.exe - {title} - {window_id}`. |

### 3.2 Tab Spec

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | string \| null | No | Tab identifier. `null` → auto-generate (`tab-N` where N is the next unused number in the window). |
| `profile` | string | **Yes** | Shell to launch. One of: `powershell`, `cmd`, `wsl`, `git-bash`. |
| `title` | string \| null | No | Custom tab title. `null` → defaults to the profile name (e.g., "powershell"). |
| `badge` | string \| null | No | Badge text displayed on the tab (e.g., "PROD", "DEV"). |
| `color` | string \| null | No | Accent color for the tab's top border (hex format, e.g., `"#ff3333"`). |
| `cwd` | string \| null | No | Working directory for the shell. `null` → shell default. |
| `admin` | boolean \| null | No | **Deprecated alias for `elevated`.** |
| `elevated` | boolean \| null | No | If `true`, the shell runs with Administrator privileges (UAC prompt on Windows). |
| `send_text` | string \| null | No | Text/command to send to the shell after it starts. Automatically appends `\r` if not present. |
| `splits` | array \| null | No | Nested split panes (see §3.3). |

### 3.3 Split Spec (recursive)

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `direction` | string | **Yes** | Where to place the new pane relative to the target: `"right"`, `"left"`, `"down"`, `"up"`. |
| `profile` | string | **Yes** | Shell profile for the new pane. |
| `id` | string \| null | No | Tab ID for the new pane. `null` → auto-generate. |
| `title` | string \| null | No | Custom title for the new pane's tab. |
| `badge` | string \| null | No | Badge for the new pane. |
| `color` | string \| null | No | Accent color for the new pane. |
| `cwd` | string \| null | No | Working directory. |
| `admin` | boolean \| null | No | Deprecated. Use `elevated`. |
| `elevated` | boolean \| null | No | Run as Administrator. |
| `send_text` | string \| null | No | Command to send after shell starts. |
| `splits` | array \| null | No | Further nested splits (recursive, unlimited depth). |

---

## 4. Split Direction Semantics

Split directions are relative to the **target tab** (the tab being split):

```
         ┌──────────┬──────────┐
         │  target  │   right  │   direction: "right" → Horizontal, insert_second
         │          │          │
         ├──────────┴──────────┤
         │       (below)       │
         └─────────────────────┘
```

| Direction | LayoutTree Operation | New Pane Position |
|-----------|---------------------|-------------------|
| `"right"` | `Horizontal, insert_first=false` | New pane to the right. |
| `"left"` | `Horizontal, insert_first=true` | New pane to the left. |
| `"down"` | `Vertical, insert_first=false` | New pane below. |
| `"up"` | `Vertical, insert_first=true` | New pane above. |

---

## 5. Window ID Resolution

When applying a YAML spec, the window ID is resolved in this order:

1. `--window` CLI flag (or `window` field in API request) → use directly.
2. `window.id` in the YAML spec → use directly.
3. Neither specified → auto-generate the next available ID (`win-1`, `win-2`, ...).

### 5.1 Suffix Mode

```bash
kterm my-layout.yaml --suffix -backend
```

If the base window ID is `win-1` and suffix is `-backend`, the resolved ID is `win-1backend`.

### 5.2 Auto-Suffix Mode

```bash
kterm my-layout.yaml --suffix-auto
```

If `win-1` already exists, tries `win-1-1`, `win-1-2`, etc. until an unused ID is found.

### 5.3 Window Reuse

If the resolved window ID already exists:
- All existing tabs in that window are closed.
- The window's layout is cleared.
- The new YAML spec is applied fresh.

Exception: If the window is the initial `win-1` and has never had tabs (untouched), it is reused silently.

---

## 6. Validation Rules

Dry-run mode (`--dry-run`) validates the YAML without making changes:

1. YAML must parse successfully.
2. Must contain at least one tab.
3. Every profile must be one of: `powershell`, `cmd`, `wsl`, `git-bash`.
4. Every split direction must be one of: `right`, `left`, `down`, `up`.
5. If the target window already exists and is not the untouched initial window, validation fails with an error.

---

## 7. Complete Example

```yaml
window:
  id: win-dev
  title: Development Environment

tabs:
  - id: backend
    profile: powershell
    title: Backend API
    badge: DEV
    color: "#4caf50"
    cwd: C:\Projects\backend
    send_text: npm run dev

  - id: frontend
    profile: git-bash
    title: Frontend
    cwd: C:\Projects\frontend
    splits:
      - direction: right
        profile: git-bash
        title: Git Status
        send_text: git status

      - direction: down
        profile: wsl
        title: Docker
        cwd: /home/user/projects
        send_text: docker compose up

  - id: database
    profile: cmd
    title: Database
    elevated: true
    badge: ADMIN
    color: "#f44336"
    send_text: mysql -u root -p
```

This creates:

```
┌────────────────┬────────────────┐
│  Backend API   │   Git Status   │
│  (powershell)  │   (git-bash)   │
│  DEV badge     │                │
├────────────────┼────────────────┤
│   Frontend     │    Docker      │
│   (git-bash)   │    (wsl)       │
├────────────────┴────────────────┤
│   Database (cmd, ADMIN)         │
└─────────────────────────────────┘
```

---

## 8. Exported YAML Format

When exporting via `--export-layout` or the UI, the YAML output follows this structure:

```yaml
window:
  id: win-1
  title: kterm.exe - A scriptable terminal - win-1
tabs:
  - id: tab-1
    profile: powershell
    title: Backend Server
    badge: DEV
    color: "#4caf50"
    splits:
      - direction: right
        profile: git-bash
        title: Git Repo
```

Notes:
- Split relationships are expressed as nested `splits` arrays within the first pane of each split.
- The exported YAML preserves all metadata (titles, badges, colors, admin status).
- Layout ratios are NOT preserved in the YAML export (splits default to 50/50 on restore).
- Untracked tabs (not in any layout tree) are appended at the end.
