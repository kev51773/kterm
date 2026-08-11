import { $, $$ } from '@wdio/globals'
import { captureScreenshot } from '../../src/helpers/screenshot.js'
import { step } from '../../src/helpers/run.js'

describe('03-admin boot (--apply with admin shell)', () => {
  it('renders an admin-badged tab from the applied session', async () => {
    step('Opened kterm with --apply boot session (boot-admin.yaml: admin powershell)')
    await browser.waitUntil(
      async () => (await $$('.tab-item.tab-admin')).length > 0,
      { timeout: 20000 },
    )
    const title = await $('.tab-item.tab-admin .tab-title').getText()
    expect(title).toContain('boot-admin-shell')
    await $('.tab-item.tab-admin .tab-admin-badge').waitForExist({ timeout: 5000 })
    await captureScreenshot('03-admin', 'admin-badge')
  })

  it('has exactly one tab', async () => {
    step('Verified the boot applied exactly one admin tab')
    expect(await $$('.tab-item').length).toBe(1)
    await captureScreenshot('03-admin', 'single-tab')
  })
})
