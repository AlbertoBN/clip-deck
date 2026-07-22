import { useEffect, useState } from 'react'
import { convertFileSrc } from '@tauri-apps/api/core'
import { callCommand } from '../state/client'
import type { ClipRepresentation } from '../state/types'

function mimeFamily(mimeType: string): 'html' | 'image' | 'text' {
  if (mimeType === 'text/html') return 'html'
  if (mimeType.startsWith('image/')) return 'image'
  return 'text'
}

export function Preview({ representation }: { representation: ClipRepresentation }) {
  const family = mimeFamily(representation.mime_type)
  const [sanitizedHtml, setSanitizedHtml] = useState<string | null>(null)

  useEffect(() => {
    if (family === 'html' && representation.text_value) {
      void callCommand<string>('sanitize_clip_html', { html: representation.text_value }).then(setSanitizedHtml)
    }
  }, [family, representation.text_value])

  if (family === 'image') {
    return representation.blob_path ? (
      <img src={convertFileSrc(representation.blob_path)} alt="Clip preview" />
    ) : null
  }

  if (family === 'html') {
    return sanitizedHtml ? <div dangerouslySetInnerHTML={{ __html: sanitizedHtml }} /> : null
  }

  return <pre>{representation.text_value}</pre>
}
