import { useEffect, useRef, useState } from 'react'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { callCommand } from '../../state/client'
import { useClipStore } from '../../state/store'

const SEARCH_DEBOUNCE_MS = 200

export function Popup() {
  const [query, setQuery] = useState('')
  const [selectedIndex, setSelectedIndex] = useState(0)
  const inputRef = useRef<HTMLInputElement>(null)
  const clips = useClipStore((s) => s.clips)
  const searchClips = useClipStore((s) => s.searchClips)

  useEffect(() => {
    inputRef.current?.focus()
    void searchClips('')
  }, [searchClips])

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
