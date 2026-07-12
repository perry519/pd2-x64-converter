import type { SettingsState } from '@/features/settings/settings.svelte.ts'
import type { AssetKind, InputPath, RunManifest } from '@/platform/tauri/converterApi.ts'
import { tick } from 'svelte'
import {
  assetTypeOptionsFor,
  plannedEntriesFor,
  reviewEntriesFor,
} from '@/features/converter/model/manifestView.ts'
import { createProgressTracker } from '@/features/converter/progress/progressTracker.svelte.ts'
import {
  cancelConvert as cancelConvertCommand,
  cancelScan as cancelScanCommand,
  convertFolder,
  exportReport,
  inspectPaths,
  pickFolders,
  pickReportPath,

  scanFolder,
  subscribeDrop,
  subscribeProgress,
} from '@/platform/tauri/converterApi.ts'
import { createDropTarget } from './dropTarget.svelte.ts'
import {
  folderPrefixFor,
  inputLabelFor,
  mergeManifests,
  prefixProgressEvents,
} from './folderBatch.ts'
import { createManifestState } from './manifestState.svelte.ts'

export type ConverterFlowState = ReturnType<typeof createConverterFlow>

type Screen = 'select' | 'scanning' | 'review' | 'converting' | 'results'

export function createConverterFlow(settings: SettingsState) {
  let screen = $state<Screen>('select')
  let root = $state('')
  let inputs = $state.raw<InputPath[]>([])
  let error = $state('')
  let scanRequestId = 0
  let confirmOpen = $state(false)
  let exportMessage = $state('')
  let convertCancelRequested = $state(false)
  let dryRun = $state(false)
  let progressFolderPrefix = ''
  let progressFolderIndex = 0
  let folderManifests: RunManifest[] = []
  let excludedAssetKinds = $state.raw<AssetKind[]>([])
  const manifests = createManifestState()
  const progressTracker = createProgressTracker()
  const dropTarget = createDropTarget({
    canSelect: () => screen === 'select',
    selectPaths: paths => void selectDroppedPaths(paths),
    setError: (message) => {
      error = message
    },
  })

  const reviewEntries = $derived(reviewEntriesFor(manifests.visibleEntries, excludedAssetKinds))
  const assetTypeOptions = $derived(assetTypeOptionsFor(manifests.visibleEntries))
  const plannedCount = $derived(plannedEntriesFor(reviewEntries).length)
  const conversionEntries = $derived(plannedEntriesFor(reviewEntries))
  const canConvert = $derived(screen === 'review' && plannedCount > 0)
  const workerProgressFallback = $derived(
    `Using ${settings.jobs} worker${settings.jobs === 1 ? '' : 's'}`,
  )
  const convertProgressTitle = $derived(dryRun ? 'Running dry run' : 'Converting assets')

  function mount() {
    let unlistenDrop: (() => void) | null = null
    let unlistenProgress: (() => void) | null = null

    subscribeDrop(dropTarget.handleTauriDrop)
      .then((next) => {
        unlistenDrop = next
      })
      .catch(() => {
        unlistenDrop = null
      })

    subscribeProgress(events => progressTracker.handleEvents(
      prefixProgressEvents(
        events,
        progressFolderPrefix,
        progressFolderIndex,
        folderManifests,
        excludedAssetKinds,
      ),
    ))
      .then((next) => {
        unlistenProgress = next
      })
      .catch(() => {
        unlistenProgress = null
      })

    return () => {
      unlistenDrop?.()
      unlistenProgress?.()
      progressTracker.dispose()
    }
  }

  async function browseFolder() {
    try {
      const selected = await pickFolders()
      if (selected.length > 0)
        selectInputs(selected.map(path => ({ isFile: false, path })))
    }
 catch (caught) {
      showError(caught)
    }
  }

  async function selectDroppedPaths(paths: string[]) {
    try {
      selectInputs(await inspectPaths(paths))
    }
    catch (caught) {
      showError(caught)
    }
  }

  function selectInputs(nextInputs: InputPath[]) {
    inputs = nextInputs
    root = inputLabelFor(inputs)
    folderManifests = []
    excludedAssetKinds = []
    manifests.reset()
    exportMessage = ''
    progressTracker.reset()
    error = ''
    settings.close()
    void startScan(inputs)
  }

  async function startScan(nextInputs: InputPath[]) {
    const requestId = beginScan()
    await paintPendingState()
    if (!isCurrentScan(requestId))
return
    try {
      const scanned: RunManifest[] = []
      const paths = nextInputs.map(input => input.path)
      folderManifests = scanned
      for (const [index, input] of nextInputs.entries()) {
        if (!isCurrentScan(requestId))
return
        progressFolderIndex = index
        progressFolderPrefix = input.isFile ? '' : folderPrefixFor(input.path, paths)
        scanned.push(await scanFolder(input.path, settings.jobs))
        root = inputLabelFor(nextInputs, scanned)
      }
      finishScan(requestId, mergeManifests(scanned, root, nextInputs))
    }
 catch (caught) {
      failScan(requestId, caught)
    }
  }

  function beginScan() {
    const requestId = ++scanRequestId
    screen = 'scanning'
    progressTracker.reset()
    return requestId
  }

  function isCurrentScan(requestId: number) {
    return requestId === scanRequestId && screen === 'scanning'
  }

  function finishScan(requestId: number, manifest: RunManifest) {
    if (!isCurrentScan(requestId))
return
    manifests.scan = manifest
    screen = 'review'
  }

  function failScan(requestId: number, caught: unknown) {
    if (requestId !== scanRequestId)
return
    screen = 'select'
    showError(caught)
  }

  function cancelScan() {
    scanRequestId += 1
    void cancelScanCommand().catch(showError)
    resetSelection()
  }

  function resetSelection() {
    root = ''
    inputs = []
    folderManifests = []
    excludedAssetKinds = []
    manifests.reset()
    exportMessage = ''
    progressTracker.reset()
    error = ''
    settings.close()
    screen = 'select'
  }

  async function runConvert() {
    if (!manifests.scan || inputs.length === 0)
return
    confirmOpen = false
    settings.close()
    screen = 'converting'
    progressTracker.reset()
    progressTracker.markPreparing()
    convertCancelRequested = false
    error = ''
    await paintPendingState()
    try {
      const converted: RunManifest[] = []
      const paths = inputs.map(input => input.path)
      for (const [index, input] of inputs.entries()) {
        progressFolderIndex = index
        progressFolderPrefix = input.isFile ? '' : folderPrefixFor(input.path, paths)
        converted.push(await convertFolder(input.path, settings.jobs, dryRun, excludedAssetKinds))
        if (convertCancelRequested || converted.at(-1)?.status === 'cancelled')
          break
      }
      manifests.result = mergeManifests(converted, root, inputs)
      manifests.scan = null
      screen = 'results'
    }
 catch (caught) {
      screen = 'review'
      showError(caught)
    }
  }

  function openConvertConfirm(nextDryRun: boolean) {
    dryRun = nextDryRun
    confirmOpen = true
  }

  function cancelConvert() {
    convertCancelRequested = true
    void cancelConvertCommand().catch(showError)
  }

  async function runExport() {
    if (!manifests.result)
return
    try {
      const targetPath = await pickReportPath(manifests.result.run_id)
      if (!targetPath)
return
      manifests.result = await exportReport(manifests.result, targetPath)
      exportMessage = `Exported to ${targetPath}`
    }
 catch (caught) {
      showError(caught)
    }
  }

  function chooseAnotherFolder() {
    scanRequestId += 1
    resetSelection()
  }

  function toggleSettings() {
    settings.toggle()
    error = ''
  }

  function setAssetKindIncluded(kind: AssetKind, included: boolean) {
    excludedAssetKinds = included
      ? excludedAssetKinds.filter(excludedKind => excludedKind !== kind)
      : excludedAssetKinds.includes(kind) ? excludedAssetKinds : [...excludedAssetKinds, kind]
  }

  function includeAllAssetKinds() {
    excludedAssetKinds = []
  }

  function showError(caught: unknown) {
    error = caught instanceof Error ? caught.message : String(caught)
  }

  async function paintPendingState() {
    await tick()
    await new Promise<void>((resolve) => {
      let settled = false
      let timeout: ReturnType<typeof setTimeout>
      const finish = () => {
        if (settled) {
          return
        }
        settled = true
        clearTimeout(timeout)
        resolve()
      }
      timeout = setTimeout(finish, 50)
      requestAnimationFrame(finish)
    })
  }

  return {
    get screen() {
      return screen
    },
    get root() {
      return root
    },
    get error() {
      return error
    },
    get resultManifest() {
      return manifests.result
    },
    get confirmOpen() {
      return confirmOpen
    },
    set confirmOpen(value: boolean) {
      confirmOpen = value
    },
    get exportMessage() {
      return exportMessage
    },
    get dragActive() {
      return dropTarget.active
    },
    get progress() {
      return progressTracker.progress
    },
    get convertCancelRequested() {
      return convertCancelRequested
    },
    get dryRun() {
      return dryRun
    },
    get visibleEntries() {
      return screen === 'review' ? reviewEntries : manifests.visibleEntries
    },
    get plannedCount() {
      return plannedCount
    },
    get conversionEntries() {
      return conversionEntries
    },
    get assetTypeOptions() {
      return assetTypeOptions
    },
    get excludedAssetKinds() {
      return excludedAssetKinds
    },
    get conversionPreparing() {
      return progressTracker.preparing
    },
    get conversionProgressVersion() {
      return progressTracker.version
    },
    conversionProgressFor: progressTracker.progressFor,
    get summary() {
      return manifests.summary
    },
    get warningCount() {
      return manifests.warningCount
    },
    get failedCount() {
      return manifests.failedCount
    },
    get resultIssueCount() {
      return manifests.issueCount
    },
    get resultTitle() {
      return manifests.resultTitle
    },
    get canConvert() {
      return canConvert
    },
    get workerProgressFallback() {
      return workerProgressFallback
    },
    get convertProgressTitle() {
      return convertProgressTitle
    },
    get convertedResultLabel() {
      return manifests.convertedResultLabel
    },
    mount,
    browseFolder,
    cancelScan,
    runConvert,
    openConvertConfirm,
    cancelConvert,
    runExport,
    chooseAnotherFolder,
    setAssetKindIncluded,
    includeAllAssetKinds,
    toggleSettings,
    handleDragOver: dropTarget.handleDragOver,
    handleDragLeave: dropTarget.handleDragLeave,
    handleDrop: dropTarget.handleDrop,
  }
}
