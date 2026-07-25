import { useEffect, useRef, useState } from 'react'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { listen } from '@tauri-apps/api/event'
import { callCommand } from '../../state/client'
import { useClipStore } from '../../state/store'
import { DAEMON_EVENT_CHANNEL, type DaemonEvent } from '../../state/types'

const SEARCH_DEBOUNCE_MS = 200

export function Popup() {
  const [query, setQuery] = useState('')
  const [selectedIndex, setSelectedIndex] = useState(0)
  const inputRef = useRef<HTMLInputElement>(null)
  const clips = useClipStore((s) => s.clips)
  const searchClips = useClipStore((s) => s.searchClips)

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
    // behavior via this listener instead.
    const unlisten = listen<DaemonEvent>(DAEMON_EVENT_CHANNEL, (event) => {
      if (event.payload.type === 'HotkeyPressed') {
        activate()
      }
    })
    return () => {
      void unlisten.then((f) => f())
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  useEffect(() => {
    const handle = setTimeout(() => {
      void searchClips(query)
    }, SEARCH_DEBOUNCE_MS)
    return () => clearTimeout(handle)
  }, [query, searchClips])

  useEffect(() => {
    setSelectedIndex(0)
  }, [clips])

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
      <input
        ref={inputRef}
        aria-label="Search clips"
        value={query}
        onChange={(event) => setQuery(event.target.value)}
        onKeyDown={handleKeyDown}
      />
      <ul role="listbox">
        {clips.map((clip, index) => (
          <li key={clip.id} role="option" aria-selected={index === selectedIndex}>
            {clip.display_text}
          </li>
        ))}
      </ul>
    </div>
  )
}
