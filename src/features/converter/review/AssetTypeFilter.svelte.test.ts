// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen } from '@testing-library/svelte'
import { afterEach, expect, it, vi } from 'vitest'
import AssetTypeFilter from './AssetTypeFilter.svelte'
import '@testing-library/jest-dom/vitest'

afterEach(cleanup)

it('shows excluded counts and reports checkbox changes', async () => {
  const includeAll = vi.fn()
  const setIncluded = vi.fn()
  render(AssetTypeFilter, {
    props: {
      excludedKinds: ['model'],
      includeAll,
      options: [{ count: 2, kind: 'model', label: 'Model' }],
      setIncluded,
    },
  })

  await fireEvent.click(screen.getByRole('button', { name: 'Choose asset types to convert' }))

  expect(screen.getByText('1 excluded')).toBeVisible()
  const checkbox = screen.getByRole('checkbox', { name: 'Model' })
  expect(checkbox).not.toBeChecked()

  await fireEvent.click(checkbox)
  expect(setIncluded).toHaveBeenCalledWith('model', true)

  await fireEvent.click(screen.getByRole('button', { name: 'Include all' }))
  expect(includeAll).toHaveBeenCalledTimes(1)
})
