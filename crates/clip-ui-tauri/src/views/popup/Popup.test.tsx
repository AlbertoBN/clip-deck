import { act, fireEvent, render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { invoke } from '@tauri-apps/api/core'

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
  convertFileSrc: vi.fn((path: string) => `asset://${path}`),
}))

let eventHandler: ((event: { payload: unknown }) => void) | undefined
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn().mockImplementation(async (_channel: string, cb: (event: { payload: unknown }) => void) => {
    eventHandler = cb
    return () => {}
  }),
}))

const hide = vi.fn().mockResolvedValue(undefined)
const startResizeDragging = vi.fn().mockResolvedValue(undefined)
vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => ({ hide, startResizeDragging }),
}))

import { listen } from '@tauri-apps/api/event'
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
  startResizeDragging.mockClear()
  eventHandler = undefined
})

describe('Popup', () => {
  it('renders a drag gripper with a vertical ClipDeck label', async () => {
    vi.mocked(invoke).mockResolvedValue([])

    render(<Popup />)

    const label = screen.getByText('ClipDeck')
    expect(label.closest('[data-tauri-drag-region]')).not.toBeNull()
  })

  it('closes the popup when the gripper close button is clicked', async () => {
    vi.mocked(invoke).mockResolvedValue([])
    const user = userEvent.setup()

    render(<Popup />)

    await user.click(screen.getByRole('button', { name: /close/i }))

    await waitFor(() => expect(hide).toHaveBeenCalled())
  })

  it('starts a native resize drag when an edge handle is pressed', async () => {
    vi.mocked(invoke).mockResolvedValue([])

    render(<Popup />)

    fireEvent.mouseDown(screen.getByTestId('resize-handle-south'))
    fireEvent.mouseDown(screen.getByTestId('resize-handle-east'))
    fireEvent.mouseDown(screen.getByTestId('resize-handle-northwest'))

    expect(startResizeDragging).toHaveBeenCalledWith('South')
    expect(startResizeDragging).toHaveBeenCalledWith('East')
    expect(startResizeDragging).toHaveBeenCalledWith('NorthWest')
  })

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

  it('closes the popup on Escape without pasting', async () => {
    vi.mocked(invoke).mockImplementation(async (command: string) => {
      if (command === 'search_clips') return [clip('c1')]
      return undefined
    })
    const user = userEvent.setup()

    render(<Popup />)
    await waitFor(() => expect(screen.getAllByRole('option')).toHaveLength(1))

    await user.keyboard('{Escape}')

    await waitFor(() => expect(hide).toHaveBeenCalled())
    expect(invoke).not.toHaveBeenCalledWith('paste_clip', expect.anything())
  })

  it('closes the popup on Escape even when focus has left the search input', async () => {
    vi.mocked(invoke).mockResolvedValue([])
    const user = userEvent.setup()

    render(<Popup />)
    await waitFor(() => expect(screen.getByLabelText(/search/i)).toHaveFocus())
    screen.getByLabelText(/search/i).blur()

    await user.keyboard('{Escape}')

    await waitFor(() => expect(hide).toHaveBeenCalled())
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

  it('unsubscribes from the daemon event stream even if unmounted before the listener finishes registering', async () => {
    // Reproduces a React StrictMode race: `subscribeToEvents()` (like
    // `listen()`) resolves asynchronously, and the effect only learns its
    // own `unlisten` function inside that `.then()`. StrictMode's dev-mode
    // mount -> cleanup -> mount double-invoke runs the cleanup function
    // *before* that promise has a chance to resolve, so if the effect
    // doesn't guard against this, the listener from the first mount is
    // never torn down - leaking a live subscription that goes on to
    // double-process every daemon event for the component's whole lifetime
    // (e.g. a single ClipCaptured event prepending the same clip twice).
    let resolveListen!: (unlisten: () => void) => void
    vi.mocked(invoke).mockResolvedValue([])
    const originalListenImpl = vi.mocked(listen).getMockImplementation()
    vi.mocked(listen).mockImplementationOnce(
      () =>
        new Promise((resolve) => {
          resolveListen = resolve as unknown as (unlisten: () => void) => void
        }),
    )

    const { unmount } = render(<Popup />)
    unmount()

    const leakedUnlisten = vi.fn()
    resolveListen(leakedUnlisten)
    await Promise.resolve()
    await Promise.resolve()

    expect(leakedUnlisten).toHaveBeenCalled()

    if (originalListenImpl) vi.mocked(listen).mockImplementation(originalListenImpl)
  })

  it('adds a newly captured clip live, without waiting for the next HotkeyPressed', async () => {
    vi.mocked(invoke).mockImplementation(async (command: string) => {
      if (command === 'search_clips') return [clip('c1')]
      if (command === 'get_clip') return clip('new1')
      return undefined
    })

    render(<Popup />)
    await waitFor(() => expect(screen.getAllByRole('option')).toHaveLength(1))

    await act(async () => {
      await eventHandler?.({ payload: { type: 'ClipCaptured', clip_id: 'new1' } })
    })

    await waitFor(() => expect(screen.getAllByRole('option')).toHaveLength(2))
  })

  it('does not let the mount-time debounced search clobber an arrow-key press made shortly after opening', async () => {
    // The debounce-search effect runs once after every mount regardless of
    // its dependency array (React always runs a fresh effect after the
    // first render), scheduling a 200ms-later re-search independent of the
    // synchronous mount-time search `activate()` already issued. If a user
    // presses an arrow key inside that 200ms window, that leftover timeout
    // firing later must not reset their selection back to the top.
    vi.useFakeTimers({ shouldAdvanceTime: true })
    vi.mocked(invoke).mockResolvedValue([clip('c1'), clip('c2')])

    render(<Popup />)
    await act(async () => {
      await Promise.resolve()
      await Promise.resolve()
    })
    expect(screen.getAllByRole('option')).toHaveLength(2)

    fireEvent.keyDown(screen.getByLabelText(/search/i), { key: 'ArrowDown' })
    expect(screen.getAllByRole('option')[1]).toHaveAttribute('aria-selected', 'true')

    await act(async () => {
      await vi.runOnlyPendingTimersAsync()
    })

    expect(screen.getAllByRole('option')[1]).toHaveAttribute('aria-selected', 'true')

    vi.useRealTimers()
  })

  it('keeps the current arrow-key selection when a clip is captured live, instead of jumping back to the top', async () => {
    vi.mocked(invoke).mockImplementation(async (command: string) => {
      if (command === 'search_clips') return [clip('c1'), clip('c2')]
      if (command === 'get_clip') return clip('new1')
      return undefined
    })
    const user = userEvent.setup()

    render(<Popup />)
    await waitFor(() => expect(screen.getAllByRole('option')).toHaveLength(2))
    await user.keyboard('{ArrowDown}')
    expect(screen.getAllByRole('option')[1]).toHaveAttribute('aria-selected', 'true')

    await act(async () => {
      await eventHandler?.({ payload: { type: 'ClipCaptured', clip_id: 'new1' } })
    })
    await waitFor(() => expect(screen.getAllByRole('option')).toHaveLength(3))

    expect(screen.getAllByRole('option')[0]).toHaveAttribute('aria-selected', 'false')
  })

  it('shows a thumbnail for an image clip instead of a blank row', async () => {
    const imageClip = clip('img1', {
      display_text: null,
      primary_mime: 'image/png',
      representations: [
        {
          mime_type: 'image/png',
          text_value: null,
          blob_path: '/blobs/full.png',
          preview_text: null,
          width: 800,
          height: 600,
          byte_size: 12345,
          ordinal: 0,
          is_preview: false,
        },
        {
          mime_type: 'image/png',
          text_value: null,
          blob_path: '/blobs/thumb.png',
          preview_text: null,
          width: 200,
          height: 150,
          byte_size: 456,
          ordinal: 1,
          is_preview: true,
        },
      ],
    })
    vi.mocked(invoke).mockResolvedValue([imageClip])

    render(<Popup />)

    const img = await screen.findByRole('img')
    expect(img).toHaveAttribute('src', 'asset:///blobs/thumb.png')
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
