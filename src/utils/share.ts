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
 *
 * CRITICAL (Tauri desktop context):
 * In a Tauri webview, window.open() does NOT open the URL in the user's
 * default system browser — it tries to navigate the webview itself, which
 * fails for cross-origin URLs due to CSP/security restrictions. To open
 * external links in the user's actual browser (Chrome, Firefox, Edge),
 * we must use @tauri-apps/plugin-shell's open() function.
 *
 * We detect the Tauri environment at runtime and dispatch accordingly:
 *   - Tauri desktop: use shell.open(url)
 *   - Plain web (Firebase preview): use window.open(url, '_blank')
 *
 * This keeps the share utility working in BOTH contexts without any
 * conditional imports or build-time flags.
 */

// Detect Tauri environment at module load. The Tauri webview injects a
// `window.__TAURI_INTERNALS__` global; in plain browsers this is undefined.
// We also check window.__TAURI__ for older Tauri versions.
const IS_TAURI = typeof window !== 'undefined'
  && (Boolean((window as any).__TAURI_INTERNALS__) || Boolean((window as any).__TAURI__));

/**
 * Lazy-load the Tauri shell open() function. We use dynamic import so that
 * the @tauri-apps/plugin-shell package is NOT bundled when running in a
 * plain web context (e.g., the Firebase preview). This avoids runtime
 * errors when the Tauri runtime is not present.
 */
async function openExternalUrl(url: string): Promise<void> {
  if (IS_TAURI) {
    try {
      const { open } = await import('@tauri-apps/plugin-shell');
      await open(url);
      return;
    } catch (err) {
      // Fallback to window.open() if the shell plugin fails for any reason
      // (e.g., permission denied, plugin not registered, scope mismatch).
      console.warn('[share] Tauri shell.open() failed, falling back to window.open():', err);
    }
  }
  // Plain web context OR Tauri fallback — use window.open()
  window.open(url, '_blank', 'noopener,noreferrer');
}

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
 *
 * Platform behavior:
 * - WhatsApp: opens wa.me/?text=... (works on web + mobile, pre-fills text)
 * - Twitter/X: opens twitter.com/intent/tweet?text=... (pre-fills text)
 * - Facebook: COPIES text to clipboard + opens Facebook + alerts user.
 *   Facebook deprecated the `quote` parameter in sharer.php — the sharer
 *   dialog no longer accepts pre-filled text via URL. The user must
 *   manually paste (Ctrl+V) in the Facebook post composer.
 * - Instagram: copies text to clipboard + alerts user (Instagram has no
 *   web share URL at all).
 *
 * Returns true if the share was initiated, false if the platform is unknown.
 */
export async function shareToPlatform(platform: SharePlatform, text: string): Promise<boolean> {
  const encoded = encodeURIComponent(text)
  let url = ''
  switch (platform) {
    case 'whatsapp':
      url = `https://wa.me/?text=${encoded}`
      break
    case 'facebook':
      // Facebook deprecated the `quote` parameter in sharer.php. Opening
      // sharer.php?quote=... shows an EMPTY share dialog. Instead, we:
      // 1. Copy the caption to clipboard so the user can paste it
      // 2. Open Facebook's home page (where they can create a post)
      // 3. Alert the user to paste (Ctrl+V) in the post composer
      try {
        await navigator.clipboard.writeText(text)
      } catch {
        // Clipboard might fail in some contexts — alert with manual copy
      }
      url = 'https://www.facebook.com/'
      // Show alert BEFORE opening the URL so the user knows to paste
      alert('Facebook caption copied to clipboard!\n\nFacebook no longer allows pre-filling posts via links. When Facebook opens, click "Create Post" and press Ctrl+V to paste your caption.')
      await openExternalUrl(url)
      return true
    case 'twitter/x':
      url = `https://twitter.com/intent/tweet?text=${encoded}`
      break
    case 'instagram':
      try {
        await navigator.clipboard.writeText(text)
        alert('Instagram caption copied to clipboard! Paste it in Instagram to share.')
      } catch {
        alert('Could not copy to clipboard. Please copy manually:\n\n' + text)
      }
      return true
    default:
      return false
  }
  await openExternalUrl(url)
  return true
}
