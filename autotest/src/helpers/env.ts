import fs from 'node:fs'
import path from 'node:path'
import { execSync } from 'node:child_process'
import { APP_BINARY, APPDATA_DIR, KTERM_CONFIG, PINNED_CONFIG } from './paths.js'

export function ensureIsolatedAppdata(): void {
  fs.mkdirSync(path.dirname(KTERM_CONFIG), { recursive: true })
  if (!fs.existsSync(KTERM_CONFIG)) {
    writePinnedConfig()
  }
}

export function writePinnedConfig(): void {
  fs.mkdirSync(path.dirname(KTERM_CONFIG), { recursive: true })
  fs.writeFileSync(KTERM_CONFIG, JSON.stringify(PINNED_CONFIG, null, 2))
}

export function readConfig(): Record<string, unknown> {
  return JSON.parse(fs.readFileSync(KTERM_CONFIG, 'utf8'))
}

export function readUserConfig(): Record<string, unknown> | null {
  const p = path.join(process.env.APPDATA ?? '', 'kterm', 'config.json')
  return fs.existsSync(p) ? JSON.parse(fs.readFileSync(p, 'utf8')) : null
}

export function killAllKterm(): void {
  for (const name of ['kterm', 'msedgewebview2', 'msedgedriver', 'tauri-driver']) {
    try {
      execSync(`taskkill /F /IM ${name}.exe /T 2>nul`, { stdio: 'ignore' })
    } catch {
      // no such process
    }
  }
}

export function buildApp(): void {
  console.log('[build] building release binary (npx tauri build --no-bundle)...')
  execSync('npx tauri build --no-bundle', { cwd: path.dirname(path.dirname(APP_BINARY)), stdio: 'inherit' })
}
