import { render, screen, waitFor } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { invoke } from '@tauri-apps/api/core'

vi.mock('@tauri-apps/api/core', async () => {
  const actual = await vi.importActual<typeof import('@tauri-apps/api/core')>('@tauri-apps/api/core')
  return {
    ...actual,
    invoke: vi.fn(),
    convertFileSrc: vi.fn((path: string) => `asset://${path}`),
  }
})

import type { ClipRepresentation } from '../state/types'
import { Preview } from './Preview'

function representation(overrides: Partial<ClipRepresentation> = {}): ClipRepresentation {
  return {
    mime_type: 'text/plain',
    text_value: null,
    blob_path: null,
    preview_text: null,
    width: null,
    height: null,
    byte_size: 0,
    ordinal: 0,
    is_preview: false,
    ...overrides,
  }
}

beforeEach(() => {
  vi.mocked(invoke).mockReset()
})

describe('Preview', () => {
  it('shows the full untruncated plain-text content', () => {
    const longText = 'a'.repeat(500)
    render(<Preview representation={representation({ mime_type: 'text/plain', text_value: longText })} />)

    expect(screen.getByText(longText)).toBeInTheDocument()
  })

  it('renders a sanitized HTML representation', async () => {
    vi.mocked(invoke).mockResolvedValue('<b>safe</b>')

    render(<Preview representation={representation({ mime_type: 'text/html', text_value: '<script>bad</script><b>safe</b>' })} />)

    await waitFor(() => expect(screen.getByText('safe')).toBeInTheDocument())
    expect(invoke).toHaveBeenCalledWith('sanitize_clip_html', { html: '<script>bad</script><b>safe</b>' })
  })

  it('renders an image from its blob path', () => {
    render(<Preview representation={representation({ mime_type: 'image/png', blob_path: '/tmp/clip.png' })} />)

    const image = screen.getByRole('img')
    expect(image).toHaveAttribute('src', 'asset:///tmp/clip.png')
  })
})
