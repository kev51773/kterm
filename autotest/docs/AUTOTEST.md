# kterm Autotest System

Maintainer's guide. For another AI agent (or human) adding tests to, or debugging, the kterm autotest suite.

Related: [screenshot-baseline-guide.md](./screenshot-baseline-guide.md) (visual baselines only).

---

## What This Tests — And Against What

These are **end-to-end tests of the true packaged app**: the release binary at
`src-tauri/target/release/kterm.exe`, running its own embedded WebView2. There is
**no dev server**. Anything less than the packaged app is a waste of time — the
system was once broken precisely because tests ran against `http://127.0.0.1:1420`
(vite dev) instead of the embedded frontend, and every suite "passed" while the
real app was broken.

**Corollary: after any change under `src/` or `src-tauri/src/`, the binary must be
rebuilt** (`npm run build` in autotest) before the tests exercise it. wdio does not
build for you. Frontend changes are baked into the release binary at build time.

---

## Architecture: One Test Run, End to End

```
npm run test:gui                          (wdio run wdio.conf.ts)
  └─ @wdio/tauri-service (services.tauri)  appArgs, env, driver config
       ├─ spawns tauri-driver              driverProvider: 'external'
       ├─ tauri-driver auto-downloads msedgedriver   (autoDownloadEdgeDriver: true)
       ├─ tauri-driver launches kterm.exe with APP_ARGS + APPDATA override
       ├─ kterm opens a WebView2 window (the "browser")
       ├─ wdio drives the webview DOM via browser./$/$  ← GUI tests
       └─ kterm's daemon listens on http://127.0.0.1:9999
            └─ CLI tests reach it by invoking kterm.exe --<flag> (cliOk helpers)
```

Key config (`wdio.conf.ts`):
- `maxInstances: 1`, `waitforTimeout: 15000`, mocha `timeout: 180000`, `connectionRetryTimeout: 120000`
- `specs` glob covers `specs/gui/*.spec.ts` + `specs/cli/*.spec.ts`; every npm script
  narrows with `--spec`.
- `onPrepare`: kills stale processes, writes an isolated APPDATA + boot fixtures.
- `afterTest`: on failure, captures a screenshot (`screenshots/actual/<spec>/FAIL-<test>.png`).

### Boot Modes — One App Instance Per Run

The app boots **once per wdio run**, with mode-dependent args. The three modes
therefore cannot be mixed in a single run:

| `KTERM_BOOT_MODE` | App args | Purpose |
|---|---|---|
| *(unset / `default`)* | `--daemon` | Bulk of GUI tests (`01`, `04`) and all CLI suites |
| `apply` | `--daemon --apply=<tmp>/appdata/boot-apply.yaml` | Session applied at startup (`02`) |
| `admin` | `--daemon --apply=<tmp>/appdata/boot-admin.yaml` | Boots an `admin: true` tab (`03`) |

Boot fixtures are written by `writeBootFixtures()` in `wdio.conf.ts`. **Arg order
matters**: `--daemon` must precede `--apply`.

## Running The Suites

From `autotest/`:

| Command | Boot mode | What runs |
|---|---|---|
| `npm run test:gui` | default | `01-core.spec.ts` (15 tests) |
| `npm run test:gui:apply` | apply | `02-apply.spec.ts` (2 tests) |
| `npm run test:gui:admin` | admin | `03-admin.spec.ts` |
| `npm run test:gui:ux` | default | `04-ux.spec.ts` (6 tests) |
| `npm run test:gui:all` | mixed | `admin → core → apply → ux` (each its own run) |
| `npm run test:cli` | default | all `specs/cli/*.spec.ts` |
| `npm run test` | default | everything in the `specs` glob |
| `npm run test:update-baseline` | — | same as test, but refreshes screenshot baselines |
| `npm run compare` | — | pixel-diff actual vs baseline (the visual gate) |
| `npm run build` | — | rebuild release binary (`cd .. && npx tauri build --no-bundle`) |

**`test:gui:all` must run from an elevated terminal** (admin suite fires a real UAC
prompt — see gotcha). Order is deliberate: admin first so the UAC prompt appears
~3s after start; the user clicks it and walks away.

## Anatomy of a Test

Tests split into three styles; the strong pattern is **hybrid**:

1. **GUI (WebDriver)** — `browser.$('.tab-item')`, `waitUntil`, `click()`, typing into the
   webview. Assert the *rendered DOM*.
2. **CLI/daemon** — `cliOk(['--list-tabs', ...])` runs `kterm.exe` against the live daemon.
   Assert *app state*.
