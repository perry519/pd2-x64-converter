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
  data-slot='card-content'
  class={cn('px-4 group-data-[size=sm]/card:px-3', className)}
  {...restProps}
>
  {@render children?.()}
</div>
