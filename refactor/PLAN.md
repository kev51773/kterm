# kterm Refactor Plan

Refactor of `src/` (frontend) and `src-tauri/src/` (backend). Verification tool
is `autotest/` — it stays untouched.

*Reevaluated 2026-08-15.* Key changes since the original plan:
- Stage order flipped: backend first (unchurned, lowest risk), frontend last
  (scrollbar-fit work made `src/main.ts` the most delicate file — extract only
  after the backend is settled).
- Stage 4.5 (E2E test system placeholder) removed: `autotest/` matured into
  exactly that system (50 tests, WebDriver+CLI, screenshot baselines + review
  workflow). Nothing to build.
- Gate facts corrected: `cargo test` now has 87 `#[test]` cases (was 111), and
  the per-stage doc is `docs/APP.md` (`NEW-CODEBASE.md` never existed).

## Approved stages

| # | Target | Work |
|---|--------|------|
| 1 | `src-tauri/src/daemon/mod.rs` | Split routes into modules (table below). `AppState` + router stay in `mod.rs`. |
| 2 | `src-tauri/src/pty/manager.rs` | Split into `pty/session.rs`, `pty/platform.rs`, `pty/elevated.rs`. |
| 3 | `src/main.ts` + daemon | Extract modules, then harden: `PaneInstance` type, unify error handling. |
| 4 | both | Security / polish: audit privileged surfaces, tighten IPC, dead-code sweep. |

Already done (do not repeat):
- Pilot: `admin_debug.log` file logging removed from `pty/manager.rs`.
- `src/components/SettingsModal.ts`, `src/components/SplitGrid.ts` extracted.
- Backend already split into `client.rs`, `config.rs`, `exporter.rs`, `yaml.rs`,
  and pty into `layout.rs`, `ring_buffer.rs`.

## Gate after every stage

1. `cargo test --bin kterm` in `src-tauri/` — expect 87 passing.
2. `npm run build` in `autotest/` (release exe, tests exercise the packaged binary).
3. `npm run test:gui:all` in `autotest/` (elevated terminal — admin suite fires UAC).
4. Update `docs/APP.md` for the stage. `docs/CODEBASE.md` is the preserved original — never modify.

## 3-way decision per stage

- **a — commit**: all tests green, screenshots clean, no new concerns.
- **b — manual review**: tests pass but screenshots diff (even transient jitter) → `npm run review`, human verdicts the changed shots, then commit. Screenshots never gate the decision; a diff is a review request, not a failure. Never auto-promote baselines (`test:update-baseline` / `UPDATE_BASELINE=1` deprecated).
- **c — investigate & fix**: tests fail or a large/unexplained diff appears → stop, fix, re-run the gate.

## Stage 1 — backend daemon split

`src-tauri/src/daemon/mod.rs` (~1228 lines, down from 1262 after `set-window-title` removal) → modules; `AppState` + router stay in `mod.rs`:

```
daemon/types.rs    — session/tab/split structs + serde
daemon/tabs.rs     — tab create/close/cycle/focus handlers
daemon/windows.rs  — window create/focus handlers
daemon/splits.rs   — split/unsplit/resize handlers
daemon/session.rs  — session lifecycle, output routing
daemon/export.rs   — export-layout handler
daemon/websocket.rs— WS upgrade + message dispatch
daemon/config.rs   — config load/save handlers
daemon/system.rs   — system info / DPI handlers
```

## Stage 2 — backend PTY split

`src-tauri/src/pty/manager.rs` (1008 lines after the debug-log removal) →

```
pty/session.rs   — PtySession struct, spawn/close/read/write
pty/platform.rs  — win32 named-pipe + elevated bridge helpers
pty/elevated.rs  — ShellExecuteW runas launch + bridge entry
```

`PtyManager` + `run_elevated_pty_bridge` dispatch stay in `manager.rs` or `mod.rs`.

## Stage 3 — frontend extraction + hardening

`src/main.ts` (~2378 lines, grew +239 lines from tab badge/color context menu feature) → extract in this order (each extraction is a mini-stage with its own gate):

