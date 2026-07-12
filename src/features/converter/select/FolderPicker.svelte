<script lang='ts'>
  import type { ConverterFlowState } from '@/features/converter/flow/flow.svelte.ts'
  import { FolderOpen } from '@lucide/svelte'
  import { Button } from '@/components/ui/button/index.js'

  export let flow: ConverterFlowState
</script>

<section
  class='
    mx-auto box-border grid min-h-[calc(100vh-92px)] w-full max-w-280 min-w-0
    flex-1 place-content-center justify-items-center gap-0 px-7 py-7.5
    text-center
    max-[900px]:min-h-[calc(100vh-84px)] max-[900px]:px-4 max-[900px]:py-5
  '
  aria-labelledby='select-title'
>
  <Button
    class={`
      h-auto min-h-112 w-[min(860px,calc(100vw-56px))] flex-col gap-6 rounded-lg
      border-2 border-dashed p-12 text-foreground shadow-(--shadow)
      hover:border-primary hover:bg-muted hover:ring-4 hover:ring-primary/20
      max-[900px]:min-h-92 max-[900px]:w-[calc(100vw-32px)] max-[900px]:gap-5
      max-[900px]:p-8
      ${
      flow.dragActive
        ? `scale-[1.015] border-primary bg-muted ring-4 ring-primary/35`
        : `border-border bg-card`
    }
    `}
    variant='outline'
    ondragenter={flow.handleDragOver}
    ondragleave={flow.handleDragLeave}
    ondragover={flow.handleDragOver}
    onclick={flow.browseFolder}
    ondrop={flow.handleDrop}
    type='button'
  >
    <FolderOpen
      class={`
        size-40 text-primary transition-transform duration-150
        max-[900px]:size-32
        ${flow.dragActive ? `scale-110` : ``}
      `}
      aria-hidden='true'
    />
    <span class='text-[32px] leading-none font-bold' id='select-title'>
      Drop mod files or folders here
    </span>
    <span class='text-base text-muted-foreground'>or click to pick a folder</span>
  </Button>
  {#if flow.error}
    <p
      class='
        mt-3.5 rounded-lg border border-destructive bg-destructive/15 px-3
        py-2.5 text-sm text-destructive
      '
    >
      {flow.error}
    </p>
  {/if}
</section>
