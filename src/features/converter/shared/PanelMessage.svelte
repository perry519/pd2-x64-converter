<script lang='ts'>
  export let message = ''
  export let tone: 'error' | 'success'

  const classes = {
    error: `
      mt-3.5 rounded-lg border border-destructive bg-destructive/15 px-3
      py-2.5 text-sm text-destructive
    `,
    success: `
      mt-3.5 rounded-lg border border-(--ok) bg-(--ok-bg) px-3 py-2.5
      text-sm wrap-anywhere text-(--ok)
    `,
  }
</script>

{#if message}
  <p class={classes[tone]}>
    {message}
  </p>
{/if}
