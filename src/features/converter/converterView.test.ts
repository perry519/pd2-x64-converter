import type { ConversionPathProgress } from './progress/progressView.ts'

import type { DragDropEvent, ManifestEntry, ProgressEvent } from '@/platform/tauri/converterApi.ts'
import { expect, it } from 'vitest'
import { createDropTarget } from './flow/dropTarget.svelte.ts'
import {
  displayPathFor,
  folderLabelFor,
  folderPrefixFor,
  inputLabelFor,
  mergeManifests,
  prefixProgressEvents,
} from './flow/folderBatch.ts'
import {
  assetTypeOptionsFor,
  convertedResultLabelFor,
  detailFor,
  plannedEntriesFor,
  reviewEntriesFor,
  reviewReadySummary,
  statusLabelFor,
  visibleEntriesFor,
} from './model/manifestView.ts'
import {
  applyProgressEvents,
  buildConversionRow,
  progressStateForPath,
} from './progress/progressView.ts'

const fileInput = (path: string) => ({ isFile: true, path })
const folderInput = (path: string) => ({ isFile: false, path })

function entry(status: ManifestEntry['status'], extra: Partial<ManifestEntry> = {}): ManifestEntry {
  return {
    index: 1,
    relative_path: 'units/payday2/props/foo.unit',
    asset_kind: 'animation',
    layout_state: 'legacy',
    status,
    warning: null,
    error: null,
    ...extra,
  }
}

it('multiple folder helpers label common roots and merge manifests', () => {
  expect(folderLabelFor(['/mods/one', '/mods/two'])).toBe('2 under /mods')
  expect(folderLabelFor(['/mods/one/assets', '/mods/two/assets'])).toBe('2 under /mods')
  expect(folderLabelFor(['C:\\mods\\one', 'C:\\mods\\two'])).toBe('2 under C:\\mods')
  expect(folderLabelFor(['/one', '/two'])).toBe('2 folders')
  expect(folderLabelFor(['C:\\one', 'D:\\two'])).toBe('2 folders')

  const merged = mergeManifests([
    manifest([entry('converted')], false, '/mods/a'),
    manifest([entry('failed')], false, '/mods/b'),
  ], '2 under /mods')
  expect(merged.root).toBe('2 under /mods')
  expect(merged.entries.map(({ index }) => index)).toEqual([0, 1])
  expect(merged.entries.map(displayPathFor)).toEqual([
    'a/units/payday2/props/foo.unit',
    'b/units/payday2/props/foo.unit',
  ])
  expect(merged.entries.map(({ relative_path }) => relative_path)).toEqual([
    'units/payday2/props/foo.unit',
    'units/payday2/props/foo.unit',
  ])
  expect(merged.summary).toMatchObject({ converted: 2, failed: 0, warning: 2 })
  expect(folderPrefixFor('/mods/a', ['/mods/a', '/mods/b'])).toBe('a')
  expect(prefixProgressEvents([{
    phase: 'process',
    processed: 0,
    total: 1,
    currentPath: 'assets/asset.animation',
  }], 'a')[0].currentPath).toBe('a/assets/asset.animation')
  expect(prefixProgressEvents([{
    phase: 'scan',
    processed: 1,
    total: 1,
    currentPath: 'assets/asset.animation',
  }], 'b', 1, [
    manifest([entry('planned')], false, '/mods/a'),
    manifest([entry('planned')], false, '/mods/b'),
  ])[0]).toMatchObject({ processed: 2, total: 2 })
  expect(prefixProgressEvents([{
    phase: 'scan',
    processed: 1,
    total: 3,
    currentPath: 'assets/asset.animation',
  }], 'b', 1, [
    manifest([entry('planned')], false, '/mods/a'),
  ])[0]).toMatchObject({
    processed: 2,
    total: 4,
    currentPath: 'b/assets/asset.animation',
  })
  expect(prefixProgressEvents([{
    phase: 'scan',
    processed: 1,
    total: 1,
    currentPath: 'b.font',
  }], '', 1, [manifest([entry('planned')])])[0]).toMatchObject({ processed: 2, total: 2 })
})

it('labels and merges single-file and mixed inputs', () => {
  expect(inputLabelFor([fileInput('/mods/a.font')])).toBe('/mods/a.font')
  expect(inputLabelFor([fileInput('/mods/a.font'), fileInput('/mods/b.font')])).toBe('2 files under /mods')
  expect(inputLabelFor([fileInput('/a.font'), fileInput('/mods/b.font')])).toBe('2 files')

  const inputs = [fileInput('/mods/a.font'), folderInput('/mods/nested')]
  const manifests = [
    manifest([entry('planned', { relative_path: 'a.font' })], false, '/mods/a.font'),
    manifest([
      entry('planned', { relative_path: 'b.font' }),
      entry('unsupported', { asset_kind: 'unsupported_unknown', relative_path: 'readme.txt' }),
      entry('warning', { relative_path: 'unknown.animation' }),
    ], false, '/mods/nested'),
  ]
  expect(inputLabelFor(inputs, manifests)).toBe('3 files under /mods')
  expect(mergeManifests(manifests, '2 files', inputs).entries.map(displayPathFor)).toEqual([
    'a.font',
    'nested/b.font',
    'nested/readme.txt',
    'nested/unknown.animation',
  ])
})

