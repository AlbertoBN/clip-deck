import { fireEvent, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'

const hide = vi.fn().mockResolvedValue(undefined)
const startResizeDragging = vi.fn().mockResolvedValue(undefined)
vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => ({ hide, startResizeDragging }),
}))

import { WindowGripper } from './WindowGripper'

describe('WindowGripper', () => {
  it('renders a drag gripper with a vertical ClipDeck label', () => {
    render(<WindowGripper />)

    const label = screen.getByText('ClipDeck')
    expect(label.closest('[data-tauri-drag-region]')).not.toBeNull()
  })

  it('hides the current window when the gripper close button is clicked', async () => {
    const user = userEvent.setup()

    render(<WindowGripper />)
    await user.click(screen.getByRole('button', { name: /close/i }))

    expect(hide).toHaveBeenCalled()
  })

  it('starts a native resize drag when an edge or corner handle is pressed', () => {
    render(<WindowGripper />)

    fireEvent.mouseDown(screen.getByTestId('resize-handle-south'))
    fireEvent.mouseDown(screen.getByTestId('resize-handle-east'))
    fireEvent.mouseDown(screen.getByTestId('resize-handle-northwest'))

    expect(startResizeDragging).toHaveBeenCalledWith('South')
    expect(startResizeDragging).toHaveBeenCalledWith('East')
    expect(startResizeDragging).toHaveBeenCalledWith('NorthWest')
  })
})
