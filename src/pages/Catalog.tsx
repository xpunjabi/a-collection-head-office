import React, { useEffect, useState, useRef, useCallback } from 'react'
import { useAppStore, Product } from '../stores/store'
import { open } from '@tauri-apps/plugin-dialog'
import { invoke } from '@tauri-apps/api/core'
import {
  Search, Download, Upload, Plus, Edit, Trash2, Image as ImageIcon,
  X, Palette, MapPin, Share2, ChevronDown, CheckSquare, Square,
  MessageCircle, Facebook, Instagram, Twitter, ShoppingCart, Globe, Link2
} from 'lucide-react'
import ProductImage from '../components/ProductImage'
import {
  shareToPlatform, buildProductShareText,
  ALL_SHARE_PLATFORMS, PLATFORM_LABELS, SharePlatform
} from '../utils/share'

interface AgentStockEntry {
  agent_id: number;
  agent_name: string;
  quantity: number;
}

const SEASONS = ['', 'Summer', 'Winter', 'Eid Special', 'Festive', 'Spring', 'Autumn']

// v0.14.2: Simplified Category — suit type only (not fabric mixed in)
const CATEGORIES = ['', '3 Piece', '2 Piece', 'Cut Piece', 'Gents']

// v0.14.2: Dedicated Fabric dropdown — 24 fabric types
const FABRICS = ['', 'Lawn', 'Cotton', 'Latha', 'Wash and Wear', 'Boski', 'Cambric',
  'Jacquard', 'Voile', 'Linen', 'Khaddar', 'Karandi', 'Marina', 'Viscose',
  'Wool', 'Tweed', 'Velvet', 'Pashmina', 'Chiffon', 'Organza', 'Silk',
  'Net', 'Georgette', 'Jamawar', 'Tissue', 'Satin']

// v0.14.2: Design Type dropdown
const DESIGN_TYPES = ['', 'Digital Print', 'Block Print', 'Screen Print',
  'Machine Embroidery', 'Hand Embroidery', 'Chikankari', 'Zari/Tilla Work',
  'Solid Plain', 'Self-Texture']

// v0.14.2: Gender dropdown
const GENDERS = ['', 'Ladies', 'Gents', 'Kids']

