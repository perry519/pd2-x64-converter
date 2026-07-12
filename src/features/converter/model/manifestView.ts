import type { AssetKind, ManifestEntry, RunManifest } from '@/platform/tauri/converterApi.ts'

type EntryStatus = ManifestEntry['status']
export type StatusIconKind = 'alert' | 'check' | 'clock' | 'file' | 'minus' | 'x'

export interface AssetTypeOption {
  count: number
  kind: AssetKind
  label: string
}

const assetKindLabels: Partial<Record<AssetKind, string>> = {
  animation: 'Animation',
  font: 'Font',
  mass_unit: 'Mass unit',
  model: 'Model',
  script_data: 'Script data',
  sound_bank: 'Sound bank',
  stream: 'Stream',
}

const statusLabels: Record<EntryStatus, string> = {
  planned: 'Planned',
  excluded: 'Excluded',
  converted: 'Converted',
  already_x64: 'Ready as-is',
  unsupported: 'Unsupported',
  warning: 'Warning',
  failed: 'Failed',
  cancelled: 'Cancelled',
}

const dryRunStatusLabels: Partial<Record<EntryStatus, string>> = {
  converted: 'Would convert',
}

const statusIconKinds: Record<EntryStatus, StatusIconKind> = {
  planned: 'clock',
  excluded: 'minus',
  converted: 'check',
  already_x64: 'file',
  unsupported: 'alert',
  warning: 'alert',
  failed: 'x',
  cancelled: 'minus',
}

const plannedStatuses: Record<EntryStatus, boolean> = {
  planned: true,
  excluded: false,
  converted: false,
  already_x64: false,
  unsupported: false,
  warning: false,
  failed: false,
  cancelled: false,
}

const visibleStatuses: Record<EntryStatus, boolean> = {
  planned: true,
  excluded: false,
  converted: true,
  already_x64: false,
  unsupported: false,
  warning: true,
  failed: true,
  cancelled: true,
}

export const statusClasses: Record<EntryStatus, string> = {
  planned: 'border-primary text-primary',
  excluded: 'border-muted-foreground text-muted-foreground',
  converted: 'border-[var(--ok)] text-[var(--ok)]',
  already_x64: 'border-[var(--ok)] text-[var(--ok)]',
  unsupported: 'border-[var(--warn)] text-[var(--warn)]',
  warning: 'border-[var(--warn)] text-[var(--warn)]',
  failed: 'border-destructive text-destructive',
  cancelled: 'border-destructive text-destructive',
}

export function isVisibleEntry(entry: ManifestEntry): boolean {
  return visibleStatuses[entry.status]
}

function isSkippedModel(entry: ManifestEntry): boolean {
  return entry.asset_kind === 'model' && entry.status === 'unsupported'
}

function isScanOnlyEntry(entry: ManifestEntry): boolean {
  return entry.status === 'already_x64' || isSkippedModel(entry)
}

export function isPlannedEntry(entry: ManifestEntry): boolean {
  return plannedStatuses[entry.status]
}

export function activeManifestFor(
  scanManifest: RunManifest | null,
  resultManifest: RunManifest | null,
): RunManifest | null {
  return resultManifest ?? scanManifest
}

export function entriesFor(manifest: RunManifest | null): ManifestEntry[] {
  return manifest?.entries ?? []
}

export function visibleEntriesFor(entries: ManifestEntry[], includeScanOnlyEntries = false): ManifestEntry[] {
  return entries.filter(entry => isVisibleEntry(entry) || (includeScanOnlyEntries && isScanOnlyEntry(entry)))
}

export function plannedEntriesFor(entries: ManifestEntry[]): ManifestEntry[] {
  return entries.filter(isPlannedEntry)
}

export function assetTypeOptionsFor(entries: ManifestEntry[]): AssetTypeOption[] {
  const counts = new Map<AssetKind, number>()
  for (const entry of entries) {
    if (assetKindLabels[entry.asset_kind] && (isVisibleEntry(entry) || isScanOnlyEntry(entry))) {
      counts.set(entry.asset_kind, (counts.get(entry.asset_kind) ?? 0) + 1)
    }
  }
  return [...counts]
    .map(([kind, count]) => ({ count, kind, label: assetKindLabels[kind] ?? kind }))
    .sort((left, right) => left.label.localeCompare(right.label))
}

export function reviewEntriesFor(
  entries: ManifestEntry[],
  excludedAssetKinds: readonly AssetKind[],
): ManifestEntry[] {
  return entries.map((entry) => {
    const canExclude = entry.status === 'planned' || entry.status === 'warning'
    return canExclude && excludedAssetKinds.includes(entry.asset_kind)
      ? { ...entry, status: 'excluded' }
      : entry
  })
}

export function reviewReadySummary(
  readyCount: number,
  reviewCount: number,
  noConversionCount: number,
): string {
  const count = readyCount === reviewCount ? `${readyCount}` : `${readyCount}/${reviewCount}`
  const noun = reviewCount === 1 ? 'file' : 'files'
  const noConversionText = noConversionCount === 1
    ? '1 file doesn\'t need conversion'
    : `${noConversionCount} files don't need conversion`
  return `${count} ${noun} ready to convert, ${noConversionText}`
}

export function summaryFor(
  scanManifest: RunManifest | null,
  resultManifest: RunManifest | null,
): RunManifest['summary'] | undefined {
  return resultManifest?.summary ?? scanManifest?.summary
}

export function resultSummaryCount(
  manifest: RunManifest | null,
  key: keyof RunManifest['summary'],
): number {
  return manifest?.summary[key] ?? 0
}

export function convertedResultLabelFor(manifest: RunManifest | null): string {
  return manifest?.dry_run ? 'Would convert' : 'Converted'
}

export function detailFor(entry: ManifestEntry): string {
  if (isSkippedModel(entry))
    return 'Unsupported model'
  if (entry.warning || entry.error)
    return entry.warning ?? entry.error ?? ''
  return ''
}

export function statusLabelFor(entry: ManifestEntry, dryRun = false): string {
  if (isSkippedModel(entry))
    return 'Skipped'
  const dryRunLabel = dryRun ? dryRunStatusLabels[entry.status] : undefined
  if (dryRunLabel)
    return dryRunLabel
  return statusLabels[entry.status]
}

export function statusIconKindFor(status: EntryStatus): StatusIconKind {
  return statusIconKinds[status]
}

export function resultTitleFor(manifest: RunManifest | null, issueCount: number): string {
  if (manifest?.dry_run) {
    return issueCount > 0 ? 'Dry run completed with issues' : 'Dry run completed'
  }
  return issueCount > 0 ? 'Completed with issues' : 'Conversion completed'
}

export function resultMarkClass(warningCount: number, failedCount: number): string {
  const tone
    = failedCount > 0
      ? 'border-destructive bg-destructive/15 text-destructive'
      : warningCount > 0
        ? 'border-[var(--warn)] bg-[var(--warn-bg)] text-[var(--warn)]'
        : 'border-[var(--ok)] bg-[var(--ok-bg)] text-[var(--ok)]'
  return `inline-flex h-10.5 w-10.5 items-center justify-center rounded-lg border ${tone}`
}
