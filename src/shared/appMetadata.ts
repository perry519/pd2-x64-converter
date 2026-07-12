import packageJson from '../../package.json'

const stableVersionPattern = /^(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)$/
const githubRepositoryPattern
  = /^https:\/\/github\.com\/([\w.-]+)\/([\w.-]+)$/

function requireStableVersion(value: unknown) {
  if (typeof value !== 'string' || !stableVersionPattern.test(value)) {
    throw new Error('package.json version must use MAJOR.MINOR.PATCH format')
  }

  return value
}

function requireGithubRepository(value: unknown) {
  if (typeof value !== 'string') {
    throw new TypeError('package.json repository must be a GitHub repository URL')
  }

  const match = githubRepositoryPattern.exec(value)
  if (!match) {
    throw new Error('package.json repository must be an https://github.com/OWNER/REPO URL')
  }

  return {
    url: value,
    owner: match[1],
    name: match[2],
  }
}

export const appMetadata = {
  version: requireStableVersion(packageJson.version),
  repository: requireGithubRepository(packageJson.repository),
} as const

export const githubLatestReleaseApiUrl = `https://api.github.com/repos/${appMetadata.repository.owner}/${appMetadata.repository.name}/releases/latest`

export function githubReleaseUrl(tagName: string) {
  return `${appMetadata.repository.url}/releases/tag/${encodeURIComponent(tagName)}`
}
