import { expect } from '@wdio/globals'
import { normalizeWindow, listTabs, closeTab, splitTab, unsplit, explodeSplit } from '../../src/helpers/daemon.js'

describe('23-splits', () => {
  after(() => normalizeWindow('win-1'))

  it('splits a tab right and down', () => {
    const t1 = normalizeWindow('win-1')
    const b = splitTab(t1, 'right')
    expect(b).toMatch(/^tab-\d+$/)
    expect(listTabs('win-1').length).toBe(2)
    const c = splitTab(t1, 'down')
    expect(listTabs('win-1').length).toBe(3)
    closeTab(b)
    closeTab(c)
    expect(listTabs('win-1').length).toBe(1)
  })

  it('unsplits a pane back to a standalone tab', () => {
    const t1 = normalizeWindow('win-1')
    splitTab(t1, 'right')
    unsplit(t1)
    expect(listTabs('win-1').length).toBe(2)
    normalizeWindow('win-1')
  })

  it('explodes a split into standalone tabs', () => {
    const t1 = normalizeWindow('win-1')
    splitTab(t1, 'right')
    splitTab(t1, 'down')
    explodeSplit(t1)
    expect(listTabs('win-1').length).toBeGreaterThanOrEqual(3)
  })
})
