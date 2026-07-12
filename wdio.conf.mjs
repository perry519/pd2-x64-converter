import { spawn } from 'node:child_process'
import process from 'node:process'

const appBinaryPath = './target/debug/pd2-x64-converter-gui'
const appGdkBackend = process.env.PD2X64_E2E_GDK_BACKEND ?? 'x11'
const embeddedPort = Number(process.env.TAURI_WEBDRIVER_PORT ?? 4565)
const statusUrl = `http://127.0.0.1:${embeddedPort}/status`

let appProcess
let appError
let appExited
let appLog = ''

process.env.TAURI_WEBDRIVER_PORT = String(embeddedPort)

export const config = {
  runner: 'local',
  specs: ['./test/e2e/**/*.e2e.mjs'],
  maxInstances: 1,
  logLevel: 'warn',
  hostname: '127.0.0.1',
  port: embeddedPort,
  path: '/',
  protocol: 'http',
  waitforTimeout: 10_000,
  connectionRetryTimeout: 120_000,
  connectionRetryCount: 1,
  transformRequest(requestOptions) {
    requestOptions.headers.delete('Content-Length')
    return requestOptions
  },
  async onPrepare() {
    if (await isEmbeddedWebDriverReady()) {
      return
    }

    appProcess = spawn(appBinaryPath, [], {
      env: {
        ...process.env,
        GDK_BACKEND: appGdkBackend,
        TAURI_WEBDRIVER_PORT: String(embeddedPort),
        WEBKIT_DISABLE_DMABUF_RENDERER: '1',
        WDIO_EMBEDDED_SERVER: 'true',
      },
      stdio: ['ignore', 'pipe', 'pipe'],
    })
    appProcess.once('exit', (code, signal) => {
      appExited = { code, signal }
    })
    appProcess.once('error', (error) => {
      appError = error
    })
    appProcess.stdout.on('data', rememberAppLog)
    appProcess.stderr.on('data', rememberAppLog)
    try {
      await waitForEmbeddedWebDriver()
    }
    catch (error) {
      stopApp()
      throw error
    }
  },
  onComplete() {
    stopApp()
  },
  capabilities: [
    {
      browserName: 'tauri',
    },
  ],
  framework: 'mocha',
  mochaOpts: {
    ui: 'bdd',
    timeout: 120_000,
  },
  reporters: ['spec'],
}

function rememberAppLog(chunk) {
  appLog = `${appLog}${chunk}`
  appLog = appLog.slice(-4_000)
}

function stopApp() {
  if (appProcess && appProcess.exitCode === null) {
    appProcess.kill()
  }
}

async function waitForEmbeddedWebDriver() {
  const deadline = Date.now() + 120_000
  while (Date.now() < deadline) {
    throwIfAppStopped()
    if (await isEmbeddedWebDriverReady()) {
      return
    }
    await sleep(100)
  }
  throw new Error(`Timed out waiting for ${statusUrl}\n${appLog}`)
}

function throwIfAppStopped() {
  if (appError) {
    throw new Error(`Failed to start Tauri app: ${appError.message}`)
  }
  if (appExited) {
    throw new Error(`Tauri app exited before WebDriver was ready: ${JSON.stringify(appExited)}\n${appLog}`)
  }
}

async function isEmbeddedWebDriverReady() {
  try {
    const response = await fetch(statusUrl, { signal: AbortSignal.timeout(1_000) })
    return response.ok && (await response.json())?.value?.ready === true
  }
  catch {
    return false
  }
}

function sleep(milliseconds) {
  return new Promise(resolve => setTimeout(resolve, milliseconds))
}
