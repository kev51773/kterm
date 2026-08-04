# Phase 6 Plan - UI Tweaks

## Objective
Enhance the terminal application's user interface and experience, with a focus on custom styling, window structure, and layout refinements.

---

## 1. Combined Title and Tab Bar (Windows Terminal Style)

### A. Disable Native Window Decorations
- Turn off native titlebar and borders in [tauri.conf.json](file:///c:/Users/Kev/Desktop/Terminal/src-tauri/tauri.conf.json) by setting `"decorations": false`.

### B. Combined Tab Bar & Titlebar HTML Structure
- Integrate window drag support by applying `data-tauri-drag-region` on the background of `#tab-bar` in [index.html](file:///c:/Users/Kev/Desktop/Terminal/index.html).
- Keep interactive controls (like adding tabs, settings, and window control buttons) outside the drag region or prevent drag events on them.

### C. Custom Window Control Buttons
- Render minimize, maximize/restore, and close buttons on the right side of the tab bar.
- Use styling that closely matches the native Windows 11 Fluent design or Windows Terminal style.
- Hook click event listeners in [main.ts](file:///c:/Users/Kev/Desktop/Terminal/src/main.ts) using `@tauri-apps/api/window` to call:
  - `getCurrentWindow().minimize()`
  - `getCurrentWindow().toggleMaximize()`
  - `getCurrentWindow().close()`

### D. Windows 11 Snap Layouts & Shadow Support
- Since native window decorations are disabled, native Snap Layout menus on maximize button hover and native window shadows are lost by default.
- Integrate a Tauri plugin/crate such as `tauri-plugin-decorum` or `tauri-plugin-snap-layout` to:
  - Overlay native hit-test area on the custom maximize button.
  - Retain native Windows 11 snap layouts hover menu.
  - Retain native window drop shadows.

---

## 2. Additional UI Tweaks & Polish

### A. Conditional Terminal Pane Border Highlight
- Only show active shell highlight border (`.split-pane-wrapper.active-focus`) if the tab contains split views.
- Do not apply this highlighted border on single-pane tabs.
