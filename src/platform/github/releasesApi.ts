import {
  githubLatestReleaseApiUrl,
  githubReleaseUrl,
} from '../../shared/appMetadata.ts'

export interface LatestRelease {
  tagName: string
  releaseUrl: string
}

export type ReleaseFetch = (
  input: string,
  init: RequestInit,
) => Promise<Pick<Response, 'json' | 'ok'>>

function isLatestReleasePayload(value: unknown): value is { tag_name: string } {
  return (
    typeof value === 'object'
    && value !== null
    && 'tag_name' in value
    && typeof value.tag_name === 'string'
  )
}

export async function fetchLatestRelease(
  fetchRelease: ReleaseFetch = fetch,
): Promise<LatestRelease | null> {
  try {
    const response = await fetchRelease(githubLatestReleaseApiUrl, {
      headers: { Accept: 'application/vnd.github+json' },
    })

    if (!response.ok) {
      return null
    }

    const payload = await response.json()
    if (!isLatestReleasePayload(payload)) {
      return null
    }

    return {
      tagName: payload.tag_name,
      releaseUrl: githubReleaseUrl(payload.tag_name),
    }
  }
  catch {
    return null
  }
}
