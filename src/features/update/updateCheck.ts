import type { LatestRelease } from '../../platform/github/releasesApi.ts'
import {
  fetchLatestRelease,

} from '../../platform/github/releasesApi.ts'
import { appMetadata } from '../../shared/appMetadata.ts'

export type StableVersion = readonly [number, number, number]

export type UpdateCheckResult
  = | { kind: 'current' }
    | {
      kind: 'available'
      latestVersion: string
      releaseUrl: string
    }

export type LatestReleaseFetcher = () => Promise<LatestRelease | null>

const stableVersionPattern
  = /^v?(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/

const currentVersion: UpdateCheckResult = { kind: 'current' }

export function parseStableVersion(value: string): StableVersion | null {
  const match = stableVersionPattern.exec(value)
  if (!match) {
    return null
  }

  return [Number(match[1]), Number(match[2]), Number(match[3])]
}

export function compareVersions(
  left: StableVersion,
  right: StableVersion,
) {
  for (const index of [0, 1, 2] as const) {
    const difference = left[index] - right[index]
    if (difference !== 0) {
      return difference
    }
  }

  return 0
}

export async function checkForUpdate(
  getLatestRelease: LatestReleaseFetcher = fetchLatestRelease,
  version = appMetadata.version,
): Promise<UpdateCheckResult> {
  const current = parseStableVersion(version)
  if (!current) {
    return currentVersion
  }

  try {
    const release = await getLatestRelease()
    const latest = release && parseStableVersion(release.tagName)
    if (!release || !latest || compareVersions(latest, current) <= 0) {
      return currentVersion
    }

    return {
      kind: 'available',
      latestVersion: release.tagName.replace(/^v/, ''),
      releaseUrl: release.releaseUrl,
    }
  }
  catch {
    return currentVersion
  }
}
