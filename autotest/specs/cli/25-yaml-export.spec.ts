import fs from 'node:fs'
import path from 'node:path'
import YAML from 'yaml'
import { expect } from '@wdio/globals'
import { normalizeWindow, spawnTab, setTabTitle, exportLayout } from '../../src/helpers/daemon.js'
import { TMP_DIR } from '../../src/helpers/paths.js'

const OUT = path.join(TMP_DIR, 'export-layout.yaml')

describe('25-yaml-export', () => {
  after(() => normalizeWindow('win-1'))

  it('exports the window layout to YAML', () => {
    const t1 = normalizeWindow('win-1')
    setTabTitle(t1, 'export-me')
    const t2 = spawnTab('cmd', 'win-1')
    setTabTitle(t2, 'export-me-too')

    exportLayout('win-1', OUT)
    const doc = YAML.parse(fs.readFileSync(OUT, 'utf8'))

    expect(doc.window.id).toBe('win-1')
    expect(doc.tabs.length).toBe(2)
    expect(doc.tabs.map((t: any) => t.title)).toContain('export-me')
    expect(doc.tabs.map((t: any) => t.title)).toContain('export-me-too')
    expect(doc.tabs.map((t: any) => t.profile)).toContain('cmd')

    const lnk = OUT.replace(/\.yaml$/i, '.lnk')
    expect(fs.existsSync(lnk)).toBe(true)
    expect(fs.statSync(lnk).size).toBeGreaterThan(0)
  })
})
