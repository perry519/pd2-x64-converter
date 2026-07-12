import type { ManifestEntry, ProgressEvent } from '@/platform/tauri/converterApi.ts'
import { displayPathFor } from '@/features/converter/flow/folderBatch.ts'
import { normalizePathKey, splitPath } from '@/shared/assetPath.ts'

export type ConversionPathProgress = Partial<Record<ProgressEvent['phase'], boolean>>
export type ConversionIconKind = 'check' | 'clock' | 'loader'
type ConversionStage = 'cleanup' | 'commit' | 'preparing' | 'process' | 'queued' | 'stage'

export interface ConversionProgressRow {
  entry: ManifestEntry
  fileName: string
  parentPath: string
  isActive: boolean
  label: string
  tone: string
  iconKind: ConversionIconKind
}

export interface ProgressBatchResult {
  progress: ProgressEvent | null
  conversionStarted: boolean
  changed: boolean
}

const progressTitles: Record<ProgressEvent['phase'], string> = {
  scan: 'Scanning folder',
  process: 'Processing file',
  stage: 'Staging conversions',
  commit: 'Committing replacements',
  cleanup: 'Cleaning up staged files',
}

const conversionStageViews: Record<
  ConversionStage,
  { iconKind: ConversionIconKind, label: string, tone: string }
> = {
  cleanup: {
    iconKind: 'check',
    label: 'Discarded',
    tone: 'border-[var(--ok)] text-[var(--ok)]',
  },
  commit: {
    iconKind: 'check',
    label: 'Converted',
    tone: 'border-[var(--ok)] text-[var(--ok)]',
  },
  preparing: {
    iconKind: 'check',
    label: 'Preparing',
    tone: 'border-border text-muted-foreground',
  },
  process: {
    iconKind: 'loader',
    label: 'Processing',
    tone: 'border-primary text-primary',
  },
  queued: {
    iconKind: 'clock',
    label: 'Queued',
    tone: 'border-border text-muted-foreground',
  },
  stage: {
    iconKind: 'check',
    label: 'Staged',
    tone: 'border-primary text-primary',
  },
}
const completedStages = ['commit', 'cleanup', 'stage'] as const

export function buildConversionRow(
  entry: ManifestEntry,
  state: ConversionPathProgress | undefined,
  isDryRun: boolean,
  isPreparing: boolean,
): ConversionProgressRow {
  const { fileName, parentPath } = splitPath(displayPathFor(entry))
  const view = conversionViewFor(conversionStageFor(state, !state && isPreparing), isDryRun)

  return {
    entry,
    fileName,
    parentPath,
    isActive: state?.process === true,
    label: view.label,
    tone: view.tone,
    iconKind: view.iconKind,
  }
}

export function progressStateForPath(
  cachedState: ConversionPathProgress | undefined,
  relativePath: string,
  progress: ProgressEvent | null,
): ConversionPathProgress | undefined {
  if (progress?.phase !== 'process' || !progress.currentPath)
    return cachedState
  if (normalizePathKey(progress.currentPath) !== normalizePathKey(relativePath)) {
    return cachedState
  }
  return { ...cachedState, process: true }
}

export function applyProgressEvents(
  progressByPath: Record<string, ConversionPathProgress>,
  events: ProgressEvent[],
): ProgressBatchResult {
  let conversionStarted = false
  let changed = false

  for (const event of events) {
    if (!event.currentPath || event.phase === 'scan')
      continue
    conversionStarted = true
    changed = true

    const pathKey = normalizePathKey(event.currentPath)
    const previousState = progressByPath[pathKey]
    progressByPath[pathKey]
      = event.phase === 'process'
        ? { ...previousState, process: true }
        : {
            ...previousState,
            process: false,
            [event.phase]: true,
          }
  }

  return {
    progress: events.at(-1) ?? null,
    conversionStarted,
    changed,
  }
}

function conversionStageFor(
  state: ConversionPathProgress | undefined,
  isPreparing: boolean,
): ConversionStage {
  if (state?.process)
    return 'process'
  const completedStage = completedStages.find(stage => state?.[stage])
  if (completedStage)
    return completedStage
  if (isPreparing)
    return 'preparing'
  return 'queued'
}

function conversionViewFor(stage: ConversionStage, isDryRun: boolean) {
  const view = conversionStageViews[stage]
  return stage === 'cleanup' && isDryRun ? { ...view, label: 'Dry-run output discarded' } : view
}

export function progressTitle(
  event: ProgressEvent | null,
  fallback: string,
): string {
  return event ? progressTitles[event.phase] : fallback
}

export function progressLine(
  event: ProgressEvent | null,
  fallback: string,
): string {
  if (!event)
    return fallback
  return [progressCount(event), event.message].filter(Boolean).join(' - ')
}

function progressCount(event: ProgressEvent): string {
  return event.total > 0 ? `${event.processed} / ${event.total}` : String(event.processed)
}

export function progressPercent(event: ProgressEvent | null): number {
  if (!event || event.total <= 0)
    return 0
  return Math.min(100, Math.max(0, Math.round((event.processed / event.total) * 100)))
}
