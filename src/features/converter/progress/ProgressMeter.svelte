<script lang='ts'>
  import type { ProgressEvent } from '@/platform/tauri/converterApi.ts'
  import { Progress } from '@/components/ui/progress/index.js'
  import { progressPercent, progressTitle } from './progressView.ts'

  export let fallbackTitle: string
  export let progress: ProgressEvent | null
</script>

{#if progress && progress.total > 0}
  <Progress
    aria-label={progressTitle(progress, fallbackTitle)}
    class='h-2 bg-muted'
    value={progressPercent(progress)}
  />
{/if}
