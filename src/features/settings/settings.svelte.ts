import type { Appearance, PersistedSettings, SettingsPersistence } from './settingsStore.ts'
import { getMaxJobs } from '@/platform/tauri/settingsApi.ts'
import {

  defaultSettings,
  normalizeJobs,

  settingsPersistence,

} from './settingsStore.ts'

export { appearanceOptions } from './settingsStore.ts'

export type SettingsState = ReturnType<typeof createSettingsState>

export function createSettingsState(
  persistence: SettingsPersistence = settingsPersistence,
  loadMaxJobs: () => Promise<number> = getMaxJobs,
) {
  let jobs = $state(defaultSettings.jobs)
  let jobsInput = $state(String(defaultSettings.jobs))
  let maxJobs = $state(Number.MAX_SAFE_INTEGER)
  let appearance = $state<Appearance>(defaultSettings.appearance)
  let settingsOpen = $state(false)
  let hydrated = false
  let jobsChangedDuringHydration = false
  let appearanceChangedDuringHydration = false
  let hydrationPromise: Promise<void> | undefined

  $effect(() => {
    if (typeof document !== 'undefined') {
      document.documentElement.dataset.appearance = appearance
    }
  })

  function open() {
    settingsOpen = true
  }

  function close() {
    settingsOpen = false
  }

  function toggle() {
    settingsOpen = !settingsOpen
  }

  function setAppearance(next: Appearance) {
    appearance = next
    if (!hydrated) {
      appearanceChangedDuringHydration = true
      return
    }
    persist()
  }

  function stepJobs(delta: number) {
    setJobs(jobs + delta)
  }

  function updateJobs(event: Event) {
    setJobs(Number((event.currentTarget as HTMLInputElement).value))
  }

  function setJobs(next: number) {
    jobs = normalizeJobs(next, maxJobs)
    jobsInput = String(jobs)
    if (!hydrated) {
      jobsChangedDuringHydration = true
      return
    }
    persist()
  }

  function hydrate(): Promise<void> {
    hydrationPromise ??= Promise.all([
      persistence.loadSettings().catch(() => defaultSettings),
      loadMaxJobs().catch(() => 1),
    ])
      .then(([snapshot, availableJobs]) => {
        maxJobs = normalizeJobs(availableJobs)
        applyPersistedSettings(snapshot)
      })
      .finally(() => {
        hydrated = true
        if (jobsChangedDuringHydration || appearanceChangedDuringHydration)
persist()
      })
    return hydrationPromise
  }

  function applyPersistedSettings(snapshot: PersistedSettings) {
    if (!jobsChangedDuringHydration) {
      jobs = normalizeJobs(snapshot.jobs, maxJobs)
    }
 else {
      jobs = normalizeJobs(jobs, maxJobs)
    }
    jobsInput = String(jobs)
    if (!appearanceChangedDuringHydration)
appearance = snapshot.appearance
  }

  function persist() {
    if (!hydrated)
return
    void persistence.saveSettings({ jobs, appearance }).catch(() => undefined)
  }

  return {
    get jobs() {
      return jobs
    },
    get jobsInput() {
      return jobsInput
    },
    set jobsInput(value: string) {
      jobsInput = value
    },
    get maxJobs() {
      return maxJobs
    },
    get appearance() {
      return appearance
    },
    get settingsOpen() {
      return settingsOpen
    },
    set settingsOpen(value: boolean) {
      settingsOpen = value
    },
    open,
    close,
    toggle,
    hydrate,
    setJobs,
    stepJobs,
    updateJobs,
    setAppearance,
  }
}
