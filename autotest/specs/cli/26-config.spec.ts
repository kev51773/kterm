import fs from 'node:fs'
import { expect } from '@wdio/globals'
import { KTERM_CONFIG } from '../../src/helpers/paths.js'

describe('26-config', () => {
  it('writes a parseable config file', () => {
    expect(fs.existsSync(KTERM_CONFIG)).toBe(true)
    const cfg = JSON.parse(fs.readFileSync(KTERM_CONFIG, 'utf8'))
    expect(typeof cfg.default_profile).toBe('string')
    expect(typeof cfg.default_cols).toBe('number')
    expect(typeof cfg.default_rows).toBe('number')
    expect(cfg.theme).toBeDefined()
    expect(cfg.theme.background).toBeDefined()
    expect(cfg.font).toBeDefined()
    expect(cfg.font.family).toBeDefined()
  })
})
