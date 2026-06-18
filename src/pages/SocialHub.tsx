import React, { useState, useEffect } from 'react'
import { useAppStore, Product } from '../stores/store'
import { invoke } from '@tauri-apps/api/core'
import { 
  Facebook, 
  Instagram, 
  Send, 
  Sparkles, 
  Hash, 
  Save, 
  FileText,
  Trash2,
  Share2
} from 'lucide-react'

interface SocialDraft {
  id: string;
  productId?: number;
  productName?: string;
  content: string;
  platforms: string[];
  created_at: string;
}

export default function SocialHub() {
  const { products, fetchProducts, settings, updateSetting, fetchSettings } = useAppStore()
  
  const [selectedProductId, setSelectedProductId] = useState<number | ''>('')
  const [postContent, setPostContent] = useState('')
  const [selectedPlatforms, setSelectedPlatforms] = useState<string[]>([])
  const [drafts, setDrafts] = useState<SocialDraft[]>([])
  const [isGenerating, setIsGenerating] = useState(false)

  useEffect(() => {
    fetchProducts()
    fetchSettings()
    loadDrafts()
  }, [])

  const loadDrafts = async () => {
    try {
      const allSettings: Record<string, string> = await invoke('get_settings')
      if (allSettings.social_drafts) {
        setDrafts(JSON.parse(allSettings.social_drafts))
      } else {
        setDrafts([])
      }
    } catch (err) {
      console.error(err)
    }
  }

  const saveDraftsList = async (newList: SocialDraft[]) => {
    try {
      await updateSetting('social_drafts', JSON.stringify(newList))
      setDrafts(newList)
    } catch (err) {
      alert(`Failed to save draft: ${err}`)
    }
  }

  const handlePlatformToggle = (platform: string) => {
    if (selectedPlatforms.includes(platform)) {
      setSelectedPlatforms(selectedPlatforms.filter(p => p !== platform))
    } else {
      setSelectedPlatforms([...selectedPlatforms, platform])
    }
  }

  const handleGenerateContent = async (type: 'facebook' | 'instagram' | 'whatsapp' | 'hashtags') => {
    if (selectedProductId === '') {
      alert('Please select a product from the catalog to generate content for.')
      return
    }

    const product = products.find(p => p.id === selectedProductId)
    if (!product) return

    setIsGenerating(true)
    let prompt = ''

    if (type === 'facebook') {
      prompt = `Write an engaging Facebook promotional post for the following clothing item:
Name: ${product.name}
SKU: ${product.sku}
Category: ${product.category || 'Clothing'}
Price: $${product.sale_price}
Description: ${product.description || 'Premium quality'}
Tags: ${product.tags || ''}
Make it sound appealing, invite people to message us to order, and include a call to action.`
    } else if (type === 'instagram') {
      prompt = `Write a catchy, modern Instagram caption for the following clothing item:
Name: ${product.name}
SKU: ${product.sku}
Price: $${product.sale_price}
Description: ${product.description || ''}
Make it trendy and short, with spacing, emojis, and a few relevant hashtags at the bottom.`
    } else if (type === 'whatsapp') {
      prompt = `Write a clean, easy-to-read WhatsApp broadcast message for the following clothing item:
Name: ${product.name}
SKU: ${product.sku}
Price: $${product.sale_price}
Description: ${product.description || ''}
Format it nicely with bullet points, emojis, and clear instructions on how they can place an order by replying to the chat.`
    } else if (type === 'hashtags') {
      prompt = `Generate 20 high-performing Instagram and TikTok hashtags for a clothing product with these details:
Name: ${product.name}
Category: ${product.category || 'fashion'}
Tags: ${product.tags || ''}`
    }

    try {
      const response: any = await invoke('ask_ai', { prompt })
      setPostContent(response.text)
    } catch (err) {
      alert(`AI Generation failed: ${err}`)
    } finally {
      setIsGenerating(false)
    }
  }

  const handleSaveDraft = async () => {
    if (!postContent.trim()) return

    const product = products.find(p => p.id === selectedProductId)
    const newDraft: SocialDraft = {
      id: String(Date.now()),
      productId: product?.id,
      productName: product?.name || 'General Post',
      content: postContent,
      platforms: selectedPlatforms,
      created_at: new Date().toLocaleDateString()
    }

    const updated = [newDraft, ...drafts]
    await saveDraftsList(updated)
    alert('Draft saved locally!')
  }

  const handleDeleteDraft = async (id: string) => {
    const updated = drafts.filter(d => d.id !== id)
    await saveDraftsList(updated)
  }

  const handleLoadDraft = (draft: SocialDraft) => {
    setPostContent(draft.content)
    setSelectedPlatforms(draft.platforms)
    if (draft.productId) {
      setSelectedProductId(draft.productId)
    } else {
      setSelectedProductId('')
    }
  }

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold tracking-tight text-white font-display">Social Hub</h1>
        <p className="text-sm text-gray-400 mt-1">Generate AI content and draft social media posts.</p>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Creator Panel */}
        <div className="glass-card p-5 lg:col-span-2 space-y-4">
          <h2 className="text-lg font-semibold text-white">Create Post</h2>
          
          <div>
            <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">Select Product *</label>
            <select
              value={selectedProductId}
              onChange={(e) => setSelectedProductId(e.target.value ? Number(e.target.value) : '')}
              className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500"
            >
              <option value="">-- Choose a Product --</option>
              {products.map(p => (
                <option key={p.id} value={p.id}>{p.name} ({p.sku})</option>
              ))}
            </select>
          </div>

          {/* AI Generator Buttons */}
          <div>
            <label className="block text-xs font-semibold uppercase text-gray-400 mb-2">AI Generators</label>
            <div className="grid grid-cols-2 md:grid-cols-4 gap-2">
              <button 
                onClick={() => handleGenerateContent('facebook')}
                disabled={isGenerating}
                className="flex items-center justify-center space-x-1 p-2 bg-violet-600/10 hover:bg-violet-600/20 text-violet-400 rounded-lg border border-violet-500/10 text-xs font-medium transition-colors disabled:opacity-50"
              >
                <Sparkles size={14} />
                <span>Facebook Post</span>
              </button>
              <button 
                onClick={() => handleGenerateContent('instagram')}
                disabled={isGenerating}
                className="flex items-center justify-center space-x-1 p-2 bg-pink-600/10 hover:bg-pink-600/20 text-pink-400 rounded-lg border border-pink-500/10 text-xs font-medium transition-colors disabled:opacity-50"
              >
                <Instagram size={14} />
                <span>Instagram Caption</span>
              </button>
              <button 
                onClick={() => handleGenerateContent('whatsapp')}
                disabled={isGenerating}
                className="flex items-center justify-center space-x-1 p-2 bg-emerald-600/10 hover:bg-emerald-600/20 text-emerald-400 rounded-lg border border-emerald-500/10 text-xs font-medium transition-colors disabled:opacity-50"
              >
                <Share2 size={14} />
                <span>WhatsApp Text</span>
              </button>
              <button 
                onClick={() => handleGenerateContent('hashtags')}
                disabled={isGenerating}
                className="flex items-center justify-center space-x-1 p-2 bg-cyan-600/10 hover:bg-cyan-600/20 text-cyan-400 rounded-lg border border-cyan-500/10 text-xs font-medium transition-colors disabled:opacity-50"
              >
                <Hash size={14} />
                <span>Hashtags</span>
              </button>
            </div>
          </div>

          <div>
            <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">Post Content</label>
            <textarea
              value={postContent}
              onChange={(e) => setPostContent(e.target.value)}
              rows={8}
              placeholder={isGenerating ? "AI is generating your post..." : "Your post content goes here..."}
              disabled={isGenerating}
              className="w-full bg-slate-950 border border-gray-800 rounded-lg p-3 text-sm text-gray-200 focus:outline-none focus:border-violet-500"
            />
          </div>

          {/* Targets */}
          <div>
            <label className="block text-xs font-semibold uppercase text-gray-400 mb-2">Publish Channels</label>
            <div className="flex space-x-3">
              {['Facebook', 'Instagram', 'WhatsApp', 'Twitter/X'].map(platform => (
                <label key={platform} className="flex items-center space-x-2 text-sm text-gray-300 cursor-pointer">
                  <input 
                    type="checkbox"
                    checked={selectedPlatforms.includes(platform)}
                    onChange={() => handlePlatformToggle(platform)}
                    className="accent-violet-500"
                  />
                  <span>{platform}</span>
                </label>
              ))}
            </div>
          </div>

          <div className="flex justify-end space-x-2 pt-2">
            <button
              onClick={handleSaveDraft}
              className="flex items-center space-x-1 px-4 py-2 bg-slate-800 hover:bg-slate-700 text-gray-200 border border-gray-700 rounded-lg text-sm transition-colors"
            >
              <Save size={16} />
              <span>Save Draft</span>
            </button>
          </div>
        </div>

        {/* Drafts List */}
        <div className="glass-card p-5 flex flex-col">
          <h2 className="text-lg font-semibold text-white mb-4">Saved Drafts</h2>
          <div className="flex-1 overflow-y-auto space-y-3 max-h-[400px]">
            {drafts.length === 0 ? (
              <p className="text-sm text-gray-500 text-center py-8">No saved drafts found.</p>
            ) : (
              drafts.map(d => (
                <div key={d.id} className="p-3 bg-slate-950 border border-gray-800 rounded-lg hover:border-violet-500/50 transition-colors flex justify-between items-start">
                  <div className="space-y-1 flex-1 cursor-pointer" onClick={() => handleLoadDraft(d)}>
                    <div className="flex items-center justify-between">
                      <span className="text-xs font-semibold text-violet-400">{d.productName}</span>
                      <span className="text-[10px] text-gray-500">{d.created_at}</span>
                    </div>
                    <p className="text-xs text-gray-300 line-clamp-2 pr-2">{d.content}</p>
                    <div className="flex gap-1">
                      {d.platforms.map(p => (
                        <span key={p} className="text-[9px] bg-slate-900 text-gray-400 px-1.5 py-0.5 rounded border border-gray-800">{p}</span>
                      ))}
                    </div>
                  </div>
                  <button 
                    onClick={() => handleDeleteDraft(d.id)}
                    className="text-gray-500 hover:text-red-400 transition-colors p-1"
                  >
                    <Trash2 size={14} />
                  </button>
                </div>
              ))
            )}
          </div>
        </div>
      </div>
    </div>
  )
}
