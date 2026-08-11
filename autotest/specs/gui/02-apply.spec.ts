import { $, $$, browser } from '@wdio/globals'
import { waitForTabCount } from '../../src/helpers/ui.js'
import { captureScreenshot } from '../../src/helpers/screenshot.js'
import { listTabs } from '../../src/helpers/daemon.js'

describe('02-apply boot (--apply session)', () => {
  it('renders the applied session tabs in the GUI', async () => {
    try {
      await waitForTabCount(2, 25000)
    } catch (e) {
      const diag = await browser.execute(async () => {
        const out: any = {
          url: location.href,
          title: document.title,
          bodySnippet: document.body ? document.body.innerHTML.slice(0, 300) : 'NO BODY',
          hasSync: typeof (window as any).__triggerSyncTabs === 'function',
          tabItems: document.querySelectorAll('.tab-item').length,
          terminalContainer: !!document.getElementById('terminal-container'),
          tabsList: !!document.getElementById('tabs-list'),
        }
        try {
          await (window as any).__triggerSyncTabs()
          await new Promise((r) => setTimeout(r, 600))
          out.afterManualSync = document.querySelectorAll('.tab-item').length
        } catch (err: any) {
          out.manualSyncErr = String(err && err.message ? err.message : err)
        }
        return out
      })
      console.log('[02] page diag:', JSON.stringify(diag))
      console.log('[02] daemon tabs for win-1:', JSON.stringify(listTabs('win-1')))
      throw e
    }
    const titles = await $$('.tab-item .tab-title').map((el) => el.getText())
    const joined = (await Promise.all(titles)).join(' ')
    expect(joined).toContain('boot-apply-a')
    expect(joined).toContain('boot-apply-b')
    expect(joined).not.toContain('boot-apply-b2')
    await captureScreenshot('02-apply', 'applied-tabs')
  })

  it('shows the split pane from the applied session', async () => {
    const clicked = await browser.execute(() => {
      const items = document.querySelectorAll('.tab-item')
      for (const el of items) {
        const title = el.querySelector('.tab-title')?.textContent ?? ''
        if (title.includes('boot-apply-b') && !title.includes('boot-apply-b2')) {
          (el as HTMLElement).click()
          return true
        }
      }
      return false
    })
    expect(clicked).toBe(true)
    await browser.pause(400)
    expect(await $$('.terminal-pane').length).toBe(2)
    expect(await $$('.split-pane-wrapper').length).toBe(2)
    await captureScreenshot('02-apply', 'applied-split')
  })
})
