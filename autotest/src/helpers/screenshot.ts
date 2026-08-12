import fs from 'node:fs'
import path from 'node:path'
import { browser } from '@wdio/globals'
import { SCREENSHOT_BASELINE, SCREENSHOT_ACTUAL, DIFF_DIR, UPDATE_BASELINE } from './paths.js'
import { step, addScreenshot, relToAutotest } from './run.js'

export interface ShotResult {
  name: string
  path: string
  status: 'baseline-missing' | 'match' | 'diff' | 'updated-baseline'
  mismatchPct?: number
}

export async function captureScreenshot(spec: string, name: string): Promise<ShotResult> {
  const actualDir = path.join(SCREENSHOT_ACTUAL, spec)
  fs.mkdirSync(actualDir, { recursive: true })
  const actualPath = path.join(actualDir, `${name}.png`)

  await browser.saveScreenshot(actualPath)
  await saveStateJson(actualPath)

  const result = await compareShot(spec, name, actualPath)

  step(`took screenshot ${path.basename(actualPath)}`)
  addScreenshot(relToAutotest(actualPath), result.status, result.mismatchPct)
  return result
}

export async function compareShot(spec: string, name: string, actualPath: string): Promise<ShotResult> {
  const baselineDir = path.join(SCREENSHOT_BASELINE, spec)
  const baselinePath = path.join(baselineDir, `${name}.png`)
  const result: ShotResult = { name, path: actualPath, status: 'match' }

  if (UPDATE_BASELINE) {
    fs.mkdirSync(baselineDir, { recursive: true })
    fs.copyFileSync(actualPath, baselinePath)
    result.status = 'updated-baseline'
    console.log(`  [shot] ${name} -> baseline updated`)
    return result
  }

  if (!fs.existsSync(baselinePath)) {
    result.status = 'baseline-missing'
    console.log(`  [shot] ${name} -> NO baseline (actual only)`)
    return result
  }

  const mismatch = await pixelDiff(baselinePath, actualPath, spec, name)
  if (mismatch === null) {
    result.status = 'match'
  } else {
    result.status = 'diff'
    result.mismatchPct = mismatch
    console.log(`  [shot] ${name} -> DIFF ${mismatch.toFixed(3)}% (baseline: ${path.basename(baselinePath)})`)
  }
  return result
}

async function pixelDiff(baselinePath: string, actualPath: string, spec: string, name: string): Promise<number | null> {
  const { PNG } = await import('pngjs')
  const pixelmatch = (await import('pixelmatch')).default

  const base = PNG.sync.read(fs.readFileSync(baselinePath))
  const act = PNG.sync.read(fs.readFileSync(actualPath))

  if (base.width !== act.width || base.height !== act.height) {
    console.log(`  [shot] ${name} -> SIZE MISMATCH ${base.width}x${base.height} vs ${act.width}x${act.height}`)
    return 100
  }

  const diff = new PNG({ width: base.width, height: base.height })
  const mismatched = pixelmatch(base.data, act.data, diff.data, base.width, base.height, { threshold: 0.15 })

  const total = base.width * base.height
  const pct = (mismatched / total) * 100

  const diffDir = path.join(DIFF_DIR, spec)
  fs.mkdirSync(diffDir, { recursive: true })
  fs.writeFileSync(path.join(diffDir, `${name}.png`), PNG.sync.write(diff))

  return pct > 0.1 ? pct : null
}

async function saveStateJson(actualPath: string): Promise<void> {
  const jsonPath = actualPath.replace(/\.png$/, '.json')
  try {
    const state = await browser.execute(() => {
      const winId = new URLSearchParams(window.location.search).get('window') ?? 'win-1'
      const tabBar = document.getElementById('tab-bar')
      const tabs = Array.from(document.querySelectorAll('.tab-item')).map((t) => ({
        title: t.querySelector('.tab-title')?.textContent ?? '',
        badge: t.querySelector('.tab-badge')?.textContent ?? null,
        active: t.classList.contains('active'),
        bracket: !!t.querySelector('.tab-title-bracket'),
        admin: !!t.querySelector('.tab-admin'),
        color: t.querySelector('.tab-item')?.getAttribute('data-color') ?? null,
      }))
      return {
        window: winId,
        title: document.title,
        tabCount: tabs.length,
        tabs,
        settingsOpen: !!document.querySelector('.settings-modal'),
        findOpen: !!document.querySelector('.find-bar'),
        paneCount: document.querySelectorAll('.terminal-pane').length,
      }
    })
    fs.writeFileSync(jsonPath, JSON.stringify(state, null, 2))
  } catch {
    // state snapshot is best-effort
  }
}
