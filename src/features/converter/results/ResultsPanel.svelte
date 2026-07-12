<script lang='ts'>
  import type { ConverterFlowState } from '@/features/converter/flow/flow.svelte.ts'
  import PanelMessage from '@/features/converter/shared/PanelMessage.svelte'
  import ResultsActions from './ResultsActions.svelte'
  import ResultsHeader from './ResultsHeader.svelte'
  import ResultsTable from './ResultsTable.svelte'
  import ResultSummaryCards from './ResultSummaryCards.svelte'

  export let flow: ConverterFlowState
</script>

<section
  class='
    mx-auto box-border w-full max-w-280 min-w-0 px-7 pt-7.5 pb-23
    max-[900px]:px-4 max-[900px]:pt-5 max-[900px]:pb-21
  '
  aria-labelledby='results-title'
>
  <ResultsHeader
    failedCount={flow.failedCount}
    issueCount={flow.resultIssueCount}
    root={flow.root}
    title={flow.resultTitle}
    warningCount={flow.warningCount}
  />
  <ResultSummaryCards
    convertedLabel={flow.convertedResultLabel}
    summary={flow.summary}
  />
  <ResultsTable
    compact
    entries={flow.visibleEntries}
    dryRun={flow.resultManifest?.dry_run ?? false}
  />
  <PanelMessage message={flow.exportMessage} tone='success' />
  <PanelMessage message={flow.error} tone='error' />
  <ResultsActions
    chooseAnotherFolder={flow.chooseAnotherFolder}
    runExport={flow.runExport}
  />
</section>
