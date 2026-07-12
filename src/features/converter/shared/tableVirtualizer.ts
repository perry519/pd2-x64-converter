import { createVirtualizer } from '@tanstack/svelte-virtual'
import { get } from 'svelte/store'

const rowHeight = 64
const rowOverscan = 6

export function createTableVirtualizer() {
  const virtualizer = createVirtualizer<HTMLDivElement, HTMLTableRowElement>({
    count: 0,
    getScrollElement: () => null,
    estimateSize: () => rowHeight,
    overscan: rowOverscan,
  })

  function setOptions(node: HTMLDivElement, count: number) {
    get(virtualizer).setOptions({
      count,
      getScrollElement: () => node,
      estimateSize: () => rowHeight,
      overscan: rowOverscan,
    })
  }

  function useVirtualizer(node: HTMLDivElement, count: number) {
    let currentCount = count
    setOptions(node, count)
    return {
      update: (nextCount: number) => {
        if (nextCount === currentCount)
          return
        currentCount = nextCount
        setOptions(node, nextCount)
      },
    }
  }

  return { virtualizer, useVirtualizer }
}
