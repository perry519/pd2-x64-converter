import type { DragDropEvent } from '@tauri-apps/api/webview'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import { open, save } from '@tauri-apps/plugin-dialog'

export type { DragDropEvent }

export type EntryStatus
  = | 'planned'
    | 'excluded'
    | 'converted'
    | 'already_x64'
    | 'unsupported'
    | 'warning'
    | 'failed'
    | 'cancelled'

export type AssetKind
  = | 'font'
    | 'animation'
    | 'mass_unit'
    | 'model'
    | 'stream'
    | 'sound_bank'
    | 'script_data'
    | 'texture_dependency'
    | 'unsupported_unknown'

export interface ManifestEntry {
  index: number
  relative_path: string
  asset_kind: AssetKind
  layout_state: string
  status: EntryStatus
  warning: string | null
  error: string | null
}

export interface RunManifest {
  run_id: string
  root: string
  status: string
  dry_run: boolean
  non_restorable: boolean
  destructive_write_warning: string
  report_path: string | null
  summary: Record<string, number>
  entries: ManifestEntry[]
}

export type ProgressPhase = 'scan' | 'process' | 'stage' | 'commit' | 'cleanup'

export interface ProgressEvent {
  phase: ProgressPhase
  processed: number
  total: number
  currentPath?: string | null
  message?: string | null
}

export interface InputPath {
  path: string
  isFile: boolean
}

export async function pickFolders(): Promise<string[]> {
  if (import.meta.env.VITE_TAURI_E2E === '1') {
    const injectedPath = await invoke<string | null>('e2e_pick_folder')
    if (injectedPath)
      return [injectedPath]
  }

  const selected = await open({
    directory: true,
    multiple: true,
    title: 'Select mod folders',
  })
  return Array.isArray(selected) ? selected : selected ? [selected] : []
}

export async function pickReportPath(runId: string): Promise<string | null> {
  return save({
    defaultPath: `${runId}.json`,
    filters: [{ name: 'JSON report', extensions: ['json'] }],
    title: 'Export report',
  })
}

export function scanFolder(root: string, jobs: number): Promise<RunManifest> {
  return invoke('scan_folder', { request: { root, jobs, writeReport: false } })
}

export function inspectPaths(paths: string[]): Promise<InputPath[]> {
  return invoke('inspect_paths', { paths })
}

export function convertFolder(
  root: string,
  jobs: number,
  dryRun = false,
  excludedAssetKinds: AssetKind[] = [],
): Promise<RunManifest> {
  return invoke('convert_folder', {
    request: { root, jobs, writeReport: false, dryRun, excludedAssetKinds },
  })
}

export function cancelScan(): Promise<void> {
  return invoke('cancel_scan')
}

export function cancelConvert(): Promise<void> {
  return invoke('cancel_convert')
}

export function exportReport(manifest: RunManifest, targetPath: string): Promise<RunManifest> {
  return invoke('export_report', { request: { manifest, targetPath } })
}

export async function subscribeProgress(handler: (events: ProgressEvent[]) => void): Promise<() => void> {
  return listen<ProgressEvent | ProgressEvent[]>('pd2x64-progress', (event) => {
    handler(Array.isArray(event.payload) ? event.payload : [event.payload])
  })
}

export async function subscribeDrop(handler: (event: DragDropEvent) => void): Promise<() => void> {
  return getCurrentWebview().onDragDropEvent(event => handler(event.payload))
}
