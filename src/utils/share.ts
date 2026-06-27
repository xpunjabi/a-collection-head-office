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
 * v0.14.6: If `imageData` (base64 JPEG) is provided, the image is:
 *   1. Saved to the user's Downloads folder as "A-Collection_<name>_<ts>.jpg"
 *      so the user can DRAG-DROP it into FB/IG/WhatsApp post composers
 *      (drag-drop is universally supported; clipboard image paste is not).
 *   2. ALSO written to the system clipboard via tauri-plugin-clipboard-manager
 *      as a bonus for platforms that DO support Ctrl+V image paste.
 *
 * v0.14.5 history: clipboard-only approach didn't work reliably on FB/IG
 * web composers (Firefox/Edge don't accept pasted images in FB's composer).
 * v0.14.6 adds Downloads-folder save as the PRIMARY mechanism, with
 * clipboard as a secondary bonus.
 *
 * @param platform   — 'whatsapp' | 'facebook' | 'twitter/x' | 'instagram'
 * @param text       — caption text to share
 * @param imageData  — base64-encoded JPEG image (optional)
 * @param productName — product name for the saved filename (optional, v0.14.6)
 */
export async function shareToPlatform(
  platform: SharePlatform,
  text: string,
  imageData?: string | null,
  productName?: string | null,
): Promise<boolean> {
  const encoded = encodeURIComponent(text)
  let url = ''

  // v0.14.6: Save image to Downloads folder FIRST (primary mechanism).
  // Drag-drop from Downloads works on ALL platforms and ALL browsers.
  let savedImagePath: string | null = null
  if (imageData && IS_TAURI) {
    try {
      const { invoke } = await import('@tauri-apps/api/core')
      savedImagePath = await invoke<string>('save_image_for_share', {
        base64Data: imageData,
        productName: productName || 'product',
      })
      console.log('[share] Image saved to Downloads:', savedImagePath)
    } catch (err) {
      console.warn('[share] Could not save image to Downloads:', err)
    }
  }

  // v0.14.5: Also write image to system clipboard as a BONUS.
  // Some platforms (WhatsApp Web in Chrome) DO accept pasted images.
  let imageOnClipboard = false
  if (imageData && IS_TAURI) {
    try {
      const cleanBase64 = imageData.includes(',') ? imageData.split(',')[1] : imageData
      const binaryString = atob(cleanBase64)
      const bytes = new Uint8Array(binaryString.length)
      for (let i = 0; i < binaryString.length; i++) {
        bytes[i] = binaryString.charCodeAt(i)
      }
      const { writeImage } = await import('@tauri-apps/plugin-clipboard-manager')
      await writeImage(bytes)
      imageOnClipboard = true
      console.log('[share] Image also written to clipboard')
    } catch (err) {
      console.warn('[share] Clipboard write failed (Downloads save still works):', err)
    }
  }

  // Copy caption text to clipboard (overwrites image on clipboard — that's
  // OK because the image is saved to Downloads as the primary mechanism).
  let textOnClipboard = false
  try {
    await navigator.clipboard.writeText(text)
    textOnClipboard = true
  } catch {
    // Clipboard writeText might fail in some Tauri contexts
  }

  // Build a helpful alert message.
  const buildAlertMsg = (platformName: string): string => {
    let msg = `${platformName} opening now!\n\n`

    if (savedImagePath) {
      // Extract just the filename for display
      const filename = savedImagePath.split(/[\\/]/).pop() || savedImagePath
      const folder = savedImagePath.split(/[\\/]/).slice(-2, -1)[0] || 'Downloads'
      msg += `📷 Product image saved to:\n   ${folder}\\${filename}\n\n`
      msg += `${platformName} pe:\n`
      msg += `1. "Create Post" / chat box open karein\n`
      msg += `2. Caption paste karne ke liye text box me Ctrl+V\n`
      msg += `3. Image attach karne ke liye:\n`
      msg += `   • DRAG the file from ${folder} folder into the post, OR\n`
      msg += `   • Click "Photo/Video" button and browse to ${folder}\n\n`
      if (imageOnClipboard) {
        msg += `Bonus: image clipboard pe bhi hai — kuch platforms (WhatsApp Web)`
        msg += ` me Ctrl+V se bhi paste ho jayegi.\n\n`
      }
    } else if (imageOnClipboard) {
      msg += `📷 Product image clipboard pe copy ho gayi hai.\n`
      msg += `Image box / image picker me Ctrl+V paste karein.\n\n`
      if (textOnClipboard) {
        msg += `📋 Caption bhi clipboard pe hai.\n`
      }
    } else if (textOnClipboard) {
      msg += `📋 Caption text clipboard pe copy ho gaya hai.\n`
      msg += `Ctrl+V se paste karein.\n\n`
      msg += `(Image save nahi ho payi — manual attach karein.)\n`
    } else {
      msg += `Caption neeche box me paste karein.\n`
      msg += `(Text share URL me already hai for WhatsApp/Twitter.)\n`
    }
    return msg
  }

  switch (platform) {
    case 'whatsapp':
      url = `https://wa.me/?text=${encoded}`
      alert(buildAlertMsg('WhatsApp'))
      break
    case 'facebook':
      url = 'https://www.facebook.com/'
      alert(buildAlertMsg('Facebook'))
      await openExternalUrl(url)
      return true
    case 'twitter/x':
      url = `https://twitter.com/intent/tweet?text=${encoded}`
      alert(buildAlertMsg('Twitter/X'))
      break
    case 'instagram':
      alert(buildAlertMsg('Instagram'))
      return true
    default:
      return false
  }
  await openExternalUrl(url)
  return true
}
