import { render, screen, waitFor } from '@testing-library/react'
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

import { useClipStore } from '../../state/store'
import type { Clip, Group } from '../../state/types'
import { Manager } from './Manager'

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

function group(id: string, name: string): Group {
  return { id, name, parent_group_id: null, sort_order: 0 }
}

beforeEach(() => {
  useClipStore.setState({ clips: [], connectionState: 'connected' })
  eventHandler = undefined
  vi.mocked(invoke).mockReset()
  vi.mocked(invoke).mockImplementation(async (command: string) => {
    if (command === 'list_groups') return [group('work', 'Work')]
    if (command === 'search_clips') return []
    return undefined
  })
})

describe('Manager', () => {
  it('re-queries with the selected group filter', async () => {
    const user = userEvent.setup()
    render(<Manager />)
    await waitFor(() => expect(screen.getByLabelText(/group/i)).toBeInTheDocument())

    vi.mocked(invoke).mockClear()
    await user.selectOptions(screen.getByLabelText(/group/i), 'work')

    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith('search_clips', expect.objectContaining({ filters: expect.objectContaining({ group_id: 'work' }) })),
    )
  })

  it('issues PinClip when the pin action is triggered', async () => {
    vi.mocked(invoke).mockImplementation(async (command: string) => {
      if (command === 'list_groups') return []
      if (command === 'search_clips') return [clip('c1')]
      return undefined
    })
    const user = userEvent.setup()
    render(<Manager />)
    await waitFor(() => expect(screen.getByLabelText('Pin c1')).toBeInTheDocument())

    await user.click(screen.getByLabelText('Pin c1'))

    expect(invoke).toHaveBeenCalledWith('pin_clip', { id: 'c1', pinned: true })
  })

  it('removes a clip from the list once its ClipDeleted event is received', async () => {
    vi.mocked(invoke).mockImplementation(async (command: string) => {
      if (command === 'list_groups') return []
      if (command === 'search_clips') return [clip('c1')]
      return undefined
    })
    render(<Manager />)
    await waitFor(() => expect(screen.getByText('clip c1')).toBeInTheDocument())

    eventHandler?.({ payload: { type: 'ClipDeleted', clip_id: 'c1' } })

    await waitFor(() => expect(screen.queryByText('clip c1')).not.toBeInTheDocument())
  })

  it('bulk clear excluding pinned removes only unpinned clips as ClipDeleted events arrive', async () => {
    const pinned = clip('pinned', { is_pinned: true })
    const unpinned = clip('unpinned')
    vi.mocked(invoke).mockImplementation(async (command: string) => {
      if (command === 'list_groups') return []
      if (command === 'search_clips') return [pinned, unpinned]
      if (command === 'clear_history') return undefined
      return undefined
    })
    const user = userEvent.setup()
    render(<Manager />)
    await waitFor(() => expect(screen.getAllByRole('listitem')).toHaveLength(2))

    await user.click(screen.getByRole('button', { name: /clear.*excluding pinned/i }))
    expect(invoke).toHaveBeenCalledWith('clear_history', { scope: 'excluding_pinned' })

    eventHandler?.({ payload: { type: 'ClipDeleted', clip_id: 'unpinned' } })

    await waitFor(() => expect(screen.getAllByRole('listitem')).toHaveLength(1))
    expect(screen.getByText('clip pinned')).toBeInTheDocument()
  })

  it('shows a newly captured clip live without a manual refresh', async () => {
    vi.mocked(invoke).mockImplementation(async (command: string, rawArgs?: unknown) => {
      const args = rawArgs as Record<string, unknown> | undefined
      if (command === 'list_groups') return []
      if (command === 'search_clips') return []
      if (command === 'get_clip' && args?.id === 'new1') return clip('new1', { display_text: 'brand new' })
      return undefined
    })
    render(<Manager />)
    await waitFor(() => expect(screen.queryByText('brand new')).not.toBeInTheDocument())

    await eventHandler?.({ payload: { type: 'ClipCaptured', clip_id: 'new1' } })

    await waitFor(() => expect(screen.getByText('brand new')).toBeInTheDocument())
  })
})
