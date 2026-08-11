# Screenshot Baseline Guide

Success screenshots captured at the end of every GUI test. Use this guide to
review and manually approve each image before it becomes a baseline.

## How screenshots are generated

- Every GUI test ends with `captureScreenshot(spec, name)` which saves the
  window as PNG (plus a companion `<name>.json` DOM-state snapshot) into
  `autotest/screenshots/actual/`.
- Screenshots are NOT auto-promoted to `baseline/`. Approval flow:
  1. Run the suite → screenshots land in `actual/` only.
  2. Review each image against this guide.
  3. Once approved, run `npm run test:gui:all` with `UPDATE_BASELINE=1` to
     copy the approved set into `baseline/`.
  4. Future runs + `npm run compare` report pixel diffs against `baseline/`.
- App under test boots with an isolated APPDATA (`autotest/tmp/appdata`),
  default profile `powershell`, 120x30 grid, dark theme. Window title is
  `kterm.exe - A scriptable terminal - win-1`.

## Boot modes

| Suite | Boot | Human interaction |
|---|---|---|
| `01-core` | `kterm.exe --daemon` | none |
| `02-apply` | `kterm.exe --daemon --apply=<boot-apply.yaml>` | none |
| `03-admin` | `kterm.exe --daemon --apply=<boot-admin.yaml>` | **UAC prompt — must click Yes** |

`02-apply` boot fixture applies two tabs: `boot-apply-a` (powershell) and
`boot-apply-b` (powershell) with `boot-apply-b2` split right.
`03-admin` fixture applies one elevated powershell tab: `boot-admin-shell`.

---

## 01-core (default boot)

### `actual/01-core/boot-shell.png`
- **Test:** boots with a shell ready in one tab
- **Shows:** app booted with exactly one tab titled `powershell`, active,
  terminal pane focused with a live shell prompt.
- **Steps:** launch `kterm.exe --daemon`; wait until the tab bar has 1
  `.tab-item`; click the terminal pane to focus it; verify the daemon reports
  1 tab with a non-empty title.

### `actual/01-core/add-tab.png`
- **Test:** opens a new tab with the add-tab button
- **Shows:** two `powershell` tabs; the second (rightmost) is active and its
  terminal pane is shown.
- **Steps:** click the `+` add-tab button in the tab bar; wait until 2 tabs
  render; verify the active tab has a title.

### `actual/01-core/close-tab.png`
- **Test:** closes a tab via the tab close button
- **Shows:** back to exactly one `powershell` tab (active).
- **Steps:** from a 2-tab state, click the `×` close button on a tab; wait
  until the tab bar returns to 1 tab.

### `actual/01-core/keyboard-tab.png`
- **Test:** opens a tab with Ctrl+Shift+= and closes with Ctrl+Shift+-
- **Shows:** one `powershell` tab (active), the state after both keyboard
  actions completed.
- **Steps:** press `Ctrl+Shift+=` (new tab) → verify 2 tabs; press
  `Ctrl+Shift+-` (close tab) → verify 1 tab.

### `actual/01-core/cycle-tabs.png`
- **Test:** cycles tabs with Ctrl+Tab
- **Shows:** two `powershell` tabs; the second tab is active (cycled forward
  from tab 1).
- **Steps:** add a second tab via the `+` button; press `Ctrl+Tab`; assert the
  active tab changed.

### `actual/01-core/switch-tab.png`
- **Test:** switches tabs by clicking
- **Shows:** two `powershell` tabs; the first tab is active after clicking it.
- **Steps:** add a second tab; click the first tab; assert the `.active` class
  moved to it.

### `actual/01-core/settings-modal.png`
- **Test:** opens and closes the settings modal (Ctrl+,)
- **Shows:** the settings modal open, centered over the terminal — options for
  theme, font, terminal grid, keybindings (incl. split shortcuts), ring buffer,
  etc., with a close button.
- **Steps:** press `Ctrl+,`; wait for `.settings-modal`; capture; click the
  modal close button; verify it closes.

### `actual/01-core/settings-dropdown.png`
- **Test:** opens settings via the settings dropdown
- **Shows:** same settings modal open — reached through the tab-bar dropdown
  instead of the keyboard shortcut.
- **Steps:** click the tab-bar dropdown button (`#tab-dropdown-btn`); click the
  **Settings** item in the profile dropdown menu; wait for `.settings-modal`;
  capture; click close; verify it closes.

