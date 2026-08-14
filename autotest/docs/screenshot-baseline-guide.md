# Screenshot Baseline Guide

Success screenshots captured at the end of every GUI test. Use this guide to
review and manually approve each image before it becomes a baseline.

## How screenshots are generated

- Every GUI test captures with `captureScreenshot(spec, name)`, saving the
  window as PNG (plus a companion `<name>.json` DOM-state snapshot) into
  `autotest/screenshots/actual/<spec>/`.
- Screenshots are NOT auto-promoted to `baseline/`. The approval flow:
  1. Run the suite → screenshots land in `actual/` only; `tmp/run-manifest.json`
     records each shot's compare status (`baseline-missing` / `match` / `diff n%`).
  2. Review each image against this guide.
  3. `npm run review` → server on `http://127.0.0.1:8765` → per-test verdicts
     (fail / pass / update) → **Update selected baselines** promotes the `update`
     shots by copying `actual/` → `baseline/`.
  4. Future runs + `npm run compare` report pixel diffs against `baseline/`.
- `UPDATE_BASELINE=1` (`npm run test:update-baseline`) auto-promotes everything
  with no review — **deprecated**. Use the review server so a human approves.
- App under test boots with an isolated APPDATA (`autotest/tmp/appdata`),
  default profile `powershell`, 120x30 grid, dark theme. Window title is
  `kterm.exe - A scriptable terminal - win-1`.

## Boot modes

| Suite | Boot | Human interaction |
|---|---|---|
| `01-core` | `kterm.exe --daemon` | none |
| `02-apply` | `kterm.exe --daemon --apply=<boot-apply.yaml>` | none |
| `03-admin` | `kterm.exe --daemon --apply=<boot-admin.yaml>` | **UAC prompt — must click Yes** (or run elevated) |

`02-apply` boot fixture applies two tabs: `boot-apply-a` (powershell) and
`boot-apply-b` (powershell) with `boot-apply-b2` split right.
`03-admin` fixture applies one elevated powershell tab: `boot-admin-shell`.
If the admin suite fails, `run-all.mjs` reclassifies its tests as **skipped**
(UAC consent not approved) — the report then shows SKIPPED, not a failure.

---

## 01-core (default boot)

- `01-boot-shell.png` — one tab titled `test-01`, active, terminal pane focused,
  shell prompt **and the echoed line `Test-01 - boot shell is live` rendered**.
  (Test settles before capture so the echo is always present.)
- `02-add-tab.png` — two tabs; the second (`test-02`, rightmost) is active and
  its terminal pane is shown.
- `03-close-tab-open.png` / `03-close-tab.png` — the "before" (two tabs) and
  "after" (back to exactly one tab) of closing the last tab.
- `04-kbd-new-tab.png` — two tabs after the new-tab keybinding.
- `05-kbd-close-tab-open.png` / `05-kbd-close-tab.png` — before/after of closing
  a tab with the keyboard (two tabs → one tab).
- `06-cycle-tabs.png` — two tabs; the second is active (cycled forward with
  Ctrl+Tab).
- `07-switch-tab.png` — two tabs; the first is active after clicking it.
- `08-settings-modal.png` — settings modal open via Ctrl+,, centered over the
  terminal (theme, font, grid, keybindings, ring buffer, etc.).
- `09-settings-dropdown.png` — the same settings modal, opened via the tab-bar
  dropdown → **Settings**.
- `10-cmd-tab.png` — two tabs; the second is a spawned Command Prompt tab
  (created from the profile dropdown).
- `11-context-menu.png` — terminal-pane context menu open (Copy, Paste, Find,
  Split submenu, Highlights…, Export buffer).
- `12-split-pane.png` — active tab titled `[powershell]` (bracketed = split
  container) split into **two side-by-side terminal panes** via a vertical divider.
- `13-find-bar.png` — find bar visible over the terminal (search input).
- `14-shell-cmd.png` / `14-shell-git-bash.png` / `14-shell-wsl.png` — a multi-shell
  walk: one tab per shell profile, each with a live prompt of that shell.

---

## 02-apply (--apply session)

- `applied-tabs.png` — two tabs from the boot YAML: `boot-apply-a` (active) and
  `[boot-apply-b]` (bracketed — carries a right split). The split child
  `boot-apply-b2` is intentionally **not** a tab.
- `applied-split.png` — the `boot-apply-b` tab active, rendered as **two
  side-by-side terminal panes** (the applied right split with `boot-apply-b2`).

---

## 03-admin (--apply with admin shell)

- `admin-badge.png` — one tab titled `boot-admin-shell` with the **admin badge**
  (e.g. shield icon) marking it as elevated. **Requires a human to accept the
  UAC elevation prompt when the app starts** (or an elevated terminal).

---

## 04-ux (default boot, interaction polish)

- `01-badge-color.png` — a tab with a badge and a color swatch rendered in the
  tab bar.
- `02-highlights-added.png` / `02-highlights-removed.png` — a word highlighted in
  the terminal buffer; then the highlight removed (empty set).
- `03-kbd-prev.png` — two tabs; the first is active after the previous-tab
  keybinding.
- `04-kbd-jump.png` — two tabs; jumped directly to tab index 1/2 with the jump
  keybinding (Ctrl+Shift+1 / Ctrl+Shift+2).
- `05-kbd-split.png` / `05-kbd-unsplit.png` — split created with the keyboard;
  then unsplit back to a single pane.
- `06-paste.png` — text pasted from the clipboard visible in the terminal.
- `07-copy-selected.png` — text selected in the terminal (selection highlight
  visible) ahead of smart Ctrl+C.
- `07-paste-split.png` — a split pane created as a paste target; the copied text
  pasted into it.
- `08-drag-resize.png` / `08-unsplit.png` — a split whose divider was dragged to
  resize the pane ratio; then unsplit back to one pane.

---

## Determinism caveats

- Terminal content (prompt text, cursor) is live, so tiny pixel differences
  between runs are normal. The pixel diff treats ≤0.1% mismatched pixels as a
  `match` (`screenshot.ts` `pixelDiff`; same-aspect size mismatch is normalized
  to undo DPI scale before diffing).
- **Transient-frame shots flag `diff` every run even when correct**: boot
  animation, tab-close fades, split-resize, tab-jump and paste-caret frames
  jitter ~0.3–0.6%. Visually identical — review once, promote, expect the wobble.
- A stable shot that starts diffing after an intentional UI/theme/font change is
  **not** a regression — re-baseline it via `npm run review`.
- Admin screenshots only reproduce when someone is present to accept UAC.
- Baseline is a fixed point for the CURRENT UI — regenerate (`npm run review` →
  update) after any layout/theme change.
