import { browser } from '@wdio/globals'
import { cliOk, listTabs, listWindows, spawnTab, waitForPrompt, sendText } from './daemon.js'

export async function resetGuiState(win = 'win-1'): Promise<void> {
  // Close all windows except the target (the webdriver session lives in the daemon;
  // closing the last window kills the daemon, so keep the target alive).
  for (const w of listWindows()) {
    if (w.id !== win) {
      cliOk(['--close-window', w.id])
    }
  }

  // Reduce the target window to exactly one tab.
  const tabs = listTabs(win)
  if (tabs.length === 0) {
    spawnTab('powershell', win)
  } else if (tabs.length > 1) {
    for (const t of tabs.slice(1)) {
      cliOk(['--select-tab', t.id, '--close'])
    }
  }

  // Give the frontend a sync cycle (2s poll) to catch up.
  await browser.pause(2500)
}

export async function settle(win = 'win-1'): Promise<void> {
  const tabs = listTabs(win)
  if (tabs.length > 0) {
    try {
      waitForPrompt(tabs[0].id, 15)
    } catch {
      // prompt may not match this shell; ignore
    }
  }
  await browser.pause(500)
}

export function sendToTab(tab: string, text: string): void {
  sendText(tab, text)
}

export async function waitForTabCount(count: number, timeoutMs = 20000): Promise<void> {
  const start = Date.now()
  while (Date.now() - start < timeoutMs) {
    const n = await browser.$$('.tab-item').length
    if (n === count) return
    await browser.pause(300)
  }
  throw new Error(`tab count did not reach ${count} in time`)
}

export async function activeTabTitle(): Promise<string | null> {
  const el = await browser.$('.tab-item.active .tab-title')
  return el.isExisting() ? el.getText() : null
}
