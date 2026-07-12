<script lang='ts'>
  import { Download, RefreshCw } from '@lucide/svelte'
  import { Button } from '@/components/ui/button/index.js'

  export let chooseAnotherFolder: () => void
  export let runExport: () => void
</script>

<div
  class='
    mt-4 flex flex-wrap justify-end gap-2.5
    max-[560px]:justify-stretch
    max-[560px]:[&_button]:w-full
  '
>
  <Button onclick={runExport}>
    <Download aria-hidden='true' />
    <span>Export report</span>
  </Button>
  <Button variant='outline' onclick={chooseAnotherFolder}>
    <RefreshCw aria-hidden='true' />
    <span>Choose another folder</span>
  </Button>
</div>
