<script lang='ts'>
  import type { HTMLAttributes } from 'svelte/elements'
  import type { WithElementRef } from '@/components/utils.js'
  import { cn } from '@/components/utils.js'

  let {
    ref = $bindable(null),
    class: className,
    children,
    ...restProps
  }: WithElementRef<HTMLAttributes<HTMLDivElement>> = $props()
</script>

<div
  bind:this={ref}
  data-slot='popover-header'
  class={cn('flex flex-col gap-0.5 text-sm', className)}
  {...restProps}
>
  {@render children?.()}
</div>