3. **Hybrid (recommended)** — use CLI helpers to *set up state*, WebDriver to *assert the
   DOM*, then CLI helpers to clean up. E.g. spawn a tab via `spawnTab()`, assert
   `.tab-item` count rendered, close it via `closeTab()`.

State setup/teardown helpers (`src/helpers/state.ts`):
- `resetGuiState(win)` — collapses to one window/tab. **Never closes the last window**:
  the webdriver session lives in the daemon, and closing the last window kills the
  daemon (and the run).
- `settle(win)` — waits for a prompt then a short pause; cheap synchronization.
- `waitForTabCount(n)` — DOM-side poll (`.tab-item` count).

## Helpers API

| Helper file | What it gives you |
|---|---|
| `src/helpers/paths.ts` | `APP_BINARY`, `DAEMON_BASE_URL`, `APPDATA_DIR`, `KTERM_CONFIG`, screenshot dirs, `PINNED_CONFIG`, `UPDATE_BASELINE` |
| `src/helpers/env.ts` | `ensureIsolatedAppdata()`, `writePinnedConfig()`, `readConfig()`, `killAllKterm()`, `buildApp()` |
| `src/helpers/daemon.ts` | `cli()` / `cliOk()` + typed wrappers: `listTabs`, `listWindows`, `spawnTab`, `waitForPrompt`, `waitFor`, `readText`, `sendText`, `closeTab`, `closeWindow`, `newWindow`, `setTabTitle`, `setBadge`, `setColor`, `splitTab`, `unsplit`, `explodeSplit`, `exportLayout`, `setWindowTitle`, `normalizeWindow`, `setClipboard`, `getClipboard`, `waitDaemonReady` |
| `src/helpers/state.ts` | `resetGuiState`, `settle`, `sendToTab`, `waitForTabCount`, `activeTabTitle` |
| `src/helpers/ui.ts` | `waitForTabCount`, `focusTerminal`, `rightClick`, `openContextMenu` (retries right-click), `getConfig` (HTTP `GET /config`) |
| `src/helpers/screenshot.ts` | `captureScreenshot(spec, name)`, `captureFailureScreenshot` |

CLI helpers shell out to `kterm.exe` (`execFileSync`, 60s timeout) and talk to the
daemon that wdio's app instance already started. **Reuse `cliOk` and the wrappers —
do not reinvent `--flag` invocations per spec.**

## Screenshots And Baselines

`captureScreenshot()` saves `screenshots/actual/<spec>/<name>.png`, then:
- no baseline yet → `baseline-missing` (does **not** fail the test);
- baseline exists → pixel-diff; a mismatch is logged, **does not fail the test**;
- `UPDATE_BASELINE=1` → overwrites baseline.

**Screenshots never gate a test.** The gate is `npm run compare`
(`compare/compare.ts`). CI-style guarding is opt-in — do not turn a screenshot
diff into a hard failure without asking.

---

## Gotchas (Learned The Hard Way — Read Before Writing Tests)

### App / binary
- **Release exe must have `custom-protocol` in tauri features** (`src-tauri/Cargo.toml`).
  Without it, release builds don't embed `dist/` and the window loads `devUrl`
  (`http://127.0.0.1:1420`); with no vite server running you get a chrome-error
  "can't reach 127.0.0.1", `hasSync: false`, empty DOM. This single omission made
  every suite fail while dev-mode testing "passed". **Verify a new failure is not
  this**: after boot, window URL must be `http://tauri.localhost/`.
- **Rebuild after ANY source change.** `npm run build` before re-running tests,
  or you are testing a stale binary.
- **Do not edit files outside `autotest/` without explicit approval.** The
  `custom-protocol` Cargo.toml change was the single approved exception. If a new
  test needs an app-side change, ask the user first.

### Boot / lifecycle
- **One app instance per run.** No suite may assume a previous suite left state;
  `onPrepare` kills everything and `resetGuiState` normalizes between tests.
- **Never kill `msedgewebview2` processes by hand** — Windows Search (SearchHost.exe)
  uses the same binary; those are not test orphans. `killAllKterm()` targets only
  `kterm`, `msedgedriver`, `tauri-driver`.
- **Closing the last window kills the daemon and the run.** All teardown keeps at
  least one window alive.
- **WebView2 `0x8007139F`** (transient connection loss) appears under load; it is
  flake, not a code bug. Rely on `waitforTimeout: 15000` + `connectionRetryTimeout`.
- **admin mode: real UAC.** `admin: true` tabs spawn via `ShellExecuteW runas` →
  an actual Windows UAC prompt. Unanswered, `ShellExecuteW` hangs → ~30s connect
  timeout → tab never created → test fails. **Run the admin suite from an elevated
  terminal** (then no prompt at all). If the prompt appears and the test fails,
  first suspect "did someone click UAC?". App's elevated-spawn trace lands in
  `C:\Users\Kev\Desktop\admin_debug.log` (truncated each run).
