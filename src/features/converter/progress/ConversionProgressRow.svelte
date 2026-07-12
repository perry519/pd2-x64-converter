<script lang='ts'>
  import type {
    ConversionIconKind,
    ConversionProgressRow,
  } from './progressView.ts'
  import { CheckCircle2, Clock3, LoaderCircle } from '@lucide/svelte'
  import { Badge } from '@/components/ui/badge/index.js'
  import { TableCell, TableRow } from '@/components/ui/table/index.js'

  export let row: ConversionProgressRow

  const rowIcons: Record<ConversionIconKind, typeof CheckCircle2> = {
    check: CheckCircle2,
    clock: Clock3,
    loader: LoaderCircle,
  }

  const rowIconClasses: Record<ConversionIconKind, string> = {
    check: '',
    clock: '',
    loader: 'animate-spin',
  }

  $: RowIcon = rowIcons[row.iconKind]
</script>

<TableRow>
  <TableCell
    class='align-top wrap-break-word whitespace-normal'
    title={row.entry.relative_path}
  >
    <span class='block font-semibold text-foreground'>
      {row.fileName}
    </span>
    {#if row.parentPath}
      <span class='block text-xs text-muted-foreground'>
        {row.parentPath}
      </span>
    {/if}
  </TableCell>
  <TableCell class='align-top'>
    <Badge
      class={`
        h-6
        ${row.tone}
      `}
      variant='outline'
    >
      <RowIcon
        class={rowIconClasses[row.iconKind]}
        aria-hidden='true'
        size={13}
      />
      <span>{row.label}</span>
    </Badge>
  </TableCell>
  <TableCell class='truncate align-top' title={row.entry.asset_kind}>
    {row.entry.asset_kind}
  </TableCell>
</TableRow>
