import { getCurrentWindow } from '@tauri-apps/api/window'

type ResizeDirection = 'East' | 'North' | 'NorthEast' | 'NorthWest' | 'South' | 'SouthEast' | 'SouthWest' | 'West'

// Windows with no native decorations get no OS/compositor-rendered resize
// border - `resizable: true` in tauri.conf.json alone has nothing to hook
// into. These invisible edge/corner overlays are the replacement, each
// starting a native resize drag on mousedown.
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

function beginResize(direction: ResizeDirection) {
  void getCurrentWindow().startResizeDragging(direction)
}

/**
 * Shared chrome for windows with no native decorations: a drag handle +
 * close button on one edge, plus invisible resize-drag overlays on every
 * edge/corner. Used identically by every undecorated window (popup,
 * manager) so their drag/close/resize behavior never drifts apart.
 */
export function WindowGripper() {
  return (
    <>
      {/* Sibling of the drag region (not nested inside it) so a click on it
          isn't swallowed by the drag region's own mousedown handling. */}
      <div className="window-gripper">
        <button
          type="button"
          aria-label="Close"
          className="window-gripper-close"
          onClick={() => getCurrentWindow().hide()}
        >
          ×
        </button>
        <div className="window-gripper-drag" data-tauri-drag-region>
          <span className="window-gripper-label">ClipDeck</span>
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
    </>
  )
}
