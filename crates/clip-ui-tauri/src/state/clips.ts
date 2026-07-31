import type { Clip } from './types'

// Prefers the generated thumbnail representation over the full image, so
// list rows stay compact; falls back to the full image if no thumbnail was
// captured (e.g. thumbnail generation failed but the full image persisted).
export function findThumbnail(clip: Clip) {
  return (
    clip.representations.find((r) => r.is_preview && r.mime_type.startsWith('image/')) ??
    clip.representations.find((r) => r.mime_type.startsWith('image/'))
  )
}
