import { expect } from '@wdio/globals'
import {
  normalizeWindow,
  spawnTab,
  listTabs,
  closeTab,
  setTabTitle,
  setBadge,
  setColor,
  setWindowTitle,
} from '../../src/helpers/daemon.js'

describe('22-tabs', () => {
  after(() => normalizeWindow('win-1'))

  it('spawns multiple tabs with distinct profiles', () => {
    const t1 = normalizeWindow('win-1')
    const t2 = spawnTab('powershell', 'win-1')
    const t3 = spawnTab('cmd', 'win-1')
    const tabs = listTabs('win-1')
    expect(tabs.length).toBe(3)
    expect(tabs.map((t: any) => t.id)).toContain(t2)
    expect(tabs.find((t: any) => t.id === t3)?.profile).toBe('cmd')
    closeTab(t3)
    expect(listTabs('win-1').length).toBe(2)
    closeTab(t2)
    expect(listTabs('win-1').length).toBe(1)
  })

  it('sets titles, badges, and colors', () => {
    const tab = normalizeWindow('win-1')
    setTabTitle(tab, 'Server Logs')
    setBadge(tab, 'PROD')
    setColor(tab, '#E53935')
    const info = listTabs('win-1').find((t: any) => t.id === tab)
    expect(info?.title).toBe('Server Logs')
    expect(info?.badge).toBe('PROD')
    expect(info?.color).toBe('#E53935')
  })

  it('sets and restores the window title', () => {
    setWindowTitle('win-1', 'Main Workspace')
    setWindowTitle('win-1', 'kterm.exe - A scriptable terminal - win-1')
  })
})
