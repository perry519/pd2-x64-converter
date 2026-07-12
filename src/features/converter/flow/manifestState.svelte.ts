import type { RunManifest } from '@/platform/tauri/converterApi.ts'
import {
  activeManifestFor,
  convertedResultLabelFor,
  entriesFor,
  plannedEntriesFor,
  resultSummaryCount,
  resultTitleFor,
  summaryFor,
  visibleEntriesFor,
} from '@/features/converter/model/manifestView.ts'

export function createManifestState() {
  let scan = $state.raw<RunManifest | null>(null)
  let result = $state.raw<RunManifest | null>(null)

  const active = $derived(activeManifestFor(scan, result))
  const entries = $derived(entriesFor(active))
  const visibleEntries = $derived(visibleEntriesFor(entries, result === null))
  const plannedCount = $derived(plannedEntriesFor(entries).length)
  const conversionEntries = $derived(plannedEntriesFor(entriesFor(scan)))
  const summary = $derived(summaryFor(scan, result))
  const warningCount = $derived(resultSummaryCount(result, 'warning'))
  const failedCount = $derived(resultSummaryCount(result, 'failed'))
  const issueCount = $derived(warningCount + failedCount)
  const resultTitle = $derived(resultTitleFor(result, issueCount))
  const convertedResultLabel = $derived(convertedResultLabelFor(result))

  function reset() {
    scan = null
    result = null
  }

  return {
    get scan() {
      return scan
    },
    set scan(value: RunManifest | null) {
      scan = value
    },
    get result() {
      return result
    },
    set result(value: RunManifest | null) {
      result = value
    },
    get visibleEntries() {
      return visibleEntries
    },
    get plannedCount() {
      return plannedCount
    },
    get conversionEntries() {
      return conversionEntries
    },
    get summary() {
      return summary
    },
    get warningCount() {
      return warningCount
    },
    get failedCount() {
      return failedCount
    },
    get issueCount() {
      return issueCount
    },
    get resultTitle() {
      return resultTitle
    },
    get convertedResultLabel() {
      return convertedResultLabel
    },
    reset,
  }
}
