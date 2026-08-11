import fs from 'node:fs'
import path from 'node:path'
import YAML from 'yaml'
import { expect } from '@wdio/globals'
import { cliOk, listTabs, listWindows, closeWindow, spawnTab, setTabTitle, exportLayout, normalizeWindow } from '../../src/helpers/daemon.js'
import { TMP_DIR } from '../../src/helpers/paths.js'

describe('27-suffix-focus', () => {
  it('focus target window without error', () => {
    cliOk(['--focus', '--window', 'win-1'])
  })

  it('applies YAML with an explicit suffix window', () => {
    const tab = spawnTab('powershell', 'win-1')
    setTabTitle(tab, 'suffix-source')

    const FIXTURE = path.join(TMP_DIR, 'suffix-window.yaml')
    fs.writeFileSync(FIXTURE, `window:
  id: null
tabs:
- id: null
  profile: powershell
  title: suffix-target
`)
    cliOk(['--apply', FIXTURE, '--window', 'win-1', '--suffix=-sfx'])
    const wins = listWindows()
    expect(wins.map((w: any) => w.id)).toContain('win-1-sfx')
    const tabs = listTabs('win-1-sfx')
    expect(tabs.map((t: any) => t.title)).toContain('suffix-target')
    closeWindow('win-1-sfx')
  })

  it('auto-suffix picks the first unused window id', () => {
    const FIXTURE = path.join(TMP_DIR, 'suffix-auto-window.yaml')
    fs.writeFileSync(FIXTURE, `window:
  id: null
tabs:
- id: null
  profile: powershell
  title: auto-suffix-a
`)
    cliOk(['--apply', FIXTURE, '--window', 'win-1', '--suffix-auto'])
    expect(listWindows().map((w: any) => w.id)).toContain('win-1-1')
    cliOk(['--apply', FIXTURE, '--window', 'win-1', '--suffix-auto'])
    expect(listWindows().map((w: any) => w.id)).toContain('win-1-2')
    closeWindow('win-1-1')
    closeWindow('win-1-2')
  })

  it('export then re-apply roundtrips the layout', () => {
    const t1 = normalizeWindow('win-1')
    setTabTitle(t1, 'roundtrip-a')
    const t2 = spawnTab('cmd', 'win-1')
    setTabTitle(t2, 'roundtrip-b')

    const OUT = path.join(TMP_DIR, 'roundtrip-export.yaml')
    exportLayout('win-1', OUT)
    const doc = YAML.parse(fs.readFileSync(OUT, 'utf8'))
    expect(doc.tabs.length).toBe(2)

    cliOk(['--apply', OUT, '--window', 'win-r1'])
    const tabs = listTabs('win-r1')
    expect(tabs.map((t: any) => t.title)).toContain('roundtrip-a')
    expect(tabs.map((t: any) => t.title)).toContain('roundtrip-b')
    closeWindow('win-r1')
  })

  it('re-applying to a suffixed window uses that window', () => {
    const FIXTURE = path.join(TMP_DIR, 'reapply-suffix.yaml')
    fs.writeFileSync(FIXTURE, `window:
  id: null
tabs:
- id: null
  profile: powershell
  title: reapply-again
`)
    cliOk(['--apply', FIXTURE, '--window', 'win-1', '--suffix=-re'])
    const win = 'win-1-re'
    expect(listTabs(win).map((t: any) => t.title)).toContain('reapply-again')
    cliOk(['--apply', FIXTURE, '--window', 'win-1', '--suffix=-re'])
    expect(listTabs(win).map((t: any) => t.title)).toContain('reapply-again')
    expect(listWindows().filter((w: any) => w.id === win).length).toBe(1)
    closeWindow(win)
  })

  after(() => {
    try {
      closeWindow('win-1-sfx')
      closeWindow('win-1-1')
      closeWindow('win-1-2')
      closeWindow('win-1-re')
      closeWindow('win-r1')
    } catch {
      /* best-effort cleanup */
    }
    normalizeWindow('win-1')
  })
})
