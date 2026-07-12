import { load } from '@tauri-apps/plugin-store'

export const appearanceOptions = ['system', 'light', 'dark'] as const

export type Appearance = (typeof appearanceOptions)[number]

export interface PersistedSettings {
  jobs: number
  appearance: Appearance
}

export interface SettingsPersistence {
  loadSettings: () => Promise<PersistedSettings>
  saveSettings: (snapshot: PersistedSettings) => Promise<void>
}

export const defaultSettings = Object.freeze<PersistedSettings>({
  jobs: 4,
  appearance: 'system',
})

interface SettingsStore {
  get: (key: string) => Promise<unknown>
  set: (key: string, value: unknown) => Promise<void>
}

type StoreLoader = () => Promise<SettingsStore>

const SETTINGS_FILE = 'settings.json'
const SETTINGS_KEY = 'preferences'
const AUTO_SAVE_DEBOUNCE_MS = 250

let storePromise: Promise<SettingsStore> | undefined

export const settingsPersistence = createSettingsPersistence(loadSettingsStore)

export function createSettingsPersistence(loadStore: StoreLoader): SettingsPersistence {
  return {
    async loadSettings() {
      try {
        const stored = await (await loadStore()).get(SETTINGS_KEY)
        return normalizeSettings(stored)
      }
      catch {
        reportPersistenceFailure('load')
        return defaultSettings
      }
    },
    async saveSettings(snapshot) {
      try {
        await (await loadStore()).set(SETTINGS_KEY, normalizeSettings(snapshot))
      }
      catch {
        reportPersistenceFailure('save')
      }
    },
  }
}

function reportPersistenceFailure(operation: 'load' | 'save') {
  console.warn(`Settings ${operation} failed; continuing with in-memory preferences.`)
}

function normalizeSettings(stored: unknown): PersistedSettings {
  if (!stored || typeof stored !== 'object')
    return defaultSettings

  const candidate = stored as Record<string, unknown>
  return {
    jobs: normalizeJobs(candidate.jobs),
    appearance: appearanceOptions.includes(candidate.appearance as Appearance)
      ? (candidate.appearance as Appearance)
      : defaultSettings.appearance,
  }
}

export function normalizeJobs(
  value: unknown,
  maxJobs = Number.MAX_SAFE_INTEGER,
): number {
  const normalizedMax
    = typeof maxJobs === 'number' && Number.isFinite(maxJobs)
      ? Math.max(1, Math.floor(maxJobs))
      : defaultSettings.jobs
  const normalizedValue
    = typeof value === 'number' && Number.isFinite(value)
      ? Math.max(1, Math.floor(value))
      : defaultSettings.jobs

  return Math.min(normalizedValue, normalizedMax)
}

function loadSettingsStore(): Promise<SettingsStore> {
  if (!storePromise) {
    storePromise = load(SETTINGS_FILE, {
      defaults: { [SETTINGS_KEY]: defaultSettings },
      autoSave: AUTO_SAVE_DEBOUNCE_MS,
    }).catch((error: unknown) => {
      storePromise = undefined
      throw error
    })
  }

  return storePromise
}
