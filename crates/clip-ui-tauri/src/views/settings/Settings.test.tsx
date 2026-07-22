import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { invoke } from '@tauri-apps/api/core'

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

import type { AppSettings, DiagnosticsReport } from '../../state/types'
import { Settings } from './Settings'

function settings(overrides: Partial<AppSettings> = {}): AppSettings {
  return {
    hotkey_binding: 'Ctrl+Shift+V',
    retention_window_days: null,
    capture_paused: false,
    default_paste_mode: 'auto',
    ...overrides,
  }
}

function diagnostics(overrides: Partial<DiagnosticsReport['capabilities']> = {}): DiagnosticsReport {
  return {
    backend: 'x11',
    capabilities: { capture: true, paste_simulation: true, hotkeys: true, focus_detection: true, ...overrides },
  }
}

beforeEach(() => {
  vi.mocked(invoke).mockReset()
  vi.mocked(invoke).mockImplementation(async (command: string) => {
    if (command === 'get_settings') return settings()
    if (command === 'get_diagnostics') return diagnostics()
    return undefined
  })
})

describe('Settings', () => {
  it('saves a valid hotkey binding via UpdateSettings', async () => {
    vi.mocked(invoke).mockImplementation(async (command: string) => {
      if (command === 'get_settings') return settings()
      if (command === 'get_diagnostics') return diagnostics()
      if (command === 'update_settings') return undefined
      return undefined
    })
    const user = userEvent.setup()
    render(<Settings />)
    await waitFor(() => expect(screen.getByLabelText(/hotkey binding/i)).toHaveValue('Ctrl+Shift+V'))

    await user.clear(screen.getByLabelText(/hotkey binding/i))
    await user.type(screen.getByLabelText(/hotkey binding/i), 'Ctrl+Shift+X')
    await user.click(screen.getByRole('button', { name: /save hotkey/i }))

    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith(
        'update_settings',
        expect.objectContaining({ settings: expect.objectContaining({ hotkey_binding: 'Ctrl+Shift+X' }) }),
      ),
    )
    await waitFor(() => expect(screen.getByText(/saved/i)).toBeInTheDocument())
  })

  it('shows the validation error and does not report saved for an invalid binding', async () => {
    vi.mocked(invoke).mockImplementation(async (command: string) => {
      if (command === 'get_settings') return settings()
      if (command === 'get_diagnostics') return diagnostics()
      if (command === 'update_settings') return Promise.reject('invalid hotkey binding')
      return undefined
    })
    const user = userEvent.setup()
    render(<Settings />)
    await waitFor(() => expect(screen.getByLabelText(/hotkey binding/i)).toHaveValue('Ctrl+Shift+V'))

    await user.clear(screen.getByLabelText(/hotkey binding/i))
    await user.type(screen.getByLabelText(/hotkey binding/i), 'NotAKey+++')
    await user.click(screen.getByRole('button', { name: /save hotkey/i }))

    await waitFor(() => expect(screen.getByRole('alert')).toHaveTextContent('invalid hotkey binding'))
    expect(screen.queryByText(/^saved$/i)).not.toBeInTheDocument()
  })

  it('shows an unsupported capability explicitly rather than hiding it', async () => {
    vi.mocked(invoke).mockImplementation(async (command: string) => {
      if (command === 'get_settings') return settings()
      if (command === 'get_diagnostics') return diagnostics({ hotkeys: false })
      return undefined
    })

    render(<Settings />)

    await waitFor(() => expect(screen.getByText(/hotkeys/i)).toBeInTheDocument())
    const row = screen.getByText(/hotkeys/i).closest('li')
    expect(row).toHaveTextContent(/unsupported/i)
  })

  it('creating a rule issues SaveRule with the entered app match', async () => {
    const user = userEvent.setup()
    render(<Settings />)
    await waitFor(() => expect(screen.getByLabelText(/new rule app match/i)).toBeInTheDocument())

    await user.type(screen.getByLabelText(/new rule app match/i), '1Password')
    await user.click(screen.getByRole('button', { name: /add rule/i }))

    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith(
        'save_rule',
        expect.objectContaining({ rule: expect.objectContaining({ app_match: '1Password', enabled: true }) }),
      ),
    )
  })

  it('deleting a rule issues DeleteRule and removes it from the list', async () => {
    const user = userEvent.setup()
    render(<Settings />)
    await waitFor(() => expect(screen.getByLabelText(/new rule app match/i)).toBeInTheDocument())
    await user.type(screen.getByLabelText(/new rule app match/i), '1Password')
    await user.click(screen.getByRole('button', { name: /add rule/i }))
    await waitFor(() => expect(screen.getByText('1Password')).toBeInTheDocument())

    await user.click(screen.getByRole('button', { name: /delete rule/i }))

    await waitFor(() => expect(invoke).toHaveBeenCalledWith('delete_rule', expect.objectContaining({ id: expect.any(String) })))
    expect(screen.queryByText('1Password')).not.toBeInTheDocument()
  })
})
