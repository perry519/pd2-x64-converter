<script lang='ts'>
  import { Badge } from '@/components/ui/badge/index.js'

  export let id: string
  export let root: string
  export let title: string
</script>

<div class='grid min-w-0 gap-2'>
  <h1 class='text-3xl leading-none' {id}>{title}</h1>
  <Badge
    class='
      w-fit max-w-full justify-start truncate border-border bg-muted
      text-muted-foreground
    '
    variant='outline'
    title={root}
  >
    <span class='truncate'>{root}</span>
  </Badge>
</div>
