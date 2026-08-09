import { useEffect, useRef, useState } from 'react'
import { convertFileSrc } from '@tauri-apps/api/core'
import { callCommand } from '../../state/client'
import { findThumbnail } from '../../state/clips'
import { useClipStore } from '../../state/store'
import type { ClearScope } from '../../state/types'

const INTERACTIVE_TAGS = new Set(['INPUT', 'SELECT', 'TEXTAREA', 'BUTTON'])

export function Manager() {
  const clips = useClipStore((s) => s.clips)
  const searchClips = useClipStore((s) => s.searchClips)
  const subscribeToEvents = useClipStore((s) => s.subscribeToEvents)
  const [pinnedOnly, setPinnedOnly] = useState(false)
  const [selectedIndex, setSelectedIndex] = useState(0)
  const itemRefs = useRef<(HTMLLIElement | null)[]>([])

  useEffect(() => {
    setSelectedIndex(0)
    void searchClips('', { pinned_only: pinnedOnly })
  }, [pinnedOnly, searchClips])

  useEffect(() => {
    // Guards against a React StrictMode dev-mode race: effects run
    // mount -> cleanup -> mount synchronously, but `subscribeToEvents`
    // resolves asynchronously - if cleanup ran before it resolved, `fn`
    // would never be captured and the listener would leak, silently
    // doubling every subsequent daemon event. `cancelled` makes a
    // late-arriving `fn` unsubscribe itself immediately instead.
    let cancelled = false
    let unlisten: (() => void) | undefined
    void subscribeToEvents().then((fn) => {
      if (cancelled) {
        fn()
      } else {
        unlisten = fn
      }
    })
    return () => {
      cancelled = true
      unlisten?.()
    }
  }, [subscribeToEvents])

  // Clamps (rather than resets) the selection when the list shrinks - e.g.
  // a live ClipDeleted event removing the selected row. A live
  // ClipCaptured/ClipUpdated event must not silently reset the user's
  // current arrow-key position back to the top; only an explicit new
  // search (the pinned-only filter, above) does that.
  useEffect(() => {
    setSelectedIndex((index) => Math.min(index, Math.max(clips.length - 1, 0)))
  }, [clips.length])

  useEffect(() => {
    itemRefs.current[selectedIndex]?.scrollIntoView({ block: 'nearest' })
  }, [selectedIndex])

  // Window-level rather than attached to a specific element: unlike the
  // hotkey popup's search input, nothing in the Manager view is focused by
  // default. Interactive controls (the checkbox, buttons) still get their
  // own native arrow-key/Enter behavior since events targeting them are
  // skipped here.
  useEffect(() => {
    const handleKeyDown = async (event: KeyboardEvent) => {
      if (event.target instanceof HTMLElement && INTERACTIVE_TAGS.has(event.target.tagName)) return

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
          await callCommand('copy_clip', { id: selected.id })
        }
      }
    }
    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [clips, selectedIndex])

  const handlePin = async (id: string, pinned: boolean) => {
    await callCommand('pin_clip', { id, pinned })
  }

  const handleDelete = async (id: string) => {
    await callCommand('delete_clip', { id })
  }

  const handleBulkClear = async (scope: ClearScope) => {
    await callCommand('clear_history', { scope })
  }

  return (
    <div className="manager">
      <div className="filters">
        <label>
          <input type="checkbox" checked={pinnedOnly} onChange={(event) => setPinnedOnly(event.target.checked)} />
          Pinned only
        </label>
        <button type="button" onClick={() => handleBulkClear('excluding_pinned')}>
          Clear history (excluding pinned)
        </button>
        <button type="button" onClick={() => handleBulkClear('all')}>
          Clear all history
        </button>
        <button type="button" onClick={() => callCommand('show_settings_window')}>
          Settings
        </button>
      </div>
      <ul>
        {clips.map((clip, index) => (
          <li
            key={clip.id}
            ref={(el) => {
              itemRefs.current[index] = el
            }}
            aria-selected={index === selectedIndex}
          >
            <span className="clip-text">
              {(() => {
                const thumbnail = findThumbnail(clip)
                return thumbnail?.blob_path ? (
                  <img className="clip-thumbnail" src={convertFileSrc(thumbnail.blob_path)} alt="Clip thumbnail" />
                ) : (
                  clip.display_text
                )
              })()}
            </span>
            <span className="clip-actions">
              <button type="button" aria-label={`Pin ${clip.id}`} onClick={() => handlePin(clip.id, !clip.is_pinned)}>
                {clip.is_pinned ? 'Unpin' : 'Pin'}
              </button>
              <button type="button" aria-label={`Delete ${clip.id}`} onClick={() => handleDelete(clip.id)}>
                Delete
              </button>
            </span>
          </li>
        ))}
      </ul>
    </div>
  )
}
