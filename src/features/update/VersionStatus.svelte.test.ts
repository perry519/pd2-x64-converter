// @vitest-environment jsdom

import type { UpdateCheckResult } from './updateCheck.ts'

import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from '@testing-library/svelte'
import { afterEach, expect, it, vi } from 'vitest'

import { appMetadata, githubReleaseUrl } from '../../shared/appMetadata.ts'
import VersionStatus from './VersionStatus.svelte'
import '@testing-library/jest-dom/vitest'

const currentVersionLabel = `v${appMetadata.version}`
const [major, minor, patch] = appMetadata.version.split('.').map(Number)
const availableVersion = `${major}.${minor}.${patch + 1}`
const availableVersionLabel = `v${availableVersion}`
const currentReleaseUrl = githubReleaseUrl(currentVersionLabel)
const releaseUrl = githubReleaseUrl(availableVersionLabel)

afterEach(cleanup)

it('shows the current version immediately and announces a newer release', async () => {
  let resolveUpdate!: (result: UpdateCheckResult) => void
  const getUpdateStatus = vi.fn(
    () =>
      new Promise<UpdateCheckResult>((resolve) => {
        resolveUpdate = resolve
      }),
  )

  render(VersionStatus, {
    props: { getUpdateStatus, openRelease: vi.fn().mockResolvedValue(undefined) },
  })

  expect(screen.getByText(currentVersionLabel)).toBeVisible()
  await waitFor(() => expect(getUpdateStatus).toHaveBeenCalledTimes(1))
  resolveUpdate({ kind: 'available', latestVersion: availableVersion, releaseUrl })

  const releaseLink = await screen.findByRole('link', {
    name: `Open version ${availableVersionLabel} on GitHub`,
  })
  expect(releaseLink).toHaveTextContent(
    `${currentVersionLabel} (${availableVersionLabel} available)`,
  )
  expect(screen.getByRole('status')).toHaveTextContent(
    `Version ${availableVersionLabel} is available.`,
  )
})

it('links the current version when no update is available', async () => {
  const openRelease = vi.fn().mockResolvedValue(undefined)

  render(VersionStatus, {
    props: {
      getUpdateStatus: vi.fn().mockResolvedValue({ kind: 'current' }),
      openRelease,
    },
  })

  const releaseLink = await screen.findByRole('link', {
    name: `Open version ${currentVersionLabel} on GitHub`,
  })
  expect(releaseLink).toHaveAttribute('href', currentReleaseUrl)

  await fireEvent.click(releaseLink)

  expect(openRelease).toHaveBeenCalledWith(currentReleaseUrl)
})

it('opens the complete available-version link', async () => {
  const openRelease = vi.fn().mockResolvedValue(undefined)

  render(VersionStatus, {
    props: {
      getUpdateStatus: vi
        .fn()
        .mockResolvedValue({ kind: 'available', latestVersion: availableVersion, releaseUrl }),
      openRelease,
    },
  })

  const releaseLink = await screen.findByRole('link', {
    name: `Open version ${availableVersionLabel} on GitHub`,
  })
  expect(releaseLink).toHaveAttribute('href', releaseUrl)

  await fireEvent.click(releaseLink)

  expect(openRelease).toHaveBeenCalledWith(releaseUrl)
})
