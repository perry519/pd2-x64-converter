<script lang='ts'>
  import type { ConverterFlowState } from '@/features/converter/flow/flow.svelte.ts'
  import ProgressPanel from './ProgressPanel.svelte'

  export let flow: ConverterFlowState
</script>

<ProgressPanel {flow} mode='scanning' />
