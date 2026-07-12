<script lang='ts'>
  import type { SettingsState } from './settings.svelte.ts'
  import { Minus, Plus, Settings } from '@lucide/svelte'
  import { Button } from '@/components/ui/button/index.js'
  import { Input } from '@/components/ui/input/index.js'
  import { Separator } from '@/components/ui/separator/index.js'
  import {
    Sheet,
    SheetContent,
    SheetHeader,
    SheetTitle,
  } from '@/components/ui/sheet/index.js'
  import { appearanceOptions } from './settings.svelte.ts'

  export let settings: SettingsState
  export let onToggle: () => void
</script>

<Sheet bind:open={settings.settingsOpen}>
  <SheetContent
    class='
      w-[calc(100vw-32px)] border-border bg-card text-foreground
      sm:max-w-107.5
    '
    side='right'
  >
    <SheetHeader class='border-b border-border'>
      <SheetTitle>Settings</SheetTitle>
    </SheetHeader>

    <div class='grid gap-4 p-4'>
      <div
        class='
          flex items-center justify-between gap-4
          max-[900px]:flex-col max-[900px]:items-stretch
        '
      >
        <div class='grid gap-1'>
          <span class='text-sm text-foreground'>Jobs</span>
          <small class='max-w-90 text-muted-foreground'>
            Capped at {settings.maxJobs} available CPU
            {settings.maxJobs === 1 ? 'thread' : 'threads'}.
          </small>
        </div>
        <div class='flex gap-1.5'>
          <Button
            aria-label='Decrease jobs'
            disabled={settings.jobs <= 1}
            size='icon'
            variant='outline'
            onclick={() => settings.stepJobs(-1)}
          >
            <Minus aria-hidden='true' />
          </Button>
          <Input
            class='
              w-18 [appearance:textfield] text-center
              [&::-webkit-inner-spin-button]:appearance-none
              [&::-webkit-outer-spin-button]:appearance-none
            '
            aria-label='Jobs'
            min='1'
            max={settings.maxJobs}
            oninput={settings.updateJobs}
            type='number'
            bind:value={settings.jobsInput}
          />
          <Button
            aria-label='Increase jobs'
            disabled={settings.jobs >= settings.maxJobs}
            size='icon'
            variant='outline'
            onclick={() => settings.stepJobs(1)}
          >
            <Plus aria-hidden='true' />
          </Button>
        </div>
      </div>

      <Separator />

      <div
        class='
          flex items-center justify-between gap-4
          max-[900px]:flex-col max-[900px]:items-stretch
        '
      >
        <span class='text-sm text-foreground'>Appearance</span>
        <div class='flex gap-1.5' role='group' aria-label='Appearance'>
          {#each appearanceOptions as option (option)}
            <Button
              aria-pressed={settings.appearance === option}
              variant={settings.appearance === option ? 'default' : 'outline'}
              onclick={() => settings.setAppearance(option)}
            >
              {option[0].toUpperCase() + option.slice(1)}
            </Button>
          {/each}
        </div>
      </div>
    </div>
  </SheetContent>
</Sheet>

<Button
  aria-expanded={settings.settingsOpen}
  aria-label='Settings'
  class='fixed top-4 right-4 z-4 shadow-(--shadow)'
  size='icon'
  title='Settings'
  variant='outline'
  onclick={onToggle}
>
  <Settings aria-hidden='true' />
</Button>
