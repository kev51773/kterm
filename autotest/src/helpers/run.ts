import fs from 'node:fs'
import path from 'node:path'
import { AUTOTEST_DIR, TMP_DIR } from './paths.js'

export type CompareStatus = 'baseline-missing' | 'match' | 'diff' | 'updated-baseline'

export interface RunShot {
  rel: string // path to actual screenshot, relative to AUTOTEST_DIR, forward slashes
  compare?: CompareStatus
  mismatchPct?: number
}

export interface RunTest {
  index: number
  spec: string
  title: string
  kind: 'cli' | 'gui'
  status: 'passed' | 'failed' | 'skipped'
  steps: string[]
  screenshots: RunShot[]
  error?: string
}

export interface RunManifest {
  generatedAt: string
  tests: RunTest[]
}

export const MANIFEST_PATH = path.join(TMP_DIR, 'run-manifest.json')

function loadExisting(): RunTest[] {
  try {
    return JSON.parse(fs.readFileSync(MANIFEST_PATH, 'utf8')).tests
  } catch {
    return []
  }
}

// KTERM_MANIFEST_APPEND=1: continue the previous run's manifest (multi-spec
// combined report, e.g. test:gui:all). Default: fresh manifest per run.
const tests: RunTest[] = process.env.KTERM_MANIFEST_APPEND === '1' ? loadExisting() : []
let current: RunTest | null = null
let lastSpec: string | null = null
const specCounters: Record<string, number> = {}

export function startTest(spec: string, title: string, kind: 'cli' | 'gui' = 'gui'): void {
  if (lastSpec !== spec) {
    lastSpec = spec
    specCounters[spec] = 0
  }
  specCounters[spec] = (specCounters[spec] ?? 0) + 1
  current = { index: specCounters[spec], spec, title, kind, status: 'passed', steps: [], screenshots: [] }
  tests.push(current)
  writeManifest()
}

export function step(text: string): void {
  if (!current) return
  current.steps.push(text)
  writeManifest()
}

export function addScreenshot(relPath: string, compare?: CompareStatus, mismatchPct?: number): void {
  if (!current) return
  const shot: RunShot = { rel: relPath }
  if (compare) shot.compare = compare
  if (mismatchPct !== undefined) shot.mismatchPct = mismatchPct
  current.screenshots.push(shot)
  writeManifest()
}

export function endTest(status: 'passed' | 'failed', error?: string): void {
  if (current) {
    current.status = status
    if (error) current.error = error
  }
  current = null
  writeManifest()
}

export function currentTest(): RunTest | null {
  return current
}

export function writeManifest(): void {
  fs.mkdirSync(TMP_DIR, { recursive: true })
  fs.writeFileSync(MANIFEST_PATH, JSON.stringify({ generatedAt: new Date().toISOString(), tests }, null, 2))
}

export function relToAutotest(absPath: string): string {
  return path.relative(AUTOTEST_DIR, absPath).split(path.sep).join('/')
}
