import { $, $$, browser } from '@wdio/globals'
import { focusTerminal, openContextMenu, waitForTabCount as waitForUiTabs } from '../../src/helpers/ui.js'
import { resetGuiState, activeTabTitle } from '../../src/helpers/state.js'
import {
  cliOk, listTabs, setTabTitle, setBadge, setColor,
  waitForPrompt, sendText, waitFor, splitTab, unsplit, getClipboard, setClipboard,
} from '../../src/helpers/daemon.js'
import { captureScreenshot } from '../../src/helpers/screenshot.js'
import { step } from '../../src/helpers/run.js'

async function waitForTabTitle(expected: string, timeout = 15000): Promise<void> {
  await browser.waitUntil(async () => (await activeTabTitle()) === expected, { timeout })
}

function tabIds(): string[] {
  return listTabs('win-1').map((t) => t.id)
}

function freshTabId(before: string[]): string {
  const tabs = listTabs('win-1')
  return tabs.find((t) => !before.includes(t.id))?.id ?? tabs[tabs.length - 1].id
}

async function firstTabId(): Promise<string> {
  return listTabs('win-1')[0].id
}

async function splitRatio(): Promise<number> {
  const nodes = JSON.parse(cliOk(['--get-layout', '--window', 'win-1']))
  const findSplit = (n: any): any => {
    if (!n) return null
    if (n.type === 'split') return n
    if (n.type === 'pane') return null
    return findSplit(n.first) ?? findSplit(n.second)
  }
  return findSplit((nodes as any)[0])?.ratio ?? -1
}

async function getElementRect(selector: string): Promise<{ x: number; y: number; width: number; height: number }> {
  return browser.execute((sel) => {
    const el = document.querySelector(sel) as HTMLElement | null
    if (!el) return null
    const r = el.getBoundingClientRect()
    return { x: r.left, y: r.top, width: r.width, height: r.height }
  }, selector)
}

async function dragSelectText(marker: string): Promise<void> {
  const rect = await browser.execute((m) => {
    for (const r of Array.from(document.querySelectorAll('.xterm-rows > div'))) {
      if ((r as HTMLElement).textContent?.includes(m)) {
        const b = (r as HTMLElement).getBoundingClientRect()
        return { x: b.x, y: b.y, width: b.width, height: b.height }
      }
    }
    return null
  }, marker)
  expect(rect).toBeTruthy()
  const r = rect as { x: number; y: number; width: number; height: number }
  for (let attempt = 0; attempt < 3; attempt++) {
    await browser.action('pointer', { parameters: { pointerType: 'mouse' } })
      .move({ x: r.x + 8, y: r.y + r.height / 2 })
      .down({ button: 0 })
      .pause(150)
      .move({ x: r.x + r.width - 8, y: r.y + r.height / 2 })
      .pause(300)
      .up({ button: 0 })
      .pause(200)
      .perform()
    if (await $$('.xterm-selection div').length > 0) return
  }
}

