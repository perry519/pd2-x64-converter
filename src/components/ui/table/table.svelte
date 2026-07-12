<script lang='ts'>
  import type { HTMLTableAttributes } from 'svelte/elements'
  import type { WithElementRef } from '@/components/utils.js'
  import { cn } from '@/components/utils.js'

  let {
    ref = $bindable(null),
    class: className,
    children,
    ...restProps
  }: WithElementRef<HTMLTableAttributes> = $props()
</script>

<table
  bind:this={ref}
  data-slot='table'
  class={cn('w-full caption-bottom text-sm', className)}
  {...restProps}
>
  {@render children?.()}
</table>
