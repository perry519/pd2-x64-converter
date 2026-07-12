<script lang='ts'>
  import type { RunManifest } from '@/platform/tauri/converterApi.ts'
  import { Card, CardContent } from '@/components/ui/card/index.js'

  export let convertedLabel: string
  export let summary: RunManifest['summary'] | undefined

  $: cards = [
    [convertedLabel, summary?.converted ?? 0],
    ['No conversion needed', summary?.already_x64 ?? 0],
    ['Warnings', summary?.warning ?? 0],
    ['Failed', summary?.failed ?? 0],
  ] as const
</script>

<div
  class='
    mb-3.5 grid grid-cols-4 gap-3
    max-[900px]:grid-cols-2
    max-[560px]:grid-cols-1
  '
>
  {#each cards as [label, value] (label)}
    <Card class='border-border bg-card' size='sm'>
      <CardContent class='grid gap-1.5'>
        <span class='text-sm text-muted-foreground'>
          {label}
        </span>
        <strong class='text-2xl'>{value}</strong>
      </CardContent>
    </Card>
  {/each}
</div>
