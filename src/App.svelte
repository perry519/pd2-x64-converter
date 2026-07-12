<script lang='ts'>
  import { getCurrentWindow } from '@tauri-apps/api/window'
  import { tick } from 'svelte'
  import ConverterFlow from './features/converter/flow/ConverterFlow.svelte'
  import { createConverterFlow } from './features/converter/flow/flow.svelte.ts'
  import { createSettingsState } from './features/settings/settings.svelte.ts'
  import SettingsSheet from './features/settings/SettingsSheet.svelte'
  import VersionStatus from './features/update/VersionStatus.svelte'

  const settings = createSettingsState()
  const flow = createConverterFlow(settings)
  let hydrated = $state(false)

  $effect(() => {
    if (hydrated)
      localStorage.setItem('pd2-x64-converter:appearance', settings.appearance)
  })

  void settings.hydrate().then(async () => {
    hydrated = true
    await tick()
    await getCurrentWindow().show()
  })
</script>

{#if hydrated}
  <main
    class='
      box-border flex h-screen flex-col items-stretch overflow-hidden
      bg-background pb-12 [font-family:var(--font-app)] text-foreground
    '
    data-app-shell
    data-appearance={settings.appearance}
  >
    <div class='flex min-h-0 flex-1 overflow-y-auto' data-converter-scroll-region>
      <ConverterFlow {flow} />
    </div>
    <SettingsSheet {settings} onToggle={flow.toggleSettings} />
    <VersionStatus />
  </main>
{/if}
