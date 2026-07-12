<script lang='ts'>
  import type { ConverterFlowState } from '@/features/converter/flow/flow.svelte.ts'
  import { Card, CardContent } from '@/components/ui/card/index.js'
  import {
    Table,
    TableBody,
    TableCell,
    TableHead,
    TableHeader,
    TableRow,
  } from '@/components/ui/table/index.js'
  import { displayPathFor } from '@/features/converter/flow/folderBatch.ts'
  import { createTableVirtualizer } from '@/features/converter/shared/tableVirtualizer.ts'
  import VirtualSpacerRow from '@/features/converter/shared/VirtualSpacerRow.svelte'
  import ConversionProgressRow from './ConversionProgressRow.svelte'
  import { buildConversionRow } from './progressView.ts'

  export let flow: ConverterFlowState

  const { virtualizer, useVirtualizer } = createTableVirtualizer()

  $: virtualRows = $virtualizer.getVirtualItems()
  $: topSpacerHeight = virtualRows[0]?.start ?? 0
  $: bottomSpacerHeight
    = virtualRows.length > 0
      ? Math.max(0, $virtualizer.getTotalSize() - (virtualRows.at(-1)?.end ?? 0))
      : 0
</script>

<Card class='mb-4 border-border bg-card py-0'>
  <CardContent class='px-0'>
    <div
      class='
        h-96 overflow-auto
        max-[900px]:h-64
      '
      use:useVirtualizer={flow.conversionEntries.length}
    >
      <Table class='min-w-160 table-fixed'>
        <colgroup>
          <col class='w-auto' />
          <col class='w-52.5' />
          <col class='w-30' />
        </colgroup>
        <TableHeader>
          <TableRow class='hover:bg-transparent'>
            <TableHead class='sticky top-0 z-1 bg-card text-muted-foreground'>
              File
            </TableHead>
            <TableHead class='sticky top-0 z-1 bg-card text-muted-foreground'>
              State
            </TableHead>
            <TableHead class='sticky top-0 z-1 bg-card text-muted-foreground'>
              Type
            </TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {#if flow.conversionEntries.length > 0}
            <VirtualSpacerRow columns={3} height={topSpacerHeight} />
            {#each virtualRows as virtualRow (virtualRow.index)}
              {@const entry = flow.conversionEntries[virtualRow.index]}
              <ConversionProgressRow
                row={buildConversionRow(
                  entry,
                  flow.conversionProgressFor(
                    displayPathFor(entry),
                    flow.conversionProgressVersion,
                  ),
                  flow.dryRun,
                  flow.conversionPreparing,
                )}
              />
            {/each}
            <VirtualSpacerRow columns={3} height={bottomSpacerHeight} />
          {:else}
            <TableRow>
              <TableCell class='text-center text-muted-foreground' colspan={3}>
                No planned conversions.
              </TableCell>
            </TableRow>
          {/if}
        </TableBody>
      </Table>
    </div>
  </CardContent>
</Card>
