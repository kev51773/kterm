import { expect } from '@wdio/globals'
import { cliOk, listWindows, listTabs, spawnTab, closeTab, normalizeWindow } from '../../src/helpers/daemon.js'

describe('20-basic', () => {
  after(() => normalizeWindow('win-1'))

  it('reports version', () => {
    expect(cliOk(['--version'])).toMatch(/^kterm \d+\.\d+\.\d+/)
  })

  it('prints help with core commands', () => {
    const h = cliOk(['--help'])
    expect(h).toContain('--daemon')
    expect(h).toContain('--apply')
    expect(h).toContain('--split-right')
    expect(h).toContain('--wait-for')
  })

  it('lists the registered window', () => {
    const wins = listWindows()
    expect(wins.length).toBeGreaterThanOrEqual(1)
    expect(wins.map((w: any) => w.id)).toContain('win-1')
  })

  it('spawns, lists, and closes a tab', () => {
    normalizeWindow('win-1')
    const before = listTabs('win-1').length
    const tab = spawnTab('powershell', 'win-1')
    expect(tab).toMatch(/^tab-\d+$/)
    expect(listTabs('win-1').length).toBe(before + 1)
    closeTab(tab)
    expect(listTabs('win-1').length).toBe(before)
  })
})
