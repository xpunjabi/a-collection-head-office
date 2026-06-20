import { useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { Image as ImageIcon } from 'lucide-react'

interface Props {
  filename: string | null
  alt: string
  className?: string
  iconSize?: number
}

export default function ProductImage({ filename, alt, className = '', iconSize = 16 }: Props) {
  const [src, setSrc] = useState<string | null>(null)

  useEffect(() => {
    let cancelled = false
    setSrc(null)
    if (!filename) return
    invoke<string>('get_image_as_base64', { filename })
      .then((uri) => {
        if (!cancelled) setSrc(uri)
      })
      .catch(() => {})
    return () => {
      cancelled = true
    }
  }, [filename])

  if (src) {
    return <img src={src} alt={alt} className={className} />
  }
  return <ImageIcon size={iconSize} className="text-gray-500" />
}
