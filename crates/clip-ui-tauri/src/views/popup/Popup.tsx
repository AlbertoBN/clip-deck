import { useEffect, useRef, useState } from 'react'
import { convertFileSrc } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { WindowGripper } from '../../components/WindowGripper'
import { callCommand } from '../../state/client'
import { findThumbnail } from '../../state/clips'
import { useClipStore } from '../../state/store'

const SEARCH_DEBOUNCE_MS = 200

export function Popup() {
  const [query, setQuery] = useState('')
  const [selectedIndex, setSelectedIndex] = useState(0)
  const inputRef = useRef<HTMLInputElement>(null)
  const clips = useClipStore((s) => s.clips)
  const searchClips = useClipStore((s) => s.searchClips)
  const subscribeToEvents = useClipStore((s) => s.subscribeToEvents)

  const activate = () => {
    inputRef.current?.focus()
    setSelectedIndex(0)
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
    // Guards against a React StrictMode dev-mode race: effects run
    // mount -> cleanup -> mount synchronously, but `subscribeToEvents`
    // resolves asynchronously - if cleanup ran before it resolved, `fn`
    // would never be captured and the listener would leak, silently
    // doubling every subsequent daemon event. `cancelled` makes a
    // late-arriving `fn` unsubscribe itself immediately instead.
    let cancelled = false
    let unlisten: (() => void) | undefined
    void subscribeToEvents((event) => {
      if (event.type === 'HotkeyPressed') {
        activate()
      }
    }).then((fn) => {
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
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [subscribeToEvents])

  // Every effect runs once after the first render regardless of its
  // dependency array, so without this guard this would schedule a
  // redundant re-search 200ms after mount even though `activate()` (above)
  // already searched synchronously on mount - and that leftover timeout,
  // firing independently of anything the user does in the meantime, would
  // reset an arrow-key selection made in that window right back to the top.
  const isFirstQueryEffect = useRef(true)
  useEffect(() => {
    if (isFirstQueryEffect.current) {
      isFirstQueryEffect.current = false
      return
    }
    const handle = setTimeout(() => {
      setSelectedIndex(0)
      void searchClips(query)
    }, SEARCH_DEBOUNCE_MS)
    return () => clearTimeout(handle)
  }, [query, searchClips])

  // Clamps (rather than resets) the selection when the list shrinks - e.g.
  // a live ClipDeleted event removing the selected row. A live
  // ClipCaptured/ClipUpdated event must not silently reset the user's
  // current arrow-key position back to the top; only an explicit new
  // search (`activate`, above, or a typed query, above) does that.
  useEffect(() => {
    setSelectedIndex((index) => Math.min(index, Math.max(clips.length - 1, 0)))
  }, [clips.length])

  // Attached to `document` rather than the search input: focus can leave
  // the input (e.g. a click elsewhere in the popup, or focus never landing
  // on it for some environment-specific reason), and every one of these
  // keys must still work regardless of what currently has focus - not just
  // Escape. Re-registers on every `clips`/`selectedIndex` change so the
  // closure below always sees fresh values (same pattern Manager.tsx uses
  // for its own window-level keydown listener).
  useEffect(() => {
    const handleDocumentKeyDown = async (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        event.preventDefault()
        // Hides rather than closes - this window is shown/hidden (not
        // destroyed) across hotkey presses, same as the main/settings
        // windows' close-button handling; a real `.close()` here would
        // leave nothing for the next hotkey press to show.
        await getCurrentWindow().hide()
      } else if (event.key === 'ArrowDown') {
        event.preventDefault()
        setSelectedIndex((index) => Math.min(index + 1, Math.max(clips.length - 1, 0)))
      } else if (event.key === 'ArrowUp') {
        event.preventDefault()
        setSelectedIndex((index) => Math.max(index - 1, 0))
      } else if (event.key === 'Enter') {
        event.preventDefault()
        const selected = clips[selectedIndex]
        if (selected) {
          // clipd's paste simulation targets whatever window is currently
          // focused at the moment it runs, not whatever was focused before
          // the popup opened - so the popup must hide (returning OS focus
          // to the target application) *before* asking the daemon to
          // simulate the paste, not after. Pasting first would send the
          // synthetic keystroke into the popup itself instead of the
          // intended app.
          await getCurrentWindow().hide()
          await callCommand('paste_clip', { id: selected.id, mode: 'auto' })
        }
      }
    }
    document.addEventListener('keydown', handleDocumentKeyDown)
    return () => document.removeEventListener('keydown', handleDocumentKeyDown)
  }, [clips, selectedIndex])

  return (
    <div className="popup">
      <div className="popup-content">
        <input
          ref={inputRef}
          aria-label="Search clips"
          value={query}
          onChange={(event) => setQuery(event.target.value)}
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
          move or close it. */}
      <WindowGripper />
    </div>
  )
}
