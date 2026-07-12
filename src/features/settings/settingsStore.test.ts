import { expect, it, vi } from 'vitest'

import {
  createSettingsPersistence,
  normalizeJobs,
} from './settingsStore.ts'

it('settings default to four jobs', async () => {
  const persistence = createSettingsPersistence(async () => ({
    get: vi.fn().mockResolvedValue(undefined),
    set: vi.fn(),
  }))

  await expect(persistence.loadSettings()).resolves.toEqual({
    jobs: 4,
    appearance: 'system',
  })
})

it('job normalization floors values and clamps them to the supplied maximum', () => {
  expect(normalizeJobs(5.9, 8)).toBe(5)
  expect(normalizeJobs(12, 8)).toBe(8)
  expect(normalizeJobs(0, 8)).toBe(1)
  expect(normalizeJobs(Number.NaN, 8)).toBe(4)
})
