import { beforeEach, describe, expect, it, vi } from 'vitest'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(),
}))

import { useClipStore } from './store'
import type { Clip, DaemonEvent } from './types'

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
  vi.mocked(listen).mockReset()
})

describe('useClipStore events', () => {
  it('appends a clip on ClipCaptured', async () => {
    let handler!: (event: { payload: DaemonEvent }) => void
    vi.mocked(listen).mockImplementation(async (_channel, cb) => {
      handler = cb as (event: { payload: DaemonEvent }) => void
      return () => {}
    })
    vi.mocked(invoke).mockResolvedValueOnce(clip('c1'))

    await useClipStore.getState().subscribeToEvents()
    await handler({ payload: { type: 'ClipCaptured', clip_id: 'c1' } })

    expect(useClipStore.getState().clips.map((c) => c.id)).toEqual(['c1'])
  })

  it('updates the matching clip on ClipUpdated', async () => {
    useClipStore.setState({ clips: [clip('c1', { is_pinned: false })] })
    let handler!: (event: { payload: DaemonEvent }) => void
    vi.mocked(listen).mockImplementation(async (_channel, cb) => {
      handler = cb as (event: { payload: DaemonEvent }) => void
      return () => {}
    })
    vi.mocked(invoke).mockResolvedValueOnce(clip('c1', { is_pinned: true }))

    await useClipStore.getState().subscribeToEvents()
    await handler({ payload: { type: 'ClipUpdated', clip_id: 'c1' } })

    expect(useClipStore.getState().clips[0].is_pinned).toBe(true)
  })

  it('removes the matching clip on ClipDeleted', async () => {
    useClipStore.setState({ clips: [clip('c1'), clip('c2')] })
    let handler!: (event: { payload: DaemonEvent }) => void
    vi.mocked(listen).mockImplementation(async (_channel, cb) => {
      handler = cb as (event: { payload: DaemonEvent }) => void
      return () => {}
    })

    await useClipStore.getState().subscribeToEvents()
    await handler({ payload: { type: 'ClipDeleted', clip_id: 'c1' } })

    expect(useClipStore.getState().clips.map((c) => c.id)).toEqual(['c2'])
  })

  it('marks the connection as disconnected when the daemon is not running, without throwing', async () => {
    vi.mocked(invoke).mockRejectedValueOnce('daemon not running')

    await expect(useClipStore.getState().searchClips('')).resolves.toBeUndefined()

    expect(useClipStore.getState().connectionState).toBe('disconnected')
  })

  it('re-throws non-daemon errors instead of silently swallowing them', async () => {
    vi.mocked(invoke).mockRejectedValueOnce('some other error')

    await expect(useClipStore.getState().searchClips('')).rejects.toBe('some other error')
  })
})
