import { act, render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { invoke } from '@tauri-apps/api/core'

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

let eventHandler: ((event: { payload: unknown }) => void) | undefined
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn().mockImplementation(async (_channel: string, cb: (event: { payload: unknown }) => void) => {
    eventHandler = cb
    return () => {}
  }),
}))

const hide = vi.fn().mockResolvedValue(undefined)
vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => ({ hide }),
}))

import { useClipStore } from '../../state/store'
import type { Clip } from '../../state/types'
import { Popup } from './Popup'

function clip(id: string, overrides: Partial<Clip> = {}): Clip {
  return {
    id,
    created_at: '2026-01-01T00:00:00Z',
    updated_at: '2026-01-01T00:00:00Z',
    last_used_at: null,
    source_app: null,
    source_window: null,
    primary_mime: 'text/plain',
    display_text: `clip ${id}`,
    content_hash: `hash-${id}`,
    byte_size: 0,
    is_favorite: false,
    is_pinned: false,
    is_deleted: false,
    group_id: null,
    paste_mode_default: 'auto',
    metadata: null,
    representations: [],
    ...overrides,
  }
}

beforeEach(() => {
  useClipStore.setState({ clips: [], connectionState: 'connected' })
  vi.mocked(invoke).mockReset()
  hide.mockClear()
  eventHandler = undefined
})

describe('Popup', () => {
  it('focuses the search input on open', async () => {
    vi.mocked(invoke).mockResolvedValue([])

    render(<Popup />)

    await waitFor(() => expect(screen.getByLabelText(/search/i)).toHaveFocus())
  })

  it('renders backend-provided ordering unchanged', async () => {
    const pinned = clip('pinned', { is_pinned: true })
    const unpinned1 = clip('u1')
    const unpinned2 = clip('u2')
    vi.mocked(invoke).mockResolvedValue([pinned, unpinned1, unpinned2])

    render(<Popup />)

    await waitFor(() => expect(screen.getAllByRole('option')).toHaveLength(3))
    const options = screen.getAllByRole('option')
    expect(options.map((o) => o.textContent)).toEqual(['clip pinned', 'clip u1', 'clip u2'])
  })

  it('moves selection down on ArrowDown', async () => {
    vi.mocked(invoke).mockResolvedValue([clip('c1'), clip('c2')])
    const user = userEvent.setup()

    render(<Popup />)
    await waitFor(() => expect(screen.getAllByRole('option')).toHaveLength(2))

    await user.keyboard('{ArrowDown}')

    const options = screen.getAllByRole('option')
    expect(options[1]).toHaveAttribute('aria-selected', 'true')
  })

  it('does not move selection past the top on ArrowUp', async () => {
    vi.mocked(invoke).mockResolvedValue([clip('c1'), clip('c2')])
    const user = userEvent.setup()

    render(<Popup />)
    await waitFor(() => expect(screen.getAllByRole('option')).toHaveLength(2))

    await user.keyboard('{ArrowUp}')

    const options = screen.getAllByRole('option')
    expect(options[0]).toHaveAttribute('aria-selected', 'true')
  })

  it('pastes the selected clip and closes the popup on Enter', async () => {
    vi.mocked(invoke).mockImplementation(async (command: string) => {
      if (command === 'search_clips') return [clip('c1')]
      if (command === 'paste_clip') return undefined
      return undefined
    })
    const user = userEvent.setup()

    render(<Popup />)
    await waitFor(() => expect(screen.getAllByRole('option')).toHaveLength(1))

    await user.keyboard('{Enter}')

    await waitFor(() => expect(hide).toHaveBeenCalled())
    expect(invoke).toHaveBeenCalledWith('paste_clip', { id: 'c1', mode: 'auto' })
  })

  it('re-focuses the search field and re-searches on a repeat HotkeyPressed event', async () => {
    vi.mocked(invoke).mockResolvedValue([])

    render(<Popup />)
    await waitFor(() => expect(screen.getByLabelText(/search/i)).toHaveFocus())
    await waitFor(() => expect(invoke).toHaveBeenCalledWith('search_clips', expect.objectContaining({ query: '' })))

    screen.getByLabelText(/search/i).blur()
    vi.mocked(invoke).mockClear()

    await act(async () => {
      eventHandler?.({ payload: { type: 'HotkeyPressed' } })
    })

    await waitFor(() => expect(screen.getByLabelText(/search/i)).toHaveFocus())
    expect(invoke).toHaveBeenCalledWith('search_clips', expect.objectContaining({ query: '' }))
  })

  it('issues a debounced SearchClips with the typed query', async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true })
    vi.mocked(invoke).mockResolvedValue([])
    const user = userEvent.setup({ delay: null, advanceTimers: vi.advanceTimersByTime })

    render(<Popup />)
    await act(async () => {
      await vi.runOnlyPendingTimersAsync()
    })
    vi.mocked(invoke).mockClear()

    await user.type(screen.getByLabelText(/search/i), 'ssh')
    await act(async () => {
      await vi.runOnlyPendingTimersAsync()
    })

    expect(invoke).toHaveBeenCalledWith('search_clips', expect.objectContaining({ query: 'ssh' }))
    vi.useRealTimers()
  })
})