export default function Catalog() {
  const { products, fetchProducts, addProduct, updateProduct, deleteProduct,
    exportProductsCsv, importProductsCsv, uploadProductImage } = useAppStore()

  const [searchTerm, setSearchTerm] = useState('')
  const [selectedCategory, setSelectedCategory] = useState('')
  const [colorSearch, setColorSearch] = useState('')
  const [showModal, setShowModal] = useState(false)
  const [editProduct, setEditProduct] = useState<Product | null>(null)
  const [agentStock, setAgentStock] = useState<AgentStockEntry[]>([])
  // v0.16.3: Track ORIGINAL agent stock values when editing so we can
  // compare on save and send/return the difference. Previously agent
  // stock was only saved for NEW products — editing existing products
  // silently dropped any changes.
  const [originalAgentStock, setOriginalAgentStock] = useState<Record<number, number>>({})

  // Form states
  const [productCode, setProductCode] = useState('')
  const [name, setName] = useState('')
  const [category, setCategory] = useState('')
  const [color, setColor] = useState('')
  const [design, setDesign] = useState('')
  const [season, setSeason] = useState('')
  const [fabric, setFabric] = useState('')
  const [designType, setDesignType] = useState('')
  const [gender, setGender] = useState('')
  // v0.14.10: Use string state for price/qty inputs so backspace can clear
  // them. Previously Number(e.target.value) returned 0 for empty string,
  // making the '0' impossible to clear with backspace. Parse on save.
  const [costPrice, setCostPrice] = useState('')
  const [salePrice, setSalePrice] = useState('')
  // v0.14.3: purchasePrice removed — was redundant with costPrice, never
  // meaningfully used. Retail/sale/cost are the 3 prices that matter.
  const [retailPrice, setRetailPrice] = useState('')
  const [description, setDescription] = useState('')
  const [tags, setTags] = useState('')
  const [stockQuantity, setStockQuantity] = useState('')
  const [status, setStatus] = useState('active')
  const [images, setImages] = useState<string[]>([])

  // Multi-select + per-row share dropdown state
  const [selectedIds, setSelectedIds] = useState<Set<number>>(new Set())
  const [openShareMenuFor, setOpenShareMenuFor] = useState<number | null>(null)
  const [bulkShareOpen, setBulkShareOpen] = useState(false)
  const shareMenuRef = useRef<HTMLDivElement>(null)
  const bulkShareRef = useRef<HTMLDivElement>(null)

  // v0.12.5: Sale modal state
  const [showSaleModal, setShowSaleModal] = useState(false)
  const [saleProduct, setSaleProduct] = useState<Product | null>(null)
  const [saleMode, setSaleMode] = useState<'direct' | 'agent'>('direct')
  const [saleQty, setSaleQty] = useState(1)
  const [saleUnitPrice, setSaleUnitPrice] = useState(0)
  const [saleChannel, setSaleChannel] = useState('head_office')
  const [saleAgentId, setSaleAgentId] = useState<number | ''>('')
  const [saleCustomerName, setSaleCustomerName] = useState('')
  const [saleCustomerPhone, setSaleCustomerPhone] = useState('')
  const [saleNotes, setSaleNotes] = useState('')
  const [saleSaving, setSaleSaving] = useState(false)
  const [agents, setAgents] = useState<{agent: {id: number; name: string; city?: string}}[]>([])

  // v0.15.0: Publish to Catalog state
  const [publishPreview, setPublishPreview] = useState<any>(null)
  const [publishing, setPublishing] = useState(false)
  const [publishResult, setPublishResult] = useState<any>(null)
  // v0.16.0: Force-publish even if errors exist
  const [forcePublish, setForcePublish] = useState(false)

  useEffect(() => { fetchProducts() }, [])

  // Load agents for the sale modal dropdown
  useEffect(() => {
    invoke('get_agents').then((data: any) => setAgents(data || [])).catch(() => {})
  }, [])

  const handleOpenSaleModal = (p: Product) => {
    setSaleProduct(p)
    setSaleMode('direct')
    setSaleQty(1)
    setSaleUnitPrice(p.sale_price)
    setSaleChannel('head_office')
    setSaleAgentId('')
    setSaleCustomerName('')
    setSaleCustomerPhone('')
    setSaleNotes('')
    setShowSaleModal(true)
  }

  const handleRecordSale = async () => {
    if (!saleProduct?.id) return
    if (saleQty <= 0) { alert('Quantity must be positive.'); return }
    setSaleSaving(true)
    try {
      await invoke('record_sale', {
        productId: saleProduct.id,
        qty: saleQty,
        unitSalePrice: saleUnitPrice,
        saleChannel: saleMode === 'agent' ? 'agent' : saleChannel,
        agentId: saleMode === 'agent' && saleAgentId !== '' ? Number(saleAgentId) : null,
        customerName: saleCustomerName || null,
        customerPhone: saleCustomerPhone || null,
        notes: saleNotes || null,
      })
      setShowSaleModal(false)
      await fetchProducts()
      alert(`Sale recorded! ${saleQty} x ${saleProduct.name} = Rs. ${(saleQty * saleUnitPrice).toFixed(0)}`)
    } catch (err) {
      alert(`Error: ${err}`)
    } finally {
      setSaleSaving(false)
    }
  }

  // Close share menus when clicking outside
  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (shareMenuRef.current && !shareMenuRef.current.contains(e.target as Node)) {
        setOpenShareMenuFor(null)
      }
      if (bulkShareRef.current && !bulkShareRef.current.contains(e.target as Node)) {
        setBulkShareOpen(false)
      }
    }
    document.addEventListener('mousedown', handler)
    return () => document.removeEventListener('mousedown', handler)
  }, [])

  const toggleSelect = (id: number) => {
    setSelectedIds(prev => {
      const next = new Set(prev)
      if (next.has(id)) next.delete(id)
      else next.add(id)
      return next
    })
  }

  const toggleSelectAll = () => {
    setSelectedIds(prev => {
      if (prev.size === filteredProducts.length) return new Set()
      return new Set(filteredProducts.filter(p => p.id).map(p => p.id!))
    })
  }

  const clearSelection = () => setSelectedIds(new Set())

  const handleShareProduct = async (p: Product, platform: SharePlatform) => {
    // Product type doesn't have retail_price as a typed field, but the DB
    // schema does store it. Read it via optional cast for the discount calc.
    const retailPrice = (p as Product & { retail_price?: number }).retail_price ?? null
    const text = buildProductShareText({
      name: p.name,
      design: p.design,
      salePrice: p.sale_price,
      retailPrice,
      description: p.description,
      hashtags: p.tags ? (() => { try { return JSON.parse(p.tags) } catch { return null } })() : null,
      includeHashtags: true,
    })
    setOpenShareMenuFor(null)
    try {
      await shareToPlatform(platform, text, null, p.name)
    } catch (err) {
      console.error('[Catalog] share failed:', err)
      alert(`Could not open ${platform}. Error: ${err}`)
    }
  }

  const handleBulkShare = async (platform: SharePlatform) => {
    const selected = products.filter(p => p.id && selectedIds.has(p.id))
    if (selected.length === 0) return
    // Build a combined message: each product on its own block, separated by ---
    const blocks = selected.map((p, idx) => {
      const retailPrice = (p as Product & { retail_price?: number }).retail_price ?? null
      const text = buildProductShareText({
        name: p.name,
        design: p.design,
        salePrice: p.sale_price,
        retailPrice,
        description: p.description,
      })
      return `${idx + 1}. ${text}`
    })
    const combined = blocks.join('\n\n---\n\n')
    setBulkShareOpen(false)
    try {
      await shareToPlatform(platform, combined)
    } catch (err) {
      console.error('[Catalog] bulk share failed:', err)
      alert(`Could not open ${platform}. Error: ${err}`)
    }
  }

  const handleOpenAdd = async () => {
    setEditProduct(null)
    setProductCode(''); setName(''); setCategory(''); setColor(''); setDesign('')
    setSeason(''); setFabric(''); setDesignType(''); setGender('')
    setCostPrice(''); setSalePrice(''); setRetailPrice('')
    setDescription(''); setTags(''); setStockQuantity(''); setStatus('active'); setImages([])
    // v0.13.7: Load agents instead of locations for Agent Stock section
    try {
      const agents: any[] = await invoke('get_agents')
      setAgentStock(agents.map((a: any) => ({ agent_id: a.agent.id, agent_name: a.agent.name, quantity: 0 })))
    } catch { setAgentStock([]) }
    setShowModal(true)
  }

  const handleOpenEdit = async (p: Product) => {
    setEditProduct(p)
    setProductCode(p.sku || ''); setName(p.name)
    setCategory(p.category || ''); setColor(p.color || ''); setDesign(p.design || '')
    setSeason(p.season || ''); setFabric((p as any).fabric || '')

    // v0.16.2: Parse designType + gender from tags (they're stored there
    // comma-separated with user tags). Previously these were hardcoded to ''
    // on edit, so the dropdowns always showed empty even though the data
    // was saved. Now we extract them.
    const DESIGN_TYPES = ['Digital Print', 'Block Print', 'Screen Print',
      'Machine Embroidery', 'Hand Embroidery', 'Chikankari', 'Zari/Tilla Work',
      'Solid Plain', 'Self-Texture']
    const GENDERS = ['Ladies', 'Gents', 'Kids']
    const tagParts = (p.tags || '').split(',').map(t => t.trim()).filter(Boolean)
    let parsedDesignType = ''
    let parsedGender = ''
    const remainingTags: string[] = []
    for (const part of tagParts) {
      if (DESIGN_TYPES.includes(part)) {
        parsedDesignType = part
      } else if (GENDERS.includes(part)) {
        parsedGender = part
      } else {
        remainingTags.push(part)
      }
    }
    setDesignType(parsedDesignType)
    setGender(parsedGender)
    setTags(remainingTags.join(', '))
    // v0.14.10: Use string state — if value is 0, show empty string so the
    // field is clearable. Number conversion happens on save.
    setCostPrice(p.cost_price ? String(p.cost_price) : '')
    setSalePrice(p.sale_price ? String(p.sale_price) : '')
    // v0.14.3: Don't fall back to sale_price for retail_price — that was
    // the cause of the recurring "Save Rs. 0" bug. If retail_price is
    // null/0 in DB, leave the field empty so user can enter a real value.
    setRetailPrice((p as any).retail_price ? String((p as any).retail_price) : '')
    setDescription(p.description || ''); setTags(p.tags || '')
    setStockQuantity(p.stock_quantity ? String(p.stock_quantity) : '')
    setStatus(p.status)
    try { setImages(JSON.parse(p.images || '[]')) } catch { setImages([]) }
    // v0.14.10: Load ACTUAL agent stock for this product (was hardcoded 0).
    // v0.16.3: Also save ORIGINAL values so handleSave can compare and
    // send/return the difference (not just for new products).
    try {
      if (p.id) {
        const agentStockData: any[] = await invoke('get_product_agent_stock', { productId: p.id })
        const stockEntries = agentStockData.map((a: any) => ({ agent_id: a.agent_id, agent_name: a.agent_name, quantity: a.quantity }))
        setAgentStock(stockEntries)
        // Save original values for comparison on save
        const origMap: Record<number, number> = {}
        stockEntries.forEach((s: any) => { origMap[s.agent_id] = s.quantity })
        setOriginalAgentStock(origMap)
      } else {
        const agents: any[] = await invoke('get_agents')
        setAgentStock(agents.map((a: any) => ({ agent_id: a.agent.id, agent_name: a.agent.name, quantity: 0 })))
        setOriginalAgentStock({})
      }
    } catch {
      // Fallback: just load agents with 0 quantity
      try {
        const agents: any[] = await invoke('get_agents')
        setAgentStock(agents.map((a: any) => ({ agent_id: a.agent.id, agent_name: a.agent.name, quantity: 0 })))
      } catch { setAgentStock([]) }
    }
    setShowModal(true)
  }

  const handleAgentStockChange = (agentId: number, quantity: number) => {
    setAgentStock(prev => prev.map(a => a.agent_id === agentId ? { ...a, quantity } : a))
  }

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault()
    // v0.16.2: Auto-generate SKU if user left product code empty.
    // Format: AC-<timestamp> (same as AI draft path in Rust).
    // User should never get blocked by "product code required" — the system
    // assigns one automatically if they don't provide it.
    if (!name) return  // Name is still required
    const finalProductCode = productCode.trim() || `AC-${Date.now()}`

    // v0.14.10: Parse string state to numbers (empty string → 0).
    // Previously state was already a number, but backspace couldn't
    // clear the field because Number('') = 0 wrote 0 back.
    const costPriceNum = Number(costPrice) || 0
    const salePriceNum = Number(salePrice) || 0
    const retailPriceNum = Number(retailPrice) || 0
    const stockQuantityNum = Number(stockQuantity) || 0

    const productData: Product = {
      id: editProduct?.id,
      sku: finalProductCode,
      name,
      category: category || undefined,
      color: color || undefined,
      design: design || undefined,
      season: season || undefined,
      // v0.14.2: Save fabric, designType, gender
      fabric: fabric || undefined,
      // Store designType in tags if set (reuse existing column)
      tags: [tags, designType, gender].filter(Boolean).join(', ') || undefined,
      cost_price: costPriceNum,
      sale_price: salePriceNum,
      // v0.14.3: purchase_price removed from form — keep cost_price as the
      // value for the legacy DB column (still in schema for backward compat).
      purchase_price: costPriceNum,
      description: description || undefined,
      stock_quantity: stockQuantityNum,
      status,
      images: JSON.stringify(images),
      created_at: editProduct?.created_at,
      updated_at: editProduct?.updated_at,
      // v0.14.3: retail_price now persisted directly to products table via
      // add_product/update_product (Rust side updated in this version).
      // Removed the `update_setting({ key: '_product_retail_<id>' })` hack
      // that wrote to the settings table — ShareCenter never read it from
      // there, so retail_price always came back as null/0 from get_products.
      // Use 0 → undefined so SQLite stores NULL (not 0) when user leaves it
      // blank, letting ShareCenter's `sale_price * 1.2` fallback kick in.
      retail_price: retailPriceNum > 0 ? retailPriceNum : undefined,
      brand: editProduct?.brand,
    }

    try {
      let productId: number
      if (editProduct?.id) {
        await updateProduct(productData)
        productId = editProduct.id
      } else {
        productId = await addProduct(productData) as unknown as number
      }
      // v0.13.7: Agent stock — for NEW products, send initial allocation.
      // v0.16.3: For EXISTING products, compare current values with original
      // and send/return the difference. Previously editing agent stock on
      // existing products was silently ignored.
      if (!editProduct?.id) {
        // NEW product — send initial allocation
        for (const ag of agentStock) {
          if (ag.quantity > 0) {
            try {
              await invoke('send_stock_to_agent', {
                agentId: ag.agent_id,
                productId,
                qty: ag.quantity,
                unitPrice: costPriceNum,
                notes: 'Initial stock from Catalog form',
              })
            } catch (err) { console.warn(`Failed to send stock to agent ${ag.agent_name}:`, err) }
          }
        }
      } else {
        // EXISTING product — compare with original and send/return difference
        for (const ag of agentStock) {
          const originalQty = originalAgentStock[ag.agent_id] || 0
          const newQty = ag.quantity
          const diff = newQty - originalQty

          if (diff > 0) {
            // Stock increased — send more to agent
            try {
              await invoke('send_stock_to_agent', {
                agentId: ag.agent_id,
                productId,
                qty: diff,
                unitPrice: costPriceNum,
                notes: 'Stock adjustment from Catalog form edit',
              })
            } catch (err) { console.warn(`Failed to send stock to agent ${ag.agent_name}:`, err) }
          } else if (diff < 0) {
            // Stock decreased — return from agent
            try {
              await invoke('return_stock_from_agent', {
                agentId: ag.agent_id,
                productId,
                qty: -diff,  // return_stock expects positive qty
                unitPrice: costPriceNum,
                notes: 'Stock adjustment from Catalog form edit',
              })
            } catch (err) { console.warn(`Failed to return stock from agent ${ag.agent_name}:`, err) }
          }
        }
      }
      setShowModal(false)
    } catch (err) { alert(`Error: ${err}`) }
  }

  const handleDelete = async (id: number) => {
    if (confirm('Delete this product?')) { try { await deleteProduct(id) } catch (err) { alert(err) } }
  }

  const handleSelectImage = async () => {
    try {
      const selected = await open({ multiple: false, filters: [{ name: 'Images', extensions: ['png', 'jpg', 'jpeg', 'webp'] }] })
      if (selected && typeof selected === 'string') {
        const newFileName = await uploadProductImage(selected, 'thumbnail')
        setImages([...images, newFileName])
      }
    } catch (err) { console.error(err) }
  }

  // v0.24.1: Add image from URL (FB/IG/TikTok product page, brand site, etc.)
  // Rust downloads the image with browser-like headers to bypass hotlink
  // protection, then runs it through the same resize/compress pipeline as
  // uploaded images.
  const handleAddImageFromUrl = async () => {
    const url = prompt('Paste image URL (https://...):')
    if (!url || !url.trim()) return
    if (!/^https?:\/\//i.test(url.trim())) {
      alert('Please paste a valid URL starting with http:// or https://')
      return
    }
    try {
      const savedName = await invoke<string>('save_image_from_url', {
        url: url.trim(),
        formatType: 'thumbnail',
      })
      setImages(prev => [...prev, savedName])
    } catch (err) {
      alert(`Failed to download image: ${err}`)
    }
  }

  // v0.14.0: Paste image from clipboard (Ctrl+V) in Catalog form
  const handlePasteImage = useCallback(async (e: React.ClipboardEvent) => {
    const items = e.clipboardData?.items
    if (!items) return
    for (const item of items) {
      if (item.type.startsWith('image/')) {
        e.preventDefault()
        const file = item.getAsFile()
        if (!file) continue
        // Convert to base64
        const reader = new FileReader()
        reader.onload = async () => {
          const base64 = reader.result as string
          // Strip data URI prefix
          const cleanBase64 = base64.includes(',') ? base64.split(',')[1] : base64
          try {
            const savedName = await invoke<string>('save_base64_image', {
              base64Data: cleanBase64,
              formatType: 'thumbnail',
            })
            setImages(prev => [...prev, savedName])
          } catch (err) {
            alert(`Failed to save pasted image: ${err}`)
          }
        }
        reader.readAsDataURL(file)
        return
      }
    }
  }, [])

  // v0.14.0: Drag-drop image into Catalog form
  const handleImageDrop = useCallback(async (e: React.DragEvent) => {
    e.preventDefault()
    const files = e.dataTransfer?.files
    if (!files || files.length === 0) return
    for (const file of Array.from(files)) {
      if (!file.type.startsWith('image/')) continue
      const reader = new FileReader()
      reader.onload = async () => {
        const base64 = reader.result as string
        const cleanBase64 = base64.includes(',') ? base64.split(',')[1] : base64
        try {
          const savedName = await invoke<string>('save_base64_image', {
            base64Data: cleanBase64,
            formatType: 'thumbnail',
          })
          setImages(prev => [...prev, savedName])
        } catch (err) {
          alert(`Failed to save dropped image: ${err}`)
        }
      }
      reader.readAsDataURL(file)
    }
  }, [])

  const handleRemoveImage = (index: number) => setImages(images.filter((_, i) => i !== index))

  // v0.24.1: Document-level paste listener — fires whenever the Add/Edit
  // Product modal is open. The previous per-div onPaste handler only fired
  // when the div itself had focus, which never happens in Tauri webview
  // (divs aren't focusable without tabIndex). This document-level approach
  // catches Ctrl+V anywhere in the window while the modal is open, then
  // reuses the same handlePasteImage logic.
  useEffect(() => {
    if (!showModal) return
    const onDocPaste = (e: ClipboardEvent) => {
      if (!e.clipboardData?.items) return
      // Skip if user is typing in an input/textarea — let it paste text
      const target = e.target as HTMLElement | null
      if (target && (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA')) {
        return
      }
      const hasImage = Array.from(e.clipboardData.items).some(it => it.type.startsWith('image/'))
      if (!hasImage) return
      // Wrap into a React-like event shape and delegate to existing handler.
      e.preventDefault()
      const synthetic = {
        clipboardData: e.clipboardData,
        preventDefault: () => e.preventDefault(),
      } as unknown as React.ClipboardEvent
      handlePasteImage(synthetic)
    }
    document.addEventListener('paste', onDocPaste)
    return () => document.removeEventListener('paste', onDocPaste)
  }, [showModal, handlePasteImage])

  const handleCsvExport = async () => {
    try {
      const csvContent = await exportProductsCsv()
      const blob = new Blob([csvContent], { type: 'text/csv;charset=utf-8;' })
      const url = URL.createObjectURL(blob)
      const link = document.createElement('a')
      link.setAttribute('href', url)
      link.setAttribute('download', `catalog_${Date.now()}.csv`)
      document.body.appendChild(link); link.click(); document.body.removeChild(link)
    } catch (err) { alert(err) }
  }

  // v0.15.0: Publish to Catalog handlers
  const handlePublishPreview = async () => {
    try {
      const preview = await invoke('preview_catalog_publish')
      setPublishPreview(preview)
    } catch (err) {
      alert(`Failed to preview: ${err}`)
    }
  }

  const handlePublishConfirm = async () => {
    setPublishing(true)
    setPublishResult(null)
    try {
      const result = await invoke('publish_catalog_to_github')
      setPublishResult(result)
      setPublishPreview(null)
    } catch (err) {
      setPublishResult({ success: false, errors: [String(err)] })
    } finally {
      setPublishing(false)
    }
  }

  const handleCsvImport = async () => {
    try {
      const selected = await open({ multiple: false, filters: [{ name: 'CSV', extensions: ['csv'] }] })
      if (selected && typeof selected === 'string') {
        const { readTextFile } = await import('@tauri-apps/plugin-fs')
        const content = await readTextFile(selected)
        await importProductsCsv(content)
        alert('Imported!')
      }
    } catch (err) { alert(`Import failed: ${err}`) }
  }

  // --- Derived data ---
  const filteredProducts = products.filter(p => {
    const s = searchTerm.toLowerCase()
    const matchSearch = !s || p.name.toLowerCase().includes(s) || p.sku.toLowerCase().includes(s) || (p.color || '').toLowerCase().includes(s)
    const matchCat = !selectedCategory || p.category === selectedCategory
    const matchColor = !colorSearch || (p.color || '').toLowerCase().includes(colorSearch.toLowerCase())
    return matchSearch && matchCat && matchColor
  })

  const categories = Array.from(new Set(products.map(p => p.category).filter((c): c is string => !!c)))

  return (
    <div className="space-y-6">
      <div className="flex flex-col md:flex-row md:items-center md:justify-between gap-4">
        <div>
          <h1 className="text-3xl font-bold tracking-tight text-white font-display">Product Catalog</h1>
          <p className="text-sm text-gray-400 mt-1">Manage inventory with color, design, location tracking.</p>
        </div>
        <div className="flex items-center space-x-2 flex-wrap">
          {/* v0.15.0: Publish to Catalog button */}
          <button
            onClick={handlePublishPreview}
            className="flex items-center space-x-1 px-3 py-2 bg-emerald-600 hover:bg-emerald-700 text-white rounded-lg text-sm font-medium"
            title="Publish to public catalog website"
          >
            <Globe size={16} /><span>Publish</span>
          </button>
          <button onClick={handleCsvImport} className="flex items-center space-x-1 px-3 py-2 bg-slate-800 hover:bg-slate-700 text-gray-200 border border-gray-700 rounded-lg text-sm">
            <Upload size={16} /><span>Import</span>
          </button>
          <button onClick={handleCsvExport} className="flex items-center space-x-1 px-3 py-2 bg-slate-800 hover:bg-slate-700 text-gray-200 border border-gray-700 rounded-lg text-sm">
            <Download size={16} /><span>Export</span>
          </button>
          <button onClick={handleOpenAdd} className="flex items-center space-x-1 px-4 py-2 bg-violet-600 hover:bg-violet-700 text-white rounded-lg text-sm font-medium">
            <Plus size={16} /><span>Add Product</span>
          </button>
        </div>
      </div>

      {/* Filters */}
      <div className="flex flex-col md:flex-row gap-3 bg-slate-900/40 p-4 rounded-xl border border-gray-800">
        <div className="relative flex-1">
          <Search className="absolute left-3 top-2.5 text-gray-500" size={18} />
          <input type="text" placeholder="Search by code, name, color..." value={searchTerm} onChange={e => setSearchTerm(e.target.value)}
            className="w-full bg-slate-950 border border-gray-800 rounded-lg pl-10 pr-4 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500" />
        </div>
        <div className="w-full md:w-44">
          <select value={selectedCategory} onChange={e => setSelectedCategory(e.target.value)}
            className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500">
            <option value="">All Categories</option>
            {CATEGORIES.filter(Boolean).map(c => <option key={c} value={c}>{c}</option>)}
            {categories.filter(c => !CATEGORIES.includes(c)).map(c => <option key={c} value={c}>{c}</option>)}
          </select>
        </div>
        <div className="relative w-full md:w-44">
          <Palette className="absolute left-3 top-2.5 text-gray-500" size={16} />
          <input type="text" placeholder="Search by color..." value={colorSearch} onChange={e => setColorSearch(e.target.value)}
            className="w-full bg-slate-950 border border-gray-800 rounded-lg pl-10 pr-4 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500" />
        </div>
      </div>

      {/* Bulk action bar — visible when any rows are selected */}
      {selectedIds.size > 0 && (
        <div className="glass-card p-3 mb-4 flex items-center justify-between flex-wrap gap-3 border-violet-500/30">
          <div className="flex items-center space-x-3">
            <span className="text-sm text-violet-300 font-semibold">
              {selectedIds.size} selected
            </span>
            <button
              onClick={clearSelection}
              className="text-xs text-gray-400 hover:text-white underline"
            >
              Clear
            </button>
          </div>
          <div className="flex items-center space-x-2 relative" ref={bulkShareRef}>
            <button
              onClick={() => setBulkShareOpen(o => !o)}
              className="flex items-center space-x-1 px-3 py-1.5 bg-violet-600 hover:bg-violet-500 text-white rounded-lg text-xs font-medium transition-colors"
            >
              <Share2 size={12} />
              <span>Share Selected</span>
              <ChevronDown size={10} />
            </button>
            {bulkShareOpen && (
              <div className="absolute top-full right-0 mt-1 z-20 bg-slate-900 border border-gray-800 rounded-lg shadow-2xl py-1 min-w-[160px]">
                {ALL_SHARE_PLATFORMS.map(plat => (
                  <button
                    key={plat}
                    onClick={() => handleBulkShare(plat)}
                    className="w-full text-left px-3 py-1.5 text-xs text-gray-300 hover:bg-slate-800 hover:text-white flex items-center space-x-2"
                  >
                    {plat === 'whatsapp' && <MessageCircle size={12} className="text-emerald-400" />}
                    {plat === 'facebook' && <Facebook size={12} className="text-blue-400" />}
                    {plat === 'instagram' && <Instagram size={12} className="text-pink-400" />}
                    {plat === 'twitter/x' && <Twitter size={12} className="text-sky-400" />}
                    <span>{PLATFORM_LABELS[plat]}</span>
                  </button>
                ))}
              </div>
            )}
          </div>
        </div>
      )}

      {/* Table */}
      <div className="glass-card overflow-hidden">
        <div className="overflow-x-auto">
          <table className="w-full text-left border-collapse">
            <thead>
              <tr className="border-b border-gray-800 bg-slate-900/50 text-xs font-semibold uppercase text-gray-400">
                <th className="py-3 px-3 w-10 text-center">
                  <button
                    onClick={toggleSelectAll}
                    title={selectedIds.size === filteredProducts.length && filteredProducts.length > 0 ? 'Deselect all' : 'Select all'}
                    className="text-gray-400 hover:text-violet-400 transition-colors"
                  >
                    {selectedIds.size === filteredProducts.length && filteredProducts.length > 0
                      ? <CheckSquare size={14} />
                      : <Square size={14} />}
                  </button>
                </th>
                <th className="py-3 px-3">Product</th>
                <th className="py-3 px-3">Code</th>
                <th className="py-3 px-3">Category</th>
                <th className="py-3 px-3">Color</th>
                <th className="py-3 px-3 text-right">Landed</th>
                <th className="py-3 px-3 text-right">Sale</th>
                <th className="py-3 px-3 text-center">HO</th>
                <th className="py-3 px-3 text-center">Agents</th>
                <th className="py-3 px-3 text-center">Sold</th>
                <th className="py-3 px-3 text-center">Actions</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-800 text-sm text-gray-300">
              {filteredProducts.length === 0 ? (
                  <tr><td colSpan={11} className="py-10 text-center text-gray-500">No products found.</td></tr>
              ) : filteredProducts.map(p => {
                const isSelected = p.id != null && selectedIds.has(p.id)
                return (
                  <tr
                    key={p.id}
                    onClick={() => p.id && handleOpenEdit(p)}
                    className={`hover:bg-slate-900/20 transition-colors cursor-pointer ${isSelected ? 'bg-violet-900/10' : ''}`}
                  >
                  <td className="py-3 px-3 text-center" onClick={(e) => e.stopPropagation()}>
                    {p.id != null && (
                      <button
                        onClick={() => toggleSelect(p.id as number)}
                        className="text-gray-400 hover:text-violet-400 transition-colors"
                      >
                        {isSelected ? <CheckSquare size={14} className="text-violet-400" /> : <Square size={14} />}
                      </button>
                    )}
                  </td>
                  <td className="py-3 px-3">
                    <div className="flex items-center space-x-3">
                      <div className="w-16 h-16 bg-slate-800 rounded-lg flex items-center justify-center overflow-hidden border border-gray-700 shrink-0">
                        {(() => {
                          try {
                            const imgs: string[] = JSON.parse(p.images || '[]')
                            return imgs.length > 0 ? (
                              <ProductImage filename={imgs[0]} alt={p.name} className="object-contain w-full h-full" />
                            ) : <ImageIcon size={20} className="text-gray-500" />
                          } catch {
                            return <ImageIcon size={20} className="text-gray-500" />
                          }
                        })()}
                      </div>
                      <div>
                        <span className="text-white text-sm">{p.name}</span>
                        {p.profit_status === 'sold_out' && (
                          <span className="ml-1 text-[9px] px-1 py-0.5 rounded bg-red-600/30 text-red-300 font-bold align-middle">SOLD</span>
                        )}
                        {p.profit_status === 'with_agent' && (
                          <span className="ml-1 text-[9px] px-1 py-0.5 rounded bg-blue-600/30 text-blue-300 font-bold align-middle">AT AGENT</span>
                        )}
                        {p.design && <span className="text-[10px] text-gray-500 block">{p.design}</span>}
                      </div>
                    </div>
                  </td>
                  <td className="py-3 px-3 font-mono text-xs">{p.sku}</td>
                  <td className="py-3 px-3">{p.category || '-'}</td>
                  <td className="py-3 px-3">
                    {p.color ? <span className="inline-flex items-center space-x-1 text-xs"><span className="w-2.5 h-2.5 rounded-full inline-block" style={{backgroundColor: p.color.toLowerCase()}} /> <span>{p.color}</span></span> : '-'}
                  </td>
                  <td className="py-3 px-3 text-right font-mono text-xs text-amber-400">
                    {p.landed_unit_cost ? `Rs.${p.landed_unit_cost.toFixed(0)}` : '-'}
                  </td>
                  <td className="py-3 px-3 text-right font-mono text-xs text-violet-400">Rs.{p.sale_price.toFixed(0)}</td>
                  <td className={`py-3 px-3 text-center font-bold text-xs ${(p.qty_in_head_office ?? p.stock_quantity) <= 5 ? 'text-red-400' : 'text-gray-300'}`}>
                    {p.qty_in_head_office ?? p.stock_quantity}
                  </td>
                  <td className="py-3 px-3 text-center font-bold text-xs text-blue-400">{p.qty_with_agents ?? 0}</td>
                  <td className="py-3 px-3 text-center font-bold text-xs text-emerald-400">{p.qty_sold ?? 0}</td>
                  <td className="py-3 px-3 text-center" onClick={(e) => e.stopPropagation()}>
                    <div className="flex items-center justify-center space-x-2">
                      <button onClick={(e) => { e.stopPropagation(); p.id && handleOpenEdit(p); }} className="p-1 hover:text-violet-400" title="Edit"><Edit size={14} /></button>
                      <button onClick={(e) => { e.stopPropagation(); p.id && handleDelete(p.id); }} className="p-1 hover:text-red-400" title="Delete"><Trash2 size={14} /></button>
                      {/* Per-row share dropdown */}
                      <div className="relative" ref={openShareMenuFor === p.id ? shareMenuRef : undefined}>
                        <button
                          onClick={(e) => {
                            e.stopPropagation()
                            const id = p.id as number
                            setOpenShareMenuFor(openShareMenuFor === id ? null : id)
                          }}
                          className="p-1 hover:text-emerald-400 transition-colors"
                          title="Share"
                        >
                          <Share2 size={14} />
                        </button>
                        {openShareMenuFor === p.id && (
                          <div className="absolute top-full right-0 mt-1 z-20 bg-slate-900 border border-gray-800 rounded-lg shadow-2xl py-1 min-w-[160px]">
                            {ALL_SHARE_PLATFORMS.map(plat => (
                              <button
                                key={plat}
                                onClick={(e) => {
                                  e.stopPropagation()
                                  handleShareProduct(p, plat)
                                }}
                                className="w-full text-left px-3 py-1.5 text-xs text-gray-300 hover:bg-slate-800 hover:text-white flex items-center space-x-2"
                              >
                                {plat === 'whatsapp' && <MessageCircle size={12} className="text-emerald-400" />}
                                {plat === 'facebook' && <Facebook size={12} className="text-blue-400" />}
                                {plat === 'instagram' && <Instagram size={12} className="text-pink-400" />}
                                {plat === 'twitter/x' && <Twitter size={12} className="text-sky-400" />}
                                <span>{PLATFORM_LABELS[plat]}</span>
                              </button>
                            ))}
                          </div>
                        )}
                      </div>
                      {/* Record Sale button */}
                      <button
                        onClick={(e) => {
                          e.stopPropagation()
                          handleOpenSaleModal(p)
                        }}
                        disabled={p.profit_status === 'sold_out'}
                        className="p-1 hover:text-amber-400 transition-colors disabled:opacity-30"
                        title="Record Sale"
                      >
                        <ShoppingCart size={14} />
                      </button>
                    </div>
                  </td>
                </tr>
                )
              })}
            </tbody>
          </table>
        </div>
      </div>

      {/* Modal */}
      {showModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm p-4">
          <div className="bg-slate-900 border border-gray-800 rounded-2xl w-full max-w-2xl overflow-hidden shadow-2xl max-h-[90vh] flex flex-col">
            <div className="flex items-center justify-between p-4 border-b border-gray-800 bg-slate-950/40 shrink-0">
              <h3 className="text-lg font-bold text-white font-display">{editProduct ? 'Edit Product' : 'Add Product'}</h3>
              <button onClick={() => setShowModal(false)} className="text-gray-400 hover:text-white"><X size={20} /></button>
            </div>
            <form onSubmit={handleSave} className="p-5 space-y-4 overflow-y-auto">
              <div className="grid grid-cols-3 gap-3">
                <div>
                  <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">Product Code <span className="text-gray-600 normal-case font-normal">(auto if empty)</span></label>
                  <input type="text" value={productCode} onChange={e => setProductCode(e.target.value)}
                    placeholder="AC-2026-001 (or leave empty)"
                    className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500" />
                </div>
                <div>
                  <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">Name *</label>
                  <input type="text" required value={name} onChange={e => setName(e.target.value)}
                    className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500" />
                </div>
                <div>
                  <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">Category</label>
                  <select value={category} onChange={e => setCategory(e.target.value)}
                    className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500">
                    {CATEGORIES.map(c => <option key={c} value={c}>{c || '-- Select --'}</option>)}
                  </select>
                </div>
              </div>

              <div className="grid grid-cols-3 gap-3">
                <div>
                  <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">Color</label>
                  <input type="text" value={color} onChange={e => setColor(e.target.value)}
                    placeholder="Bottle Green"
                    className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500" />
                </div>
                <div>
                  <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">Fabric</label>
                  <select value={fabric} onChange={e => setFabric(e.target.value)}
                    className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500">
                    {FABRICS.map(f => <option key={f} value={f}>{f || '-- Select --'}</option>)}
                  </select>
                </div>
                <div>
                  <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">Design Type</label>
                  <select value={designType} onChange={e => setDesignType(e.target.value)}
                    className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500">
                    {DESIGN_TYPES.map(d => <option key={d} value={d}>{d || '-- Select --'}</option>)}
                  </select>
                </div>
              </div>

              <div className="grid grid-cols-3 gap-3">
                <div>
                  <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">Gender</label>
                  <select value={gender} onChange={e => setGender(e.target.value)}
                    className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500">
                    {GENDERS.map(g => <option key={g} value={g}>{g || '-- Select --'}</option>)}
                  </select>
                </div>
                <div>
                  <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">Brand/Design</label>
                  <input type="text" value={design} onChange={e => setDesign(e.target.value)}
                    placeholder="Nishat, Maria B, etc."
                    className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500" />
                </div>
                <div>
                  <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">Season</label>
                  <select value={season} onChange={e => setSeason(e.target.value)}
                    className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500">
                    {SEASONS.map(s => <option key={s} value={s}>{s || '-- Select --'}</option>)}
                  </select>
                </div>
              </div>

              <div className="grid grid-cols-3 gap-3">
                <div>
                  <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">Cost Price (Rs)</label>
                  <input type="number" step="1" value={costPrice} onChange={e => setCostPrice(e.target.value)}
                    className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500" />
                </div>
                <div>
                  <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">Retail Price (Rs)</label>
                  <input type="number" step="1" value={retailPrice} onChange={e => setRetailPrice(e.target.value)}
                    className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500" />
                </div>
                <div>
                  <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">Sale Price (Rs)</label>
                  <input type="number" step="1" value={salePrice} onChange={e => setSalePrice(e.target.value)}
                    className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500" />
                </div>
              </div>

              {/* Total Stock (HO) — v0.14.3: full-width now that Purchase Price field is removed */}
              <div>
                <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">Total Stock (Head Office)</label>
                <input type="number" value={stockQuantity} onChange={e => setStockQuantity(e.target.value)}
                  className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500" />
              </div>

              {/* Agent Stock (v0.13.7: replaces Location Stock) */}
              <div>
                <label className="block text-xs font-semibold uppercase text-gray-400 mb-2 flex items-center space-x-1">
                  <MapPin size={12} /><span>Agent Stock (initial allocation)</span>
                </label>
                <div className="space-y-1.5">
                  {agentStock.length === 0 ? (
                    <p className="text-xs text-gray-600 italic">No agents added yet. Go to Agents tab to add agents.</p>
                  ) : agentStock.map(ag => (
                    <div key={ag.agent_id} className="flex items-center space-x-2">
                      <span className="text-xs text-gray-400 w-32">{ag.agent_name}</span>
                      <input type="number" value={ag.quantity} onChange={e => handleAgentStockChange(ag.agent_id, Number(e.target.value))}
                        className="w-20 bg-slate-950 border border-gray-800 rounded px-2 py-1 text-xs text-gray-200 focus:outline-none focus:border-violet-500" />
                    </div>
                  ))}
                </div>
              </div>

              <div>
                <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">Description</label>
                <textarea value={description} onChange={e => setDescription(e.target.value)} rows={2}
                  className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500" />
              </div>

              <div>
                <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">Tags</label>
                <input type="text" value={tags} onChange={e => setTags(e.target.value)}
                  placeholder="summer, cotton, lawn"
                  className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500" />
              </div>

              {/* Images — v0.14.0: clipboard paste + drag-drop + file picker
                   v0.24.1: + URL paste button + document-level paste listener
                   v0.24.2: removed div onPaste (was duplicating with document listener) */}
              <div
                onDrop={handleImageDrop}
                onDragOver={(e) => e.preventDefault()}
              >
                <label className="block text-xs font-semibold uppercase text-gray-400 mb-2">
                  Images
                </label>
                <div className="flex flex-wrap gap-2">
                  {images.map((imgName, idx) => (
                    <div key={idx} className="relative w-24 h-24 bg-slate-950 border border-gray-800 rounded-lg overflow-hidden flex items-center justify-center">
                      <ProductImage filename={imgName} alt={`Product image ${idx + 1}`} className="object-contain w-full h-full" />
                      <button type="button" onClick={() => handleRemoveImage(idx)}
                        className="absolute top-0.5 right-0.5 bg-red-600 text-white rounded-full p-0.5"><X size={10} /></button>
                    </div>
                  ))}
                  <button type="button" onClick={handleSelectImage}
                    className="w-24 h-24 bg-slate-950 hover:bg-slate-900 border border-dashed border-gray-800 hover:border-violet-500 rounded-lg flex flex-col items-center justify-center text-gray-400 hover:text-violet-400 transition-colors">
                    <Plus size={20} /><span className="text-[10px]">Photo</span>
                  </button>
                  <button type="button" onClick={handleAddImageFromUrl}
                    className="w-24 h-24 bg-slate-950 hover:bg-slate-900 border border-dashed border-gray-800 hover:border-violet-500 rounded-lg flex flex-col items-center justify-center text-gray-400 hover:text-violet-400 transition-colors">
                    <Link2 size={18} /><span className="text-[10px]">From URL</span>
                  </button>
                </div>
                <p className="text-[10px] text-gray-600 mt-1">3 ways to add: Ctrl+V paste image, drag-drop, click + or From URL</p>
              </div>

              <div className="flex justify-end space-x-2 pt-3 border-t border-gray-800">
                <button type="button" onClick={() => setShowModal(false)}
                  className="px-4 py-2 bg-slate-800 hover:bg-slate-700 text-gray-200 rounded-lg text-sm">Cancel</button>
                <button type="submit"
                  className="px-4 py-2 bg-violet-600 hover:bg-violet-700 text-white rounded-lg text-sm font-medium">Save</button>
              </div>
            </form>
          </div>
        </div>
      )}

      {/* Sale Modal (v0.12.5) */}
      {showSaleModal && saleProduct && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm p-4">
          <div className="bg-slate-900 border border-gray-800 rounded-2xl w-full max-w-md overflow-hidden shadow-2xl">
            <div className="flex items-center justify-between p-4 border-b border-gray-800 bg-slate-950/40">
              <h3 className="text-lg font-bold text-white font-display">Record Sale</h3>
              <button onClick={() => setShowSaleModal(false)} className="text-gray-400 hover:text-white"><X size={20} /></button>
            </div>
            <div className="p-5 space-y-3">
              {/* Product info */}
              <div className="bg-slate-950/50 border border-gray-800 rounded-lg p-3">
                <div className="text-sm font-semibold text-white">{saleProduct.name}</div>
                <div className="text-[10px] text-gray-500">SKU: {saleProduct.sku} • HO: {saleProduct.qty_in_head_office ?? saleProduct.stock_quantity} • Agents: {saleProduct.qty_with_agents ?? 0}</div>
              </div>

              {/* Mode tabs */}
              <div className="flex space-x-1 bg-slate-950 p-1 rounded-lg border border-gray-800">
                <button
                  onClick={() => setSaleMode('direct')}
                  className={`flex-1 px-3 py-1.5 rounded text-xs font-medium ${saleMode === 'direct' ? 'bg-violet-600 text-white' : 'text-gray-400'}`}
                >
                  Direct Sale (HO)
                </button>
                <button
                  onClick={() => setSaleMode('agent')}
                  className={`flex-1 px-3 py-1.5 rounded text-xs font-medium ${saleMode === 'agent' ? 'bg-violet-600 text-white' : 'text-gray-400'}`}
                >
                  Agent Sale
                </button>
              </div>

              {/* Agent selector (only for agent mode) */}
              {saleMode === 'agent' && (
                <div>
                  <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">Agent *</label>
                  <select
                    value={saleAgentId}
                    onChange={e => setSaleAgentId(e.target.value ? Number(e.target.value) : '')}
                    className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500"
                  >
                    <option value="">-- Select Agent --</option>
                    {agents.map(a => (
                      <option key={a.agent.id} value={a.agent.id}>{a.agent.name} ({a.agent.city || '—'})</option>
                    ))}
                  </select>
                </div>
              )}

              {/* Channel (only for direct mode) */}
              {saleMode === 'direct' && (
                <div>
                  <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">Sale Channel</label>
                  <select
                    value={saleChannel}
                    onChange={e => setSaleChannel(e.target.value)}
                    className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500"
                  >
                    <option value="head_office">Head Office (Walk-in)</option>
                    <option value="whatsapp">WhatsApp</option>
                    <option value="facebook">Facebook</option>
                    <option value="instagram">Instagram</option>
                    <option value="tiktok">TikTok</option>
                  </select>
                </div>
              )}

              {/* Qty + Price */}
              <div className="grid grid-cols-2 gap-3">
                <div>
                  <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">Quantity *</label>
                  <input type="number" min={1} value={saleQty} onChange={e => setSaleQty(Math.max(1, Number(e.target.value)))}
                    className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500" />
                </div>
                <div>
                  <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">Unit Price (Rs.)</label>
                  <input type="number" min={0} step={0.01} value={saleUnitPrice} onChange={e => setSaleUnitPrice(Number(e.target.value))}
                    className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500" />
                </div>
              </div>

              {/* Total */}
              <div className="bg-violet-900/20 border border-violet-500/30 rounded-lg p-2 text-xs flex justify-between">
                <span className="text-violet-300">Total Sale Amount:</span>
                <span className="text-violet-300 font-bold">Rs. {(saleQty * saleUnitPrice).toFixed(0)}</span>
              </div>

              {/* Customer (optional) */}
              <div className="grid grid-cols-2 gap-3">
                <div>
                  <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">Customer Name</label>
                  <input type="text" value={saleCustomerName} onChange={e => setSaleCustomerName(e.target.value)}
                    placeholder="Optional"
                    className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500" />
                </div>
                <div>
                  <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">Customer Phone</label>
                  <input type="text" value={saleCustomerPhone} onChange={e => setSaleCustomerPhone(e.target.value)}
                    placeholder="Optional"
                    className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500" />
                </div>
              </div>

              {/* Notes */}
              <div>
                <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">Notes</label>
                <input type="text" value={saleNotes} onChange={e => setSaleNotes(e.target.value)}
                  placeholder="Optional"
                  className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500" />
              </div>

              {/* Buttons */}
              <div className="flex justify-end space-x-2 pt-3 border-t border-gray-800">
                <button type="button" onClick={() => setShowSaleModal(false)} disabled={saleSaving}
                  className="px-4 py-2 bg-slate-800 hover:bg-slate-700 text-gray-200 rounded-lg text-sm disabled:opacity-50">Cancel</button>
                <button type="button" onClick={handleRecordSale} disabled={saleSaving || (saleMode === 'agent' && saleAgentId === '')}
                  className="px-4 py-2 bg-amber-600 hover:bg-amber-700 text-white rounded-lg text-sm font-medium disabled:opacity-50">
                  {saleSaving ? 'Recording...' : 'Record Sale'}
                </button>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* v0.15.0: Publish Preview Modal */}
      {publishPreview && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm p-4">
          <div className="bg-slate-900 border border-gray-800 rounded-2xl w-full max-w-md overflow-hidden shadow-2xl">
            <div className="flex items-center justify-between p-4 border-b border-gray-800 bg-slate-950/40">
              <h3 className="text-lg font-bold text-white flex items-center space-x-2">
                <Globe size={18} className="text-emerald-400" />
                <span>Publish to Catalog</span>
              </h3>
              <button
                onClick={() => setPublishPreview(null)}
                disabled={publishing}
                className="text-gray-400 hover:text-white disabled:opacity-50"
              ><X size={20} /></button>
            </div>
            <div className="p-5 space-y-3 max-h-[70vh] overflow-y-auto">
              <div className="bg-slate-950/50 border border-gray-800 rounded-lg p-3 space-y-2 text-sm">
                <div className="flex justify-between"><span className="text-gray-400">Brand:</span><span className="text-white font-medium">{publishPreview.brand}</span></div>
                <div className="flex justify-between"><span className="text-gray-400">Products:</span><span className="text-white font-bold">{publishPreview.product_count}</span></div>
                <div className="flex justify-between"><span className="text-gray-400">Images:</span><span className="text-white font-bold">{publishPreview.image_count}</span></div>
                <div className="flex justify-between"><span className="text-gray-400">Total size:</span><span className="text-white font-bold">{publishPreview.total_size_display}</span></div>
                <div className="border-t border-gray-800 pt-2 mt-2">
                  <div className="flex justify-between"><span className="text-gray-400">Repo:</span><span className="text-gray-300 text-xs font-mono">{publishPreview.repo}</span></div>
                </div>
                <div className="flex justify-between"><span className="text-gray-400">URL:</span><span className="text-emerald-400 text-xs font-mono truncate ml-2">{publishPreview.catalog_url}</span></div>
              </div>

              {/* v0.16.0: Validation errors */}
              {publishPreview.errors && publishPreview.errors.length > 0 && (
                <div className="bg-red-950/30 border border-red-800/40 rounded-lg p-3">
                  <p className="text-xs text-red-300 font-bold mb-2">
                    ❌ Errors ({publishPreview.errors.length}) — fix these before publishing:
                  </p>
                  <ul className="text-xs text-red-200 space-y-1 max-h-32 overflow-y-auto">
                    {publishPreview.errors.map((err: any, i: number) => (
                      <li key={i} className="flex gap-2">
                        <span className="text-red-400 shrink-0">•</span>
                        <span><strong>{err.product_name}:</strong> {err.issue}</span>
                      </li>
                    ))}
                  </ul>
                  <label className="flex items-center gap-2 mt-2 text-xs text-red-300 cursor-pointer">
                    <input
                      type="checkbox"
                      checked={forcePublish}
                      onChange={(e) => setForcePublish(e.target.checked)}
                      className="rounded"
                    />
                    <span>Publish anyway (I'll fix these later)</span>
                  </label>
                </div>
              )}

              {/* v0.16.0: Validation warnings */}
              {publishPreview.warnings && publishPreview.warnings.length > 0 && (
                <div className="bg-amber-950/30 border border-amber-800/40 rounded-lg p-3">
                  <p className="text-xs text-amber-300 font-bold mb-2">
                    ⚠️ Warnings ({publishPreview.warnings.length}) — non-blocking:
                  </p>
                  <ul className="text-xs text-amber-200 space-y-1 max-h-32 overflow-y-auto">
                    {publishPreview.warnings.map((warn: any, i: number) => (
                      <li key={i} className="flex gap-2">
                        <span className="text-amber-400 shrink-0">{warn.severity === 'info' ? 'ℹ' : '⚠'}</span>
                        <span><strong>{warn.product_name}:</strong> {warn.issue}</span>
                      </li>
                    ))}
                  </ul>
                </div>
              )}

              <div className="bg-amber-950/30 border border-amber-800/40 rounded-lg p-3">
                <p className="text-xs text-amber-300">
                  ⚠️ Only public fields will be published (name, price, images, color, fabric, category).
                  Cost price, supplier, profit, and internal notes will NOT be shared.
                </p>
              </div>
              <p className="text-xs text-gray-500">
                Publishing will upload catalog.json + images to GitHub via Contents API.
                GitHub Pages auto-deploys within ~30 seconds. Customers can browse immediately after.
              </p>
            </div>
            <div className="flex justify-end space-x-2 p-4 border-t border-gray-800 bg-slate-950/40">
              <button
                onClick={() => setPublishPreview(null)}
                disabled={publishing}
                className="px-4 py-2 bg-slate-800 hover:bg-slate-700 text-gray-200 rounded-lg text-sm disabled:opacity-50"
              >Cancel</button>
              <button
                onClick={handlePublishConfirm}
                disabled={publishing || (publishPreview.errors && publishPreview.errors.length > 0 && !forcePublish)}
                className="px-4 py-2 bg-emerald-600 hover:bg-emerald-700 text-white rounded-lg text-sm font-medium disabled:opacity-50 flex items-center space-x-1"
              >
                <Globe size={14} />
                <span>{publishing ? 'Publishing...' : (publishPreview.errors && publishPreview.errors.length > 0 && !forcePublish ? 'Fix errors first' : 'Publish Now')}</span>
              </button>
            </div>
          </div>
        </div>
      )}

      {/* v0.15.0: Publish Result Modal */}
      {publishResult && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm p-4">
          <div className="bg-slate-900 border border-gray-800 rounded-2xl w-full max-w-md overflow-hidden shadow-2xl">
            <div className={`flex items-center justify-between p-4 border-b border-gray-800 ${publishResult.success ? 'bg-emerald-950/40' : 'bg-red-950/40'}`}>
              <h3 className={`text-lg font-bold flex items-center space-x-2 ${publishResult.success ? 'text-emerald-400' : 'text-red-400'}`}>
                {publishResult.success ? '✅ Published!' : '❌ Failed'}
              </h3>
              <button onClick={() => setPublishResult(null)} className="text-gray-400 hover:text-white"><X size={20} /></button>
            </div>
            <div className="p-5 space-y-3 text-sm">
              {publishResult.success ? (
                <>
                  <div className="bg-slate-950/50 border border-gray-800 rounded-lg p-3 space-y-2">
                    <div className="flex justify-between"><span className="text-gray-400">Products published:</span><span className="text-white font-bold">{publishResult.products_published}</span></div>
                    <div className="flex justify-between"><span className="text-gray-400">Images uploaded:</span><span className="text-white font-bold">{publishResult.images_uploaded}</span></div>
                    {publishResult.images_deleted > 0 && (
                      <div className="flex justify-between"><span className="text-gray-400">Orphan images deleted:</span><span className="text-white font-bold">{publishResult.images_deleted}</span></div>
                    )}
                  </div>
                  <div className="bg-emerald-950/30 border border-emerald-800/40 rounded-lg p-3">
                    <p className="text-xs text-emerald-300 font-semibold">📡 Live in ~30 seconds at:</p>
                    <a
                      href={publishResult.catalog_url}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="text-emerald-400 text-xs font-mono break-all hover:underline mt-1 block"
                    >{publishResult.catalog_url}</a>
                  </div>
                </>
              ) : (
                <div className="bg-red-950/30 border border-red-800/40 rounded-lg p-3">
                  <p className="text-xs text-red-300 font-semibold mb-2">Errors:</p>
                  <ul className="text-xs text-red-200 space-y-1 list-disc list-inside">
                    {(publishResult.errors || [String(publishResult)]).map((err: string, i: number) => (
                      <li key={i} className="font-mono break-all">{err}</li>
                    ))}
                  </ul>
                </div>
              )}
            </div>
            <div className="flex justify-end p-4 border-t border-gray-800 bg-slate-950/40">
              <button
                onClick={() => setPublishResult(null)}
                className="px-4 py-2 bg-slate-800 hover:bg-slate-700 text-gray-200 rounded-lg text-sm"
              >Close</button>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}
