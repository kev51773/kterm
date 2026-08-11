import fs from 'node:fs'
import path from 'node:path'
import { expect } from '@wdio/globals'
import { cliOk, listTabs, closeTab, normalizeWindow } from '../../src/helpers/daemon.js'
import { TMP_DIR } from '../../src/helpers/paths.js'

const FIXTURE = path.join(TMP_DIR, 'yaml-apply-session.yaml')

const SESSION_YAML = `window:
  id: null
tabs:
- id: null
  profile: powershell
  title: yaml-app-a
- id: null
  profile: cmd
  title: yaml-app-b
  splits:
  - direction: right
    profile: powershell
    title: yaml-app-b2
`

function tabCount(): number {
  return listTabs('win-1').length
}

describe('24-yaml-apply', () => {
  before(() => {
    fs.mkdirSync(TMP_DIR, { recursive: true })
    fs.writeFileSync(FIXTURE, SESSION_YAML)
  })

  after(() => normalizeWindow('win-1'))

  it('dry-run validates without creating tabs', () => {
    const before = tabCount()
    const out = cliOk(['--apply', FIXTURE, '--window', 'win-1', '--dry-run'])
    expect(out).toContain('YAML specification is valid.')
    expect(tabCount()).toBe(before)
  })

  it('applies a session YAML with splits', () => {
    const before = tabCount()
    cliOk(['--apply', FIXTURE, '--window', 'win-1'])
    const tabs = listTabs('win-1')
    expect(tabs.length).toBe(before + 3)
    expect(tabs.map((t: any) => t.title)).toContain('yaml-app-a')
    expect(tabs.map((t: any) => t.title)).toContain('yaml-app-b')
    expect(tabs.map((t: any) => t.title)).toContain('yaml-app-b2')
  })

  it('cleans up applied tabs', () => {
    const tabs = listTabs('win-1')
    for (const t of tabs) {
      if (t.title?.startsWith('yaml-app')) closeTab(t.id)
    }
    normalizeWindow('win-1')
  })
})
