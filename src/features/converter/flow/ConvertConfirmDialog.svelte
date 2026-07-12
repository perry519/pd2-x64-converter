<script lang='ts'>
  import type { ConverterFlowState } from './flow.svelte.ts'
  import { AlertTriangle, CheckCircle2 } from '@lucide/svelte'
  import {
    AlertDialog,
    AlertDialogAction,
    AlertDialogCancel,
    AlertDialogContent,
    AlertDialogDescription,
    AlertDialogFooter,
    AlertDialogHeader,
    AlertDialogTitle,
  } from '@/components/ui/alert-dialog/index.js'

  export let flow: ConverterFlowState

  type ConfirmView = {
    actionLabel: string
    actionVariant: 'default' | 'destructive'
    description: string
    title: string
    titleClass: string
  }

  const confirmViews: Record<'convert' | 'dryRun', ConfirmView> = {
    convert: {
      actionLabel: 'Convert',
      actionVariant: 'destructive',
      description: 'This is irreversible. Backup your mods before continuing.',
      title: 'Convert assets?',
      titleClass: 'text-(--warn)',
    },
    dryRun: {
      actionLabel: 'Run dry run',
      actionVariant: 'default',
      description:
        'Original files will not be replaced. Temporary converted outputs are discarded after validation.',
      title: 'Run dry run?',
      titleClass: 'text-(--ok)',
    },
  }

  $: confirmView = confirmViews[flow.dryRun ? 'dryRun' : 'convert']
</script>

<AlertDialog bind:open={flow.confirmOpen}>
  <AlertDialogContent class='border-border bg-card text-foreground'>
    <AlertDialogHeader class='place-items-start text-left'>
      <div
        class={`
          flex items-center gap-2.5
          ${confirmView.titleClass}
        `}
      >
        {#if flow.dryRun}
          <CheckCircle2 aria-hidden='true' size={22} />
        {:else}
          <AlertTriangle aria-hidden='true' size={22} />
        {/if}
        <AlertDialogTitle>
          {confirmView.title}
        </AlertDialogTitle>
      </div>
      <AlertDialogDescription>
        {confirmView.description}
      </AlertDialogDescription>
    </AlertDialogHeader>
    <AlertDialogFooter>
      <AlertDialogCancel>Cancel</AlertDialogCancel>
      <AlertDialogAction
        variant={confirmView.actionVariant}
        onclick={flow.runConvert}
      >
        {confirmView.actionLabel}
      </AlertDialogAction>
    </AlertDialogFooter>
  </AlertDialogContent>
</AlertDialog>
