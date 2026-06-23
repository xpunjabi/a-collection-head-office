/**
 * Social sharing utility — extracted from SocialHub.tsx so it can be reused
 * by Catalog.tsx, ProductDraftCard, and any other component that needs to
 * share product content to social platforms.
 *
 * All sharing is done via platform-specific share URLs (no API keys, no OAuth).
 * - WhatsApp: wa.me/?text=... (works on web + mobile, opens WhatsApp app)
 * - Facebook: facebook.com/sharer/sharer.php?quote=... (opens FB share dialog)
 * - Twitter/X: twitter.com/intent/tweet?text=... (opens Tweet composer)
 * - Instagram: no web share URL — copy to clipboard + alert user (IG requires
 *   in-app paste because Instagram does not support third-party image/text
 *   share intents on web)
 */

export type SharePlatform = 'whatsapp' | 'facebook' | 'twitter/x' | 'instagram'

export const ALL_SHARE_PLATFORMS: SharePlatform[] = ['whatsapp', 'facebook', 'twitter/x', 'instagram']

export const PLATFORM_LABELS: Record<SharePlatform, string> = {
  'whatsapp': 'WhatsApp',
  'facebook': 'Facebook',
  'twitter/x': 'Twitter/X',
  'instagram': 'Instagram',
}

/**
 * Build a shareable text block for a product.
 * Format: "<name>\n<design>\nRs.<sale_price>\n\n<description>"
 * Truncated to 1000 chars to stay within WhatsApp/Twitter limits.
 */
export function buildProductShareText(opts: {
  name: string
  design?: string | null
  salePrice?: number | null
  retailPrice?: number | null
  description?: string | null
  hashtags?: string[] | null
  includeHashtags?: boolean
}): string {
  const parts: string[] = []
  if (opts.name) parts.push(opts.name)
  if (opts.design) parts.push(opts.design)
  if (opts.salePrice != null) {
    parts.push(`Rs. ${opts.salePrice.toFixed(0)}`)
    if (opts.retailPrice != null && opts.retailPrice > opts.salePrice) {
      const discount = Math.round(((opts.retailPrice - opts.salePrice) / opts.retailPrice) * 100)
      parts.push(`(${discount}% off from Rs. ${opts.retailPrice.toFixed(0)})`)
    }
  }
  if (opts.description) parts.push('', opts.description)
  if (opts.includeHashtags && opts.hashtags && opts.hashtags.length > 0) {
    parts.push('', opts.hashtags.map(h => h.startsWith('#') ? h : `#${h}`).join(' '))
  }
  let text = parts.join('\n')
  if (text.length > 1000) text = text.slice(0, 997) + '...'
  return text
}

/**
 * Share `text` to the given platform.
 * - WhatsApp / Facebook / Twitter/X: opens a new browser tab with the
 *   platform's share URL prefilled with the text.
 * - Instagram: copies text to clipboard and shows an alert telling the user
 *   to paste it in Instagram (Instagram does not support third-party web
 *   share URLs).
 *
 * Returns true if the share was initiated, false if the platform is unknown.
 */
export function shareToPlatform(platform: SharePlatform, text: string): boolean {
  const encoded = encodeURIComponent(text)
  let url = ''
  switch (platform) {
    case 'whatsapp':
      url = `https://wa.me/?text=${encoded}`
      break
    case 'facebook':
      url = `https://www.facebook.com/sharer/sharer.php?quote=${encoded}`
      break
    case 'twitter/x':
      url = `https://twitter.com/intent/tweet?text=${encoded}`
      break
    case 'instagram':
      try {
        navigator.clipboard.writeText(text)
        alert('Instagram caption copied to clipboard! Paste it in Instagram to share.')
      } catch {
        alert('Could not copy to clipboard. Please copy manually:\n\n' + text)
      }
      return true
    default:
      return false
  }
  window.open(url, '_blank', 'noopener,noreferrer')
  return true
}