describe('04-ux', () => {
  after(async () => {
    await resetGuiState()
  })

  it('1. Tab badge and color render in the tab bar', async () => {
    await resetGuiState()
    const t = await firstTabId()
    setTabTitle(t, 'ux-badge-color')
    setBadge(t, 'PROD')
    setColor(t, '#E53935')

    await browser.waitUntil(
      async () => (await $$('.tab-badge').length) > 0,
      { timeout: 15000 },
    )
    const badgeText = await $('.tab-badge').getText()
    expect(badgeText).toBe('PROD')

    await browser.waitUntil(async () => {
      const c = await browser.execute(() => {
        const el = document.querySelector('.tab-item') as HTMLElement | null
        return el ? getComputedStyle(el).borderColor : null
      })
      return c === 'rgb(229, 57, 53)'
    }, { timeout: 15000 })
    await captureScreenshot('04-ux', '01-badge-color')
    await resetGuiState()
  })

  it('1b. Set badge and color from tab context menu', async () => {
    await resetGuiState()
    await openContextMenu('.tab-item')
    const badgeItem = await $('#ctx-tab-badge')
    await badgeItem.click()

    const modalInput = await $('.custom-modal-input')
    await modalInput.waitForExist({ timeout: 5000 })
    await modalInput.setValue('GUI-BADGE')
    const confirmBtn = await $('.confirm-btn')
    await confirmBtn.click()

    await browser.waitUntil(
      async () => (await $$('.tab-badge').length) > 0,
      { timeout: 15000 },
    )
    const badgeText = await $('.tab-badge').getText()
    expect(badgeText).toBe('GUI-BADGE')

    await openContextMenu('.tab-item')
    const colorItem = await $('#ctx-tab-color')
    await colorItem.click()

    const modalInput2 = await $('.custom-modal-input')
    await modalInput2.waitForExist({ timeout: 5000 })
    await modalInput2.setValue('#00FF00')
    const confirmBtn2 = await $('.confirm-btn')
    await confirmBtn2.click()

    await browser.waitUntil(async () => {
      const c = await browser.execute(() => {
        const el = document.querySelector('.tab-item') as HTMLElement | null
        return el ? getComputedStyle(el).borderColor : null
      })
      return c === 'rgb(0, 255, 0)'
    }, { timeout: 15000 })
    await resetGuiState()
  })

  it('2. Word highlights: add, render, remove', async () => {
    await resetGuiState()
    const t = await firstTabId()
    waitForPrompt(t, 30)
    sendText(t, 'echo UX-HIGHLIGHT-DEMO-4711')
    waitFor(t, 'UX-HIGHLIGHT-DEMO-4711', 30)
    step('Focused the terminal pane')
    await focusTerminal()
    step('Right-clicked the terminal pane')
    await openContextMenu('.terminal-pane')
    step('Clicked "Highlights" in the context menu')
    await (await $('#ctx-highlights')).click()

    const modal = await $('.highlights-modal')
    await modal.waitForExist({ timeout: 5000 })
    const input = await $('.highlights-input')
    step('Typed highlight "UX-HIGHLIGHT-DEMO-4711"')
    await input.setValue('UX-HIGHLIGHT-DEMO-4711')
    step('Clicked "Add"')
    await (await $('.highlights-add-btn')).click()

    await browser.waitUntil(
      async () => (await $$('.xterm-decoration').length) > 0,
      { timeout: 10000 },
    )
    // Highlights flash between two phases every 600ms (main.ts:99-100); wait
    // for the blue phase so the screenshot is deterministic vs the baseline.
    await browser.waitUntil(
      async () => {
        const dec = await $('.xterm-decoration')
        if (!(await dec.isExisting())) return false
        const bg = await dec.getCSSProperty('background-color')
        const v = typeof bg.value === 'string' ? bg.value : ''
        return /rgba?\(0,\s*68,\s*255/.test(v)
      },
      { timeout: 5000 },
    )
    await captureScreenshot('04-ux', '02-highlights-added')

    step('Clicked remove (×) on the highlight')
    await (await $('.highlight-remove-btn')).click()
    await browser.waitUntil(
      async () => (await $$('.xterm-decoration').length) === 0,
      { timeout: 10000 },
    )
    step('Clicked "Done"')
    await (await $('.highlights-done-btn')).click()
    await modal.waitForExist({ timeout: 5000, reverse: true })
    await captureScreenshot('04-ux', '02-highlights-removed')
  })

  it('3. Keyboard: previous tab', async () => {
    await resetGuiState()
    const b0 = tabIds()
    step('Clicked the "+" add tab button')
    await (await $('#add-tab-btn')).click()
    await waitForUiTabs(2)
    setTabTitle(b0[0], 'ux-k1')
    setTabTitle(freshTabId(b0), 'ux-k2')
    await waitForTabTitle('ux-k2')
    step('Pressed Ctrl+Shift+Tab (previous tab)')
    await browser.keys(['Control', 'Shift', 'Tab'])
    await waitForTabTitle('ux-k1')
    await captureScreenshot('04-ux', '03-kbd-prev')
  })

  it('4. Keyboard: jump to tab index', async () => {
    await resetGuiState()
    const b0 = tabIds()
    step('Clicked the "+" add tab button')
    await (await $('#add-tab-btn')).click()
    await waitForUiTabs(2)
    setTabTitle(b0[0], 'ux-j1')
    setTabTitle(freshTabId(b0), 'ux-j2')
    await waitForTabTitle('ux-j2')
    step('Pressed Ctrl+Shift+1 (jump to tab index 1)')
    await browser.keys(['Control', 'Shift', '1'])
    await waitForTabTitle('ux-j1')
    step('Pressed Ctrl+Shift+2 (jump to tab index 2)')
    await browser.keys(['Control', 'Shift', '2'])
    await waitForTabTitle('ux-j2')
    await captureScreenshot('04-ux', '04-kbd-jump')
  })

  it('5. Keyboard: split / unsplit', async () => {
    await resetGuiState()
    step('Focused the terminal pane')
    await focusTerminal()
    step('Pressed Ctrl+Shift+→ (split right)')
    await browser.keys(['Control', 'Shift', 'ArrowRight'])
    await browser.waitUntil(async () => (await $$('.split-pane-wrapper').length) === 2, { timeout: 10000 })
    await captureScreenshot('04-ux', '05-kbd-split')
    step('Pressed Ctrl+Shift+W (unsplit)')
    await browser.keys(['Control', 'Shift', 'w'])
    await browser.waitUntil(async () => (await $$('.split-pane-wrapper').length) === 1, { timeout: 10000 })
    await captureScreenshot('04-ux', '05-kbd-unsplit')
  })

  it('6. Paste from clipboard', async () => {
    await resetGuiState()
    const t = await firstTabId()
    waitForPrompt(t, 30)
    step('Copied "KTERM-PASTE-DEMO-8842" to the Windows clipboard')
    setClipboard('KTERM-PASTE-DEMO-8842')
    step('Focused the terminal pane')
    await focusTerminal()
    step('Pressed Ctrl+V (paste)')
    await browser.keys(['Control', 'v'])
    waitFor(t, 'KTERM-PASTE-DEMO-8842', 30)
    await captureScreenshot('04-ux', '06-paste')
    sendText(t, '')
    waitForPrompt(t, 15)
    step('Cleared the clipboard')
    setClipboard(' ')
  })

  it('7. Smart Ctrl+C: copy selection to clipboard', async () => {
    await resetGuiState()
    const t = await firstTabId()
    waitForPrompt(t, 30)
    sendText(t, 'echo KTERM-COPY-DEMO-5509')
    waitFor(t, 'KTERM-COPY-DEMO-5509', 30)
    step('Focused the terminal pane')
    await focusTerminal()
    step('Dragged to select the "KTERM-COPY-DEMO-5509" text')
    await dragSelectText('KTERM-COPY-DEMO-5509')
    await browser.waitUntil(async () => (await $$('.xterm-selection div').length) > 0, { timeout: 5000 })
    await captureScreenshot('04-ux', '07-copy-selected')

    step('Pressed Ctrl+C (smart copy to clipboard)')
    await browser.keys(['Control', 'c'])
    await browser.waitUntil(() => getClipboard().includes('KTERM-COPY-DEMO-5509'), {
      timeout: 8000,
      interval: 300,
    })
    await browser.waitUntil(async () => (await $$('.xterm-selection div').length) === 0, { timeout: 5000 })

    step('Split the pane right to make a paste target')
    splitTab(t, 'right', 'powershell')
    await browser.waitUntil(async () => (await $$('.split-pane-wrapper').length) === 2, { timeout: 10000 })
    const target = listTabs('win-1').find((x) => x.id !== t)
    expect(target).toBeDefined()
    setTabTitle(target!.id, 'ux-copy-target')
    step('Clicked the new split pane to focus it')
    const focused = await browser.execute((tabId) => {
      const el = document.querySelector(`.split-pane-wrapper[data-tab-id="${tabId}"]`) as HTMLElement | null
      el?.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, button: 0 }))
      return !!el
    }, target!.id)
    expect(focused).toBe(true)
    step('Pressed Ctrl+V (paste into the new split)')
    await browser.keys(['Control', 'v'])
    waitFor(target!.id, 'KTERM-COPY-DEMO-5509', 30)
    await browser.pause(800)
    await captureScreenshot('04-ux', '07-paste-split')
  })

  it('8. Split divider drag resizes the pane ratio', async () => {
    await resetGuiState()
    const t = await firstTabId()
    splitTab(t, 'right', 'powershell')
    await browser.waitUntil(async () => (await $$('.split-pane-wrapper').length) === 2, { timeout: 10000 })
    await browser.pause(1500)
    const before = await splitRatio()
    expect(before).toBeGreaterThanOrEqual(0.4)
    expect(before).toBeLessThanOrEqual(0.6)

    step('Dragged the split divider 120px to the right')
    const div = await $('.split-divider.horizontal')
    await div.waitForExist({ timeout: 5000 })
    const rect = await getElementRect('.split-divider.horizontal')
    const mx = rect.x + rect.width / 2
    const my = rect.y + rect.height / 2
    await browser.action('pointer', { parameters: { pointerType: 'mouse' } })
      .move({ x: mx, y: my })
      .down({ button: 0 })
      .move({ x: mx + 120, y: my })
      .pause(200)
      .up({ button: 0 })
      .pause(500)
      .perform()

    await browser.waitUntil(async () => (await splitRatio()) > before + 0.05, { timeout: 10000 })
    await captureScreenshot('04-ux', '08-drag-resize')
    unsplit(t)
    await browser.waitUntil(async () => (await $$('.split-pane-wrapper').length) === 1, { timeout: 10000 })
    await captureScreenshot('04-ux', '08-unsplit')
  })
})
