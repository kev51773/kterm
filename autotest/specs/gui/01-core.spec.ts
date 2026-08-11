import { $, $$, browser } from '@wdio/globals'
import { focusTerminal, openContextMenu, waitForTabCount as waitForUiTabs } from '../../src/helpers/ui.js'
import { resetGuiState, settle, activeTabTitle } from '../../src/helpers/state.js'
import {
  listTabs, listWindows, newWindow, closeWindow, spawnTab,
  setTabTitle, waitForPrompt, waitFor, sendText, splitTab,
} from '../../src/helpers/daemon.js'
import { captureScreenshot } from '../../src/helpers/screenshot.js'
import { step } from '../../src/helpers/run.js'

async function hoverSplitMenu(): Promise<void> {
  const menu = await $('#ctx-split-menu')
  await menu.waitForExist({ timeout: 5000 })
  await browser.action('pointer', { parameters: { pointerType: 'mouse' } })
    .move({ origin: menu })
    .pause(300)
    .perform()
}

async function clickTabCloseOnLastTab(): Promise<void> {
  const clicked = await browser.execute(() => {
    const tabs = document.querySelectorAll('.tab-item')
    const btn = tabs[tabs.length - 1]?.querySelector('.tab-close-btn') as HTMLElement | null
    btn?.click()
    return !!btn
  })
  expect(clicked).toBe(true)
}

async function clickFirstTab(): Promise<void> {
  const clicked = await browser.execute(() => {
    const el = document.querySelector('.tab-item') as HTMLElement | null
    el?.click()
    return !!el
  })
  expect(clicked).toBe(true)
}

async function waitForTabTitle(expected: string, timeout = 10000): Promise<void> {
  await browser.waitUntil(async () => (await activeTabTitle()) === expected, { timeout })
}

// Matches any tab-bar entry, active or not. CLI/daemon spawns never activate
// the new tab (only the UI path switchTab()s), so active-tab assertions only
// hold for UI-driven flows.
async function expectTabTitleVisible(expected: string, timeout = 10000): Promise<void> {
  await browser.waitUntil(async () => {
    const titles = await browser.execute(() =>
      Array.from(document.querySelectorAll('.tab-item .tab-title')).map((el) => el.textContent ?? '')
    )
    return titles.some((t) => t.includes(expected))
  }, { timeout })
}

function tabIds(): string[] {
  return listTabs('win-1').map((t) => t.id)
}

async function waitForTabCountDaemon(count: number, timeoutMs = 15000): Promise<void> {
  const start = Date.now()
  while (Date.now() - start < timeoutMs) {
    if (tabIds().length === count) return
    await browser.pause(250)
  }
  throw new Error(`daemon tab count did not reach ${count} in time`)
}

function freshTabId(before: string[]): string {
  const tabs = listTabs('win-1')
  return tabs.find((t) => !before.includes(t.id))?.id ?? tabs[tabs.length - 1].id
}

function tabByProfile(profile: string): string {
  const t = listTabs('win-1').find((tab) => tab.profile === profile)
  if (!t) throw new Error(`no tab with profile '${profile}'`)
  return t.id
}

function echoText(tab: string, text: string, timeout = 30): void {
  waitForPrompt(tab, timeout)
  sendText(tab, `echo ${text}`)
  waitFor(tab, text, timeout)
}

