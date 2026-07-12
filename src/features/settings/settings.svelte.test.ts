// @vitest-environment jsdom

import type { SettingsState } from './settings.svelte.ts'
import type {
  PersistedSettings,
  SettingsPersistence,
} from './settingsStore.ts'

import { flushSync, mount, tick, unmount } from 'svelte'
import { expect, it, vi } from 'vitest'
import {
  createSettingsState,

} from './settings.svelte.ts'
import SettingsSheet from './SettingsSheet.svelte'

function createPersistence(snapshot: PersistedSettings) {
  const saveSettings = vi.fn<(snapshot: PersistedSettings) => Promise<void>>()
    .mockResolvedValue(undefined)
  const persistence: SettingsPersistence = {
    loadSettings: vi.fn().mockResolvedValue(snapshot),
    saveSettings,
  }

  return { persistence, saveSettings }
}

function createState(
  persistence: SettingsPersistence,
  loadMaxJobs: () => Promise<number>,
) {
  let settings!: SettingsState
  const cleanup = $effect.root(() => {
    settings = createSettingsState(persistence, loadMaxJobs)
  })
  flushSync()

  return {
    get settings() {
      return settings
    },
    cleanup,
  }
}

it('hydration clamps persisted jobs to the logical CPU limit', async () => {
  const { persistence, saveSettings } = createPersistence({
    jobs: 12,
    appearance: 'dark',
  })
  const state = createState(persistence, async () => 6)

  await state.settings.hydrate()

  expect(state.settings.maxJobs).toBe(6)
  expect(state.settings.jobs).toBe(6)
  expect(state.settings.jobsInput).toBe('6')
  expect(state.settings.appearance).toBe('dark')
  expect(saveSettings).not.toHaveBeenCalled()
  state.cleanup()
})

it('a failed CPU-limit lookup falls back to one safe job', async () => {
  const { persistence } = createPersistence({
    jobs: 4,
    appearance: 'system',
  })
  const state = createState(persistence, async () => {
    throw new Error('IPC unavailable')
  })

  await state.settings.hydrate()

  expect(state.settings.maxJobs).toBe(1)
  expect(state.settings.jobs).toBe(1)
  state.cleanup()
})

it('job edits clamp to the hydrated maximum and persist normalized snapshots', async () => {
  const { persistence, saveSettings } = createPersistence({
    jobs: 4,
    appearance: 'light',
  })
  const state = createState(persistence, async () => 6)
  await state.settings.hydrate()

  expect(state.settings.jobs).toBe(4)
  state.settings.setJobs(20)
  state.settings.stepJobs(-20)

  expect(state.settings.jobs).toBe(1)
  expect(saveSettings).toHaveBeenNthCalledWith(1, {
    jobs: 6,
    appearance: 'light',
  })
  expect(saveSettings).toHaveBeenNthCalledWith(2, {
    jobs: 1,
    appearance: 'light',
  })
  state.cleanup()
})

it('the jobs input displays the clamped value', async () => {
  const { persistence } = createPersistence({
    jobs: 6,
    appearance: 'system',
  })
  const state = createState(persistence, async () => 6)
  await state.settings.hydrate()
  state.settings.settingsOpen = true
  const target = document.createElement('div')
  document.body.append(target)
  const component = mount(SettingsSheet, {
    target,
    props: { settings: state.settings, onToggle: vi.fn() },
  })
  flushSync()
  const input = document.querySelector<HTMLInputElement>(
    'input[aria-label="Jobs"]',
  )

  expect(input).not.toBeNull()
  input!.value = '20'
  input!.dispatchEvent(new InputEvent('input', { bubbles: true }))
  await tick()

  expect(state.settings.jobs).toBe(6)
  expect(input!.value).toBe('6')
  await unmount(component)
  await new Promise(resolve => setTimeout(resolve, 25))
  target.remove()
  state.cleanup()
})

it('an edit made during hydration is preserved but capped by the loaded maximum', async () => {
  let resolveMaxJobs!: (maxJobs: number) => void
  const maxJobs = new Promise<number>((resolve) => {
    resolveMaxJobs = resolve
  })
  const { persistence, saveSettings } = createPersistence({
    jobs: 2,
    appearance: 'system',
  })
  const state = createState(persistence, () => maxJobs)

  const hydration = state.settings.hydrate()
  state.settings.setJobs(20)
  resolveMaxJobs(6)
  await hydration

  expect(state.settings.jobs).toBe(6)
  expect(saveSettings).toHaveBeenCalledWith({
    jobs: 6,
    appearance: 'system',
  })
  state.cleanup()
})
