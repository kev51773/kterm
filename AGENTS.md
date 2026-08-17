# kterm

Scriptable terminal: Tauri + WebView2 app (`src/` frontend, `src-tauri/` backend),
with an end-to-end WebDriver/CLI autotest suite in `autotest/`.

## Before doing anything in this repo

Read, in order:

1. `autotest/docs/AUTOTEST.md` — architecture, run commands, helpers, gotchas, how to add tests.
2. `autotest/docs/screenshot-baseline-guide.md` — the visual evidence system and how baselines are reviewed/promoted.

## Mandatory workflow when changing app code

1. Change code under `src/` or `src-tauri/src/`.
2. Rebuild the release binary: `./make-release.sh` or `npm run build` in `autotest/` (tests exercise the packaged exe, never a dev server).
3. Run `autotest/run-all.sh` or `npm run test:gui:all` in `autotest/` (must be an elevated terminal — admin suite fires UAC).
4. **If any screenshot differs, do NOT promote or update baselines yourself.** Run `autotest/review.sh` or `npm run review` in `autotest/` and ask the human to verdict the changed shots. Screenshots never gate tests; a diff is a review request, not a failure.
5. Never auto-promote baselines (`npm run test:update-baseline` / `UPDATE_BASELINE=1` is deprecated).

The docs above are the source of truth — read them fully before writing or modifying tests.
