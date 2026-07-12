export interface SplitAssetPath {
  fileName: string
  parentPath: string
}

export function splitPath(relativePath: string): SplitAssetPath {
  const normalized = relativePath.replace(/\\/g, '/')
  const divider = normalized.lastIndexOf('/')
  if (divider === -1)
    return { fileName: relativePath, parentPath: '' }
  return {
    fileName: normalized.slice(divider + 1) || relativePath,
    parentPath: normalized.slice(0, divider),
  }
}

export function normalizePathKey(path: string): string {
  return path.replace(/\\/g, '/').replace(/^\.?\//, '')
}
