import { useState } from 'react'
import { useAppStore, ProductDraft } from '../stores/store'
import { Check, X, Edit3, RefreshCw, Sparkles } from 'lucide-react'
import ProductImage from './ProductImage'

interface Props {
  draft: ProductDraft
  confidence: number
  missingFields: string[]
  index: number
}

export default function ProductDraftCard({ draft: initialDraft, confidence, missingFields, index }: Props) {
  const { updateAiProductDraft, removeAiProductDraft, addDraftToCatalog } = useAppStore()
  const [draft, setDraft] = useState<ProductDraft>(initialDraft)
  const [isEditing, setIsEditing] = useState(false)
  const [isSaving, setIsSaving] = useState(false)
  const [saved, setSaved] = useState(false)

  const handleFieldChange = (field: keyof ProductDraft, value: any) => {
    const updated = { ...draft, [field]: value }
    setDraft(updated)
    updateAiProductDraft(index, updated)
  }

  const handleApprove = async () => {
    setIsSaving(true)
    try {
      await addDraftToCatalog(draft)
      setSaved(true)
      removeAiProductDraft(index)
      setTimeout(() => setSaved(false), 2000)
    } catch (err) {
      alert(`Error saving: ${err}`)
    }
    setIsSaving(false)
  }

  const confidenceColor = confidence >= 0.8 ? 'text-green-400' : confidence >= 0.5 ? 'text-yellow-400' : 'text-red-400'
  const confidenceLabel = confidence >= 0.8 ? 'High' : confidence >= 0.5 ? 'Medium' : 'Low'

  if (saved) {
    return (
      <div className="p-3 bg-green-900/20 border border-green-800/40 rounded-xl text-sm text-green-400 text-center">
        <Check size={16} className="inline mr-1" /> Added to Catalog!
      </div>
    )
  }

  return (
    <div className="bg-slate-800/40 border border-violet-500/20 rounded-xl p-3 space-y-2 text-sm">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center space-x-2">
          <Sparkles size={14} className="text-violet-400" />
          <span className="text-xs font-semibold text-violet-300">Product Draft</span>
          <span className={`text-[10px] font-medium ${confidenceColor}`}>
            {confidenceLabel} ({Math.round(confidence * 100)}%)
          </span>
        </div>
        <div className="flex items-center space-x-1">
          <button onClick={() => setIsEditing(!isEditing)} className="p-1 hover:text-violet-400 transition-colors">
            <Edit3 size={12} />
          </button>
          <button onClick={() => removeAiProductDraft(index)} className="p-1 hover:text-red-400 transition-colors">
            <X size={12} />
          </button>
        </div>
      </div>

      {/* Fields */}
      <div className="space-y-1.5">
        {isEditing ? (
          <>
            {['name', 'sku', 'category', 'brand', 'fabric', 'color', 'design', 'season'].map(field => (
              <div key={field}>
                <label className="text-[10px] uppercase text-gray-500 block">{field}</label>
                <input
                  type="text"
                  value={(draft as any)[field] || ''}
                  onChange={e => handleFieldChange(field as keyof ProductDraft, e.target.value)}
                  className="w-full bg-slate-950 border border-gray-800 rounded px-2 py-1 text-xs text-gray-200 focus:outline-none focus:border-violet-500"
                />
              </div>
            ))}
            <div>
              <label className="text-[10px] uppercase text-gray-500 block">Description</label>
              <textarea
                value={draft.description || ''}
                onChange={e => handleFieldChange('description', e.target.value)}
                rows={2}
                className="w-full bg-slate-950 border border-gray-800 rounded px-2 py-1 text-xs text-gray-200 focus:outline-none focus:border-violet-500"
              />
            </div>
            <div className="grid grid-cols-3 gap-2">
              {['cost_price', 'sale_price', 'retail_price'].map(field => (
                <div key={field}>
                  <label className="text-[10px] uppercase text-gray-500 block">{field.replace('_', ' ')}</label>
                  <input
                    type="number"
                    step="0.01"
                    value={(draft as any)[field] || 0}
                    onChange={e => handleFieldChange(field as keyof ProductDraft, parseFloat(e.target.value) || 0)}
                    className="w-full bg-slate-950 border border-gray-800 rounded px-2 py-1 text-xs text-gray-200 focus:outline-none focus:border-violet-500"
                  />
                </div>
              ))}
            </div>
          </>
        ) : (
          <>
            {/* Image preview */}
            {draft.images && draft.images.length > 0 && (
              <div className="flex -mx-3 -mt-3 mb-2 overflow-hidden rounded-t-xl bg-slate-900 items-center justify-center aspect-square">
                <ProductImage
                  filename={draft.images[0]}
                  alt={draft.name || 'Product image'}
                  className="w-full h-full object-contain"
                  iconSize={24}
                />
              </div>
            )}
            <div className="flex items-center justify-between">
              <span className="text-white font-medium text-sm">{draft.name || 'Unnamed Product'}</span>
              {draft.sku && <span className="text-[10px] font-mono text-gray-500">{draft.sku}</span>}
            </div>
            <div className="flex flex-wrap gap-1">
              {draft.category && <span className="text-[10px] bg-slate-700 px-1.5 py-0.5 rounded">{draft.category}</span>}
              {draft.color && <span className="text-[10px] bg-slate-700 px-1.5 py-0.5 rounded">{draft.color}</span>}
              {draft.season && <span className="text-[10px] bg-slate-700 px-1.5 py-0.5 rounded">{draft.season}</span>}
              {draft.brand && <span className="text-[10px] bg-slate-700 px-1.5 py-0.5 rounded">{draft.brand}</span>}
              {draft.fabric && <span className="text-[10px] bg-slate-700 px-1.5 py-0.5 rounded">{draft.fabric}</span>}
            </div>
            <div className="text-xs text-gray-400">
              Sale: <span className="text-violet-400">Rs.{draft.sale_price?.toFixed(0) || '—'}</span>
              {draft.retail_price && <> | Retail: Rs.{draft.retail_price.toFixed(0)}</>}
              {draft.cost_price && <> | Cost: Rs.{draft.cost_price.toFixed(0)}</>}
            </div>
            {draft.description && <p className="text-[11px] text-gray-500 line-clamp-2">{draft.description}</p>}
            {draft.keywords && draft.keywords.length > 0 && (
              <div className="flex flex-wrap gap-0.5">
                {draft.keywords.map((k, i) => <span key={i} className="text-[9px] text-gray-600">#{k}</span>)}
              </div>
            )}
          </>
        )}
      </div>

      {/* Missing fields */}
      {missingFields.length > 0 && !isEditing && (
        <div className="text-[10px] text-yellow-500 space-y-0.5">
          <span className="font-semibold">Missing:</span>
          {missingFields.map((f, i) => (
            <span key={i} className="block">• {f}</span>
          ))}
        </div>
      )}

      {/* Actions */}
      <div className="flex items-center space-x-2 pt-1">
        <button
          onClick={handleApprove}
          disabled={isSaving}
          className="flex items-center space-x-1 px-3 py-1.5 bg-violet-600 hover:bg-violet-700 text-white text-xs rounded-lg transition-colors disabled:opacity-50"
        >
          {isSaving ? <RefreshCw size={12} className="animate-spin" /> : <Check size={12} />}
          <span>Add To Catalog</span>
        </button>
        <button
          onClick={() => setIsEditing(!isEditing)}
          className="flex items-center space-x-1 px-3 py-1.5 bg-slate-700 hover:bg-slate-600 text-gray-200 text-xs rounded-lg transition-colors"
        >
          <Edit3 size={12} />
          <span>{isEditing ? 'Done Editing' : 'Edit Draft'}</span>
        </button>
      </div>
    </div>
  )
}