```
src/state.ts        — window/tab/pane state store (tabsMap, currentWindowId,
                      highlight/decoration maps, fit flags incl. unsplitShrinkPending)
src/daemon.ts       — WebSocket client + backend RPC (DAEMON_URL/WS_URL, connectWebSocket)
src/terminal.ts     — xterm lifecycle per pane + fit/scrollbar logic
                      (renderActiveLayout fit sites, resizeObserver, getXtermCellDimensions)
src/tabs.ts         — tab model + tab-bar headers (createTabLocal, updateTabLocal,
                      removeTabLocal, switchTab, cycleTabs, renderTabBarHeaders,
                      checkTabOverflow, spawnDefaultTab, tab badge/color state)
src/splits.ts       — split tree + resize/unsplit (renderActiveLayout, containsTab,
                      getTabIdsInNode, activeLayoutIsSplit, splitGrowPending)
src/findBar.ts      — FindBar (showFindBar, closeFindBar, positionFindBar, getTerminalBufferText)
src/highlights.ts   — highlight markers (updatePaneHighlights, highlightFlashPhase)
src/config.ts       — config load/apply (applyAppConfig, getContainerGridDimensions, PROFILES)
src/components/InputModal.ts      — modal for cmd input (showInputModal)
src/components/HighlightsModal.ts — modal for highlight rules (showHighlightsModal)
src/components/TabBar.ts          — tab strip
src/components/ProfileDropdown.ts — profile switcher (renderProfileDropdownMenu,
                                     open/closeProfileSubMenu)
src/components/ContextMenu.ts     — menu rendering (showTerminalContextMenu,
                                     showTabHeaderContextMenu, positionContextMenu,
                                     color swatches & badge menu logic)
```

Each extraction: move code verbatim, wire imports, gate, 3-way decision. No
behavior changes during extraction. **Risk note:** the phantom-scrollbar fit
guards (`historyApplied`, `splitGrowPending`, `unsplitShrinkPending`,
`hasAdjustedWindowSize`/`lastAdjustedWasSplit`) are the subtlest code in the
file and are read across every fit site — extract them as one block
(`src/state.ts` + `src/terminal.ts`) and lean on the full autotest gate after
each move.

Final mini-stage — hardening (no new features):
- Introduce `PaneInstance` type to replace ad-hoc pane state shapes.
- Unify error paths: single WS error handler, one fallback UI for terminal failures.

## Stage 4 — security / polish

- Audit elevated-spawn surface (UAC + named pipes) for trust-boundary validation.
- Tighten IPC: validate/whitelist WS command names and args.
- Polish: dead-code sweep flagged by codegraph/token-savior audits.

## Progress log

- [x] 2026-08-14 — Pilot: removed `debug_log` / `admin_debug.log` from `pty/manager.rs` (old Stage 5 item done early). 111 Rust tests, 50/50 autotest, 8 jitter-band screenshot diffs human-confirmed as known jitter.
- [x] 2026-08-15 — Plan reevaluated: reordered (backend first), stage 4.5 dropped (autotest is the E2E system), gate facts corrected to 87 tests + `docs/APP.md`. No stages executed.
- [x] 2026-08-15 — Stage 1 executed: Split `src-tauri/src/daemon/mod.rs` into `types.rs`, `tabs.rs`, `windows.rs`, `splits.rs`, `session.rs`, `export.rs`, `websocket.rs`, `config.rs`, `system.rs`. 87/87 binary unit tests (111/111 total cargo tests) green, release binary built, `docs/APP.md` updated.
- [x] 2026-08-15 — Stage 2 executed: Split `src-tauri/src/pty/manager.rs` into `pty/session.rs`, `pty/platform.rs`, `pty/elevated.rs`. 87/87 binary unit tests (111/111 total cargo tests) green, release binary built, `docs/APP.md` updated.
- [x] 2026-08-15 — Stage 3 executed: Extracted `src/main.ts` into modular components (`state.ts`, `config.ts`, `daemon.ts`, `terminal.ts`, `tabs.ts`, `splits.ts`, `findBar.ts`, `highlights.ts`, `components/InputModal.ts`, `components/HighlightsModal.ts`, `components/ContextMenu.ts`, `components/ProfileDropdown.ts`, `components/TabBar.ts`). Typed `PaneInstance` introduced, TypeScript typecheck passed cleanly, 111/111 cargo tests green, `docs/APP.md` updated.



