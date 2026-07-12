<script lang='ts'>
  import type { AssetTypeOption } from '@/features/converter/model/manifestView.ts'
  import type { AssetKind } from '@/platform/tauri/converterApi.ts'
  import { ChevronDown, ChevronUp, Funnel } from '@lucide/svelte'
  import { Badge } from '@/components/ui/badge/index.js'
  import { Button } from '@/components/ui/button/index.js'
  import { Checkbox } from '@/components/ui/checkbox/index.js'
  import * as Field from '@/components/ui/field/index.js'
  import * as Popover from '@/components/ui/popover/index.js'
  import { Separator } from '@/components/ui/separator/index.js'

  interface Props {
    excludedKinds: AssetKind[]
    includeAll: () => void
    options: AssetTypeOption[]
    setIncluded: (kind: AssetKind, included: boolean) => void
  }

  const { excludedKinds, includeAll, options, setIncluded }: Props = $props()
  let open = $state(false)
  const id = $props.id()
  const excludedCount = $derived(excludedKinds.length)
</script>

<Popover.Root bind:open>
  <Popover.Trigger>
    {#snippet child({ props })}
      <Button {...props} variant='outline' aria-label='Choose asset types to convert'>
        <Funnel data-icon='inline-start' />
        <span>Asset types</span>
        {#if excludedCount > 0}
          <Badge variant='secondary'>{excludedCount} excluded</Badge>
        {/if}
        {#if open}
          <ChevronUp data-icon='inline-end' />
        {:else}
          <ChevronDown data-icon='inline-end' />
        {/if}
      </Button>
    {/snippet}
  </Popover.Trigger>
  <Popover.Content align='end' class='w-72'>
    <Popover.Header>
      <Popover.Title>Convert asset types</Popover.Title>
    </Popover.Header>
    <Field.Set>
      <Field.Legend class='sr-only'>Asset types</Field.Legend>
      <Field.Group data-slot='checkbox-group' class='gap-3'>
        {#each options as option (option.kind)}
          <Field.Field orientation='horizontal'>
            <Checkbox
              id={`${id}-${option.kind}`}
              checked={!excludedKinds.includes(option.kind)}
              onCheckedChange={checked => setIncluded(option.kind, checked)}
            />
            <Field.Label for={`${id}-${option.kind}`} class='font-normal'>
              {option.label}
            </Field.Label>
            <span class='ml-auto text-xs text-muted-foreground'>
              {option.count} {option.count === 1 ? 'file' : 'files'}
            </span>
          </Field.Field>
        {/each}
      </Field.Group>
    </Field.Set>
    <Separator />
    <div class='flex items-center justify-between gap-2'>
      <Button disabled={excludedCount === 0} variant='link' size='sm' onclick={includeAll}>
        Include all
      </Button>
      <Button size='sm' onclick={() => (open = false)}>Done</Button>
    </div>
  </Popover.Content>
</Popover.Root>
