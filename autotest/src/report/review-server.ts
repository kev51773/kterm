import http from 'node:http'
import fs from 'node:fs'
import path from 'node:path'
import { spawn } from 'node:child_process'
import { AUTOTEST_DIR, SCREENSHOT_ACTUAL, SCREENSHOT_BASELINE, TMP_DIR } from '../helpers/paths.js'
import { readManifest, renderReportHtml } from './report.js'

const PORT = 8765
const SCREENSHOTS_DIR = path.join(AUTOTEST_DIR, 'screenshots')

function send(res: http.ServerResponse, code: number, body: string, type = 'text/html; charset=utf-8'): void {
  res.writeHead(code, { 'Content-Type': type, 'Cache-Control': 'no-store' })
  res.end(body)
}

async function readBody(req: http.IncomingMessage): Promise<string> {
  return new Promise((resolve, reject) => {
    let data = ''
    req.on('data', (c) => (data += c))
    req.on('end', () => resolve(data))
    req.on('error', reject)
  })
}

const server = http.createServer(async (req, res) => {
  try {
    const url = new URL(req.url ?? '/', `http://127.0.0.1:${PORT}`)
    const p = url.pathname

    if (req.method === 'GET' && p === '/') {
      const run = readManifest()
      if (!run) return send(res, 404, 'no run-manifest.json — run a test suite first (tmp/run-manifest.json)')
      return send(res, 200, renderReportHtml(run, '/'))
    }

    if (req.method === 'GET' && p === '/manifest') {
      const run = readManifest()
      return send(res, run ? 200 : 404, run ? JSON.stringify(run) : 'not found', 'application/json')
    }

    if (req.method === 'GET' && p === '/health') {
      return send(res, 200, 'ok', 'text/plain')
    }

    if (req.method === 'POST' && p === '/api/update-baselines') {
      const body = JSON.parse(await readBody(req))
      const updates: Array<{ spec: string; name: string }> = Array.isArray(body?.updates) ? body.updates : []
      const updated: string[] = []
      for (const u of updates) {
        const src = path.join(SCREENSHOT_ACTUAL, String(u.spec), `${String(u.name)}.png`)
        if (!src.startsWith(SCREENSHOT_ACTUAL) || !fs.existsSync(src)) continue
        const dstDir = path.join(SCREENSHOT_BASELINE, String(u.spec))
        fs.mkdirSync(dstDir, { recursive: true })
        fs.copyFileSync(src, path.join(dstDir, `${String(u.name)}.png`))
        updated.push(`${u.spec}/${u.name}`)
      }
      return send(res, 200, JSON.stringify({ updated }), 'application/json')
    }

    if (req.method === 'GET' && p.startsWith('/screenshots/')) {
      const rel = p.replace(/^\/screenshots\//, '')
      const abs = path.resolve(SCREENSHOTS_DIR, rel)
      if (!abs.startsWith(SCREENSHOTS_DIR) || !fs.existsSync(abs)) return send(res, 404, 'not found', 'text/plain')
      const ext = path.extname(abs).toLowerCase()
      const type = ext === '.png' ? 'image/png' : ext === '.json' ? 'application/json' : 'application/octet-stream'
      res.writeHead(200, { 'Content-Type': type, 'Cache-Control': 'no-store' })
      res.end(fs.readFileSync(abs))
      return
    }

    send(res, 404, 'not found', 'text/plain')
  } catch (e) {
    send(res, 500, `error: ${(e as Error).message}`, 'text/plain')
  }
})

server.listen(PORT, '127.0.0.1', () => {
  console.log(`kterm review server: http://127.0.0.1:${PORT}`)
  console.log(`  screenshots: ${SCREENSHOTS_DIR}`)
  spawn('cmd.exe', ['/c', 'start', `http://127.0.0.1:${PORT}/`], { detached: true, stdio: 'ignore' }).unref()
})
