import fs from 'node:fs'
import path from 'node:path'
import { SCREENSHOT_BASELINE, SCREENSHOT_ACTUAL, ARTIFACT_BASELINE, ARTIFACT_ACTUAL, DIFF_DIR } from '../src/helpers/paths.js'

const PNG_THRESHOLD = 0.1 // max % mismatched pixels before a screenshot counts as a diff

async function compareDir(baseRoot: string, actRoot: string, kind: 'png' | 'text'): Promise<number> {
  const diffRoot = kind === 'png' ? DIFF_DIR : path.join(DIFF_DIR, 'artifacts')
  if (!fs.existsSync(baseRoot) || !fs.existsSync(actRoot)) return 0

  const baseFiles: string[] = []
  const walk = (dir: string, out: string[]): void => {
    for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
      const p = path.join(dir, e.name)
      if (e.isDirectory()) walk(p, out)
      else if (e.name.endsWith(kind === 'png' ? '.png' : '.txt')) out.push(p)
    }
  }
  walk(baseRoot, baseFiles)

  let diffs = 0
  const report: string[] = []

  for (const basePath of baseFiles) {
    const rel = path.relative(baseRoot, basePath)
    const actPath = path.join(actRoot, rel)
    if (!fs.existsSync(actPath)) {
      report.push(`  [${kind}] MISSING actual: ${rel}`)
      diffs++
      continue
    }

    if (kind === 'png') {
      const mismatch = await pixelDiffPct(basePath, actPath)
      if (mismatch === null) continue
      const pct = mismatch as number
      if (pct > PNG_THRESHOLD) {
        report.push(`  [png] DIFF ${pct.toFixed(3)}%: ${rel}`)
        diffs++
      }
    } else {
      const a = fs.readFileSync(basePath, 'utf8').replace(/\s+/g, ' ').trim()
      const b = fs.readFileSync(actPath, 'utf8').replace(/\s+/g, ' ').trim()
      if (a !== b) {
        report.push(`  [text] DIFF: ${rel}`)
        diffs++
      }
    }
  }

  if (report.length > 0) {
    console.log(`\n== ${kind.toUpperCase()} DIFFS (${baseRoot}) ==`)
    console.log(report.slice(0, 60).join('\n'))
    if (report.length > 60) console.log(`  ... and ${report.length - 60} more`)
  }
  return diffs
}

async function pixelDiffPct(aPath: string, bPath: string): Promise<number | null> {
  const { PNG } = await import('pngjs')
  const pixelmatch = (await import('pixelmatch')).default
  const a = PNG.sync.read(fs.readFileSync(aPath))
  const b = PNG.sync.read(fs.readFileSync(bPath))
  if (a.width !== b.width || a.height !== b.height) return 100
  const diff = new PNG({ width: a.width, height: a.height })
  const n = pixelmatch(a.data, b.data, diff.data, a.width, a.height, { threshold: 0.15 })
  const rel = path.relative(SCREENSHOT_BASELINE, aPath)
  fs.mkdirSync(path.join(DIFF_DIR, path.dirname(rel)), { recursive: true })
  fs.writeFileSync(path.join(DIFF_DIR, rel), PNG.sync.write(diff))
  return (n / (a.width * a.height)) * 100
}

async function main(): Promise<void> {
  console.log('== kterm autotest compare ==')
  let total = 0
  total += await compareDir(SCREENSHOT_BASELINE, SCREENSHOT_ACTUAL, 'png')
  total += await compareDir(ARTIFACT_BASELINE, ARTIFACT_ACTUAL, 'text')
  console.log(`\n${total === 0 ? 'PASS: no diffs' : `FAIL: ${total} diff(s) — see ${DIFF_DIR}`}`)
  process.exit(total === 0 ? 0 : 1)
}

main()
