import { spawnSync } from 'node:child_process'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const ROOT = path.resolve(__dirname, '..')
const WDIO = path.join(ROOT, 'node_modules', '@wdio', 'cli', 'bin', 'wdio.js')

function run(spec, extraEnv) {
  const r = spawnSync(process.execPath, [WDIO, 'run', 'wdio.conf.ts', '--spec', spec], {
    cwd: ROOT,
    env: { ...process.env, ...extraEnv },
    stdio: 'inherit',
  })
  return r.status ?? 1
}

// Sequential: parallel workers each boot their own app and fight over the
// daemon port. First spec wipes tmp (fresh manifest), the rest append.
let failed = false
const cliDir = path.join(ROOT, 'specs', 'cli')
const specs = fs.readdirSync(cliDir).filter((f) => f.endsWith('.spec.ts')).sort()
specs.forEach((f, i) => {
  const s = run(`specs/cli/${f}`, i === 0 ? {} : { KTERM_MANIFEST_APPEND: '1' })
  if (s !== 0) failed = true
})
spawnSync(process.execPath, ['--import', 'tsx', 'src/report/report.ts'], { cwd: ROOT, stdio: 'inherit' })
process.exit(failed ? 1 : 0)
