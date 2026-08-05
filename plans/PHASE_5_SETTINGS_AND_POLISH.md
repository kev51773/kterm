# Phase 5 Unified Plan: Settings, Frameless Window, UX & Polish

Unified blueprint combining configuration management, category-based Settings UI, custom frameless window titlebar with Windows 11 Snap Layouts, keyboard shortcuts, dual context menus, conditional pane borders, and known bug fixes.

---

## 1. Configuration & Settings Modal (Sidebar Layout)

### A. Config Engine (`config.rs` & `%APPDATA%\kterm\config.json`)
- Persistent `config.json` storing:
  - `default_profile`: Default shell (e.g. `"powershell"`).
  - `ring_buffer_kb`: Capacity in KB (default `256`).
  - `terminal_padding`: Internal CSS padding in pixels (default `8`).
  - `font`: `{ "family": "Consolas, 'Courier New', monospace", "size": 14 }`.
  - `theme`: `{ "background": "#0d0e11", "foreground": "#cccccc", "accent": "#61afef" }`.
- Daemon endpoints: `GET /config`, `POST /config`.

### B. Settings Modal (`SettingsModal.ts`)
- Left-hand category sidebar:
  - **General**: Default profile, ring buffer size.
  - **Appearance**: Font family, font size, theme colors, terminal padding.
  - **Keybindings**: View keyboard shortcuts list.
- Triggered by ⚙️ gear icon in tab bar or `Ctrl+,`.

---

## 2. Custom Frameless Window & Titlebar (Windows Terminal Style)

### A. Frameless Window Setup
- Set `"decorations": false` in `tauri.conf.json`.
- Apply `data-tauri-drag-region` on tab bar background.

### B. Custom Window Controls
- Windows 11 style Minimize (`_`), Maximize/Restore (`□`), and Close (`✕`) buttons on top right of tab bar.
- Connected via `@tauri-apps/api/window` (`minimize`, `toggleMaximize`, `close`).
- Retain Windows 11 Snap Layouts and window drop shadows.

---

## 3. Terminal UX, Shortcuts & Dual Context Menus

### A. Keyboard Shortcuts
- **`Ctrl+Shift+T`**: Spawn new default tab.
- **`Ctrl+Tab` / `Ctrl+Shift+Tab`**: Cycle tabs forward/backward.
- **`Ctrl+Shift+1..9`**: Jump to tab index 1–9.
- **`Ctrl+,`**: Open Settings modal.

### B. Smart `Ctrl+C` Copy
- If text is selected in xterm -> copy selection to clipboard.
- If NO text is selected -> send `\x03` (SIGINT) to process.

### C. Dual Context Menus
- **Terminal Pane Context Menu**: Right-click in terminal -> *Copy*, *Paste*.
- **Tab Header Context Menu**: Right-click tab header -> *New Tab*, *Split Right*, *Split Down*, *Close Tab*.

---

## 4. Visual Polish & Bug Fixes

### A. Conditional Active Focus Highlight
- Only show active focus highlight border (`.split-pane-wrapper.active-focus`) if tab contains split panes.
- Hide border highlight on single-pane tabs.

### B. Known Bug Fixes
- **Fix Bug 1**: Resolve initial shell launch PTY exit code 1 / spawn timing race.
- **Fix Bug 2**: Resolve initial scrollbar blank padding before prompt (trigger fit addon resize on term container mount).
