import { listen } from '@tauri-apps/api/event'
import { create } from 'zustand'
import { callCommand } from './client'
import type { Clip, DaemonEvent, SearchFilters } from './types'
import { DAEMON_EVENT_CHANNEL } from './types'

export type ConnectionState = 'connected' | 'disconnected'

interface ClipStoreState {
  clips: Clip[]
  connectionState: ConnectionState
  searchClips: (query: string, filters?: SearchFilters) => Promise<void>
  subscribeToEvents: () => Promise<() => void>
}

function isDaemonNotRunning(error: unknown): boolean {
  return typeof error === 'string' && error.toLowerCase().includes('daemon not running')
}

export const useClipStore = create<ClipStoreState>((set) => ({
  clips: [],
  connectionState: 'connected',

  searchClips: async (query, filters = {}) => {
    try {
      const clips = await callCommand<Clip[]>('search_clips', { query, filters, limit: 50, offset: 0 })
      set({ clips, connectionState: 'connected' })
    } catch (error) {
      if (isDaemonNotRunning(error)) {
        set({ connectionState: 'disconnected' })
      } else {
        throw error
      }
    }
  },

  subscribeToEvents: () =>
    listen<DaemonEvent>(DAEMON_EVENT_CHANNEL, async (event) => {
      const daemonEvent = event.payload
      switch (daemonEvent.type) {
        case 'ClipCaptured': {
          const newClip = await callCommand<Clip>('get_clip', { id: daemonEvent.clip_id })
          set((state) => ({ clips: [newClip, ...state.clips] }))
          break
        }
        case 'ClipUpdated': {
          const updated = await callCommand<Clip>('get_clip', { id: daemonEvent.clip_id })
          set((state) => ({ clips: state.clips.map((c) => (c.id === updated.id ? updated : c)) }))
          break
        }
        case 'ClipDeleted': {
          set((state) => ({ clips: state.clips.filter((c) => c.id !== daemonEvent.clip_id) }))
          break
        }
        default:
          break
      }
    }),
}))
