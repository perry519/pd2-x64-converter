<script lang='ts'>
  export let path: string | null | undefined
</script>

{#if path}
  <p class='truncate text-muted-foreground' title={path}>
    {path}
  </p>
{/if}
