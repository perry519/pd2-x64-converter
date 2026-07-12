<script lang='ts'>
  import type { ConverterFlowState } from '@/features/converter/flow/flow.svelte.ts'
  import ConversionProgressTable from './ConversionProgressTable.svelte'
  import ProgressCancelButton from './ProgressCancelButton.svelte'
  import ProgressHeader from './ProgressHeader.svelte'
  import ProgressStatusCard from './ProgressStatusCard.svelte'

  type ProgressPanelMode = 'converting' | 'scanning'
  type ProgressPanelCopy = {
    fallbackLine: (flow: ConverterFlowState) => string
    fallbackTitle: (flow: ConverterFlowState) => string
    title: (flow: ConverterFlowState) => string
    titleId: string
  }

  export let flow: ConverterFlowState
  export let mode: ProgressPanelMode

  const progressPanelCopy: Record<ProgressPanelMode, ProgressPanelCopy> = {
    converting: {
      fallbackLine: flow => flow.workerProgressFallback,
      fallbackTitle: flow => flow.convertProgressTitle,
      title: flow => flow.convertProgressTitle,
      titleId: 'convert-title',
    },
    scanning: {
      fallbackLine: () => 'Checking candidate assets',
      fallbackTitle: () => 'Scanning folder',
      title: () => 'Scan & Review',
      titleId: 'scan-title',
    },
  }

  $: isConverting = mode === 'converting'
  $: copy = progressPanelCopy[mode]
  $: title = copy.title(flow)
  $: titleId = copy.titleId
  $: fallbackTitle = copy.fallbackTitle(flow)
  $: fallbackLine = copy.fallbackLine(flow)
</script>

<section
  class='
    mx-auto box-border w-full max-w-280 min-w-0 px-7 pt-7.5 pb-23
    max-[900px]:px-4 max-[900px]:pt-5 max-[900px]:pb-21
  '
  aria-labelledby={titleId}
>
  <ProgressHeader
    cancelScan={flow.cancelScan}
    {isConverting}
    root={flow.root}
    {title}
    {titleId}
  />
  <ProgressStatusCard
    {fallbackLine}
    {fallbackTitle}
    progress={flow.progress}
  />
  {#if isConverting}
    <ConversionProgressTable {flow} />
  {/if}
  <ProgressCancelButton
    cancelConvert={flow.cancelConvert}
    cancelScan={flow.cancelScan}
    convertCancelRequested={flow.convertCancelRequested}
    {isConverting}
  />
</section>
