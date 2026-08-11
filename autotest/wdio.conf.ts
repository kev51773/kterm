import fs from 'node:fs'
import path from 'node:path'
import type { Options } from '@wdio/types'
import { APP_BINARY, APPDATA_DIR } from './src/helpers/paths.js'
import { ensureIsolatedAppdata, killAllKterm } from './src/helpers/env.js'
import { captureFailureScreenshot } from './src/helpers/screenshot.js'

// Boot mode selects how the app under test launches. Each mode = its own app
// instance (one boot), so the three modes cannot share a wdio run.
//   default: plain daemon boot (the bulk of GUI tests)
//   apply:   boot with --apply <boot-apply.yaml>   (session applied at startup)
//   admin:   boot with --apply <boot-admin.yaml>   (applies an admin shell tab)
const BOOT_MODE = process.env.KTERM_BOOT_MODE ?? 'default'
const BOOT_FIXTURES: Record<string, string> = {
  apply: path.join(APPDATA_DIR, 'boot-apply.yaml'),
  admin: path.join(APPDATA_DIR, 'boot-admin.yaml'),
}
const APP_ARGS = BOOT_MODE === 'default'
  ? ['--daemon']
  : ['--daemon', `--apply=${BOOT_FIXTURES[BOOT_MODE]}`]

function writeBootFixtures(): void {
  fs.mkdirSync(APPDATA_DIR, { recursive: true })
  fs.writeFileSync(path.join(APPDATA_DIR, 'boot-apply.yaml'), `window:
  id: null
tabs:
- id: null
  profile: powershell
  title: boot-apply-a
- id: null
  profile: cmd
  title: boot-apply-b
  splits:
  - direction: right
    profile: powershell
    title: boot-apply-b2
`)
  fs.writeFileSync(path.join(APPDATA_DIR, 'boot-admin.yaml'), `window:
  id: null
tabs:
- id: null
  profile: powershell
  admin: true
  title: boot-admin-shell
`)
}

export const config: Options.Testrunner = {
  runner: 'local',
  specs: ['./specs/gui/*.spec.ts', './specs/cli/*.spec.ts'],
  maxInstances: 1,
  logLevel: 'info',
  outputDir: 'logs',
  waitforTimeout: 15000,
  connectionRetryTimeout: 120000,
  connectionRetryCount: 3,
  framework: 'mocha',
  mochaOpts: { ui: 'bdd', timeout: 180000 },
  reporters: ['spec'],
  services: [
    [
      'tauri',
      {
        appBinaryPath: APP_BINARY,
        appArgs: APP_ARGS,
        driverProvider: 'external',
        autoInstallTauriDriver: true,
        autoDownloadEdgeDriver: true,
        captureBackendLogs: true,
        env: { APPDATA: APPDATA_DIR },
        startTimeout: 60000,
      },
    ],
  ],
  capabilities: [
    {
      browserName: 'tauri',
      'tauri:options': {
        application: APP_BINARY,
        args: APP_ARGS,
      },
    },
  ],
  onPrepare: () => {
    killAllKterm()
    ensureIsolatedAppdata()
    writeBootFixtures()
  },
  afterTest: async (test, context, { error }) => {
    if (error) {
      const file = context?.file ?? ''
      const spec = file.split(/[\\/]/).pop()?.replace(/\.spec\.ts$/, '') ?? 'unknown'
      const name = `FAIL-${String(test.title).replace(/\s+/g, '-')}`
      await captureFailureScreenshot(spec, name)
    }
  },
}
