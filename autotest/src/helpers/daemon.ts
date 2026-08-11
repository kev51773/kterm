import { execFileSync } from 'node:child_process'
import { APP_BINARY, DAEMON_BASE_URL } from './paths.js'

export interface CliResult {
  stdout: string
  stderr: string
  exitCode: number
}

export function cli(args: string[]): CliResult {
  try {
    const stdout = execFileSync(APP_BINARY, args, { encoding: 'utf8', timeout: 60000 })
    return { stdout, stderr: '', exitCode: 0 }
  } catch (e: any) {
    const err = e as { stdout?: string; stderr?: string; status?: number }
    return {
      stdout: err.stdout ?? '',
      stderr: err.stderr ?? '',
      exitCode: err.status ?? -1,
    }
  }
}

export function cliOk(args: string[]): string {
  const r = cli(args)
  if (r.exitCode !== 0) {
    throw new Error(`kterm.exe ${args.join(' ')} failed (${r.exitCode}): ${r.stderr || r.stdout}`)
  }
  return r.stdout.trim()
}

export function listTabs(window = 'win-1'): any[] {
  return JSON.parse(cliOk(['--list-tabs', '--window', window, '--json']))
}

export function listWindows(): any[] {
  return JSON.parse(cliOk(['--list-windows', '--json']))
}

export function spawnTab(profile = 'powershell', window = 'win-1'): string {
  return cliOk(['--profile', profile, '--window', window])
}

export function waitForPrompt(tab: string, timeout = 30): void {
  cliOk(['--select-tab', tab, '--wait-for-prompt', '--timeout', String(timeout)])
}

export function waitFor(tab: string, pattern: string, timeout = 30): void {
  cliOk(['--select-tab', tab, '--wait-for', pattern, '--from-history', '--timeout', String(timeout)])
}

export function readText(tab: string, tail = 50, raw = false): string {
  const args = ['--select-tab', tab, '--read-text', '--tail', String(tail)]
  if (raw) args.push('--raw')
  return cliOk(args)
}

export function sendText(tab: string, text: string): void {
  cliOk(['--select-tab', tab, '--send-text', text])
}

export function closeWindow(window: string): void {
  cliOk(['--close-window', window])
}

export function closeTab(tab: string): void {
  cliOk(['--select-tab', tab, '--close', '--force'])
}

export function newWindow(): string {
  return cliOk(['--new-window'])
}

export function setTabTitle(tab: string, title: string): void {
  cliOk(['--select-tab', tab, '--send-title', title])
}

export function setBadge(tab: string, badge: string): void {
  cliOk(['--select-tab', tab, '--set-badge', badge])
}

export function setColor(tab: string, color: string): void {
  cliOk(['--select-tab', tab, '--set-color', color])
}

export function splitTab(tab: string, direction: 'right' | 'left' | 'down' | 'up', profile = 'powershell'): string {
  return cliOk(['--select-tab', tab, `--split-${direction}`, '--profile', profile])
}

export function unsplit(tab: string): void {
  cliOk(['--select-tab', tab, '--unsplit'])
}

export function explodeSplit(tab: string): void {
  cliOk(['--select-tab', tab, '--explode-split'])
}

export function exportLayout(window: string, file: string): void {
  cliOk(['--export-layout', file, '--window', window])
}

export function setWindowTitle(window: string, title: string): void {
  cliOk(['--window', window, '--set-window-title', title])
}

export function normalizeWindow(window = 'win-1', keep = 1): string {
  let tabs = listTabs(window)
  while (tabs.length > keep) {
    closeTab(tabs[tabs.length - 1].id)
    tabs = listTabs(window)
  }
  if (tabs.length === 0) return spawnTab('powershell', window)
  return tabs[0].id
}

export function setClipboard(text: string): void {
  execFileSync('powershell.exe', ['-NoProfile', '-Command', `Set-Clipboard -Value '${text}'`], {
    encoding: 'utf8',
    timeout: 30000,
  })
}

export function getClipboard(): string {
  return execFileSync('powershell.exe', ['-NoProfile', '-Command', 'Get-Clipboard'], {
    encoding: 'utf8',
    timeout: 30000,
  }).trim()
}

export async function daemonHealth(): Promise<boolean> {
  try {
    const res = await fetch(`${DAEMON_BASE_URL}/health`)
    return res.ok
  } catch {
    return false
  }
}

export async function waitDaemonReady(timeoutMs = 30000): Promise<void> {
  const start = Date.now()
  while (Date.now() - start < timeoutMs) {
    if (await daemonHealth()) return
    await new Promise((r) => setTimeout(r, 250))
  }
  throw new Error('daemon did not become healthy within timeout')
}
