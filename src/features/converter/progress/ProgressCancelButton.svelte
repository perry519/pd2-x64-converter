<script lang='ts'>
  import { XCircle } from '@lucide/svelte'
  import { Button } from '@/components/ui/button/index.js'

  export let cancelConvert: () => void
  export let cancelScan: () => void
  export let convertCancelRequested: boolean
  export let isConverting: boolean

  $: label = isConverting && convertCancelRequested ? 'Cancelling' : 'Cancel'
  $: disabled = isConverting && convertCancelRequested
  $: click = isConverting ? cancelConvert : cancelScan
</script>

<Button {disabled} variant='outline' onclick={click}>
  <XCircle aria-hidden='true' />
  <span>{label}</span>
</Button>
