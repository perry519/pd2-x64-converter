<script lang='ts'>
  import type { ProgressEvent } from '@/platform/tauri/converterApi.ts'
  import { LoaderCircle } from '@lucide/svelte'
  import { Card, CardContent } from '@/components/ui/card/index.js'
  import ProgressMeter from './ProgressMeter.svelte'
  import ProgressPath from './ProgressPath.svelte'
  import { progressLine, progressTitle } from './progressView.ts'

  export let fallbackLine: string
  export let fallbackTitle: string
  export let progress: ProgressEvent | null
</script>

<Card class='mb-4 border-border bg-card' aria-live='polite'>
  <CardContent class='flex items-start gap-4'>
    <LoaderCircle
      class='mt-0.5 shrink-0 animate-spin text-primary'
      aria-hidden='true'
      size={30}
    />
    <div class='grid w-full min-w-0 gap-1.5'>
      <h2 class='text-lg font-semibold'>
        {progressTitle(progress, fallbackTitle)}
      </h2>
      <p class='text-muted-foreground'>
        {progressLine(progress, fallbackLine)}
      </p>
      <ProgressPath path={progress?.currentPath} />
      <ProgressMeter {fallbackTitle} {progress} />
    </div>
  </CardContent>
</Card>
