<script lang='ts'>
  import type { ReleaseOpener } from '../../platform/tauri/openerApi.ts'

  import type { UpdateCheckResult } from './updateCheck.ts'
  import { onMount } from 'svelte'
  import { openReleaseUrl } from '../../platform/tauri/openerApi.ts'
  import { appMetadata, githubReleaseUrl } from '../../shared/appMetadata.ts'
  import {
    checkForUpdate,

  } from './updateCheck.ts'

  interface Props {
    getUpdateStatus?: () => Promise<UpdateCheckResult>
    openRelease?: ReleaseOpener
  }

  const {
    getUpdateStatus = checkForUpdate,
    openRelease = openReleaseUrl,
  }: Props = $props()

  let updateStatus = $state<UpdateCheckResult>({ kind: 'current' })
  const currentReleaseUrl = githubReleaseUrl(`v${appMetadata.version}`)

  const releaseUrl = $derived(
    updateStatus.kind === 'available'
      ? updateStatus.releaseUrl
      : currentReleaseUrl,
  )
  const releaseVersion = $derived(
    updateStatus.kind === 'available'
      ? updateStatus.latestVersion
      : appMetadata.version,
  )

  onMount(() => {
    void (async () => {
      try {
        const status = await getUpdateStatus()
        updateStatus = status
      }
      catch {
      // Keep the current-version status when the background check fails.
      }
    })()
  })

  function openDisplayedRelease(event: MouseEvent) {
    event.preventDefault()

    void (async () => {
      try {
        await openRelease(releaseUrl)
      }
      catch {
      // A failed browser handoff must not disrupt converter use.
      }
    })()
  }
</script>

<div
  class='fixed bottom-4 left-4 z-1 text-xs text-muted-foreground'
  data-version-status
>
  {#if updateStatus.kind === 'available'}
    <span aria-live='polite' class='sr-only' role='status'>
      Version v{updateStatus.latestVersion} is available.
    </span>
  {/if}
  <a
    aria-label={`Open version v${releaseVersion} on GitHub`}
    class='cursor-pointer'
    href={releaseUrl}
    onclick={openDisplayedRelease}
  >
    {#if updateStatus.kind === 'available'}
      v{appMetadata.version} (v{updateStatus.latestVersion} available)
    {:else}
      v{appMetadata.version}
    {/if}
  </a>
</div>
