import fs from 'node:fs'
import path from 'node:path'
import type { Options } from '@wdio/types'
import { APP_BINARY, APPDATA_DIR, SCREENSHOT_ACTUAL } from './src/helpers/paths.js'
import { ensureIsolatedAppdata, killAllKterm } from './src/helpers/env.js'
import { startTest, endTest, step, addScreenshot, currentTest, relToAutotest } from './src/helpers/run.js'
import { generate as generateReport } from './src/report/report.js'
import { browser } from '@wdio/globals'

function specFromFile(file?: string): string {
  return file?.split(/[\\/]/).pop()?.replace(/\.spec\.ts$/, '') ?? 'unknown'
}

function shotSlug(title: string): string {
  return title.toLowerCase().replace(/^\d+[.)\s]*/, '').replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '')
}

function shotName(index: number, title: string): string {
  return `T${index}-${shotSlug(title) || 'test'}.png`
}

async function captureEndScreenshot(): Promise<void> {
  const cur = currentTest()
  if (!cur) return
  const file = `${cur.spec}/${shotName(cur.index, cur.title)}`
  const abs = path.join(SCREENSHOT_ACTUAL, file)
  try {
    fs.mkdirSync(path.dirname(abs), { recursive: true })
    await browser.saveScreenshot(abs)
    step(`took screenshot ${path.basename(abs)}`)
    addScreenshot(relToAutotest(abs))
  } catch (e) {
    console.error('[afterTest] end screenshot failed:', (e as Error)?.message ?? e, '| abs:', abs, '| ext:', path.extname(abs))
  }
}

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
  beforeTest: (test, context) => {
    startTest(specFromFile(context?.test?.parent?.file), test.title)
  },
  afterTest: async (test, context, { error }) => {
    await captureEndScreenshot()
    endTest(error ? 'failed' : 'passed')
  },
  onComplete: () => {
    try {
      generateReport()
    } catch (e) {
      console.error('[onComplete] report generation failed:', (e as Error)?.message ?? e)
    }
  },
}
