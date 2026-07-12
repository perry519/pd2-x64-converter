<script lang='ts'>
  import type { ConverterFlowState } from '@/features/converter/flow/flow.svelte.ts'
  import { AlertTriangle, ArrowLeft, CheckCircle2, FolderCheck } from '@lucide/svelte'
  import { Button } from '@/components/ui/button/index.js'
  import { reviewReadySummary } from '@/features/converter/model/manifestView.ts'
  import ResultsTable from '@/features/converter/results/ResultsTable.svelte'
  import RootTitle from '@/features/converter/shared/RootTitle.svelte'
  import AssetTypeFilter from './AssetTypeFilter.svelte'

  export let flow: ConverterFlowState

  $: hasReviewableFiles = flow.visibleEntries.length > 0
  $: noConversionCount = flow.summary?.already_x64 ?? 0
</script>

<section
  class='
    mx-auto box-border w-full max-w-280 min-w-0 px-7 pt-7.5 pb-23
    max-[900px]:px-4 max-[900px]:pt-5 max-[900px]:pb-21
  '
  aria-labelledby='review-title'
>
  <header
    class='
      mb-4.5 flex items-start gap-3.5
      max-[900px]:flex-col max-[900px]:items-stretch
    '
  >
    <Button class='w-fit' variant='outline' onclick={flow.chooseAnotherFolder}>
      <ArrowLeft aria-hidden='true' />
      <span>Back</span>
    </Button>
    <RootTitle id='review-title' root={flow.root} title='Scan & Review' />
  </header>
  {#if hasReviewableFiles}
    <div class='mb-2.5 flex items-center justify-between gap-3 px-1'>
      <p class='text-sm font-medium text-foreground'>
        {reviewReadySummary(
          flow.plannedCount,
          flow.visibleEntries.length - noConversionCount,
          noConversionCount,
        )}
      </p>
      <AssetTypeFilter
        options={flow.assetTypeOptions}
        excludedKinds={flow.excludedAssetKinds}
        includeAll={flow.includeAllAssetKinds}
        setIncluded={flow.setAssetKindIncluded}
      />
    </div>
    <ResultsTable entries={flow.visibleEntries} />
  {:else}
    <section
      class='
        grid h-128 place-content-center justify-items-center gap-6 text-center
        max-[900px]:h-96
      '
      aria-label='No files to convert'
    >
      <FolderCheck
        class='
          size-40 text-primary
          max-[900px]:size-32
        '
        aria-hidden='true'
      />
      <h2 class='text-[32px] leading-none font-bold'>Nothing to convert</h2>
      <p class='text-base text-muted-foreground'>
        No supported legacy files were found in this folder.
      </p>
    </section>
  {/if}
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
  {#if hasReviewableFiles}
    <div
      class='
        mt-4 flex flex-wrap items-center justify-end gap-2.5
        max-[560px]:justify-stretch
        max-[560px]:[&_button]:w-full
      '
    >
      <Button disabled={!flow.canConvert} onclick={() => flow.openConvertConfirm(true)}>
        <CheckCircle2 aria-hidden='true' />
        <span>Dry run</span>
      </Button>
      <Button
        disabled={!flow.canConvert}
        onclick={() => flow.openConvertConfirm(false)}
        variant='destructive'
      >
        <AlertTriangle aria-hidden='true' />
        <span>Convert</span>
      </Button>
    </div>
  {/if}
</section>
