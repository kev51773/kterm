import fs from 'node:fs'
import path from 'node:path'
import { PNG } from 'pngjs'
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
  const pixelmatch = (await import('pixelmatch')).default

  const base = PNG.sync.read(fs.readFileSync(baselinePath))
  const act = PNG.sync.read(fs.readFileSync(actualPath))

  if (base.width !== act.width || base.height !== act.height) {
    // Same aspect ratio = uniform scale (Windows DPI scaling 100% vs 150%).
    // Normalize to the smaller canvas so pixel diff means content, not zoom.
    const baseAspect = base.width / base.height
    const actAspect = act.width / act.height
    const aspectDelta = Math.abs(baseAspect - actAspect) / Math.max(baseAspect, actAspect)
    if (aspectDelta > 0.01) {
      console.log(`  [shot] ${name} -> SIZE MISMATCH ${base.width}x${base.height} vs ${act.width}x${act.height} (aspect differs)`)
      return 100
    }
    const dw = Math.min(base.width, act.width)
    const dh = Math.min(base.height, act.height)
    console.log(`  [shot] ${name} -> DPI SCALE ${base.width}x${base.height} vs ${act.width}x${act.height}, normalizing to ${dw}x${dh}`)
    if (base.width !== dw || base.height !== dh) {
      const r = resizeBilinear(base, dw, dh)
      base.width = r.width; base.height = r.height; base.data = r.data
    }
    if (act.width !== dw || act.height !== dh) {
      const r = resizeBilinear(act, dw, dh)
      act.width = r.width; act.height = r.height; act.data = r.data
    }
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

// Bilinear RGBA resize. Used only to undo DPI scale (same aspect ratio) so
// pixel diff measures content, not zoom. pngjs has no resize built in.
function resizeBilinear(src: PNG, dw: number, dh: number): PNG {
  const out = new PNG({ width: dw, height: dh })
  const sw = src.width
  const sh = src.height
  const sx = sw / dw
  const sy = sh / dh
  for (let y = 0; y < dh; y++) {
    const fy = (y + 0.5) * sy - 0.5
    const y0 = Math.max(0, Math.floor(fy))
    const y1 = Math.min(sh - 1, y0 + 1)
    const wy = fy - y0
    for (let x = 0; x < dw; x++) {
      const fx = (x + 0.5) * sx - 0.5
      const x0 = Math.max(0, Math.floor(fx))
      const x1 = Math.min(sw - 1, x0 + 1)
      const wx = fx - x0
      const o = (y * dw + x) * 4
      for (let c = 0; c < 4; c++) {
        const top = src.data[(y0 * sw + x0) * 4 + c] * (1 - wx) + src.data[(y0 * sw + x1) * 4 + c] * wx
        const bot = src.data[(y1 * sw + x0) * 4 + c] * (1 - wx) + src.data[(y1 * sw + x1) * 4 + c] * wx
        out.data[o + c] = Math.round(top * (1 - wy) + bot * wy)
      }
    }
  }
  return out
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
