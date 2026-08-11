import { expect } from '@wdio/globals'
import { normalizeWindow, waitForPrompt, sendText, waitFor, readText } from '../../src/helpers/daemon.js'

describe('21-io', () => {
  after(() => normalizeWindow('win-1'))

  it('echo roundtrip via send-text/wait-for/read-text', () => {
    const tab = normalizeWindow('win-1')
    waitForPrompt(tab, 30)
    sendText(tab, 'echo hello-io-1')
    waitFor(tab, 'hello-io-1', 20)
    expect(readText(tab, 20)).toContain('hello-io-1')
  })

  it('passes multi-word arguments', () => {
    const tab = normalizeWindow('win-1')
    waitForPrompt(tab, 30)
    sendText(tab, 'echo "hello world with spaces"')
    waitFor(tab, 'hello world with spaces', 20)
  })

  it('handles long scrollback and tail reads', () => {
    const tab = normalizeWindow('win-1')
    waitForPrompt(tab, 30)
    sendText(tab, '1..60 | ForEach-Object { "line-$_" }')
    waitFor(tab, 'line-60', 20)
    const tail = readText(tab, 8)
    expect(tail).toContain('line-60')
  })

  it('roundtrips unicode output', () => {
    const tab = normalizeWindow('win-1')
    waitForPrompt(tab, 30)
    sendText(tab, 'echo héllo-ünïcode-π')
    waitFor(tab, 'héllo-ünïcode-π', 20)
  })
})
