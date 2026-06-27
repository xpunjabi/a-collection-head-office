import { useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { useAppStore } from '../stores/store'
import {
  Share2, MessageCircle, Facebook, Instagram, Copy, Check,
  AlertTriangle, RefreshCw, Sparkle, Send, Save, Trash2, Brain,
  Edit3, Hash, Image as ImageIcon
} from 'lucide-react'
import {
  shareToPlatform,
  SharePlatform
} from '../utils/share'
import ProductImage from '../components/ProductImage'

// Types
interface ShareLog {
  id: number
  product_id: number | null
  platform: string
  share_angle: string
  caption_text: string
  shared_by: string
  shared_at: string
  notes: string
  product_name: string
}

interface StaleProduct {
  id: number
  name: string
  sku: string
  sale_price: number
  stock_quantity: number
  images: string
  last_shared_at: string | null
}

interface SegmentCustomer {
  id: number
  name: string
  phone: string | null
  location: string | null
  segment: string
}

interface SocialDraft {
  id: string;
  productId?: number;
  productName?: string;
  content: string;
  platform: string;
  angle: string;
  created_at: string;
}

type ShareAngle = 'new_arrival' | 'discount' | 'premium' | 'budget' | 'limited_stock'

const SHARE_ANGLES: { id: ShareAngle; label: string; hint: string }[] = [
  { id: 'new_arrival', label: 'New Arrival', hint: 'Fresh stock, just landed' },
  { id: 'discount', label: 'Discount', hint: 'Price drop, limited time' },
  { id: 'premium', label: 'Premium Look', hint: 'High-end, luxury angle' },
  { id: 'budget', label: 'Budget Pick', hint: 'Affordable, value for money' },
  { id: 'limited_stock', label: 'Limited Stock', hint: 'Only a few left' },
]

// All platforms — unified list, no duplicates
const PLATFORMS = [
  { id: 'whatsapp_status', label: 'WhatsApp Status', icon: MessageCircle, color: 'text-emerald-400' },
  { id: 'whatsapp_direct', label: 'WhatsApp Direct', icon: MessageCircle, color: 'text-emerald-400' },
  { id: 'facebook', label: 'Facebook', icon: Facebook, color: 'text-blue-400' },
  { id: 'instagram', label: 'Instagram', icon: Instagram, color: 'text-pink-400' },
  { id: 'tiktok', label: 'TikTok', icon: Hash, color: 'text-sky-400' },
] as const

// Map platform_id to SharePlatform for shareToPlatform()
const PLATFORM_SHARE_MAP: Record<string, SharePlatform> = {
  'whatsapp_status': 'whatsapp',
  'whatsapp_direct': 'whatsapp',
  'facebook': 'facebook',
  'instagram': 'instagram',
  'tiktok': 'twitter/x',
}

export default function ShareCenter() {
  const { products, fetchProducts, updateSetting } = useAppStore()
  const [selectedProductId, setSelectedProductId] = useState<number | ''>('')
  const [shareAngle, setShareAngle] = useState<ShareAngle>('new_arrival')
  const [isGenerating, setIsGenerating] = useState(false)
  const [generatingPlatform, setGeneratingPlatform] = useState<string | null>(null)

  // Caption state — editable per platform
  const [captions, setCaptions] = useState<Record<string, string>>({})
  const [editingPlatform, setEditingPlatform] = useState<string | null>(null)
  const [copiedPlatform, setCopiedPlatform] = useState<string | null>(null)

  // Drafts
  const [drafts, setDrafts] = useState<SocialDraft[]>([])

  // Share logs
  const [shareLogs, setShareLogs] = useState<ShareLog[]>([])

  // Stale stock
  const [staleProducts, setStaleProducts] = useState<StaleProduct[]>([])

  // Segments + bulk broadcast
  const [segments, setSegments] = useState<string[]>([])
  const [selectedSegment, setSelectedSegment] = useState<string>('')
  const [segmentCustomers, setSegmentCustomers] = useState<SegmentCustomer[]>([])
  const [bulkBroadcastText, setBulkBroadcastText] = useState('')
  const [bulkBroadcastLoading, setBulkBroadcastLoading] = useState(false)

  // Tab state
  const [activeTab, setActiveTab] = useState<'share_pack' | 'drafts' | 'broadcast' | 'stale' | 'history'>('share_pack')

  useEffect(() => {
    fetchProducts()
    loadShareLogs()
    loadStaleProducts()
    loadSegments()
    loadDrafts()
  }, [])

  // === DATA LOADERS ===

  const loadShareLogs = async () => {
    try {
      const logs: ShareLog[] = await invoke('get_share_logs', { limit: 30 })
      setShareLogs(logs)
    } catch (err) { console.error('Failed to load share logs:', err) }
  }

  const loadStaleProducts = async () => {
    try {
      const stale: StaleProduct[] = await invoke('get_stale_products', { days: 7 })
      setStaleProducts(stale)
    } catch (err) { console.error('Failed to load stale products:', err) }
  }

  const loadSegments = async () => {
    try {
      const segs: string[] = await invoke('get_customer_segments')
      const defaults = ['women', 'girls', 'vip', 'agent', 'general']
      const all = Array.from(new Set([...defaults, ...segs])).sort()
      setSegments(all)
    } catch (err) { console.error('Failed to load segments:', err) }
  }

  const loadSegmentCustomers = async (seg: string) => {
    try {
      const customers: SegmentCustomer[] = await invoke('get_customers_by_segment', {
        segment: seg || null,
      })
      setSegmentCustomers(customers)
    } catch (err) {
      console.error('Failed to load segment customers:', err)
      setSegmentCustomers([])
    }
  }

  const loadDrafts = async () => {
    try {
      const allSettings: Record<string, string> = await invoke('get_settings')
      if (allSettings.social_drafts) {
        setDrafts(JSON.parse(allSettings.social_drafts))
      }
    } catch (err) { console.error('Failed to load drafts:', err) }
  }

  const saveDraftsList = async (newList: SocialDraft[]) => {
    try {
      await updateSetting('social_drafts', JSON.stringify(newList))
      setDrafts(newList)
    } catch (err) {
      alert(`Failed to save draft: ${err}`)
    }
  }

  // === AI CAPTION GENERATION ===

  const selectedProduct = products.find(p => p.id === selectedProductId)

  const handleGenerateCaption = async (platformId: string) => {
    if (!selectedProductId) {
      alert('Please select a product first.')
      return
    }
    const product = selectedProduct!
    setIsGenerating(true)
    setGeneratingPlatform(platformId)

    const angleText: Record<ShareAngle, string> = {
      new_arrival: 'NEW ARRIVAL — freshness, pehle hi haath lago!',
      discount: 'DISCOUNT — price drop, urgency, limited time!',
      premium: 'PREMIUM — quality, luxury, exclusivity!',
      budget: 'BUDGET-FRIENDLY — value for money, affordability!',
      limited_stock: 'LIMITED STOCK — scarcity, abhi kharid lo!',
    }

    const retailPrice = (product as any).retail_price?.toFixed(0) || (product.sale_price * 1.2).toFixed(0)
    const saveAmount = (parseFloat(retailPrice) - product.sale_price).toFixed(0)
    // Only show crossed-out retail + save amount if retail > sale (otherwise Save Rs. 0 looks bad)
    const hasDiscount = parseFloat(retailPrice) > product.sale_price
    const priceBlock = hasDiscount
      ? `🔥 SALE Rs. ${product.sale_price.toFixed(0)}\n~~Retail Rs. ${retailPrice}~~\nSave Rs. ${saveAmount}!`
      : `🔥 PRICE Rs. ${product.sale_price.toFixed(0)}`

    const platformPrompts: Record<string, string> = {
      'whatsapp_status': `Short WhatsApp Status (2-3 lines). PRICE FIRST at top:
${priceBlock}
Then 2 lines warm+urgent. One Punjabi phrase if natural. End with 'WhatsApp karein!'`,
      'whatsapp_direct': `2 line punchy pitch. Price first line: ${priceBlock.split('\n')[0]}. 'Abhi DM karein!'`,
      'facebook': `Facebook post (3-5 lines). PRICE BLOCK at top:
${priceBlock}
Then emotional content. Address 'Girls ❤️' or 'Ladies ❤️'. Include #NarowalFashion #Zafarwal #Shakargarh. Emojis.`,
      'instagram': `Instagram caption (3-5 lines). PRICE BLOCK first:
${priceBlock}
Then trendy+emotional. Line breaks. 5-8 hashtags: #NarowalFashion #NarowalLawn #Zafarwal #Shakargarh #instafashion`,
      'tiktok': `TikTok caption (2-3 lines). Price first: ${priceBlock.split('\n')[0]}. Hook-driven. MUST include #fyp #foryou #NarowalFashion #Zafarwal.`,
    }

    const prompt = `${angleText[shareAngle]}

Product: ${product.name} (${product.sku})
Category: ${product.category || 'Clothing'}
Sale Price: Rs. ${product.sale_price.toFixed(0)}
Retail Price: Rs. ${retailPrice}
Description: ${product.description || ''}

RULES: Price at top. Hinglish. Warm+emotional tone. One Punjabi phrase max. No 'Elegant/Beautiful/Premium' generic words. Target: Narowal women/girls 10-50.

${platformPrompts[platformId]}

Return ONLY the post text.`

    try {
      const response: any = await invoke('ask_ai', { prompt })
      const text = response.text || response || ''
      setCaptions(prev => ({ ...prev, [platformId]: text }))
    } catch (err) {
      alert(`AI Generation failed: ${err}`)
    } finally {
      setIsGenerating(false)
      setGeneratingPlatform(null)
    }
  }

  const handleGenerateAll = async () => {
    if (!selectedProductId) {
      alert('Please select a product first.')
      return
    }
    const product = selectedProduct!
    setIsGenerating(true)
    setGeneratingPlatform('all')

    const angleText: Record<ShareAngle, string> = {
      new_arrival: 'This is a NEW ARRIVAL — emphasize freshness and being the first to get it.',
      discount: 'This is a DISCOUNT post — emphasize the price drop and urgency.',
      premium: 'This is a PREMIUM product — emphasize quality, luxury, and exclusivity.',
      budget: 'This is a BUDGET-FRIENDLY pick — emphasize value for money and affordability.',
      limited_stock: 'This is a LIMITED STOCK alert — emphasize scarcity and urgency to buy now.',
    }

    // v0.13.2: SINGLE API call with JSON output — saves 4 API calls
    // (was 5 sequential calls, now just 1). Faster for user + saves quota.
    const prompt = `${angleText[shareAngle]}

Product Details:
- Name: ${product.name}
- SKU: ${product.sku}
- Category: ${product.category || 'Clothing'}
- Sale Price: Rs. ${product.sale_price.toFixed(0)}
- Retail Price: Rs. ${(product as any).retail_price?.toFixed(0) || (product.sale_price * 1.2).toFixed(0)}
- Description: ${product.description || 'Premium quality'}
- Tags: ${product.tags || ''}

CRITICAL MARKETING RULES — APPLY TO ALL PLATFORMS:
1. PRICE FIRST: Every caption MUST start with the sale price at the very top. Format:
   🔥 SALE Rs. ${product.sale_price.toFixed(0)}
   ~~Retail Rs. ${(product as any).retail_price?.toFixed(0) || (product.sale_price * 1.2).toFixed(0)}~~
   Save Rs. ${(((product as any).retail_price || (product.sale_price * 1.2)) - product.sale_price).toFixed(0)}!
   Then continue with marketing content below the price block.
2. EMOTIONAL TONE: Warm, exciting, friendly. Write like a local ladies clothing seller.
   Use openers like 'Girls ❤️' or 'Ladies ❤️' or 'Beautiful Girls ❤️' when appropriate.
3. LOCAL PUNJABI FLAVOR: Use ONE short Roman Punjabi phrase per post (max). Examples:
   'Raulay pe gaye je!' / 'Hun gal ban gayi!' / 'Oye hoye, ki gal ae!'
   Only when it naturally fits. Do NOT force into every post.
4. FORBIDDEN WORDS: Never use 'Elegant', 'Beautiful', 'Premium quality' as generic descriptors.
5. HINGLISH ONLY: Roman Urdu + English. Never pure English or Urdu script.
6. LOCAL HASHTAGS: Always include #NarowalFashion #NarowalLawn #Zafarwal #Shakargarh
7. AGGRESSIVE & PERSUASIVE: Create urgency, FOMO, excitement. Make them want to buy NOW.
8. TARGET: Narowal district women/girls age 10-50 (rural + urban)

Generate marketing content for ALL 5 platforms in ONE response. Return ONLY valid JSON:

{
  "whatsapp_status": "Start with price block. Then 2-3 lines warm+urgent. One Punjabi phrase if natural. 1-2 emojis. End with 'WhatsApp karein order ke liye!'",
  "whatsapp_direct": "Price first line. Then 1 line punchy pitch. 'Abhi DM karein!' ",
  "facebook": "Price block at top. Then 3-5 lines emotional+persuasive. Address 'Girls ❤️' or 'Ladies ❤️'. Include #NarowalFashion #Zafarwal #Shakargarh. Emojis.",
  "instagram": "Price block first. Then 3-5 lines trendy+emotional. Line breaks. 5-8 hashtags: #NarowalFashion #NarowalLawn #Zafarwal #Shakargarh #instafashion",
  "tiktok": "Price first. Then 2-3 lines hook-driven. MUST include #fyp #foryou #NarowalFashion #Zafarwal.",
  "hashtags": ["#NarowalFashion", "#NarowalLawn", "#Zafarwal", "#Shakargarh", "#fyp"],
  "cta": "Short aggressive Hinglish call-to-action"
}

Write in Hinglish. Return ONLY the JSON.`

    try {
      const response: any = await invoke('ask_ai', { prompt })
      const text = response.text || response || ''

      // Parse JSON from response (AI may wrap in markdown or add text)
      let jsonStr = text.trim()
      // Remove markdown code block if present
      if (jsonStr.includes('```')) {
        const start = jsonStr.indexOf('{')
        const end = jsonStr.lastIndexOf('}')
        if (start !== -1 && end !== -1) {
          jsonStr = jsonStr.substring(start, end + 1)
        }
      } else {
        const start = jsonStr.indexOf('{')
        const end = jsonStr.lastIndexOf('}')
        if (start !== -1 && end !== -1) {
          jsonStr = jsonStr.substring(start, end + 1)
        }
      }

      try {
        const parsed = JSON.parse(jsonStr)
        setCaptions({
          'whatsapp_status': parsed.whatsapp_status || '',
          'whatsapp_direct': parsed.whatsapp_direct || '',
          'facebook': parsed.facebook || '',
          'instagram': parsed.instagram || '',
          'tiktok': parsed.tiktok || '',
        })
        // Store hashtags + CTA for later use (e.g., Copy All)
        if (parsed.hashtags) {
          setCaptions(prev => ({ ...prev, '_hashtags': Array.isArray(parsed.hashtags) ? parsed.hashtags.join(' ') : '' }))
        }
        if (parsed.cta) {
          setCaptions(prev => ({ ...prev, '_cta': parsed.cta }))
        }
      } catch (parseErr) {
        // JSON parse failed — fallback: put raw text in all platforms
        console.error('JSON parse failed:', parseErr)
        alert('AI returned non-JSON response. Try again or generate per-platform.')
        setCaptions({ 'whatsapp_status': text })
      }
    } catch (err) {
      alert(`AI Generation failed: ${err}`)
    } finally {
      setIsGenerating(false)
      setGeneratingPlatform(null)
    }
  }

  // === CAPTION EDITING ===

  const handleEditCaption = (platformId: string) => {
    setEditingPlatform(editingPlatform === platformId ? null : platformId)
  }

  const handleCaptionChange = (platformId: string, text: string) => {
    setCaptions(prev => ({ ...prev, [platformId]: text }))
  }

  const handleCopyCaption = async (platformId: string) => {
    const text = captions[platformId]
    if (!text) return
    try {
      await navigator.clipboard.writeText(text)
      setCopiedPlatform(platformId)
      setTimeout(() => setCopiedPlatform(null), 2000)
    } catch (err) {
      alert(`Copy failed: ${err}`)
    }
  }

  // v0.13.2: Copy All captions to clipboard as formatted text
  const handleCopyAll = async () => {
    const allText = PLATFORMS.map(p => {
      const caption = captions[p.id]
      if (!caption) return null
      return `=== ${p.label} ===\n${caption}`
    }).filter(Boolean).join('\n\n')
    if (!allText) {
      alert('No captions to copy.')
      return
    }
    try {
      await navigator.clipboard.writeText(allText)
      setCopiedPlatform('all')
      setTimeout(() => setCopiedPlatform(null), 2000)
    } catch (err) {
      alert(`Copy failed: ${err}`)
    }
  }

  // === SHARING ===

  const handleShare = async (platformId: string) => {
    const caption = captions[platformId]
    if (!caption) {
      alert('Generate a caption first.')
      return
    }
    const sharePlatform = PLATFORM_SHARE_MAP[platformId] || 'whatsapp'

    // v0.13.4: Get product image for sharing (if available)
    let imageData: string | null = null
    if (selectedProduct) {
      try {
        const images = JSON.parse(selectedProduct.images || '[]')
        if (images.length > 0 && images[0]) {
          imageData = await invoke<string>('get_image_as_base64', { filename: images[0] })
        }
      } catch (err) {
        console.warn('[Share] Could not load product image:', err)
      }
    }

    try {
      await shareToPlatform(sharePlatform, caption, imageData, selectedProduct?.name)
      // Log the share
      await invoke('log_share', {
        productId: selectedProductId || null,
        platform: platformId,
        shareAngle,
        captionText: caption,
        notes: null,
      })
      await loadShareLogs()
      await loadStaleProducts()
    } catch (err) {
      console.error('Share failed:', err)
      alert(`Share failed: ${err}`)
    }
  }

  // === DRAFTS ===

  const handleSaveDraft = async () => {
    const generatedCaptions = Object.entries(captions).filter(([_, text]) => text && text.trim())
    if (generatedCaptions.length === 0) {
      alert('No captions to save. Generate at least one first.')
      return
    }
    const product = selectedProduct
    for (const [platform, content] of generatedCaptions) {
      const newDraft: SocialDraft = {
        id: String(Date.now()) + '-' + platform,
        productId: product?.id,
        productName: product?.name || 'Unknown',
        content,
        platform,
        angle: shareAngle,
        created_at: new Date().toLocaleString(),
      }
      const updated = [newDraft, ...drafts]
      await saveDraftsList(updated)
    }
    alert(`${generatedCaptions.length} draft(s) saved!`)
  }

  const handleDeleteDraft = async (id: string) => {
    const updated = drafts.filter(d => d.id !== id)
    await saveDraftsList(updated)
  }

  const handleLoadDraft = (draft: SocialDraft) => {
    setCaptions({ [draft.platform]: draft.content })
    if (draft.productId) setSelectedProductId(draft.productId)
    setShareAngle((draft.angle as ShareAngle) || 'new_arrival')
    setActiveTab('share_pack')
  }

  // === TEACH AI ===

  const handleTeachAI = async () => {
    const allCaptions = Object.values(captions).filter(c => c && c.trim()).join('\n\n---\n\n')
    if (!allCaptions) {
      alert('No captions to teach.')
      return
    }
    try {
      await invoke('save_knowledge', {
        topic: `Post Style - ${selectedProduct?.name || 'General'}`,
        content: allCaptions,
        source: 'share-center',
      })
      alert('Saved to AI Knowledge! The AI will learn from this style in future.')
    } catch (err) {
      alert(`Failed to save: ${err}`)
    }
  }

  // === BULK BROADCAST ===

  const handleBulkBroadcast = async () => {
    if (!selectedSegment) { alert('Select a customer segment first.'); return }
    if (!bulkBroadcastText.trim()) { alert('Enter broadcast message.'); return }
    if (segmentCustomers.length === 0) { alert('No customers in this segment.'); return }
    setBulkBroadcastLoading(true)
    try {
      const customersWithPhone = segmentCustomers.filter(c => c.phone && c.phone.trim())
      if (customersWithPhone.length === 0) {
        alert('No customers in this segment have phone numbers.')
        setBulkBroadcastLoading(false)
        return
      }
      const confirmed = confirm(`This will open ${customersWithPhone.length} WhatsApp chat windows. Continue?`)
      if (!confirmed) { setBulkBroadcastLoading(false); return }

      for (const c of customersWithPhone) {
        const phone = c.phone!.replace(/[^0-9]/g, '')
        if (phone.length < 10) continue
        try {
          await shareToPlatform('whatsapp', bulkBroadcastText)
          await new Promise(r => setTimeout(r, 500))
        } catch (err) {
          console.error(`Failed for ${c.name}:`, err)
        }
      }
      await invoke('log_share', {
        productId: null,
        platform: 'whatsapp_direct',
        shareAngle: 'bulk_broadcast',
        captionText: bulkBroadcastText,
        notes: `Bulk broadcast to segment: ${selectedSegment} (${customersWithPhone.length} customers)`,
      })
      await loadShareLogs()
      alert(`Broadcast initiated for ${customersWithPhone.length} customers.`)
    } catch (err) {
      alert(`Broadcast failed: ${err}`)
    } finally {
      setBulkBroadcastLoading(false)
    }
  }

  // === HELPERS ===
  const fmtDate = (iso: string) => new Date(iso).toLocaleDateString('en-PK', { day: '2-digit', month: 'short', hour: '2-digit', minute: '2-digit' })
  const fmtMoney = (n: number) => `Rs. ${n.toFixed(0)}`

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold tracking-tight text-white font-display">Share Center</h1>
        <p className="text-sm text-gray-400 mt-1">AI-powered marketing. Generate, edit, save, share — all in one place.</p>
      </div>

      {/* Tab switcher */}
      <div className="flex space-x-1 bg-slate-900/50 p-1 rounded-lg border border-gray-800 w-fit overflow-x-auto">
        {[
          { id: 'share_pack', label: 'AI Share Pack', icon: Sparkle },
          { id: 'drafts', label: `Drafts (${drafts.length})`, icon: Save },
          { id: 'broadcast', label: 'Bulk Broadcast', icon: Send },
          { id: 'stale', label: `Stale (${staleProducts.length})`, icon: AlertTriangle },
          { id: 'history', label: 'History', icon: RefreshCw },
        ].map(t => (
          <button
            key={t.id}
            onClick={() => setActiveTab(t.id as any)}
            className={`flex items-center space-x-1 px-3 py-1.5 rounded-md text-xs font-medium transition-colors whitespace-nowrap ${
              activeTab === t.id ? 'bg-violet-600 text-white' : 'text-gray-400 hover:text-white'
            }`}
          >
            <t.icon size={12} />
            <span>{t.label}</span>
          </button>
        ))}
      </div>

      {/* === AI SHARE PACK TAB === */}
      {activeTab === 'share_pack' && (
        <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
          {/* Left: product + angle picker */}
          <div className="glass-card p-5 space-y-4">
            <h2 className="text-lg font-semibold text-white">1. Select Product</h2>
            <div>
              <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">Product *</label>
              <select
                value={selectedProductId}
                onChange={e => { setSelectedProductId(e.target.value ? Number(e.target.value) : ''); setCaptions({}) }}
                className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500"
              >
                <option value="">-- Select Product --</option>
                {products.filter(p => p.status === 'active').map(p => (
                  <option key={p.id} value={p.id}>{p.name} ({p.sku}) — {fmtMoney(p.sale_price)}</option>
                ))}
              </select>
            </div>
            {selectedProduct && (
              <div className="bg-slate-950/50 border border-gray-800 rounded-lg p-3 text-xs text-gray-400 space-y-2">
                {/* v0.14.6: Product image preview so user sees what will be shared.
                    Previously the summary card only showed text (name, SKU, price,
                    stock) — user couldn't see the image that would accompany the
                    caption when sharing to social platforms. */}
                {(() => {
                  try {
                    const imgs: string[] = JSON.parse(selectedProduct.images || '[]')
                    if (imgs.length > 0 && imgs[0]) {
                      return (
                        <div className="w-full h-32 bg-slate-900 rounded-lg overflow-hidden border border-gray-800 flex items-center justify-center">
                          <ProductImage filename={imgs[0]} alt={selectedProduct.name} className="object-contain w-full h-full" />
                        </div>
                      )
                    }
                  } catch {}
                  return (
                    <div className="w-full h-32 bg-slate-900 rounded-lg overflow-hidden border border-gray-800 flex items-center justify-center text-gray-600">
                      <ImageIcon size={32} />
                    </div>
                  )
                })()}
                <div className="text-gray-300 font-semibold">{selectedProduct.name}</div>
                <div>SKU: {selectedProduct.sku}</div>
                <div>Price: {fmtMoney(selectedProduct.sale_price)}</div>
                <div>HO Stock: {selectedProduct.qty_in_head_office ?? selectedProduct.stock_quantity}</div>
              </div>
            )}
            <div>
              <label className="block text-xs font-semibold uppercase text-gray-400 mb-2">Marketing Angle</label>
              <div className="space-y-1">
                {SHARE_ANGLES.map(a => (
                  <button
                    key={a.id}
                    onClick={() => setShareAngle(a.id)}
                    className={`w-full flex items-center justify-between p-2 rounded-lg border text-left text-xs transition-colors ${
                      shareAngle === a.id
                        ? 'bg-violet-600/20 border-violet-500 text-white'
                        : 'bg-slate-950 border-gray-800 text-gray-400 hover:border-violet-500/50'
                    }`}
                  >
                    <div>
                      <div className="font-semibold">{a.label}</div>
                      <div className="text-[10px] text-gray-500">{a.hint}</div>
                    </div>
                    {shareAngle === a.id && <Check size={14} className="text-violet-400" />}
                  </button>
                ))}
              </div>
            </div>
            <button
              onClick={handleGenerateAll}
              disabled={!selectedProductId || isGenerating}
              className="w-full flex items-center justify-center space-x-2 px-4 py-2 bg-violet-600 hover:bg-violet-700 disabled:opacity-50 text-white rounded-lg text-sm font-medium transition-colors"
            >
              <Sparkle size={14} />
              <span>{isGenerating ? 'AI Generating (1 call)...' : 'Generate All (AI — 1 call)'}</span>
            </button>
            <div className="flex space-x-2">
              <button
                onClick={handleSaveDraft}
                disabled={Object.keys(captions).length === 0}
                className="flex-1 flex items-center justify-center space-x-1 px-3 py-2 bg-slate-800 hover:bg-slate-700 disabled:opacity-50 text-gray-200 rounded-lg text-xs font-medium"
              >
                <Save size={12} /><span>Save Drafts</span>
              </button>
              <button
                onClick={handleCopyAll}
                disabled={Object.keys(captions).length === 0}
                className="flex-1 flex items-center justify-center space-x-1 px-3 py-2 bg-slate-800 hover:bg-slate-700 disabled:opacity-50 text-gray-200 rounded-lg text-xs font-medium"
              >
                {copiedPlatform === 'all' ? <Check size={12} className="text-emerald-400" /> : <Copy size={12} />}
                <span>{copiedPlatform === 'all' ? 'Copied!' : 'Copy All'}</span>
              </button>
              <button
                onClick={handleTeachAI}
                disabled={Object.keys(captions).length === 0}
                className="flex-1 flex items-center justify-center space-x-1 px-3 py-2 bg-amber-600/20 hover:bg-amber-600/40 disabled:opacity-50 text-amber-300 rounded-lg text-xs font-medium"
              >
                <Brain size={12} /><span>Teach AI</span>
              </button>
            </div>
            {/* Teach AI clarification */}
            <div className="bg-slate-950/50 border border-gray-800 rounded-lg p-2 text-[10px] text-gray-500">
              <strong className="text-amber-400">Teach AI</strong> saves captions to local SQLite database (ai_knowledge table). AI reads these on future calls — no cloud dependency.
            </div>
          </div>

          {/* Right: generated captions (editable) */}
          <div className="lg:col-span-2 glass-card p-5 space-y-3">
            <h2 className="text-lg font-semibold text-white">2. AI-Generated Captions</h2>
            <p className="text-xs text-gray-400">Each caption is AI-generated for the specific platform. Click Generate per platform or "Generate All". Edit any caption before sharing.</p>
            {PLATFORMS.map(p => {
              const caption = captions[p.id]
              const isEditing = editingPlatform === p.id
              const isThisGenerating = generatingPlatform === p.id
              const Icon = p.icon
              return (
                <div key={p.id} className="bg-slate-950/50 border border-gray-800 rounded-lg p-3 space-y-2">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center space-x-2">
                      <Icon size={14} className={p.color} />
                      <span className="text-xs font-semibold text-white">{p.label}</span>
                      {caption && <Check size={10} className="text-emerald-400" />}
                    </div>
                    <div className="flex items-center space-x-1">
                      <button
                        onClick={() => handleGenerateCaption(p.id)}
                        disabled={isGenerating || !selectedProductId}
                        className="flex items-center space-x-1 px-2 py-0.5 bg-violet-600/20 hover:bg-violet-600/40 disabled:opacity-30 text-violet-300 rounded text-[10px] font-medium"
                      >
                        <Sparkle size={8} />
                        <span>{isThisGenerating ? '...' : 'AI'}</span>
                      </button>
                      {caption && (
                        <>
                          <button onClick={() => handleCopyCaption(p.id)} className="p-1 text-gray-500 hover:text-violet-400" title="Copy">
                            {copiedPlatform === p.id ? <Check size={10} className="text-emerald-400" /> : <Copy size={10} />}
                          </button>
                          <button onClick={() => handleEditCaption(p.id)} className="p-1 text-gray-500 hover:text-violet-400" title="Edit">
                            <Edit3 size={10} />
                          </button>
                          <button
                            onClick={() => handleShare(p.id)}
                            className="flex items-center space-x-1 px-2 py-0.5 bg-emerald-600/20 hover:bg-emerald-600/40 text-emerald-300 rounded text-[10px] font-medium"
                          >
                            <Share2 size={8} /><span>Share</span>
                          </button>
                        </>
                      )}
                    </div>
                  </div>
                  {caption ? (
                    isEditing ? (
                      <textarea
                        value={caption}
                        onChange={e => handleCaptionChange(p.id, e.target.value)}
                        onBlur={() => setEditingPlatform(null)}
                        rows={5}
                        autoFocus
                        className="w-full bg-black/30 border border-violet-500/30 rounded p-2 text-xs text-gray-200 focus:outline-none focus:border-violet-500"
                      />
                    ) : (
                      <p className="text-xs text-gray-300 whitespace-pre-wrap bg-black/20 rounded p-2 max-h-32 overflow-y-auto">{caption}</p>
                    )
                  ) : (
                    <p className="text-xs text-gray-600 italic py-2">Not generated yet. Click "AI" to generate.</p>
                  )}
                </div>
              )
            })}
          </div>
        </div>
      )}

      {/* === DRAFTS TAB === */}
      {activeTab === 'drafts' && (
        <div className="space-y-4">
          <div className="glass-card p-4">
            <h2 className="text-lg font-semibold text-white">Saved Drafts</h2>
            <p className="text-xs text-gray-400 mt-1">Load a draft to edit and share, or delete unwanted drafts.</p>
          </div>
          {drafts.length === 0 ? (
            <div className="glass-card p-8 text-center text-gray-500 text-sm">
              <Save size={24} className="mx-auto mb-2 text-gray-700" />
              No drafts saved yet. Generate captions and click "Save Drafts".
            </div>
          ) : (
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
              {drafts.map(d => (
                <div key={d.id} className="glass-card p-3 border border-gray-800">
                  <div className="flex items-start justify-between mb-2">
                    <div>
                      <h3 className="text-sm font-semibold text-white">{d.productName}</h3>
                      <p className="text-[10px] text-gray-500">{d.platform.replace(/_/g, ' ')} • {d.angle.replace(/_/g, ' ')}</p>
                    </div>
                    <button onClick={() => handleDeleteDraft(d.id)} className="p-1 text-gray-500 hover:text-red-400">
                      <Trash2 size={12} />
                    </button>
                  </div>
                  <p className="text-xs text-gray-400 line-clamp-3 mb-2">{d.content}</p>
                  <div className="flex items-center justify-between">
                    <span className="text-[10px] text-gray-600">{d.created_at}</span>
                    <button
                      onClick={() => handleLoadDraft(d)}
                      className="text-[10px] px-2 py-0.5 bg-violet-600/20 hover:bg-violet-600/40 text-violet-300 rounded font-medium"
                    >
                      Load & Edit
                    </button>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {/* === BULK BROADCAST TAB === */}
      {activeTab === 'broadcast' && (
        <div className="glass-card p-5 space-y-4 max-w-2xl">
          <div>
            <h2 className="text-lg font-semibold text-white">Bulk WhatsApp Broadcast</h2>
            <p className="text-xs text-gray-400 mt-1">Send the same message to all customers in a segment. Opens WhatsApp for each customer.</p>
          </div>
          <div>
            <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">Customer Segment</label>
            <select
              value={selectedSegment}
              onChange={e => { setSelectedSegment(e.target.value); loadSegmentCustomers(e.target.value) }}
              className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500"
            >
              <option value="">All Active Customers</option>
              {segments.map(s => (
                <option key={s} value={s}>{s.charAt(0).toUpperCase() + s.slice(1)}</option>
              ))}
            </select>
          </div>
          {selectedSegment && (
            <div className="bg-slate-950/50 border border-gray-800 rounded-lg p-3 text-xs text-gray-400">
              <div>Customers found: {segmentCustomers.length}</div>
              <div>With phone: {segmentCustomers.filter(c => c.phone && c.phone.trim()).length}</div>
            </div>
          )}
          <div>
            <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">Broadcast Message</label>
            <textarea
              value={bulkBroadcastText}
              onChange={e => setBulkBroadcastText(e.target.value)}
              rows={6}
              placeholder="Type your broadcast message here..."
              className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500"
            />
          </div>
          <button
            onClick={handleBulkBroadcast}
            disabled={bulkBroadcastLoading}
            className="w-full flex items-center justify-center space-x-2 px-4 py-2 bg-emerald-600 hover:bg-emerald-700 disabled:opacity-50 text-white rounded-lg text-sm font-medium transition-colors"
          >
            <Send size={14} />
            <span>{bulkBroadcastLoading ? 'Opening WhatsApp...' : 'Broadcast to Segment'}</span>
          </button>
        </div>
      )}

      {/* === STALE STOCK TAB === */}
      {activeTab === 'stale' && (
        <div className="space-y-4">
          <div className="glass-card p-4 flex items-center justify-between">
            <div>
              <h2 className="text-lg font-semibold text-white flex items-center">
                <AlertTriangle size={16} className="text-amber-400 mr-2" />
                Stale Stock — Not Shared in 7+ Days
              </h2>
              <p className="text-xs text-gray-400 mt-1">Products with stock but no recent shares. Generate captions and push them.</p>
            </div>
            <button onClick={loadStaleProducts} className="p-2 text-gray-400 hover:text-violet-400">
              <RefreshCw size={14} />
            </button>
          </div>
          {staleProducts.length === 0 ? (
            <div className="glass-card p-8 text-center text-gray-500 text-sm">
              <Check size={24} className="mx-auto mb-2 text-emerald-400" />
              All products have been shared recently!
            </div>
          ) : (
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
              {staleProducts.map(p => (
                <div key={p.id} className="glass-card p-3 border border-gray-800">
                  <div className="flex items-start justify-between">
                    <div>
                      <h3 className="text-sm font-semibold text-white">{p.name}</h3>
                      <p className="text-[10px] text-gray-500">{p.sku} • {fmtMoney(p.sale_price)}</p>
                    </div>
                    <span className="text-[10px] px-1.5 py-0.5 rounded bg-amber-900/40 text-amber-300">
                      {p.stock_quantity} in stock
                    </span>
                  </div>
                  <div className="mt-2 text-[10px] text-gray-500">
                    {p.last_shared_at ? `Last shared: ${fmtDate(p.last_shared_at)}` : 'Never shared'}
                  </div>
                  <button
                    onClick={() => {
                      setSelectedProductId(p.id)
                      setCaptions({})
                      setActiveTab('share_pack')
                    }}
                    className="mt-2 w-full flex items-center justify-center space-x-1 px-2 py-1 bg-violet-600/20 hover:bg-violet-600/40 text-violet-300 rounded text-[10px] font-medium"
                  >
                    <Sparkle size={10} />
                    <span>Generate Captions</span>
                  </button>
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {/* === HISTORY TAB === */}
      {activeTab === 'history' && (
        <div className="space-y-4">
          <div className="glass-card p-4 flex items-center justify-between">
            <h2 className="text-lg font-semibold text-white">Share History (Last 30)</h2>
            <button onClick={loadShareLogs} className="p-2 text-gray-400 hover:text-violet-400">
              <RefreshCw size={14} />
            </button>
          </div>
          {shareLogs.length === 0 ? (
            <div className="glass-card p-8 text-center text-gray-500 text-sm">
              No shares logged yet. Use AI Share Pack to generate and share.
            </div>
          ) : (
            <div className="glass-card overflow-hidden">
              <div className="overflow-x-auto">
                <table className="w-full text-xs">
                  <thead>
                    <tr className="border-b border-gray-800 text-gray-500">
                      <th className="text-left py-2 px-3">Date</th>
                      <th className="text-left py-2 px-3">Product</th>
                      <th className="text-left py-2 px-3">Platform</th>
                      <th className="text-left py-2 px-3">Angle</th>
                      <th className="text-left py-2 px-3 max-w-[200px]">Caption Preview</th>
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-gray-800">
                    {shareLogs.map(log => (
                      <tr key={log.id} className="text-gray-300">
                        <td className="py-2 px-3 text-gray-500">{fmtDate(log.shared_at)}</td>
                        <td className="py-2 px-3">{log.product_name}</td>
                        <td className="py-2 px-3">
                          <span className="px-1.5 py-0.5 rounded text-[10px] bg-slate-800 text-gray-300">
                            {log.platform.replace(/_/g, ' ')}
                          </span>
                        </td>
                        <td className="py-2 px-3 text-gray-500">{log.share_angle.replace(/_/g, ' ') || '—'}</td>
                        <td className="py-2 px-3 text-gray-500 max-w-[200px] truncate">{log.caption_text.slice(0, 80)}...</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  )
}
