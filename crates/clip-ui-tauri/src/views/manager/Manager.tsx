import { useEffect, useRef, useState } from 'react'
import { convertFileSrc } from '@tauri-apps/api/core'
import { callCommand } from '../../state/client'
import { useClipStore } from '../../state/store'
import type { Clip, ClearScope, Group } from '../../state/types'

const INTERACTIVE_TAGS = new Set(['INPUT', 'SELECT', 'TEXTAREA', 'BUTTON'])

// Prefers the generated thumbnail representation over the full image, so
// list rows stay compact; falls back to the full image if no thumbnail was
// captured (e.g. thumbnail generation failed but the full image persisted).
function findThumbnail(clip: Clip) {
  return (
    clip.representations.find((r) => r.is_preview && r.mime_type.startsWith('image/')) ??
    clip.representations.find((r) => r.mime_type.startsWith('image/'))
  )
}

export function Manager() {
  const clips = useClipStore((s) => s.clips)
  const searchClips = useClipStore((s) => s.searchClips)
  const subscribeToEvents = useClipStore((s) => s.subscribeToEvents)
  const [groups, setGroups] = useState<Group[]>([])
  const [groupId, setGroupId] = useState<string | null>(null)
  const [pinnedOnly, setPinnedOnly] = useState(false)
  const [selectedIndex, setSelectedIndex] = useState(0)
  const itemRefs = useRef<(HTMLLIElement | null)[]>([])

  useEffect(() => {
    void callCommand<Group[]>('list_groups').then(setGroups)
  }, [])

  useEffect(() => {
    void searchClips('', { group_id: groupId, pinned_only: pinnedOnly })
  }, [groupId, pinnedOnly, searchClips])

  useEffect(() => {
    let unlisten: (() => void) | undefined
    void subscribeToEvents().then((fn) => {
      unlisten = fn
    })
    return () => unlisten?.()
  }, [subscribeToEvents])

  useEffect(() => {
    setSelectedIndex(0)
  }, [clips])

  useEffect(() => {
    itemRefs.current[selectedIndex]?.scrollIntoView({ block: 'nearest' })
  }, [selectedIndex])

  // Window-level rather than attached to a specific element: unlike the
  // hotkey popup's search input, nothing in the Manager view is focused by
  // default. Interactive controls (the group filter, checkbox, buttons)
  // still get their own native arrow-key/Enter behavior since events
  // targeting them are skipped here.
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

  const handleAssignGroup = async (id: string, newGroupId: string | null) => {
    await callCommand('assign_group', { id, group_id: newGroupId })
  }

  const handleBulkClear = async (scope: ClearScope) => {
    await callCommand('clear_history', { scope })
  }

  return (
    <div className="manager">
      <div className="filters">
        <label>
          Group
          <select
            aria-label="Group filter"
            value={groupId ?? ''}
            onChange={(event) => setGroupId(event.target.value || null)}
          >
            <option value="">All groups</option>
            {groups.map((group) => (
              <option key={group.id} value={group.id}>
                {group.name}
              </option>
            ))}
          </select>
        </label>
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
              <select
                aria-label={`Group for ${clip.id}`}
                value={clip.group_id ?? ''}
                onChange={(event) => handleAssignGroup(clip.id, event.target.value || null)}
              >
                <option value="">No group</option>
                {groups.map((group) => (
                  <option key={group.id} value={group.id}>
                    {group.name}
                  </option>
                ))}
              </select>
            </span>
          </li>
        ))}
      </ul>
    </div>
  )
}