it('forwards mixed paths from a desktop drop', () => {
  let selected: string[] = []
  const dropTarget = createDropTarget({
    canSelect: () => true,
    selectPaths: (paths) => {
      selected = paths
    },
    setError: () => {},
  })

  dropTarget.handleTauriDrop({
    type: 'drop',
    paths: ['/mods/one', '/mods/two.font'],
    position: { x: 0, y: 0 },
  } as DragDropEvent)

  expect(selected).toEqual(['/mods/one', '/mods/two.font'])
})

it('manifest helpers expose review and result behavior', () => {
  expect(statusLabelFor(entry('already_x64'))).toBe('Ready as-is')
  expect(detailFor(entry('warning', { asset_kind: 'animation', warning: 'needs review' }))).toBe(
    'needs review',
  )
  expect(detailFor(entry('failed', { error: 'failed' }))).toBe('failed')

  const scanManifest = manifest([
    entry('planned'),
    entry('excluded'),
    entry('already_x64'),
    entry('unsupported', { asset_kind: 'unsupported_unknown' }),
  ])
  const resultManifest = manifest([entry('converted')], true)
  expect(visibleEntriesFor(scanManifest.entries)).toHaveLength(1)
  expect(visibleEntriesFor(scanManifest.entries, true)).toEqual([
    scanManifest.entries[0],
    scanManifest.entries[2],
  ])
  expect(plannedEntriesFor(scanManifest.entries)).toHaveLength(1)
  expect(convertedResultLabelFor(resultManifest)).toBe('Would convert')
  expect(reviewReadySummary(958, 1064, 741)).toBe(
    '958/1064 files ready to convert, 741 files don\'t need conversion',
  )
  expect(reviewReadySummary(0, 1, 1)).toBe('0/1 file ready to convert, 1 file doesn\'t need conversion')
})

it('asset type exclusions replace review status and remove planned work', () => {
  const entries = [
    entry('planned', { asset_kind: 'font' }),
    entry('warning', { asset_kind: 'font' }),
    entry('already_x64', { asset_kind: 'font' }),
    entry('planned', { asset_kind: 'animation' }),
  ]

  expect(assetTypeOptionsFor(entries)).toEqual([
    { count: 1, kind: 'animation', label: 'Animation' },
    { count: 3, kind: 'font', label: 'Font' },
  ])

  const reviewEntries = reviewEntriesFor(entries, ['font'])
  expect(reviewEntries.map(({ status }) => status)).toEqual([
    'excluded',
    'excluded',
    'already_x64',
    'planned',
  ])
  expect(plannedEntriesFor(reviewEntries)).toHaveLength(1)
})

function manifest(entries: ManifestEntry[], dryRun = false, root = '/tmp/mod') {
  return {
    run_id: 'run',
    root,
    status: 'ok',
    dry_run: dryRun,
    non_restorable: false,
    destructive_write_warning: '',
    report_path: null,
    summary: {
      already_x64: 1,
      converted: 1,
      failed: 0,
      warning: 1,
    },
    entries,
  }
}

const progressPath = 'units/payday2/props/foo.unit'
function rowFor(state: ConversionPathProgress | undefined, isDryRun = false, isPreparing = false) {
  return buildConversionRow(entry('planned'), state, isDryRun, isPreparing)
}

it('progress batch reducer mutates cache and keeps latest per path state', () => {
  const progressByPath: Record<string, ConversionPathProgress> = {
    'units/old.unit': { commit: true },
  }
  const events: ProgressEvent[] = [
    {
      phase: 'process',
      processed: 0,
      total: 2,
      currentPath: 'units\\payday2\\props\\foo.unit',
    },
    {
      phase: 'stage',
      processed: 1,
      total: 2,
      currentPath: 'units/payday2/props/foo.unit',
    },
    {
      phase: 'process',
      processed: 1,
      total: 2,
      currentPath: 'units/payday2/props/bar.unit',
    },
  ]

  const result = applyProgressEvents(progressByPath, events)

  expect(result.progress).toBe(events[2])
  expect(result.conversionStarted).toBe(true)
  expect(result.changed).toBe(true)
  expect(progressByPath[progressPath]).toEqual({ process: false, stage: true })
  expect(progressByPath['units/payday2/props/bar.unit']).toEqual({ process: true })
  expect(progressByPath['units/old.unit']).toEqual({ commit: true })

  const scanOnly = applyProgressEvents(progressByPath, [
    { phase: 'scan', processed: 1, total: 2, currentPath: 'ignored.unit' },
  ])
  expect(scanOnly.conversionStarted).toBe(false)
  expect(scanOnly.changed).toBe(false)
})

it('current process event keeps the matching row visibly active', () => {
  const cachedState: ConversionPathProgress = { stage: true, process: false }
  const progress: ProgressEvent = {
    phase: 'process',
    processed: 2,
    total: 10,
    currentPath: 'units\\payday2\\props\\foo.unit',
  }

  const activeState = progressStateForPath(cachedState, progressPath, progress)

  expect(activeState).toEqual({ stage: true, process: true })
  expect(rowFor(activeState)).toMatchObject({
    isActive: true,
    label: 'Processing',
    iconKind: 'loader',
  })
  expect(progressStateForPath(cachedState, 'units/payday2/props/bar.unit', progress)).toBe(
    cachedState,
  )
})
