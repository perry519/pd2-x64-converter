<script lang='ts'>
  import type { HTMLAttributes } from 'svelte/elements'
  import type { WithElementRef } from '@/components/utils.js'
  import { cn } from '@/components/utils.js'

  let {
    ref = $bindable(null),
    class: className,
    variant = 'legend',
    children,
    ...restProps
  }: WithElementRef<HTMLAttributes<HTMLLegendElement>> & {
    variant?: 'legend' | 'label'
  } = $props()
</script>

<legend
  bind:this={ref}
  data-slot='field-legend'
  data-variant={variant}
  class={cn('mb-1.5 font-medium data-[variant=label]:text-sm data-[variant=legend]:text-base', className)}
  {...restProps}
>
  {@render children?.()}
</legend>