### `actual/01-core/profile-dropdown.png`
- **Test:** opens the profile dropdown and selects a profile
- **Shows:** two `powershell` tabs; the second is active — created by choosing
  the first profile entry in the dropdown.
- **Steps:** click `#tab-dropdown-btn`; assert the dropdown lists at least one
  profile; click the first item; wait until a 2nd tab renders.

### `actual/01-core/context-menu.png`
- **Test:** right-click opens the context menu
- **Shows:** the terminal pane context menu open with items **Copy**
  (Ctrl+Shift+C), **Paste** (Ctrl+Shift+V), **Find** (Ctrl+Shift+F), a
  **Split** submenu (Split Right/Left/Down/Up, Un-split Pane), **Highlights...**,
  **Export buffer**.
- **Steps:** right-click the terminal pane (pointer button 2); wait for
  `.context-menu` with at least one item; capture (menu still open).

### `actual/01-core/split-pane.png`
- **Test:** splits a pane right and unsplits
- **Shows:** the active tab now titled `[powershell]` (bracketed = split
  container) split into **two side-by-side terminal panes** via a vertical
  divider.
- **Steps:** right-click the terminal pane; hover the **Split** submenu
  (pointer-move required — submenu is hover-only); click **Split Right**; wait
  for 2 `.split-pane-wrapper` panes; capture; then unsplit back to one pane via
  the context menu **Un-split Pane**.

### `actual/01-core/find-bar.png`
- **Test:** opens the find bar with Ctrl+F
- **Shows:** the find bar visible at the top of the terminal pane (search
  input) over the shell prompt.
- **Steps:** focus the terminal pane; press `Ctrl+F`; wait for the `.find-input`;
  capture; click the find bar close button; verify it closes.

### `actual/01-core/second-window.png`
- **Test:** opens and closes a second window
- **Shows:** win-1 with its single `powershell` tab — captured after the daemon
  accepted the close of the second window (win-2).
- **Steps:** daemon `--new-window`; wait until daemon lists 2 windows; daemon
  `--close-window` on the new window. NOTE: under a WebDriver session,
  tauri-driver holds WebView2 windows open, so physical teardown cannot be
  asserted — the test asserts the daemon accepts the close request. The
  screenshot is of the primary window, which is unaffected.

---

## 02-apply (--apply session)

### `actual/02-apply/applied-tabs.png`
- **Test:** renders the applied session tabs in the GUI
- **Shows:** two tabs from the boot YAML: `boot-apply-a` (active) and
  `[boot-apply-b]` (bracketed — carries a right split). The split child
  `boot-apply-b2` is intentionally not a tab.
- **Steps:** boot with `--apply=<boot-apply.yaml>`; wait for 2 `.tab-item`;
  verify titles `boot-apply-a` and `boot-apply-b` are present and
  `boot-apply-b2` is NOT a tab.

### `actual/02-apply/applied-split.png`
- **Test:** shows the split pane from the applied session
- **Shows:** the `boot-apply-b` tab active, rendered as **two side-by-side
  terminal panes** (the applied right split with `boot-apply-b2`).
- **Steps:** click the `boot-apply-b` tab; wait for 2 `.terminal-pane` and 2
  `.split-pane-wrapper`.

---

## 03-admin (--apply with admin shell)

### `actual/03-admin/admin-badge.png`
- **Test:** renders an admin-badged tab from the applied session
- **Shows:** one tab titled `boot-admin-shell` with the **admin badge** (e.g.
  shield icon) marking it as elevated.
- **Steps:** boot with `--apply=<boot-admin.yaml>` (admin shell tab); wait for
  `.tab-item.tab-admin`; verify title `boot-admin-shell`; verify the
  `.tab-admin-badge` renders. **Requires a human to accept the UAC elevation
  prompt when the app starts.**

### `actual/03-admin/single-tab.png`
- **Test:** has exactly one tab
- **Shows:** the same single `boot-admin-shell` tab, confirming no extra tabs
  were created by the admin boot path.
- **Steps:** assert `$$('.tab-item').length === 1`. **UAC prompt required.**

---

## Determinism caveats

- Terminal content (prompt text, cursor) is live, so tiny pixel differences
  between runs are normal. `compare.ts` treats ≤0.1% mismatched pixels as a
  match; re-baseline after any intentional UI/theme/font change.
- Admin screenshots only reproduce when someone is present to accept UAC.
- Baseline is a fixed point for the CURRENT UI — regenerate (`UPDATE_BASELINE=1`)
  after any layout/theme change.
