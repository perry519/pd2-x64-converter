<script lang='ts'>
  import type { StatusIconKind } from '@/features/converter/model/manifestView.ts'
  import type { ManifestEntry } from '@/platform/tauri/converterApi.ts'
  import {
    AlertTriangle,
    CheckCircle2,
    Clock3,
    FileText,
    MinusCircle,
    XCircle,
  } from '@lucide/svelte'
  import { Badge } from '@/components/ui/badge/index.js'
  import { TableCell, TableRow } from '@/components/ui/table/index.js'
  import { cn } from '@/components/utils.js'
  import { displayPathFor } from '@/features/converter/flow/folderBatch.ts'
  import {
    detailFor,
    statusClasses,

    statusIconKindFor,
    statusLabelFor,
  } from '@/features/converter/model/manifestView.ts'
  import { splitPath } from '@/shared/assetPath.ts'

  export let entry: ManifestEntry
  export let dryRun = false

  const statusIcons: Record<StatusIconKind, typeof CheckCircle2> = {
    alert: AlertTriangle,
    check: CheckCircle2,
    clock: Clock3,
    file: FileText,
    minus: MinusCircle,
    x: XCircle,
  }

  $: displayPath = displayPathFor(entry)
  $: path = splitPath(displayPath)
  $: details = detailFor(entry)
  $: StatusIcon = statusIcons[statusIconKindFor(entry.status)]
</script>

<TableRow class={cn(entry.status === 'excluded' && 'opacity-50')}>
  <TableCell
    class='text-left align-top wrap-break-word whitespace-normal'
    title={displayPath}
  >
    <span class='block font-semibold text-foreground'>{path.fileName}</span>
    {#if path.parentPath}
      <span class='block text-xs text-muted-foreground'>{path.parentPath}</span>
    {/if}
  </TableCell>
  <TableCell class='truncate text-left align-top' title={entry.asset_kind}>
    {entry.asset_kind}
  </TableCell>
  <TableCell class='text-left align-top'>
    <Badge
      class={`
        h-6
        ${statusClasses[entry.status]}
      `}
      variant='outline'
    >
      <StatusIcon aria-hidden='true' size={13} />
      <span>{statusLabelFor(entry, dryRun)}</span>
    </Badge>
  </TableCell>
  <TableCell
    class='
      text-left align-top text-xs wrap-break-word whitespace-normal
      text-muted-foreground
    '
    title={details}
  >
    {details}
  </TableCell>
</TableRow>
