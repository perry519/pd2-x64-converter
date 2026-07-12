<script lang='ts'>
  import { TableCell, TableRow } from '@/components/ui/table/index.js'

  export let columns: number
  export let height: number
</script>

{#if height > 0}
  <TableRow
    class='
      border-0
      hover:bg-transparent
    '
    aria-hidden='true'
  >
    <TableCell class='p-0' colspan={columns} style={`height: ${height}px;`} />
  </TableRow>
{/if}
