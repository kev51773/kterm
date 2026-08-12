import fs from 'node:fs'
import path from 'node:path'
import type { Options } from '@wdio/types'
import { APP_BINARY, APPDATA_DIR, SCREENSHOT_ACTUAL, TMP_DIR } from './src/helpers/paths.js'
import { ensureIsolatedAppdata, killAllKterm } from './src/helpers/env.js'
import { startTest, endTest, step, addScreenshot, currentTest, relToAutotest } from './src/helpers/run.js'
import { compareShot } from './src/helpers/screenshot.js'
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

async function captureEndScreenshot(spec: string, specFile: string): Promise<void> {
  const cur = currentTest()
  if (!cur) return
  if (cur.screenshots.length > 0) return // test already captured its own evidence
  // CLI tests drive the daemon, not the GUI — their end-shots are racy noise.
  if (/specs[\\/]cli/.test(specFile)) return
  const name = shotName(cur.index, cur.title)
  const abs = path.join(SCREENSHOT_ACTUAL, spec, `${name}.png`)
  try {
    fs.mkdirSync(path.dirname(abs), { recursive: true })
    await browser.saveScreenshot(abs)
    const result = await compareShot(spec, name, abs)
    step(`took screenshot ${name}.png`)
    addScreenshot(relToAutotest(abs), result.status, result.mismatchPct)
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
    profile: cmd
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

function cleanTmp(): void {
  const append = process.env.KTERM_MANIFEST_APPEND === '1'
  // Screenshots are the report's evidence: wipe them only on a fresh GUI run
  // (append modes need the earlier modes' shots; CLI runs must not nuke them).
  const gui = /specs[\\/]gui/.test(process.argv.join(' '))
  if (!append) {
    fs.rmSync(TMP_DIR, { recursive: true, force: true })
    if (gui) fs.rmSync(SCREENSHOT_ACTUAL, { recursive: true, force: true })
  } else {
    // append mode: keep the manifest the next mode accumulates into
    for (const entry of fs.readdirSync(TMP_DIR)) {
      if (entry === 'run-manifest.json') continue
      fs.rmSync(path.join(TMP_DIR, entry), { recursive: true, force: true })
    }
  }
  fs.mkdirSync(TMP_DIR, { recursive: true })
  fs.mkdirSync(SCREENSHOT_ACTUAL, { recursive: true })
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
    cleanTmp()
    killAllKterm()
    ensureIsolatedAppdata()
    writeBootFixtures()
  },
  beforeTest: (test, context) => {
    startTest(specFromFile(context?.test?.parent?.file), test.title)
  },
  afterTest: async (test, context, { error }) => {
    const spec = specFromFile(context?.test?.parent?.file)
    await captureEndScreenshot(spec, context?.test?.parent?.file ?? '')
    endTest(error ? 'failed' : 'passed', error?.message)
  },
  onComplete: () => {
    try {
      generateReport()
    } catch (e) {
      console.error('[onComplete] report generation failed:', (e as Error)?.message ?? e)
    }
  },
}
