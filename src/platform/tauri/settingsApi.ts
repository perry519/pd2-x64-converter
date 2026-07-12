import { invoke } from '@tauri-apps/api/core'

export function getMaxJobs(): Promise<number> {
  return invoke('max_jobs')
}
