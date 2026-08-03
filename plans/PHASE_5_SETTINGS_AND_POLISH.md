# Phase 5 Implementation Plan - Settings, Custom Shells & UI Polish

## Objective
Implement user configuration management (`config.json`), a dedicated Settings UI modal, custom context menus, smart clipboard interactions, shell padding, and general UI/UX polish.

---

## 1. Terminal UX & Clipboard Enhancements

### A. Internal Shell Padding & Layout
- Add internal CSS padding (`6px 8px`) around terminal panes to prevent text from touching window borders.

### B. Custom Right-Click Context Menu
- Override default browser right-click menu (`e.preventDefault()`).
- Display custom `kterm` context menu with actions:
  - *Copy* / *Paste*
  - *Split Right* / *Split Down*
  - *New Tab* / *Close Pane*

### C. Smart `Ctrl+C` Copy
- If text is selected in xterm -> `Ctrl+C` copies selection to clipboard.
- If NO text is selected -> `Ctrl+C` sends standard `\x03` (SIGINT) to running process.

### E. Tab Navigation & Creation Shortcuts
- **`Ctrl+Shift+T`**: Spawn a new tab with default profile (`powershell`).
- **`Ctrl+Tab` / `Ctrl+Shift+Tab`**: Cycle forward / backward through top-level tabs.
- **`Ctrl+Shift+1..9`**: Jump directly to tab index 1 through 9.

### F. Multi-Window Tab Movement
- **CLI Flag `--move-to-window <WIN_ID>`**: CLI command to move an active tab session from its current window into another active GUI window (`kterm.exe --select-tab tab-101 --move-to-window win-2`).
- **REST Handler `POST /tabs/move`**: Re-parents `tab_id` to `target_window_id` and updates layout trees across windows.

---


## 2. Configuration Engine (`config.json`)

### File Path
`%APPDATA%\kterm\config.json` (or `~/.config/kterm/config.json`)

### Schema Structure
```json
{
  "default_profile": "powershell",
  "close_on_exit": "graceful", // "graceful" | "always" | "never"
  "font": {

    "family": "Consolas, 'Courier New', monospace",
    "size": 14
  },
  "theme": {
    "background": "#0d0e11",
    "foreground": "#cccccc",
    "accent": "#61afef"
  },
  "profiles": [
    {
      "name": "powershell",
      "command": "powershell.exe",
      "args": ["-NoExit"],
      "env": {}
    },
    {
      "name": "cmd",
      "command": "cmd.exe",
      "args": ["/K"],
      "env": {}
    },
    {
      "name": "wsl",
      "command": "wsl.exe",
      "args": [],
      "env": {}
    },
    {
      "name": "git-bash",
      "command": "C:\\Program Files\\Git\\bin\\bash.exe",
      "args": ["--login"],
      "env": {}
    }
  ]
}
```

---

## 3. Settings UI Modal / Drawer

- **Trigger**: Gear icon in tab bar or `Ctrl+,` keyboard shortcut.
- **Features**:
  - **Profiles Tab**: Add, edit, or remove custom shell profiles and launch arguments.
  - **Appearance Tab**: Font size, font family, theme color scheme, terminal padding.
  - **Keybindings Tab**: View and customize keyboard shortcuts.

---

## 4. Future / Backlog Additions

*(Reserved for additional user-requested UI polish, extensions, and custom features)*
