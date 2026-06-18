import React, { useEffect, useState } from 'react'
import { useAppStore, Product } from '../stores/store'
import { open } from '@tauri-apps/plugin-dialog'
import { 
  Search, 
  Filter, 
  Download, 
  Upload, 
  Plus, 
  Edit, 
  Trash2, 
  Image as ImageIcon,
  X,
  FileSpreadsheet
} from 'lucide-react'

export default function Catalog() {
  const { 
    products, 
    fetchProducts, 
    addProduct, 
    updateProduct, 
    deleteProduct,
    exportProductsCsv,
    importProductsCsv,
    uploadProductImage
  } = useAppStore()

  const [searchTerm, setSearchTerm] = useState('')
  const [selectedCategory, setSelectedCategory] = useState('')
  const [showModal, setShowModal] = useState(false)
  const [editProduct, setEditProduct] = useState<Product | null>(null)
  
  // Form States
  const [sku, setSku] = useState('')
  const [name, setName] = useState('')
  const [category, setCategory] = useState('')
  const [costPrice, setCostPrice] = useState(0)
  const [salePrice, setSalePrice] = useState(0)
  const [description, setDescription] = useState('')
  const [tags, setTags] = useState('')
  const [stockQuantity, setStockQuantity] = useState(0)
  const [status, setStatus] = useState('active')
  const [images, setImages] = useState<string[]>([])

  useEffect(() => {
    fetchProducts()
  }, [])

  const handleOpenAdd = () => {
    setEditProduct(null)
    setSku('')
    setName('')
    setCategory('')
    setCostPrice(0)
    setSalePrice(0)
    setDescription('')
    setTags('')
    setStockQuantity(0)
    setStatus('active')
    setImages([])
    setShowModal(true)
  }

  const handleOpenEdit = (p: Product) => {
    setEditProduct(p)
    setSku(p.sku)
    setName(p.name)
    setCategory(p.category || '')
    setCostPrice(p.cost_price)
    setSalePrice(p.sale_price)
    setDescription(p.description || '')
    setTags(p.tags || '')
    setStockQuantity(p.stock_quantity)
    setStatus(p.status)
    try {
      setImages(JSON.parse(p.images || '[]'))
    } catch {
      setImages([])
    }
    setShowModal(true)
  }

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!sku || !name) return

    const productData: Product = {
      id: editProduct?.id,
      sku,
      name,
      category,
      cost_price: Number(costPrice),
      sale_price: Number(salePrice),
      description,
      tags,
      stock_quantity: Number(stockQuantity),
      status,
      images: JSON.stringify(images)
    }

    try {
      if (editProduct) {
        await updateProduct(productData)
      } else {
        await addProduct(productData)
      }
      setShowModal(false)
    } catch (err) {
      alert(`Error saving product: ${err}`)
    }
  }

  const handleDelete = async (id: number) => {
    if (confirm('Are you sure you want to delete this product?')) {
      try {
        await deleteProduct(id)
      } catch (err) {
        alert(err)
      }
    }
  }

  const handleSelectImage = async () => {
    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: 'Images', extensions: ['png', 'jpg', 'jpeg', 'webp'] }]
      })

      if (selected && typeof selected === 'string') {
        // Process image with Tauri backend (saves in local images dir)
        const newFileName = await uploadProductImage(selected, 'thumbnail')
        setImages([...images, newFileName])
      }
    } catch (err) {
      console.error(err)
    }
  }

  const handleRemoveImage = (index: number) => {
    setImages(images.filter((_, i) => i !== index))
  }

  const handleCsvExport = async () => {
    try {
      const csvContent = await exportProductsCsv()
      const blob = new Blob([csvContent], { type: 'text/csv;charset=utf-8;' })
      const url = URL.createObjectURL(blob)
      const link = document.createElement('a')
      link.setAttribute('href', url)
      link.setAttribute('download', `catalog_export_${Date.now()}.csv`)
      document.body.appendChild(link)
      link.click()
      document.body.removeChild(link)
    } catch (err) {
      alert(err)
    }
  }

  const handleCsvImport = async () => {
    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: 'CSV Files', extensions: ['csv'] }]
      })

      if (selected && typeof selected === 'string') {
        // Read file using standard Tauri JS API or native command
        // For simplicity, we can invoke a file read or read it in rust
        // We will create a command for importing directly via file path
        const { readTextFile } = await import('@tauri-apps/plugin-fs')
        const content = await readTextFile(selected)
        await importProductsCsv(content)
        alert('Products imported successfully!')
      }
    } catch (err) {
      alert(`CSV Import Failed: ${err}`)
    }
  }

  // Filter products
  const filteredProducts = products.filter(p => {
    const matchesSearch = p.name.toLowerCase().includes(searchTerm.toLowerCase()) || 
                          p.sku.toLowerCase().includes(searchTerm.toLowerCase())
    const matchesCategory = selectedCategory === '' || p.category === selectedCategory
    return matchesSearch && matchesCategory
  })

  // Get unique categories for filter dropdown
  const categories = Array.from(new Set(products.map(p => p.category).filter(Boolean)))

  return (
    <div className="space-y-6">
      <div className="flex flex-col md:flex-row md:items-center md:justify-between gap-4">
        <div>
          <h1 className="text-3xl font-bold tracking-tight text-white font-display">Product Catalog</h1>
          <p className="text-sm text-gray-400 mt-1">Manage and track your clothing product list.</p>
        </div>
        <div className="flex items-center space-x-2">
          <button 
            onClick={handleCsvImport}
            className="flex items-center space-x-1 px-3 py-2 bg-slate-800 hover:bg-slate-700 text-gray-200 border border-gray-700 rounded-lg text-sm transition-colors"
          >
            <Upload size={16} />
            <span>Import CSV</span>
          </button>
          <button 
            onClick={handleCsvExport}
            className="flex items-center space-x-1 px-3 py-2 bg-slate-800 hover:bg-slate-700 text-gray-200 border border-gray-700 rounded-lg text-sm transition-colors"
          >
            <Download size={16} />
            <span>Export CSV</span>
          </button>
          <button 
            onClick={handleOpenAdd}
            className="flex items-center space-x-1 px-4 py-2 bg-violet-600 hover:bg-violet-700 text-white rounded-lg text-sm font-medium transition-colors"
          >
            <Plus size={16} />
            <span>Add Product</span>
          </button>
        </div>
      </div>

      {/* Filters & Search */}
      <div className="flex flex-col md:flex-row gap-4 bg-slate-900/40 p-4 rounded-xl border border-gray-800">
        <div className="relative flex-1">
          <Search className="absolute left-3 top-2.5 text-gray-500" size={18} />
          <input 
            type="text"
            placeholder="Search by SKU, Name..."
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
            className="w-full bg-slate-950 border border-gray-800 rounded-lg pl-10 pr-4 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500 transition-colors"
          />
        </div>
        <div className="w-full md:w-48">
          <select
            value={selectedCategory}
            onChange={(e) => setSelectedCategory(e.target.value)}
            className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500 transition-colors"
          >
            <option value="">All Categories</option>
            {categories.map(cat => (
              <option key={cat} value={cat}>{cat}</option>
            ))}
          </select>
        </div>
      </div>

      {/* Catalog Table */}
      <div className="glass-card overflow-hidden">
        <div className="overflow-x-auto">
          <table className="w-full text-left border-collapse">
            <thead>
              <tr className="border-b border-gray-800 bg-slate-900/50 text-xs font-semibold uppercase text-gray-400">
                <th className="py-3.5 px-4">Product Info</th>
                <th className="py-3.5 px-4">SKU</th>
                <th className="py-3.5 px-4">Category</th>
                <th className="py-3.5 px-4 text-right">Cost Price</th>
                <th className="py-3.5 px-4 text-right">Sale Price</th>
                <th className="py-3.5 px-4 text-center">Stock</th>
                <th className="py-3.5 px-4 text-center">Status</th>
                <th className="py-3.5 px-4 text-center">Actions</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-800 text-sm text-gray-300">
              {filteredProducts.length === 0 ? (
                <tr>
                  <td colSpan={8} className="py-10 text-center text-gray-500">
                    No products found. Add a product to get started.
                  </td>
                </tr>
              ) : (
                filteredProducts.map(p => (
                  <tr key={p.id} className="hover:bg-slate-900/20 transition-colors">
                    <td className="py-4 px-4 font-medium text-white flex items-center space-x-3">
                      <div className="w-10 h-10 bg-slate-800 rounded-lg flex items-center justify-center overflow-hidden border border-gray-700">
                        {p.images && JSON.parse(p.images).length > 0 ? (
                          <img 
                            // Note: In Tauri, we can serve local image using custom protocol or read it.
                            // We can use the file name directly since they are in local app data.
                            // To display, we can use a placeholder for local-only builds or a generic icon
                            // in this mock interface, or load via custom protocol if configured.
                            src="https://images.unsplash.com/photo-1596755094514-f87e34085b2c?w=100&auto=format&fit=crop&q=60"
                            alt={p.name}
                            className="object-cover w-full h-full"
                          />
                        ) : (
                          <ImageIcon size={18} className="text-gray-500" />
                        )}
                      </div>
                      <span>{p.name}</span>
                    </td>
                    <td className="py-4 px-4 font-mono text-xs">{p.sku}</td>
                    <td className="py-4 px-4">{p.category || '-'}</td>
                    <td className="py-4 px-4 text-right font-mono">${p.cost_price.toFixed(2)}</td>
                    <td className="py-4 px-4 text-right font-mono text-violet-400">${p.sale_price.toFixed(2)}</td>
                    <td className={`py-4 px-4 text-center font-bold ${p.stock_quantity <= 5 ? 'text-red-400' : 'text-gray-300'}`}>
                      {p.stock_quantity}
                    </td>
                    <td className="py-4 px-4 text-center">
                      <span className={`inline-flex items-center px-2 py-0.5 rounded-full text-xs font-semibold ${
                        p.status === 'active' ? 'bg-emerald-500/10 text-emerald-400' : 'bg-red-500/10 text-red-400'
                      }`}>
                        {p.status}
                      </span>
                    </td>
                    <td className="py-4 px-4 text-center">
                      <div className="flex items-center justify-center space-x-2">
                        <button 
                          onClick={() => handleOpenEdit(p)}
                          className="p-1 hover:text-violet-400 transition-colors"
                        >
                          <Edit size={16} />
                        </button>
                        <button 
                          onClick={() => p.id && handleDelete(p.id)}
                          className="p-1 hover:text-red-400 transition-colors"
                        >
                          <Trash2 size={16} />
                        </button>
                      </div>
                    </td>
                  </tr>
                ))
              )}
            </tbody>
          </table>
        </div>
      </div>

      {/* Modal Dialog */}
      {showModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm p-4">
          <div className="bg-slate-900 border border-gray-800 rounded-2xl w-full max-w-lg overflow-hidden shadow-2xl animate-in fade-in zoom-in-95 duration-150">
            <div className="flex items-center justify-between p-4 border-b border-gray-800 bg-slate-950/40">
              <h3 className="text-lg font-bold text-white font-display">
                {editProduct ? 'Edit Product' : 'Add New Product'}
              </h3>
              <button 
                onClick={() => setShowModal(false)}
                className="text-gray-400 hover:text-white transition-colors"
              >
                <X size={20} />
              </button>
            </div>

            <form onSubmit={handleSave} className="p-6 space-y-4 max-h-[80vh] overflow-y-auto">
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">SKU *</label>
                  <input 
                    type="text" 
                    required 
                    value={sku}
                    onChange={(e) => setSku(e.target.value)}
                    className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500"
                    placeholder="E.g. SKU-KURTA-001"
                  />
                </div>
                <div>
                  <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">Name *</label>
                  <input 
                    type="text" 
                    required 
                    value={name}
                    onChange={(e) => setName(e.target.value)}
                    className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500"
                    placeholder="E.g. Linen Kurta"
                  />
                </div>
              </div>

              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">Category</label>
                  <input 
                    type="text" 
                    value={category}
                    onChange={(e) => setCategory(e.target.value)}
                    className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500"
                    placeholder="E.g. Kurta, Pant, Shirt"
                  />
                </div>
                <div>
                  <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">Status</label>
                  <select 
                    value={status}
                    onChange={(e) => setStatus(e.target.value)}
                    className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500"
                  >
                    <option value="active">Active</option>
                    <option value="inactive">Inactive</option>
                  </select>
                </div>
              </div>

              <div className="grid grid-cols-3 gap-4">
                <div>
                  <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">Cost Price ($)</label>
                  <input 
                    type="number" 
                    step="0.01" 
                    value={costPrice}
                    onChange={(e) => setCostPrice(Number(e.target.value))}
                    className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500"
                  />
                </div>
                <div>
                  <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">Sale Price ($)</label>
                  <input 
                    type="number" 
                    step="0.01" 
                    value={salePrice}
                    onChange={(e) => setSalePrice(Number(e.target.value))}
                    className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500"
                  />
                </div>
                <div>
                  <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">Stock Qty</label>
                  <input 
                    type="number" 
                    value={stockQuantity}
                    onChange={(e) => setStockQuantity(Number(e.target.value))}
                    className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500"
                  />
                </div>
              </div>

              <div>
                <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">Description</label>
                <textarea 
                  value={description}
                  onChange={(e) => setDescription(e.target.value)}
                  rows={3}
                  className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500"
                  placeholder="Detailed description of the product..."
                />
              </div>

              <div>
                <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">Tags (Comma separated)</label>
                <input 
                  type="text" 
                  value={tags}
                  onChange={(e) => setTags(e.target.value)}
                  className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500"
                  placeholder="E.g. cotton, summer, fit"
                />
              </div>

              {/* Image Manager inside Modal */}
              <div>
                <label className="block text-xs font-semibold uppercase text-gray-400 mb-2">Product Images</label>
                <div className="flex flex-wrap gap-2">
                  {images.map((imgName, idx) => (
                    <div key={idx} className="relative w-16 h-16 bg-slate-950 border border-gray-800 rounded-lg overflow-hidden flex items-center justify-center">
                      <ImageIcon size={18} className="text-gray-600" />
                      <button 
                        type="button"
                        onClick={() => handleRemoveImage(idx)}
                        className="absolute top-0.5 right-0.5 bg-red-600 text-white rounded-full p-0.5"
                      >
                        <X size={10} />
                      </button>
                    </div>
                  ))}
                  <button
                    type="button"
                    onClick={handleSelectImage}
                    className="w-16 h-16 bg-slate-950 hover:bg-slate-900 border border-dashed border-gray-800 hover:border-violet-500 rounded-lg flex flex-col items-center justify-center transition-colors text-gray-400 hover:text-violet-400"
                  >
                    <Plus size={16} />
                    <span className="text-[10px] mt-1">Upload</span>
                  </button>
                </div>
              </div>

              <div className="flex justify-end space-x-2 pt-4 border-t border-gray-800">
                <button
                  type="button"
                  onClick={() => setShowModal(false)}
                  className="px-4 py-2 bg-slate-800 hover:bg-slate-700 text-gray-200 rounded-lg text-sm transition-colors"
                >
                  Cancel
                </button>
                <button
                  type="submit"
                  className="px-4 py-2 bg-violet-600 hover:bg-violet-700 text-white rounded-lg text-sm font-medium transition-colors"
                >
                  Save Product
                </button>
              </div>
            </form>
          </div>
        </div>
      )}
    </div>
  )
}
