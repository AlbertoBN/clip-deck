import { useEffect, useState } from 'react'
import { callCommand } from '../../state/client'
import { useClipStore } from '../../state/store'
import type { ClearScope, Group } from '../../state/types'

export function Manager() {
  const clips = useClipStore((s) => s.clips)
  const searchClips = useClipStore((s) => s.searchClips)
  const subscribeToEvents = useClipStore((s) => s.subscribeToEvents)
  const [groups, setGroups] = useState<Group[]>([])
  const [groupId, setGroupId] = useState<string | null>(null)
  const [pinnedOnly, setPinnedOnly] = useState(false)

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
        {clips.map((clip) => (
          <li key={clip.id}>
            {clip.display_text}
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
          </li>
        ))}
      </ul>
    </div>
  )
}
