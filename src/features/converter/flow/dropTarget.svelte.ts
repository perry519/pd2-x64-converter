import type { DragDropEvent } from '@/platform/tauri/converterApi.ts'

type FileWithPath = File & { path?: string }

const dropPathError = 'Drop files or folders from the desktop, or click to Browse.'
const activeDropTypes = new Set(['enter', 'over'])

export function createDropTarget(options: {
  canSelect: () => boolean
  selectPaths: (paths: string[]) => void
  setError: (message: string) => void
}) {
  let active = $state(false)

  function handleDragOver(event: DragEvent) {
    event.preventDefault()
    active = true
  }

  function handleDragLeave() {
    active = false
  }

  function handleDrop(event: DragEvent) {
    event.preventDefault()
    active = false
    const files = [...(event.dataTransfer?.files ?? [])] as FileWithPath[]
    selectPaths(files.map(file => file.path))
  }

  function handleTauriDrop(payload: DragDropEvent) {
    if (!options.canSelect())
return
    active = activeDropTypes.has(payload.type)
    if (payload.type === 'drop')
selectPaths(payload.paths)
  }

  function selectPaths(paths: (string | undefined)[]) {
    const selected = paths.filter((path, index) => paths.indexOf(path) === index)
    if (selected.length > 0 && selected.every(path => path && isAbsolutePath(path))) {
      options.selectPaths(selected as string[])
      return
    }
    options.setError(dropPathError)
  }

  return {
    get active() {
      return active
    },
    handleDragOver,
    handleDragLeave,
    handleDrop,
    handleTauriDrop,
  }
}

function isAbsolutePath(path: string) {
  return path.startsWith('/') || path.startsWith('\\\\') || /^[A-Z]:[\\/]/i.test(path)
}
