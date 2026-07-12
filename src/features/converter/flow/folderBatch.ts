import type {
  AssetKind,
  InputPath,
  ManifestEntry,
  ProgressEvent,
  RunManifest,
} from '@/platform/tauri/converterApi.ts'
import { visibleEntriesFor } from '@/features/converter/model/manifestView.ts'

const displayPath = Symbol('displayPath')

type DisplayEntry = ManifestEntry & { [displayPath]?: string }

export function folderLabelFor(paths: readonly string[]): string {
  if (paths.length === 1)
    return paths[0]

  const commonRoot = commonRootFor(paths)
  if (!commonRoot)
    return `${paths.length} folders`

  const separator = paths[0].includes('\\') ? '\\' : '/'
  return `${paths.length} under ${commonRoot.replaceAll('/', separator)}`
}

export function inputLabelFor(inputs: readonly InputPath[], manifests: readonly RunManifest[] = []): string {
  const files = inputs.filter(input => input.isFile)
  if (files.length === 0)
    return folderLabelFor(inputs.map(input => input.path))

  const filePaths: string[] = []
  for (const [index, input] of inputs.entries()) {
    const manifest = manifests[index]
    if (!manifest) {
      if (input.isFile)
        filePaths.push(input.path)
      continue
    }
    filePaths.push(...visibleEntriesFor(manifest.entries, true).map(entry => input.isFile
      ? input.path
      : joinPath(input.path, entry.relative_path)))
  }
  if (filePaths.length === 1)
    return filePaths[0]

  const commonRoot = commonRootFor(filePaths)
  if (!commonRoot)
    return `${filePaths.length} files`

  const separator = filePaths[0].includes('\\') ? '\\' : '/'
  return `${filePaths.length} files under ${commonRoot.replaceAll('/', separator)}`
}

export function displayPathFor(entry: ManifestEntry): string {
  return (entry as DisplayEntry)[displayPath] ?? entry.relative_path
}

export function folderPrefixFor(path: string, paths: readonly string[]): string {
  return paths.length > 1 ? relativeFolderPrefixFor(path, commonRootFor(paths)) : ''
}

export function prefixProgressEvents(
  events: ProgressEvent[],
  prefix: string,
  folderIndex = 0,
  manifests: RunManifest[] = [],
  excludedAssetKinds: readonly AssetKind[] = [],
): ProgressEvent[] {
  if (!prefix && manifests.length === 0)
    return events
  return events.map((event) => {
    const completedScanTotal = event.phase === 'scan' && manifests.length === folderIndex
      ? manifests.reduce((sum, manifest) => sum + manifest.entries.length, 0)
      : null
    const totals = manifests.length > 1
      ? manifests.map(manifest => event.phase === 'scan'
          ? manifest.entries.length
          : manifest.entries.filter(entry => entry.status === 'planned'
            && !excludedAssetKinds.includes(entry.asset_kind)).length)
      : []
    const processed = completedScanTotal === null
      ? totals.length > 0
        ? totals.slice(0, folderIndex).reduce((sum, count) => sum + count, 0) + event.processed
        : event.processed
      : completedScanTotal + event.processed
    const total = completedScanTotal === null
      ? totals.length > 0
        ? totals.reduce((sum, count) => sum + count, 0)
        : event.total
      : completedScanTotal + event.total

    return {
      ...event,
      processed,
      total,
      currentPath: prefix && event.currentPath
        ? `${prefix}/${event.currentPath}`
        : event.currentPath,
    }
  })
}

function commonRootFor(paths: readonly string[]): string | null {
  const normalized = paths.map(normalizeFolder)
  const parts = normalized.map(path => path.split('/'))
  const first = parts[0] ?? []
  const mismatch = first.findIndex((part, index) => parts.some(path => path[index] !== part))
  const common = first.slice(0, mismatch < 0 ? first.length : mismatch)
  const isDriveOnly = common.length === 1 && /^[A-Z]:$/i.test(common[0])
  const isUnixRootOnly = common.length === 1 && common[0] === ''
  const isIncompleteUnc = common[0] === '' && common[1] === '' && common.length < 4

  return common.length === 0 || isDriveOnly || isUnixRootOnly || isIncompleteUnc
    ? null
    : common.join('/')
}

function normalizeFolder(path: string): string {
  return path.replaceAll('\\', '/').replace(/\/+$/, '')
}

function joinPath(root: string, relativePath: string): string {
  const separator = root.includes('\\') ? '\\' : '/'
  return `${root.replace(/[\\/]+$/, '')}${separator}${relativePath.replaceAll('/', separator)}`
}

function relativeFolderPrefixFor(path: string, commonRoot: string | null): string {
  const normalized = normalizeFolder(path)
  if (commonRoot && normalized.startsWith(`${commonRoot}/`))
    return normalized.slice(commonRoot.length + 1)
  return normalized.split('/').at(-1) ?? normalized
}

export function mergeManifests(
  manifests: RunManifest[],
  root: string,
  inputs: readonly InputPath[] = manifests.map(manifest => ({ isFile: false, path: manifest.root })),
): RunManifest {
  const first = manifests[0]
  if (!first)
    throw new Error('Cannot merge an empty manifest list')

  const summary: Record<string, number> = {}
  const manifestRoots = manifests.map(manifest => manifest.root)
  const entries = manifests.flatMap((manifest, index) => {
    if (manifests.length === 1)
      return manifest.entries
    if (inputs[index]?.isFile)
      return manifest.entries.map(entry => ({ ...entry, [displayPath]: entry.relative_path }))
    const prefix = folderPrefixFor(manifest.root, manifestRoots)
    return manifest.entries.map(entry => ({
      ...entry,
      [displayPath]: `${prefix}/${entry.relative_path}`,
    }))
  })
  for (const manifest of manifests) {
    for (const [key, count] of Object.entries(manifest.summary))
      summary[key] = (summary[key] ?? 0) + count
  }

  return {
    ...first,
    root,
    status: manifests.find(manifest => manifest.status === 'cancelled')?.status
      ?? manifests.find(manifest => manifest.status === 'completed_with_failures')?.status
      ?? first.status,
    non_restorable: manifests.some(manifest => manifest.non_restorable),
    report_path: null,
    summary,
    entries: entries.map((entry, index) => ({ ...entry, index })),
  }
}
