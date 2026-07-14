import { spawnSync } from 'node:child_process'
import process, { platform } from 'node:process'

const args = process.argv.slice(2)
const webdriverPort = process.env.TAURI_WEBDRIVER_PORT ?? '4565'
if (args[0] === 'dev' && process.env.WDIO_EMBEDDED_SERVER === 'true' && await isWebDriverReady()) {
  console.log(`Tauri WebDriver app already running on port ${webdriverPort}.`)
  process.exit(0)
}

const wantsAppImage = args[0] === 'build' && args.includes('appimage')
const wantsNativeBuild = args[0] === 'build'
  && !args.some(arg => arg === '--no-bundle' || /^(?:-b|--bundles)(?:=|$)/.test(arg))
const wantsWindowsBundle
  = args[0] === 'build' && args.some(arg => /\b(?:nsis|msi)\b/i.test(arg))

if (wantsWindowsBundle && platform !== 'win32') {
  console.error('Windows bundles must be built on Windows.')
  console.error('Run this in PowerShell on Windows: pnpm run build:gui:windows:installer')
  process.exit(1)
}

const command = wantsAppImage ? 'bash' : 'pnpm'
const commandArgs = wantsAppImage
  ? ['scripts/build-appimage.sh']
  : ['exec', 'tauri', ...args, ...(wantsNativeBuild ? ['--no-bundle'] : [])]

const result = spawnSync(command, commandArgs, {
  shell: platform === 'win32',
  stdio: 'inherit',
})

process.exit(result.status ?? 1)

async function isWebDriverReady() {
  try {
    const response = await fetch(`http://127.0.0.1:${webdriverPort}/status`, {
      signal: AbortSignal.timeout(500),
    })
    return response.ok && (await response.json())?.value?.ready === true
  }
  catch {
    return false
  }
}
