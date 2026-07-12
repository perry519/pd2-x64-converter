import type { ConversionPathProgress } from './progressView.ts'
import type { ProgressEvent } from '@/platform/tauri/converterApi.ts'
import { normalizePathKey } from '@/shared/assetPath.ts'
import {
  applyProgressEvents,

  progressStateForPath,
} from './progressView.ts'

export function createProgressTracker() {
  let progress = $state<ProgressEvent | null>(null)
  let progressByPath: Record<string, ConversionPathProgress> = {}
  let version = $state(0)
  let preparing = $state(false)
  let pendingEvents: ProgressEvent[] = []
  let flushHandle: number | null = null

  function reset() {
    progress = null
    progressByPath = {}
    version = 0
    preparing = false
    pendingEvents = []
    dispose()
  }

  function markPreparing() {
    preparing = true
  }

  function handleEvents(events: ProgressEvent[]) {
    if (events.length === 0)
return
    pendingEvents.push(...events)
    if (events.some(event => event.phase === 'process')) {
      dispose()
      flushEvents()
      return
    }
    if (flushHandle !== null)
return
    flushHandle = requestAnimationFrame(flushEvents)
  }

  function flushEvents() {
    flushHandle = null
    const events = pendingEvents
    pendingEvents = []
    const result = applyProgressEvents(progressByPath, events)
    if (result.progress)
progress = result.progress
    if (result.changed)
version += 1
    if (result.conversionStarted)
preparing = false
  }

  function progressFor(relativePath: string, currentVersion: number) {
    if (currentVersion < 0)
return undefined
    return progressStateForPath(
      progressByPath[normalizePathKey(relativePath)],
      relativePath,
      progress,
    )
  }

  function dispose() {
    if (flushHandle === null)
return
    cancelAnimationFrame(flushHandle)
    flushHandle = null
  }

  return {
    get progress() {
      return progress
    },
    get version() {
      return version
    },
    get preparing() {
      return preparing
    },
    reset,
    markPreparing,
    handleEvents,
    progressFor,
    dispose,
  }
}
