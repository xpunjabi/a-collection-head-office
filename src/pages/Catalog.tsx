import React, { useEffect, useState } from 'react'
import { useAppStore, Product } from '../stores/store'
import { open } from '@tauri-apps/plugin-dialog'
import { invoke } from '@tauri-apps/api/core'
import { 
  Search, Download, Upload, Plus, Edit, Trash2, Image as ImageIcon,
  X, Palette, MapPin
} from 'lucide-react'
import ProductImage from '../components/ProductImage'

interface LocationStock {
  location_id: number;
  location_name: string;
  quantity: number;
}

const SEASONS = ['', 'Summer', 'Winter', 'Eid Special', 'Festive', 'Spring', 'Autumn']
const CATEGORIES = ['', '3 Piece', '2 Piece', 'Lawn', 'Cotton', 'Printed', 'Embroidery',
  'Cut Piece', 'Gents Cotton', 'Gents Washing Wear', 'Seasonal']

export default function Catalog() {
  const { products, fetchProducts, addProduct, updateProduct, deleteProduct,
    exportProductsCsv, importProductsCsv, uploadProductImage } = useAppStore()

  const [searchTerm, setSearchTerm] = useState('')
  const [selectedCategory, setSelectedCategory] = useState('')
  const [colorSearch, setColorSearch] = useState('')
  const [showModal, setShowModal] = useState(false)
  const [editProduct, setEditProduct] = useState<Product | null>(null)
  const [locations, setLocations] = useState<LocationStock[]>([])
  const [, setAllLocations] = useState<{id: number; name: string}[]>([])

  // Form states
  const [productCode, setProductCode] = useState('')
  const [name, setName] = useState('')
  const [category, setCategory] = useState('')
  const [color, setColor] = useState('')
  const [design, setDesign] = useState('')
  const [season, setSeason] = useState('')
  const [costPrice, setCostPrice] = useState(0)
  const [salePrice, setSalePrice] = useState(0)
  const [purchasePrice, setPurchasePrice] = useState(0)
  const [description, setDescription] = useState('')
  const [tags, setTags] = useState('')
  const [stockQuantity, setStockQuantity] = useState(0)
  const [status, setStatus] = useState('active')
  const [images, setImages] = useState<string[]>([])

  useEffect(() => { fetchProducts() }, [])

  const handleOpenAdd = async () => {
    setEditProduct(null)
    setProductCode(''); setName(''); setCategory(''); setColor(''); setDesign('')
    setSeason(''); setCostPrice(0); setSalePrice(0); setPurchasePrice(0)
    setDescription(''); setTags(''); setStockQuantity(0); setStatus('active'); setImages([])
    try {
      const locs: {id: number; name: string}[] = await invoke('get_locations')
      setAllLocations(locs)
      setLocations(locs.map(l => ({ location_id: l.id, location_name: l.name, quantity: 0 })))
    } catch { setAllLocations([]); setLocations([]) }
    setShowModal(true)
  }

  const handleOpenEdit = async (p: Product) => {
    setEditProduct(p)
    setProductCode(p.sku || ''); setName(p.name)
    setCategory(p.category || ''); setColor(p.color || ''); setDesign(p.design || '')
    setSeason(p.season || ''); setCostPrice(p.cost_price); setSalePrice(p.sale_price)
    setPurchasePrice(p.purchase_price || p.cost_price)
    setDescription(p.description || ''); setTags(p.tags || '')
    setStockQuantity(p.stock_quantity); setStatus(p.status)
    try { setImages(JSON.parse(p.images || '[]')) } catch { setImages([]) }
    try {
      const locs: {id: number; name: string}[] = await invoke('get_locations')
      setAllLocations(locs)
      if (p.id) {
        const plocs: LocationStock[] = await invoke('get_product_locations', { productId: p.id })
        setLocations(locs.map(l => {
          const existing = plocs.find(pl => pl.location_id === l.id)
          return { location_id: l.id, location_name: l.name, quantity: existing ? existing.quantity : 0 }
        }))
      } else {
        setLocations(locs.map(l => ({ location_id: l.id, location_name: l.name, quantity: 0 })))
      }
    } catch { setAllLocations([]); setLocations([]) }
    setShowModal(true)
  }

  const handleLocationChange = (locationId: number, quantity: number) => {
    setLocations(prev => {
      const existing = prev.find(l => l.location_id === locationId)
      if (existing) {
        return prev.map(l => l.location_id === locationId ? { ...l, quantity } : l)
      }
      return [...prev, { location_id: locationId, location_name: '', quantity }]
    })
  }

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!productCode || !name) return

    const productData: Product = {
      id: editProduct?.id,
      sku: productCode,
      name,
      category: category || undefined,
      color: color || undefined,
      design: design || undefined,
      season: season || undefined,
      cost_price: Number(costPrice),
      sale_price: Number(salePrice),
      purchase_price: Number(purchasePrice) || Number(costPrice),
      description: description || undefined,
      tags: tags || undefined,
      stock_quantity: Number(stockQuantity),
      status,
      images: JSON.stringify(images),
      created_at: editProduct?.created_at,
      updated_at: editProduct?.updated_at,
    }

    try {
      let productId: number
      if (editProduct?.id) {
        await updateProduct(productData)
        productId = editProduct.id
      } else {
        productId = await addProduct(productData) as unknown as number
      }
      // Save location stocks
      for (const loc of locations) {
        if (loc.location_id) {
          await invoke('upsert_product_location', { productId, locationId: loc.location_id, quantity: loc.quantity })
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

  const handleRemoveImage = (index: number) => setImages(images.filter((_, i) => i !== index))

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
        <div className="flex items-center space-x-2">
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

      {/* Table */}
      <div className="glass-card overflow-hidden">
        <div className="overflow-x-auto">
          <table className="w-full text-left border-collapse">
            <thead>
              <tr className="border-b border-gray-800 bg-slate-900/50 text-xs font-semibold uppercase text-gray-400">
                <th className="py-3 px-3">Product</th>
                <th className="py-3 px-3">Code</th>
                <th className="py-3 px-3">Category</th>
                <th className="py-3 px-3">Color</th>
                <th className="py-3 px-3">Season</th>
                <th className="py-3 px-3 text-right">Purchase</th>
                <th className="py-3 px-3 text-right">Sale</th>
                <th className="py-3 px-3 text-center">Stock</th>
                <th className="py-3 px-3 text-center">Actions</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-800 text-sm text-gray-300">
              {filteredProducts.length === 0 ? (
                  <tr><td colSpan={9} className="py-10 text-center text-gray-500">No products found.</td></tr>
              ) : filteredProducts.map(p => (
                  <tr key={p.id} onClick={() => p.id && handleOpenEdit(p)} className="hover:bg-slate-900/20 transition-colors cursor-pointer">
                  <td className="py-3 px-3">
                    <div className="flex items-center space-x-3">
                      <div className="w-9 h-9 bg-slate-800 rounded-lg flex items-center justify-center overflow-hidden border border-gray-700 shrink-0">
                        {(() => {
                          try {
                            const imgs: string[] = JSON.parse(p.images || '[]')
                            return imgs.length > 0 ? (
                              <ProductImage filename={imgs[0]} alt={p.name} className="object-contain w-full h-full" />
                            ) : <ImageIcon size={16} className="text-gray-500" />
                          } catch {
                            return <ImageIcon size={16} className="text-gray-500" />
                          }
                        })()}
                      </div>
                      <div>
                        <span className="text-white text-sm">{p.name}</span>
                        {p.design && <span className="text-[10px] text-gray-500 block">{p.design}</span>}
                      </div>
                    </div>
                  </td>
                  <td className="py-3 px-3 font-mono text-xs">{p.sku}</td>
                  <td className="py-3 px-3">{p.category || '-'}</td>
                  <td className="py-3 px-3">
                    {p.color ? <span className="inline-flex items-center space-x-1 text-xs"><span className="w-2.5 h-2.5 rounded-full inline-block" style={{backgroundColor: p.color.toLowerCase()}} /> <span>{p.color}</span></span> : '-'}
                  </td>
                  <td className="py-3 px-3 text-xs">{p.season || '-'}</td>
                  <td className="py-3 px-3 text-right font-mono text-xs">Rs.{p.purchase_price?.toFixed(0) || p.cost_price.toFixed(0)}</td>
                  <td className="py-3 px-3 text-right font-mono text-xs text-violet-400">Rs.{p.sale_price.toFixed(0)}</td>
                  <td className={`py-3 px-3 text-center font-bold text-xs ${p.stock_quantity <= 5 ? 'text-red-400' : 'text-gray-300'}`}>{p.stock_quantity}</td>
                  <td className="py-3 px-3 text-center">
                    <div className="flex items-center justify-center space-x-2">
                      <button onClick={(e) => { e.stopPropagation(); p.id && handleOpenEdit(p); }} className="p-1 hover:text-violet-400"><Edit size={14} /></button>
                      <button onClick={(e) => { e.stopPropagation(); p.id && handleDelete(p.id); }} className="p-1 hover:text-red-400"><Trash2 size={14} /></button>
                    </div>
                  </td>
                </tr>
              ))}
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
                  <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">Product Code *</label>
                  <input type="text" required value={productCode} onChange={e => setProductCode(e.target.value)}
                    placeholder="AC-2026-001"
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
                  <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">Design</label>
                  <input type="text" value={design} onChange={e => setDesign(e.target.value)}
                    placeholder="Digital Print"
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
                  <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">Purchase Price (Rs)</label>
                  <input type="number" step="1" value={purchasePrice} onChange={e => setPurchasePrice(Number(e.target.value))}
                    className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500" />
                </div>
                <div>
                  <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">Sale Price (Rs)</label>
                  <input type="number" step="1" value={salePrice} onChange={e => setSalePrice(Number(e.target.value))}
                    className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500" />
                </div>
                <div>
                  <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">Total Stock</label>
                  <input type="number" value={stockQuantity} onChange={e => setStockQuantity(Number(e.target.value))}
                    className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500" />
                </div>
              </div>

              {/* Location Stock */}
              <div>
                <label className="block text-xs font-semibold uppercase text-gray-400 mb-2 flex items-center space-x-1">
                  <MapPin size={12} /><span>Location Stock</span>
                </label>
                <div className="space-y-1.5">
                  {locations.map(loc => (
                    <div key={loc.location_id} className="flex items-center space-x-2">
                      <span className="text-xs text-gray-400 w-32">{loc.location_name}</span>
                      <input type="number" value={loc.quantity} onChange={e => handleLocationChange(loc.location_id, Number(e.target.value))}
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

              {/* Images */}
              <div>
                <label className="block text-xs font-semibold uppercase text-gray-400 mb-2">Images</label>
                <div className="flex flex-wrap gap-2">
                  {images.map((imgName, idx) => (
                    <div key={idx} className="relative w-14 h-14 bg-slate-950 border border-gray-800 rounded-lg overflow-hidden flex items-center justify-center">
                      <ProductImage filename={imgName} alt={`Product image ${idx + 1}`} className="object-contain w-full h-full" />
                      <button type="button" onClick={() => handleRemoveImage(idx)}
                        className="absolute top-0.5 right-0.5 bg-red-600 text-white rounded-full p-0.5"><X size={8} /></button>
                    </div>
                  ))}
                  <button type="button" onClick={handleSelectImage}
                    className="w-14 h-14 bg-slate-950 hover:bg-slate-900 border border-dashed border-gray-800 hover:border-violet-500 rounded-lg flex flex-col items-center justify-center text-gray-400 hover:text-violet-400">
                    <Plus size={14} /><span className="text-[8px]">Photo</span>
                  </button>
                </div>
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
    </div>
  )
}
