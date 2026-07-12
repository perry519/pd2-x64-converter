import { openUrl } from '@tauri-apps/plugin-opener'

export type ReleaseOpener = (releaseUrl: string) => Promise<void>

export const openReleaseUrl: ReleaseOpener = async (releaseUrl) => {
  await openUrl(releaseUrl)
}
