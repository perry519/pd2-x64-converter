<script lang='ts'>
  import { AlertTriangle, CheckCircle2 } from '@lucide/svelte'
  import { resultMarkClass } from '@/features/converter/model/manifestView.ts'
  import RootTitle from '@/features/converter/shared/RootTitle.svelte'

  export let failedCount: number
  export let issueCount: number
  export let root: string
  export let title: string
  export let warningCount: number
</script>

<header
  class='
    mb-4.5 flex items-start gap-3.5
    max-[900px]:flex-col max-[900px]:items-stretch
  '
>
  <div class={resultMarkClass(warningCount, failedCount)}>
    {#if issueCount > 0}
      <AlertTriangle aria-hidden='true' size={24} />
    {:else}
      <CheckCircle2 aria-hidden='true' size={24} />
    {/if}
  </div>
  <RootTitle id='results-title' {root} {title} />
</header>