- **Apply/admin modes change the boot fixture**, not just the config. Both suites
  require their `KTERM_BOOT_MODE`; running `02-apply` under `default` mode fails.

### Clipboard (04-ux, flakiest area)
- `Set-Clipboard -Value ''` throws `ArgumentNullException` ("Value cannot be null").
  **Never clear the clipboard with an empty string** — use a sentinel like `' '`
  (harmless: no test asserts the clipboard equals `''`, and stale values can't
  false-pass `includes()` checks anyway).
- `navigator.clipboard.writeText` inside the webview is async and focus-sensitive;
  paste/copy assertions need generous waits (8s in 04-ux). Selection must exist
  before "smart Ctrl+C" copies anything.
- Clipboard is process-global and survives between tests — **if test N+1 is
  clipboard-flaky, suspect leftover clipboard content from test N**, not the code.

### wdio / drivers
- **`postinstall` patches `@wdio/tauri-service`** (`maxAttempts` 100 → 1) because the
  app has no `tauri-plugin-wdio`; the service's plugin check otherwise retries 100×
  per focus command (~13min/suite of pure waste). **After `npm install` (or adding
  deps) the patch must re-apply** — `node scripts/patch-tauri-service.cjs` warns if
  the layout changed.
- msedgedriver versions auto-download to `%TEMP%/msedgedriver/<ver>-*`; a stale
  driver binary of the wrong version causes opaque connect failures. `onPrepare`
  kills `msedgedriver`; if driver weirdness persists, clear that temp dir.
- Driver health is checked over WebDriver (not the app's tauri plugin). A test
  that "passes to the daemon but hangs on the first `browser.$`" is a driver boot
  problem, not an app problem.

### Windows shell quirks
- The toolchain assumes a POSIX-ish shell (git-bash). Prefer `execFileSync` with
  explicit arg arrays; when you must shell out to PowerShell use
  `powershell.exe -NoProfile -Command "..."` (as `setClipboard`/`getClipboard` do).
- A stray file named `nul` may appear in repo roots — Windows redirect artifact;
  ignore/delete, don't chase it.
- `logs/wdio-*.log` capture backend output (`captureBackendLogs: true`); the
  relevant run's log is named by timestamp — correlate with the failing run time.

---

## Adding A New Test — Checklist

1. **Does it belong here?** New behavior in `src/` or `src-tauri/` → yes. Decide GUI
   (DOM), CLI (daemon state), or hybrid.
2. **Ask before touching app code.** If the test needs an app-side capability that
   doesn't exist, stop and ask — don't quietly extend the app.
3. **Place it**: GUI → `specs/gui/0X-*.spec.ts`, CLI → `specs/cli/2X-*.spec.ts`
   (respect the numeric ordering). New suites need a matching npm script + entry in
   `test:gui:all`.
4. **Use helpers, not raw flags.** Compose from `daemon.ts` / `state.ts` / `ui.ts`.
   If a helper is missing, add it to the helper file — tests should stay thin.
5. **Isolate state** — start with `resetGuiState()`, clean up in the test (or
   `afterEach`), never close the last window.
6. **Screenshots are evidence, not assertions** — unless the user asks for a visual gate.
7. **Rebuild** if any source changed, then run just your spec:
   `npx wdio run wdio.conf.ts --spec specs/gui/0X-*.spec.ts`.
8. **Run the neighboring suite twice** — flakes are state- or timing-related, and
   one green run proves nothing. Clipboard-heavy tests especially.

## Troubleshooting Table

| Symptom | Likely cause | Fix |
|---|---|---|
| chrome-error "can't reach 127.0.0.1" / empty DOM / `hasSync: false` | release exe without `custom-protocol`, or stale build | rebuild; verify window URL is `http://tauri.localhost/` |
| Test passes in dev, fails in suite | testing stale binary | `npm run build` |
| admin suite fails with a UAC prompt visible | prompt unanswered, or shell not elevated | run from elevated terminal |
| clipboard test fails only after another clipboard test | leftover clipboard | clear with `setClipboard(' ')`; widen wait |
| everything hangs on first `browser.$` | driver/driver-version problem | kill `msedgedriver`/`tauri-driver` (via `onPrepare`), clear `%TEMP%/msedgedriver` |
| post-`npm install`, wdio is ~13min slower | `postinstall` patch lost | run `node scripts/patch-tauri-service.cjs` |
| daemon died mid-run | last window closed somewhere | keep ≥1 window; use `resetGuiState` not `closeWindow`-all |
