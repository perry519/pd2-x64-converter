import type { LatestReleaseFetcher } from './updateCheck.ts'

import { describe, expect, it, vi } from 'vitest'
import { fetchLatestRelease } from '../../platform/github/releasesApi.ts'
import { checkForUpdate } from './updateCheck.ts'

function jsonResponse(payload: unknown, status = 200) {
  return new Response(JSON.stringify(payload), {
    status,
    headers: { 'Content-Type': 'application/json' },
  })
}

function release(tagName: string) {
  return {
    tagName,
    releaseUrl: `https://github.com/perry519/pd2-x64-converter/releases/tag/${tagName}`,
  }
}

describe('update availability', () => {
  it('reports only a strictly newer stable release', async () => {
    const newer: LatestReleaseFetcher = async () => release('v1.10.0')
    const equal: LatestReleaseFetcher = async () => release('v1.0.0')
    const older: LatestReleaseFetcher = async () => release('v0.9.9')

    await expect(checkForUpdate(newer, '1.0.0')).resolves.toEqual({
      kind: 'available',
      latestVersion: '1.10.0',
      releaseUrl:
        'https://github.com/perry519/pd2-x64-converter/releases/tag/v1.10.0',
    })
    await expect(checkForUpdate(equal, '1.0.0')).resolves.toEqual({
      kind: 'current',
    })
    await expect(checkForUpdate(older, '1.0.0')).resolves.toEqual({
      kind: 'current',
    })
  })

  it('fails closed for malformed releases and failed release lookups', async () => {
    const malformed: LatestReleaseFetcher = async () => release('v1.0.0-beta.1')
    const rejected: LatestReleaseFetcher = async () => {
      throw new Error('GitHub unavailable')
    }

    await expect(checkForUpdate(malformed, '1.0.0')).resolves.toEqual({
      kind: 'current',
    })
    await expect(checkForUpdate(rejected, '1.0.0')).resolves.toEqual({
      kind: 'current',
    })
  })
})

describe('gitHub latest-release boundary', () => {
  it('accepts the tag name from a successful latest-release response', async () => {
    const fetcher = vi.fn().mockResolvedValue(jsonResponse({ tag_name: 'v1.0.1' }))

    await expect(fetchLatestRelease(fetcher)).resolves.toEqual(release('v1.0.1'))
    expect(fetcher).toHaveBeenCalledWith(
      'https://api.github.com/repos/perry519/pd2-x64-converter/releases/latest',
      { headers: { Accept: 'application/vnd.github+json' } },
    )
  })

  it.each([
    ['a non-success response', async () => jsonResponse({}, 403)],
    ['an invalid response payload', async () => jsonResponse({ tag_name: 101 })],
    ['an invalid JSON response', async () => ({ ok: true, json: async () => { throw new Error('invalid JSON') } })],
    ['a rejected request', async () => { throw new Error('network unavailable') }],
  ])('returns no release for %s', async (_description, fetcher) => {
    await expect(fetchLatestRelease(fetcher)).resolves.toBeNull()
  })
})
