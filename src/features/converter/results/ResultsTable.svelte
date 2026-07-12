<script lang='ts'>
  import type { ManifestEntry } from '@/platform/tauri/converterApi.ts'
  import { Table, TableBody, TableHead, TableHeader, TableRow } from '@/components/ui/table/index.js'
  import { cn } from '@/components/utils.js'
  import { createTableVirtualizer } from '@/features/converter/shared/tableVirtualizer.ts'
  import VirtualSpacerRow from '@/features/converter/shared/VirtualSpacerRow.svelte'
  import ResultsTableRow from './ResultsTableRow.svelte'

  export let entries: ManifestEntry[] = []
  export let dryRun = false
  export let compact = false

  const { virtualizer, useVirtualizer } = createTableVirtualizer()

  $: virtualRows = $virtualizer.getVirtualItems()
  $: topSpacerHeight = virtualRows[0]?.start ?? 0
  $: bottomSpacerHeight
    = virtualRows.length > 0
      ? Math.max(0, $virtualizer.getTotalSize() - (virtualRows.at(-1)?.end ?? 0))
      : 0
</script>

<div
  class={cn(
    'overflow-auto rounded-lg border border-border',
    compact
      ? `
        h-112
        max-[900px]:h-80
      `
      : `
        h-128
        max-[900px]:h-96
      `,
  )}
  use:useVirtualizer={entries.length}
>
  <Table class='min-w-220 table-fixed'>
    <colgroup>
      <col class='w-auto' />
      <col class='w-35' />
      <col class='w-40' />
      <col class='w-70' />
    </colgroup>
    <TableHeader>
      <TableRow class='hover:bg-transparent'>
        <TableHead class='sticky top-0 z-1 bg-card text-muted-foreground'>
          File
        </TableHead>
        <TableHead class='sticky top-0 z-1 bg-card text-muted-foreground'>
          Type
        </TableHead>
        <TableHead class='sticky top-0 z-1 bg-card text-muted-foreground'>
          Status
        </TableHead>
        <TableHead class='sticky top-0 z-1 bg-card text-muted-foreground'>
          Details
        </TableHead>
      </TableRow>
    </TableHeader>
    <TableBody>
      <VirtualSpacerRow columns={4} height={topSpacerHeight} />
      {#each virtualRows as virtualRow (virtualRow.index)}
        <ResultsTableRow entry={entries[virtualRow.index]} {dryRun} />
      {/each}
      <VirtualSpacerRow columns={4} height={bottomSpacerHeight} />
    </TableBody>
  </Table>
</div>
