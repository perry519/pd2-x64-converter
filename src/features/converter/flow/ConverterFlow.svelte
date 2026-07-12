<script lang='ts'>
  import type { ConverterFlowState } from './flow.svelte.ts'
  import { onMount } from 'svelte'
  import ConvertingPanel from '@/features/converter/progress/ConvertingPanel.svelte'
  import ScanningPanel from '@/features/converter/progress/ScanningPanel.svelte'
  import ResultsPanel from '@/features/converter/results/ResultsPanel.svelte'
  import ReviewPanel from '@/features/converter/review/ReviewPanel.svelte'
  import FolderPicker from '@/features/converter/select/FolderPicker.svelte'
  import ConvertConfirmDialog from './ConvertConfirmDialog.svelte'

  export let flow: ConverterFlowState

  const screenComponents = {
    converting: ConvertingPanel,
    results: ResultsPanel,
    review: ReviewPanel,
    scanning: ScanningPanel,
    select: FolderPicker,
  }

  onMount(() => flow.mount())
</script>

<svelte:component this={screenComponents[flow.screen]} {flow} />
<ConvertConfirmDialog {flow} />
