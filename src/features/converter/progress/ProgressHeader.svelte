<script lang='ts'>
  import { ArrowLeft } from '@lucide/svelte'
  import { Button } from '@/components/ui/button/index.js'
  import RootTitle from '@/features/converter/shared/RootTitle.svelte'

  export let isConverting: boolean
  export let root: string
  export let title: string
  export let titleId: string
  export let cancelScan: () => void
</script>

<header
  class='
    mb-4.5 flex items-start gap-3.5
    max-[900px]:flex-col max-[900px]:items-stretch
  '
>
  {#if !isConverting}
    <Button class='w-fit' variant='outline' onclick={cancelScan}>
      <ArrowLeft aria-hidden='true' />
      <span>Back</span>
    </Button>
  {/if}
  <RootTitle id={titleId} {root} {title} />
</header>