describe('01-core (default boot)', () => {
  beforeEach(async () => {
    await resetGuiState()
    await settle()
  })

  it('1. Boot shell', async () => {
    await waitForUiTabs(1)
    step('Focused the terminal pane')
    await focusTerminal()
    setTabTitle(tabByProfile('powershell'), 'test-01')
    await waitForTabTitle('test-01')
    await captureScreenshot('01-core', '01-boot-shell')
  })

  it('2. Add tab', async () => {
    const before = tabIds()
    step('Clicked the "+" add tab button')
    await (await $('#add-tab-btn')).click()
    await waitForUiTabs(2)
    setTabTitle(freshTabId(before), 'test-02')
    await waitForTabTitle('test-02')
    await captureScreenshot('01-core', '02-add-tab')
  })

  it('3. Close tab', async () => {
    const before = tabIds()
    step('Clicked the "+" add tab button')
    await (await $('#add-tab-btn')).click()
    await waitForUiTabs(2)
    setTabTitle(freshTabId(before), 'test-03-temp')
    await waitForTabTitle('test-03-temp')
    step('Clicked the close button on the last tab')
    await clickTabCloseOnLastTab()
    await waitForUiTabs(1)
    echoText(before[0], 'Test-03 - closed the tab that test 02 opened', 20)
    await captureScreenshot('01-core', '03-close-tab')
  })

  it('4. Keyboard: new tab', async () => {
    const before = tabIds()
    step('Pressed Ctrl+Shift+= (new tab)')
    await browser.keys(['Control', 'Shift', '='])
    await waitForUiTabs(2)
    setTabTitle(freshTabId(before), 'test-04')
    await waitForTabTitle('test-04')
    await captureScreenshot('01-core', '04-kbd-new-tab')
  })

  it('5. Keyboard: close tab', async () => {
    const before = tabIds()
    step('Pressed Ctrl+Shift+= (new tab)')
    await browser.keys(['Control', 'Shift', '='])
    await waitForUiTabs(2)
    setTabTitle(freshTabId(before), 'test-05-temp')
    await waitForTabTitle('test-05-temp')
    step('Pressed Ctrl+Shift+- (close tab)')
    await browser.keys(['Control', 'Shift', '-'])
    await waitForUiTabs(1)
    echoText(before[0], 'Test-05 - keyboard closed the tab that test 04 opened', 20)
    await captureScreenshot('01-core', '05-kbd-close-tab')
  })

  it('6. Cycle tabs', async () => {
    const before = tabIds()
    step('Clicked the "+" add tab button')
    await (await $('#add-tab-btn')).click()
    await waitForUiTabs(2)
    const fresh = freshTabId(before)
    setTabTitle(before[0], 'test-06a')
    setTabTitle(fresh, 'test-06b')
    await waitForTabTitle('test-06b')
    step('Pressed Ctrl+Tab (next tab)')
    await browser.keys(['Control', 'Tab'])
    await waitForTabTitle('test-06a')
    await captureScreenshot('01-core', '06-cycle-tabs')
  })

  it('7. Switch tab by click', async () => {
    const before = tabIds()
    step('Clicked the "+" add tab button')
    await (await $('#add-tab-btn')).click()
    await waitForUiTabs(2)
    const fresh = freshTabId(before)
    setTabTitle(before[0], 'test-07a')
    setTabTitle(fresh, 'test-07b')
    await waitForTabTitle('test-07b')
    step('Clicked the first tab in the tab bar')
    await clickFirstTab()
    await waitForTabTitle('test-07a')
    await captureScreenshot('01-core', '07-switch-tab')
  })

  it('8. Settings modal (Ctrl+,)', async () => {
    const before = tabIds()
    step('Clicked the "+" add tab button')
    await (await $('#add-tab-btn')).click()
    await waitForUiTabs(2)
    setTabTitle(freshTabId(before), 'test-08')
    await waitForTabTitle('test-08')
    step('Pressed Ctrl+, (open settings)')
    await browser.keys(['Control', ','])
    const modal = await $('.settings-modal')
    await modal.waitForExist({ timeout: 5000 })
    await captureScreenshot('01-core', '08-settings-modal')
    step('Clicked the settings close button')
    await (await modal.$('.settings-close-btn')).click()
    await modal.waitForExist({ timeout: 5000, reverse: true })
  })

  it('9. Settings via dropdown', async () => {
    step('Clicked the tab dropdown button')
    await (await $('#tab-dropdown-btn')).click()
    const items = await $$('.profile-dropdown-menu .profile-dropdown-item')
    let target: Awaited<typeof items[0]> | null = null
    for (let i = 0; i < (await items.length); i++) {
      if ((await items[i].getText()).includes('Settings')) {
        target = items[i]
        break
      }
    }
    if (!target) throw new Error('settings item not found in profile dropdown')
    step('Clicked "Settings" in the dropdown')
    await target.click()
    const modal = await $('.settings-modal')
    await modal.waitForExist({ timeout: 5000 })
    await captureScreenshot('01-core', '09-settings-dropdown')
    step('Clicked the settings close button')
    await (await modal.$('.settings-close-btn')).click()
    await modal.waitForExist({ timeout: 5000, reverse: true })
  })

  it('10. Spawn cmd tab from dropdown', async () => {
    step('Clicked the tab dropdown button')
    await (await $('#tab-dropdown-btn')).click()
    const items = await $$('.profile-dropdown-menu .profile-dropdown-item')
    let target: Awaited<typeof items[0]> | null = null
    for (let i = 0; i < (await items.length); i++) {
      if ((await items[i].getText()).includes('Command Prompt')) {
        target = items[i]
        break
      }
    }
    if (!target) throw new Error('cmd item not found in profile dropdown')
    step('Clicked "Command Prompt" in the dropdown')
    await target.click()
    await waitForUiTabs(2)
    const cmdTab = tabByProfile('cmd')
    setTabTitle(cmdTab, 'test-10-cmd')
    await waitForTabTitle('test-10-cmd')
    echoText(cmdTab, 'Test-10 - Command Prompt shell loaded', 40)
    await captureScreenshot('01-core', '10-cmd-tab')
  })

  it('11. Context menu', async () => {
    step('Right-clicked the terminal pane to open the context menu')
    await openContextMenu('.terminal-pane')
    const items = await $$('.context-menu:not(.profile-submenu) .context-menu-item')
    expect(items.length).toBeGreaterThan(0)
    await captureScreenshot('01-core', '11-context-menu')
    step('Focused the terminal pane')
    await focusTerminal()
  })

  it('12. Split pane right', async () => {
    const main = tabIds()[0]
    setTabTitle(main, 'test-12-split')
    await waitForTabTitle('test-12-split')
    step('Right-clicked the terminal pane')
    await openContextMenu('.terminal-pane')
    step('Hovered the Split submenu')
    await hoverSplitMenu()
    step('Clicked "Split Right"')
    await (await $('#ctx-split-right')).click()
    await browser.waitUntil(async () => (await $$('.split-pane-wrapper').length) === 2, { timeout: 10000 })
    await browser.pause(800)
    await captureScreenshot('01-core', '12-split-pane')
    step('Focused the terminal pane')
    await focusTerminal()
    step('Right-clicked the terminal pane')
    await openContextMenu('.terminal-pane')
    step('Hovered the Split submenu')
    await hoverSplitMenu()
    step('Clicked "Unsplit"')
    await (await $('#ctx-unsplit')).click()
    await browser.waitUntil(async () => (await $$('.split-pane-wrapper').length) === 1, { timeout: 10000 })
  })

  it('13. Find bar', async () => {
    step('Focused the terminal pane')
    await focusTerminal()
    step('Pressed Ctrl+F (find bar)')
    await browser.keys(['Control', 'f'])
    const input = await $('.find-input')
    await input.waitForExist({ timeout: 5000 })
    step('Typed find query "Test-13 find query"')
    await input.setValue('Test-13 find query')
    await captureScreenshot('01-core', '13-find-bar')
    step('Clicked the find close button')
    await (await $('.find-close')).click()
    await input.waitForExist({ timeout: 5000, reverse: true })
  })

  it('14. Second window', async () => {
    const before = listWindows().length
    const winId = newWindow()
    await browser.waitUntil(() => listWindows().length === before + 1, { timeout: 10000, interval: 200 })
    // ponytail: physical teardown cannot be asserted under a WebDriver session —
    // tauri-driver's WebView2 automation holds every window open (both win.close()
    // and win.destroy() return Ok yet the webview never destroys). Verified working
    // standalone and in manual use. Here we assert the daemon accepts the close.
    expect(() => closeWindow(winId)).not.toThrow()
    await captureScreenshot('01-core', '14-second-window')
    await resetGuiState()
  })

  it('15. Multi-shell walk (cmd, git-bash, wsl)', async function () {
    this.timeout(600000)
    const SHELLS = [
      { profile: 'cmd', marker: 'KTERM-CMD-SHELL-OK', prompt: 60, echo: 60 },
      { profile: 'git-bash', marker: 'KTERM-GITBASH-SHELL-OK', prompt: 90, echo: 90 },
      { profile: 'wsl', marker: 'KTERM-WSL-SHELL-OK', prompt: 180, echo: 180 },
    ]
    for (const s of SHELLS) {
      const before = tabIds()
      step(`Shell walk: ${s.profile}`)
      spawnTab(s.profile)
      // DOM .tab-item counts layout nodes, not tabs — after any split those
      // diverge. Wait on daemon truth; expectTabTitleVisible is the UI barrier.
      await waitForTabCountDaemon(before.length + 1)
      const fresh = freshTabId(before)
      setTabTitle(fresh, `test-15-${s.profile}`)

      const titled = listTabs('win-1').find((t) => t.id === fresh)
      expect(titled).toBeDefined()
      expect(titled!.title).toBe(`test-15-${s.profile}`)
      await expectTabTitleVisible(`test-15-${s.profile}`)

      waitForPrompt(fresh, s.prompt)
      sendText(fresh, `echo ${s.marker}`)
      waitFor(fresh, s.marker, s.echo)

      splitTab(fresh, 'right', s.profile)
      const splitPane = listTabs('win-1').find((t) => t.id !== fresh && t.profile === s.profile)
      expect(splitPane).toBeDefined()
      await expectTabTitleVisible(`[test-15-${s.profile}]`, 15000)

      await browser.pause(800)
      await captureScreenshot('01-core', `15-shell-${s.profile}`)
    }
  })
})
