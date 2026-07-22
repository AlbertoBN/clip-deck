import { describe, expect, it, vi } from 'vitest'
import { invoke } from '@tauri-apps/api/core'

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

import { callCommand } from './client'

describe('callCommand', () => {
  it('resolves with the payload on a successful invoke', async () => {
    vi.mocked(invoke).mockResolvedValueOnce({ id: 'c1' })

    const result = await callCommand<{ id: string }>('get_clip', { id: 'c1' })

    expect(result).toEqual({ id: 'c1' })
    expect(invoke).toHaveBeenCalledWith('get_clip', { id: 'c1' })
  })

  it('rejects with the error message when invoke rejects', async () => {
    vi.mocked(invoke).mockRejectedValueOnce('clip not found')

    await expect(callCommand('get_clip', { id: 'missing' })).rejects.toBe('clip not found')
  })
})
