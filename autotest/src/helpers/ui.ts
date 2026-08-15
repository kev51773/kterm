import { browser, $, $$ } from '@wdio/globals'
import { cliOk } from './daemon.js'

export async function waitForTabCount(n: number, timeout = 15000): Promise<void> {
  await browser.waitUntil(async () => (await $$('.tab-item').length) === n, { timeout })
}

export async function focusTerminal(timeout = 15000): Promise<void> {
  const pane = await $('.terminal-pane')
  await pane.waitForExist({ timeout })
  await pane.click()
  await browser.pause(200)
}

export async function rightClick(selector: string): Promise<void> {
  const el = await $(selector)
  await el.waitForExist({ timeout: 15000 })
  await browser.action('pointer', { parameters: { pointerType: 'mouse' } })
    .move({ origin: el })
    .down({ button: 2 })
    .up({ button: 2 })
    .pause(200)
    .perform()
}

export async function openContextMenu(selector: string, tries = 4): Promise<void> {
  const menu = $('.context-menu:not(.profile-submenu)')
  for (let i = 0; i < tries; i++) {
    await rightClick(selector)
    await browser.pause(400)
    const items = await $$('.context-menu:not(.profile-submenu) .context-menu-item').length
    if (items > 0) return
  }
  throw new Error(`context menu did not open after ${tries} right-clicks on ${selector}`)
}

export async function getConfig(): Promise<any> {
  return JSON.parse(cliOk(['--get-config']))
}
