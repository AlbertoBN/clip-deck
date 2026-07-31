import { useEffect, useRef, useState } from 'react'
import { convertFileSrc } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { callCommand } from '../../state/client'
import { findThumbnail } from '../../state/clips'
import { useClipStore } from '../../state/store'

const SEARCH_DEBOUNCE_MS = 200

type ResizeDirection = 'East' | 'North' | 'NorthEast' | 'NorthWest' | 'South' | 'SouthEast' | 'SouthWest' | 'West'

// Popup has no native decorations, so the OS/compositor doesn't render any
// resize border for it - `resizable: true` in tauri.conf.json alone has
// nothing to hook into. These invisible edge/corner overlays are the
// replacement, each starting a native resize drag on mousedown.
const RESIZE_HANDLES: { direction: ResizeDirection; className: string }[] = [
  { direction: 'North', className: 'resize-n' },
  { direction: 'South', className: 'resize-s' },
  { direction: 'West', className: 'resize-w' },
  { direction: 'East', className: 'resize-e' },
  { direction: 'NorthWest', className: 'resize-nw' },
  { direction: 'NorthEast', className: 'resize-ne' },
  { direction: 'SouthWest', className: 'resize-sw' },
  { direction: 'SouthEast', className: 'resize-se' },
]

export function Popup() {
  const [query, setQuery] = useState('')
  const [selectedIndex, setSelectedIndex] = useState(0)
  const inputRef = useRef<HTMLInputElement>(null)
  const clips = useClipStore((s) => s.clips)
  const searchClips = useClipStore((s) => s.searchClips)
  const subscribeToEvents = useClipStore((s) => s.subscribeToEvents)

  const activate = () => {
    inputRef.current?.focus()
    void searchClips('')
  }

  useEffect(() => {
    activate()
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [searchClips])

  useEffect(() => {
    // The window is shown/hidden rather than created/destroyed on each
    // hotkey press, so the mount effect above only covers the very first
    // activation - repeat presses re-run the same focus-and-empty-search
    // behavior via this subscription instead. Routing through the store's
    // subscribeToEvents (rather than a separate `listen` call) also keeps
    // the popup's clip list live for ClipCaptured/ClipUpdated/ClipDeleted
    // while it's open, not just on the next hotkey press.
    let unlisten: (() => void) | undefined
    void subscribeToEvents((event) => {
      if (event.type === 'HotkeyPressed') {
        activate()
      }
    }).then((fn) => {
      unlisten = fn
    })
    return () => unlisten?.()
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [subscribeToEvents])

  useEffect(() => {
    const handle = setTimeout(() => {
      void searchClips(query)
    }, SEARCH_DEBOUNCE_MS)
    return () => clearTimeout(handle)
  }, [query, searchClips])

  useEffect(() => {
    setSelectedIndex(0)
  }, [clips])

  // Attached to `document` rather than the search input: focus can leave
  // the input (e.g. a click elsewhere in the popup), and Escape should still
  // hide the popup regardless of what currently has focus. Hides rather than
  // closes - this window is shown/hidden (not destroyed) across hotkey
  // presses, same as the main/settings windows' close-button handling; a
  // real `.close()` here would leave nothing for the next hotkey press to
  // show.
  useEffect(() => {
    const handleDocumentKeyDown = async (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        event.preventDefault()
        await getCurrentWindow().hide()
      }
    }
    document.addEventListener('keydown', handleDocumentKeyDown)
    return () => document.removeEventListener('keydown', handleDocumentKeyDown)
  }, [])

  const beginResize = (direction: ResizeDirection) => {
    void getCurrentWindow().startResizeDragging(direction)
  }

  const handleKeyDown = async (event: React.KeyboardEvent<HTMLInputElement>) => {
    if (event.key === 'ArrowDown') {
      event.preventDefault()
      setSelectedIndex((index) => Math.min(index + 1, Math.max(clips.length - 1, 0)))
    } else if (event.key === 'ArrowUp') {
      event.preventDefault()
      setSelectedIndex((index) => Math.max(index - 1, 0))
    } else if (event.key === 'Enter') {
      event.preventDefault()
      const selected = clips[selectedIndex]
      if (selected) {
        await callCommand('paste_clip', { id: selected.id, mode: 'auto' })
        await getCurrentWindow().hide()
      }
    }
  }

  return (
    <div className="popup">
      <div className="popup-content">
        <input
          ref={inputRef}
          aria-label="Search clips"
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          onKeyDown={handleKeyDown}
        />
        <ul role="listbox">
          {clips.map((clip, index) => {
            const thumbnail = findThumbnail(clip)
            return (
              <li key={clip.id} role="option" aria-selected={index === selectedIndex}>
                {thumbnail?.blob_path ? (
                  <img className="clip-thumbnail" src={convertFileSrc(thumbnail.blob_path)} alt="Clip thumbnail" />
                ) : (
                  <span className="clip-text">{clip.display_text}</span>
                )}
              </li>
            )
          })}
        </ul>
      </div>
      {/* Popup has no native decorations, so this gripper is the only way to
          move or close it. The close button is a sibling of the drag
          region (not nested inside it) so a click on it isn't swallowed by
          the drag region's own mousedown handling. */}
      <div className="popup-gripper">
        <button type="button" aria-label="Close" className="popup-gripper-close" onClick={() => getCurrentWindow().hide()}>
          ×
        </button>
        <div className="popup-gripper-drag" data-tauri-drag-region>
          <span className="popup-gripper-label">ClipDeck</span>
        </div>
      </div>
      {RESIZE_HANDLES.map(({ direction, className }) => (
        <div
          key={direction}
          className={`resize-handle ${className}`}
          data-testid={`resize-handle-${direction.toLowerCase()}`}
          onMouseDown={() => beginResize(direction)}
        />
      ))}
    </div>
  )
}
