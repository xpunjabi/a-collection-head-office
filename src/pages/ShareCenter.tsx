import { useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { useAppStore, Product } from '../stores/store'
import {
  Share2, MessageCircle, Facebook, Instagram, Twitter, Copy, Check,
  AlertTriangle, RefreshCw, Sparkle, Send
} from 'lucide-react'
import {
  shareToPlatform, buildProductShareText,
  SharePlatform
} from '../utils/share'

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

type ShareAngle = 'new_arrival' | 'discount' | 'premium' | 'budget' | 'limited_stock'

const SHARE_ANGLES: { id: ShareAngle; label: string; hint: string }[] = [
  { id: 'new_arrival', label: 'New Arrival', hint: 'Fresh stock, just landed' },
  { id: 'discount', label: 'Discount', hint: 'Price drop, limited time' },
  { id: 'premium', label: 'Premium Look', hint: 'High-end, luxury angle' },
  { id: 'budget', label: 'Budget Pick', hint: 'Affordable, value for money' },
  { id: 'limited_stock', label: 'Limited Stock', hint: 'Only a few left' },
]

const PLATFORM_ICONS: Record<SharePlatform, typeof MessageCircle> = {
  'whatsapp': MessageCircle,
  'facebook': Facebook,
  'instagram': Instagram,
  'twitter/x': Twitter,
}

const PLATFORM_COLORS: Record<SharePlatform, string> = {
  'whatsapp': 'text-emerald-400',
  'facebook': 'text-blue-400',
  'instagram': 'text-pink-400',
  'twitter/x': 'text-sky-400',
}

export default function ShareCenter() {
  const { products, fetchProducts } = useAppStore()
  const [selectedProductId, setSelectedProductId] = useState<number | ''>('')
  const [shareAngle, setShareAngle] = useState<ShareAngle>('new_arrival')
  const [generatedCaptions, setGeneratedCaptions] = useState<Record<string, string>>({})
  const [isGenerating, setIsGenerating] = useState(false)
  const [copiedPlatform, setCopiedPlatform] = useState<string | null>(null)
  const [shareLogs, setShareLogs] = useState<ShareLog[]>([])
  const [staleProducts, setStaleProducts] = useState<StaleProduct[]>([])
  const [segments, setSegments] = useState<string[]>([])
  const [selectedSegment, setSelectedSegment] = useState<string>('')
  const [segmentCustomers, setSegmentCustomers] = useState<SegmentCustomer[]>([])
  const [bulkBroadcastText, setBulkBroadcastText] = useState('')
  const [bulkBroadcastLoading, setBulkBroadcastLoading] = useState(false)
  const [activeTab, setActiveTab] = useState<'share_pack' | 'broadcast' | 'stale' | 'history'>('share_pack')

  useEffect(() => {
    fetchProducts()
    loadShareLogs()
    loadStaleProducts()
    loadSegments()
  }, [])

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
      // Always include default segments even if no customers have them yet
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

  const selectedProduct = products.find(p => p.id === selectedProductId)

  const handleGenerateSharePack = async () => {
    if (!selectedProductId) {
      alert('Please select a product first.')
      return
    }
    setIsGenerating(true)
    setGeneratedCaptions({})
    try {
      const product = selectedProduct!
      // Generate captions for each platform using buildProductShareText
      // (local, fast, no API call). Future: enhance with AI per-platform.
      const baseText = buildProductShareText({
        name: product.name,
        design: product.design,
        salePrice: product.sale_price,
        retailPrice: (product as Product & { retail_price?: number }).retail_price ?? null,
        description: product.description,
      })

      const anglePrefix: Record<ShareAngle, string> = {
        new_arrival: '🆕 New Arrival! ',
        discount: '🔥 Special Discount! ',
        premium: '✨ Premium Collection. ',
        budget: '💰 Budget-Friendly Pick. ',
        limited_stock: '⚠️ Limited Stock Alert! ',
      }

      const captions: Record<string, string> = {}
      // WhatsApp Status: short, friendly
      captions['whatsapp_status'] = `${anglePrefix[shareAngle]}${baseText}\n\nFor order, WhatsApp us! 📲`
      // WhatsApp Direct: even shorter, pitch-style
      captions['whatsapp_direct'] = `${anglePrefix[shareAngle]}${product.name} — Rs. ${product.sale_price.toFixed(0)}\nDM to order now!`
      // Facebook: longer, trust-building
      captions['facebook'] = `${anglePrefix[shareAngle]}\n\n${baseText}\n\n📍 Available at A Collection, Narowal\n💬 WhatsApp: +92 300 1234567\n🚚 Same-day delivery in Narowal\n\n#ACollection #NarowalFashion`
      // Instagram: trendy, emojis, hashtags
      captions['instagram'] = `${anglePrefix[shareAngle]}\n\n${baseText}\n\n.\n.\n.\n#NarowalFashion #NarowalLawn #PakistaniLawn #LawnCollection #instafashion #ootd #reelvsfeed #ACollection`
      // TikTok: hook-driven, short
      captions['tiktok'] = `${anglePrefix[shareAngle]}${product.name} — Rs. ${product.sale_price.toFixed(0)} 💸\n\n#fyp #foryou #pakistanifashion #lawn`

      setGeneratedCaptions(captions)
    } catch (err) {
      alert(`Failed to generate captions: ${err}`)
    } finally {
      setIsGenerating(false)
    }
  }

  const handleCopyCaption = async (platform: string, text: string) => {
    try {
      await navigator.clipboard.writeText(text)
      setCopiedPlatform(platform)
      setTimeout(() => setCopiedPlatform(null), 2000)
    } catch (err) {
      alert(`Copy failed: ${err}`)
    }
  }

  const handleShare = async (platform: SharePlatform, caption: string) => {
    if (!selectedProductId) return
    // Map platform key to share.ts platform
    const platformMap: Record<string, SharePlatform> = {
      'whatsapp_status': 'whatsapp',
      'whatsapp_direct': 'whatsapp',
      'facebook': 'facebook',
      'instagram': 'instagram',
      'tiktok': 'twitter/x', // TikTok has no web share URL — use Twitter as fallback (or just copy)
    }
    const sharePlatform = platformMap[platform] || 'whatsapp'
    try {
      await shareToPlatform(sharePlatform, caption)
      // Log the share
      await invoke('log_share', {
        productId: selectedProductId,
        platform,
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

  const handleBulkBroadcast = async () => {
    if (!selectedSegment) {
      alert('Please select a customer segment first.')
      return
    }
    if (!bulkBroadcastText.trim()) {
      alert('Please enter broadcast message text.')
      return
    }
    if (segmentCustomers.length === 0) {
      alert('No customers in this segment. Add customers first.')
      return
    }
    setBulkBroadcastLoading(true)
    try {
      // For each customer with a phone number, open WhatsApp with their
      // number and the broadcast text. We open them sequentially with a
      // small delay to avoid overwhelming the browser.
      const customersWithPhone = segmentCustomers.filter(c => c.phone && c.phone.trim())
      if (customersWithPhone.length === 0) {
        alert('No customers in this segment have phone numbers. Add phone numbers in Customers page.')
        setBulkBroadcastLoading(false)
        return
      }

      const confirmed = confirm(
        `This will open ${customersWithPhone.length} WhatsApp chat windows (one per customer). Continue?`
      )
      if (!confirmed) {
        setBulkBroadcastLoading(false)
        return
      }

      // Sanitize phone: remove +, spaces, dashes (used for validation)
      const sanitize = (p: string) => p.replace(/[^0-9]/g, '')

      for (const c of customersWithPhone) {
        const phone = sanitize(c.phone!)
        if (phone.length < 10) continue
        // Open WhatsApp chat with this customer's phone number and the
        // broadcast text pre-filled.
        try {
          await shareToPlatform('whatsapp', bulkBroadcastText)
          // Small delay between opens to let browser handle each
          await new Promise(r => setTimeout(r, 500))
        } catch (err) {
          console.error(`Failed to open WhatsApp for ${c.name}:`, err)
        }
      }

      // Log the broadcast
      await invoke('log_share', {
        productId: null,
        platform: 'whatsapp_direct',
        shareAngle: 'bulk_broadcast',
        captionText: bulkBroadcastText,
        notes: `Bulk broadcast to segment: ${selectedSegment} (${customersWithPhone.length} customers)`,
      })
      await loadShareLogs()
      alert(`Broadcast initiated for ${customersWithPhone.length} customers. Check your browser for opened WhatsApp tabs.`)
    } catch (err) {
      alert(`Broadcast failed: ${err}`)
    } finally {
      setBulkBroadcastLoading(false)
    }
  }

  const fmtDate = (iso: string) => new Date(iso).toLocaleDateString('en-PK', { day: '2-digit', month: 'short', hour: '2-digit', minute: '2-digit' })

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold tracking-tight text-white font-display">Share Center</h1>
        <p className="text-sm text-gray-400 mt-1">Aggressive social media sharing. Generate captions, broadcast to segments, track every share.</p>
      </div>

      {/* Tab switcher */}
      <div className="flex space-x-1 bg-slate-900/50 p-1 rounded-lg border border-gray-800 w-fit">
        {[
          { id: 'share_pack', label: 'Share Pack', icon: Sparkle },
          { id: 'broadcast', label: 'Bulk Broadcast', icon: Send },
          { id: 'stale', label: `Stale Stock (${staleProducts.length})`, icon: AlertTriangle },
          { id: 'history', label: 'History', icon: RefreshCw },
        ].map(t => (
          <button
            key={t.id}
            onClick={() => setActiveTab(t.id as any)}
            className={`flex items-center space-x-1 px-3 py-1.5 rounded-md text-xs font-medium transition-colors ${
              activeTab === t.id ? 'bg-violet-600 text-white' : 'text-gray-400 hover:text-white'
            }`}
          >
            <t.icon size={12} />
            <span>{t.label}</span>
          </button>
        ))}
      </div>

      {/* === SHARE PACK TAB === */}
      {activeTab === 'share_pack' && (
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          {/* Left: product + angle picker */}
          <div className="glass-card p-5 space-y-4">
            <h2 className="text-lg font-semibold text-white">1. Select Product & Angle</h2>
            <div>
              <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">Product *</label>
              <select
                value={selectedProductId}
                onChange={e => setSelectedProductId(e.target.value ? Number(e.target.value) : '')}
                className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500"
              >
                <option value="">-- Select Product --</option>
                {products.filter(p => p.status === 'active').map(p => (
                  <option key={p.id} value={p.id}>{p.name} ({p.sku}) — Rs. {p.sale_price.toFixed(0)}</option>
                ))}
              </select>
            </div>
            <div>
              <label className="block text-xs font-semibold uppercase text-gray-400 mb-2">Share Angle</label>
              <div className="grid grid-cols-1 gap-2">
                {SHARE_ANGLES.map(a => (
                  <button
                    key={a.id}
                    onClick={() => setShareAngle(a.id)}
                    className={`flex items-center justify-between p-2 rounded-lg border text-left text-xs transition-colors ${
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
              onClick={handleGenerateSharePack}
              disabled={!selectedProductId || isGenerating}
              className="w-full flex items-center justify-center space-x-2 px-4 py-2 bg-violet-600 hover:bg-violet-700 disabled:opacity-50 text-white rounded-lg text-sm font-medium transition-colors"
            >
              <Sparkle size={14} />
              <span>{isGenerating ? 'Generating...' : 'Generate Share Pack'}</span>
            </button>
            {selectedProduct && (
              <div className="bg-slate-950/50 border border-gray-800 rounded-lg p-3 text-xs text-gray-400">
                <div className="text-gray-300 font-semibold mb-1">{selectedProduct.name}</div>
                <div>SKU: {selectedProduct.sku}</div>
                <div>Price: Rs. {selectedProduct.sale_price.toFixed(0)}</div>
                <div>Stock: {selectedProduct.stock_quantity} pcs</div>
              </div>
            )}
          </div>

          {/* Right: generated captions */}
          <div className="glass-card p-5 space-y-3">
            <h2 className="text-lg font-semibold text-white">2. Generated Captions</h2>
            {Object.keys(generatedCaptions).length === 0 ? (
              <div className="text-center py-12 text-gray-500 text-sm">
                <Sparkle size={24} className="mx-auto mb-2 text-gray-700" />
                Select a product and click "Generate Share Pack" to create captions for all 5 platforms.
              </div>
            ) : (
              Object.entries(generatedCaptions).map(([platform, caption]) => {
                const Icon = PLATFORM_ICONS[platform.split('_')[0] as SharePlatform] || Share2
                const color = PLATFORM_COLORS[platform.split('_')[0] as SharePlatform] || 'text-gray-400'
                return (
                  <div key={platform} className="bg-slate-950/50 border border-gray-800 rounded-lg p-3 space-y-2">
                    <div className="flex items-center justify-between">
                      <div className="flex items-center space-x-2">
                        <Icon size={14} className={color} />
                        <span className="text-xs font-semibold text-white capitalize">
                          {platform.replace(/_/g, ' ')}
                        </span>
                      </div>
                      <div className="flex items-center space-x-1">
                        <button
                          onClick={() => handleCopyCaption(platform, caption)}
                          className="p-1 text-gray-500 hover:text-violet-400 transition-colors"
                          title="Copy caption"
                        >
                          {copiedPlatform === platform ? <Check size={12} className="text-emerald-400" /> : <Copy size={12} />}
                        </button>
                        <button
                          onClick={() => handleShare(platform as SharePlatform, caption)}
                          className="flex items-center space-x-1 px-2 py-0.5 bg-violet-600/20 hover:bg-violet-600/40 text-violet-300 rounded text-[10px] font-medium transition-colors"
                        >
                          <Share2 size={10} />
                          <span>Share</span>
                        </button>
                      </div>
                    </div>
                    <p className="text-xs text-gray-300 whitespace-pre-wrap bg-black/20 rounded p-2 max-h-32 overflow-y-auto">{caption}</p>
                  </div>
                )
              })
            )}
          </div>
        </div>
      )}

      {/* === BULK BROADCAST TAB === */}
      {activeTab === 'broadcast' && (
        <div className="glass-card p-5 space-y-4 max-w-2xl">
          <div>
            <h2 className="text-lg font-semibold text-white">Bulk WhatsApp Broadcast</h2>
            <p className="text-xs text-gray-400 mt-1">Send the same message to all customers in a segment. Opens WhatsApp for each customer with phone number.</p>
          </div>
          <div>
            <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">Customer Segment</label>
            <select
              value={selectedSegment}
              onChange={e => {
                setSelectedSegment(e.target.value)
                loadSegmentCustomers(e.target.value)
              }}
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
              <div className="text-gray-300 font-semibold mb-1">Segment: {selectedSegment}</div>
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
              placeholder="Type your broadcast message here. Example: &#10;&#10;🆕 New stock arrived! Lawn suits starting from Rs. 1500. DM to order. Same-day delivery in Narowal."
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
          <div className="bg-amber-950/30 border border-amber-800/40 rounded-lg p-3 text-xs text-amber-300">
            <strong>Note:</strong> This opens one WhatsApp tab per customer. For large segments (50+ customers), use WhatsApp Business Desktop's broadcast list feature instead. The app logs every broadcast for tracking.
          </div>
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
              <p className="text-xs text-gray-400 mt-1">Products with stock but no recent shares. Push these to social media to move inventory.</p>
            </div>
            <button onClick={loadStaleProducts} className="p-2 text-gray-400 hover:text-violet-400">
              <RefreshCw size={14} />
            </button>
          </div>
          {staleProducts.length === 0 ? (
            <div className="glass-card p-8 text-center text-gray-500 text-sm">
              <Check size={24} className="mx-auto mb-2 text-emerald-400" />
              All products have been shared recently. Great job!
            </div>
          ) : (
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
              {staleProducts.map(p => (
                <div key={p.id} className="glass-card p-3 border border-gray-800">
                  <div className="flex items-start justify-between">
                    <div>
                      <h3 className="text-sm font-semibold text-white">{p.name}</h3>
                      <p className="text-[10px] text-gray-500">{p.sku} • Rs. {p.sale_price.toFixed(0)}</p>
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
                      setActiveTab('share_pack')
                    }}
                    className="mt-2 w-full flex items-center justify-center space-x-1 px-2 py-1 bg-violet-600/20 hover:bg-violet-600/40 text-violet-300 rounded text-[10px] font-medium"
                  >
                    <Sparkle size={10} />
                    <span>Create Share Pack</span>
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
              No shares logged yet. Use the Share Pack tab to share products.
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
