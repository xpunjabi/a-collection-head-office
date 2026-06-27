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
 * v0.14.5: If `imageData` (base64 JPEG) is provided, the image is written
 * to the SYSTEM CLIPBOARD via tauri-plugin-clipboard-manager BEFORE the
 * share URL opens. This means after the platform opens (FB/IG/WhatsApp),
 * the user can press Ctrl+V to paste BOTH the caption text AND the image
 * directly into the post composer — no manual file attach required.
 *
 * The previous flow (v0.13.4) only saved the image to the app's images
 * folder, which the user couldn't easily find. The new flow puts the
 * image on the clipboard where it's a single Ctrl+V away.
 *
 * Platform behavior:
 * - WhatsApp: opens wa.me/?text=... (text pre-filled). Image on clipboard
 *   ready to paste into WhatsApp's image picker.
 * - Twitter/X: opens twitter.com/intent/tweet?text=... (text pre-filled).
 *   Image on clipboard ready to paste.
 * - Facebook: opens facebook.com. Text + image both on clipboard — user
 *   pastes into "Create Post".
 * - Instagram: opens instagram.com. Text + image both on clipboard — user
 *   pastes into the caption box + image picker.
 *
 * Returns true if the share was initiated, false if the platform is unknown.
 */
export async function shareToPlatform(
  platform: SharePlatform,
  text: string,
  imageData?: string | null,
): Promise<boolean> {
  const encoded = encodeURIComponent(text)
  let url = ''

  // v0.14.5: Write text + image to system clipboard BEFORE opening the
  // share URL. The order is: image first (so it's the "current" clipboard
  // content for image-paste operations), then text on a separate clipboard
  // write (text is what the user sees when they Ctrl+V in a text box).
  // Note: a clipboard can hold EITHER text OR image at a time, not both.
  // Strategy: put IMAGE on clipboard (since text is also pre-filled in the
  // share URL for WhatsApp/Twitter), and alert the user that caption text
  // was copied separately so they can paste it after the image.
  let imageOnClipboard = false
  let textOnClipboard = false

  // Always copy caption text first — this is the primary content.
  try {
    await navigator.clipboard.writeText(text)
    textOnClipboard = true
  } catch {
    // Clipboard writeText might fail in some Tauri contexts; the
    // share URL still carries the text for WhatsApp/Twitter.
  }

  // Then, if an image is provided, write it to the clipboard OVERWRITING
  // the text. We alert the user that they need to paste text first, then
  // re-copy image (or just paste image into the image picker box).
  if (imageData && IS_TAURI) {
    try {
      // Convert base64 string to Uint8Array for writeImage
      const cleanBase64 = imageData.includes(',') ? imageData.split(',')[1] : imageData
      const binaryString = atob(cleanBase64)
      const bytes = new Uint8Array(binaryString.length)
      for (let i = 0; i < binaryString.length; i++) {
        bytes[i] = binaryString.charCodeAt(i)
      }
      const { writeImage } = await import('@tauri-apps/plugin-clipboard-manager')
      await writeImage(bytes)
      imageOnClipboard = true
      console.log('[share] Image written to system clipboard')
    } catch (err) {
      console.warn('[share] Could not write image to clipboard:', err)
      // Fall back: save image to disk so user can attach manually
      try {
        const { invoke } = await import('@tauri-apps/api/core')
        await invoke<string>('save_base64_image', {
          base64Data: imageData,
          formatType: 'thumbnail',
        })
        console.log('[share] Image saved to disk as fallback')
      } catch (e) {
        console.warn('[share] Disk save fallback also failed:', e)
      }
    }
  }

  // Build a helpful alert message depending on what's on the clipboard.
  const buildAlertMsg = (platformName: string): string => {
    let msg = ''
    if (imageOnClipboard && textOnClipboard) {
      msg = `${platformName} opening now!\n\n`
        + '📋 Caption text + 📷 product image dono clipboard pe hain.\n\n'
        + `${platformName} pe:\n`
        + '1. "Create Post" / "New Tweet" / chat box me click karein\n'
        + '2. Caption paste karne ke liye pehle text box me Ctrl+V (text aayega)\n'
        + '3. Image box / image picker me Ctrl+V (image aayega)\n\n'
        + 'Note: clipboard ek waqt me ya to text ya image hold karta hai. '
        + 'Agar image paste nahi hoti to wapas app me aao aur "Copy Caption" '
        + 'se text dobara copy karein.'
    } else if (imageOnClipboard) {
      msg = `${platformName} opening now!\n\n`
        + '📷 Product image clipboard pe copy ho gayi hai.\n\n'
        + 'Image picker / image box me Ctrl+V paste karein.\n'
        + 'Caption text neehe box me type karne ke liye diya gaya hai.'
    } else if (textOnClipboard) {
      msg = `${platformName} opening now!\n\n`
        + '📋 Caption text clipboard pe copy ho gaya hai.\n'
        + 'Ctrl+V se paste karein.'
    } else {
      msg = `${platformName} opening now!\n\n`
        + 'Caption neehe box me paste karein (text share URL me already hai for WhatsApp/Twitter).'
    }
    return msg
  }

  switch (platform) {
    case 'whatsapp':
      url = `https://wa.me/?text=${encoded}`
      // For WhatsApp, the share URL already carries the text — so we can
      // keep the image on the clipboard (overwrites text). User pastes
      // image into WhatsApp's image picker.
      if (imageOnClipboard) {
        alert(buildAlertMsg('WhatsApp'))
      }
      break
    case 'facebook':
      // FB doesn't accept pre-filled text via share URL anymore. Re-copy
      // text to clipboard (overwrites image) since FB composer is a text
      // box first. User can drag-drop image from app folder as fallback.
      if (textOnClipboard) {
        try { await navigator.clipboard.writeText(text) } catch {}
      }
      url = 'https://www.facebook.com/'
      alert(buildAlertMsg('Facebook'))
      await openExternalUrl(url)
      return true
    case 'twitter/x':
      url = `https://twitter.com/intent/tweet?text=${encoded}`
      if (imageOnClipboard) {
        alert(buildAlertMsg('Twitter/X'))
      }
      break
    case 'instagram':
      // IG has no web share URL. Text + image both via clipboard paste.
      // For IG, text is more important (IG composer is text-first), so
      // re-copy text over image.
      if (textOnClipboard) {
        try { await navigator.clipboard.writeText(text) } catch {}
      }
      alert(buildAlertMsg('Instagram'))
      return true
    default:
      return false
  }
  await openExternalUrl(url)
  return true
}
