import assert from 'node:assert/strict'
import { Buffer } from 'node:buffer'
import { mkdtemp, readFile, rm, stat, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import path from 'node:path'
import process from 'node:process'
import { $, browser } from '@wdio/globals'

let fixtureRoot
let fixtureFontPath
let removeFixture = false

/* eslint-disable antfu/consistent-list-newline */
const legacyFontBytes = Buffer.from([
  1, 0, 0, 0, 1, 0, 0, 0, 92, 0, 0, 0, 0, 0, 0, 0,
  0, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 104, 0, 0, 0,
  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
  0, 0, 0, 0, 112, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0,
  0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 2, 3, 4,
  5, 6, 7, 8, 9, 10, 0, 0, 65, 0, 0, 0, 0, 0, 0, 0,
  109, 101, 116, 97, 100, 97, 116, 97, 45, 122, 83, 48, 55,
])
/* eslint-enable antfu/consistent-list-newline */

describe('converter native flow', () => {
  before(async () => {
    fixtureRoot = process.env.PD2X64_E2E_ROOT
    if (fixtureRoot) {
      const rootStats = await stat(fixtureRoot)
      assert.equal(rootStats.isDirectory(), true)
      return
    }

    fixtureRoot = await mkdtemp(path.join(tmpdir(), 'pd2x64-e2e-'))
    fixtureFontPath = path.join(fixtureRoot, 'asset.font')
    await writeFile(fixtureFontPath, legacyFontBytes)
    removeFixture = true
  })

  after(async () => {
    if (removeFixture && fixtureRoot) {
      await rm(fixtureRoot, { force: true, recursive: true })
    }
  })

  it('selects an E2E folder and completes a dry run', async () => {
    await browser.refresh()
    await waitForTauriInvoke()
    await invoke('e2e_set_pick_folder', { path: fixtureRoot })

    const pickerButton = await $('//button[.//*[@id="select-title"]]')
    await pickerButton.waitForClickable({ timeout: 30_000 })
    await pickerButton.click()

    const reviewTitle = await waitForDisplayed('#review-title', 60_000)
    assert.match(await reviewTitle.getText(), /Scan & Review/)
    assert.match(await $('body').getText(), /1 file ready to convert/)

    await clickButton('Dry run')
    await clickButton('Run dry run')

    const resultsTitle = await waitForDisplayed('#results-title', 60_000)
    assert.match(await resultsTitle.getText(), /Dry run completed/)
    assert.match(await $('body').getText(), /Would convert/)

    if (fixtureFontPath) {
      assert.equal(Buffer.compare(await readFile(fixtureFontPath), legacyFontBytes), 0)
    }
  })
})

async function invoke(command, args) {
  return browser.execute(
    (innerCommand, innerArgs) => window.__TAURI__.core.invoke(innerCommand, innerArgs),
    command,
    args,
  )
}

async function waitForTauriInvoke() {
  await browser.waitUntil(
    () => browser.execute(() => typeof window.__TAURI__?.core?.invoke === 'function'),
    { timeout: 30_000 },
  )
}

async function clickButton(label) {
  const button = await $(`//button[.//*[normalize-space(.)="${label}"] or normalize-space(.)="${label}"]`)
  await button.waitForClickable({ timeout: 30_000 })
  await button.click()
}

async function waitForDisplayed(selector, timeout) {
  const element = await $(selector)
  try {
    await element.waitForDisplayed({ timeout })
  }
  catch (error) {
    throw new Error(`${selector} did not appear. Page text: ${await pageText()}`, { cause: error })
  }
  return element
}

async function pageText() {
  const text = await $('body').getText()
  return text.replace(/\s+/g, ' ').slice(0, 1_000)
}
