import { useRef, useState, useCallback, useEffect } from 'react'
import { useAppStore, CatalogDraft } from '../stores/store'
import ProductDraftCard from './ProductDraftCard'
import FormattedMessage from './FormattedMessage'
import { invoke } from '@tauri-apps/api/core'
import {
  Send, X, Plus, Image, Link2, FileText, Upload,
  Trash2, GripVertical, Sparkles, Check, Ban, Copy, Sparkle
} from 'lucide-react'
import { shareToPlatform } from '../utils/share'

/**
 * Renders a remote web image with graceful fallback to a base64-uploaded image
 * if the remote URL fails to load (404, 403 hotlink-protected, expired params,
 * CORS-blocked, etc). Also resets on `src` change so a stale error state from
 * a previous URL does not poison the next render.
 *
 * `referrerPolicy="no-referrer"` is critical: many e-commerce sites hotlink-
 * protect their images and return 403 if the Referer header is from a desktop
 * app rather than their own domain.
 */
function WebImageWithFallback({
  src,
  fallbackBase64,
  className,
}: {
  src: string
  fallbackBase64?: string | null
  className?: string
}) {
  const [errored, setErrored] = useState(false)

  useEffect(() => {
    setErrored(false)
  }, [src])

  if (errored) {
    if (fallbackBase64) {
      return (
        <img
          src={`data:image/jpeg;base64,${fallbackBase64}`}
          alt="Product thumbnail"
          className={className}
        />
      )
    }
    return null
  }

  return (
    <img
      src={src}
      alt="Product from web"
      className={className}
      onError={() => setErrored(true)}
      referrerPolicy="no-referrer"
    />
  )
}

export default function AiWorkspace() {
  const {
    aiMessages, isAiLoading, sendAiMessage, clearAiChat,
    showAiAssistant, setVectorAssistant,
    aiWorkspaceWidth, setAiWorkspaceWidth,
    aiProductDrafts, removeAiMessage
  } = useAppStore()

  const [inputText, setInputText] = useState('')
  const [pendingImage, setPendingImage] = useState<string | null>(null)
  const [pendingImageName, setPendingImageName] = useState('')
  const [showMenu, setShowMenu] = useState(false)
  const [isDragging, setIsDragging] = useState(false)
  const [toast, setToast] = useState<string | null>(null)
  const [savingIndex, setSavingIndex] = useState<number | null>(null)
  const [generatingIndex, setGeneratingIndex] = useState<number | null>(null)
  const messagesEndRef = useRef<HTMLDivElement>(null)
  const fileInputRef = useRef<HTMLInputElement>(null)
  const panelRef = useRef<HTMLDivElement>(null)
  const isResizing = useRef(false)

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' })
  }, [aiMessages])

  useEffect(() => {
    if (toast) {
      const t = setTimeout(() => setToast(null), 3000)
      return () => clearTimeout(t)
    }
  }, [toast])

  const handleAddToCatalog = async (index: number, draft: import('../stores/store').CatalogDraft) => {
    setSavingIndex(index)
    try {
      await invoke('save_catalog_draft', { draft })
      setToast('Item added to catalog!')
      removeAiMessage(index)
    } catch (err) {
      const msg = String(err)
      if (msg.includes('Duplicate item found')) {
        setToast('Warning: This item is already in your catalog!')
      } else {
        setToast(`Failed to save: ${err}`)
      }
    } finally {
      setSavingIndex(null)
    }
  }

  const handleDiscardDraft = (index: number) => {
    removeAiMessage(index)
  }

  const handleSaveAndGenerate = async (index: number, draft: CatalogDraft) => {
    setGeneratingIndex(index)
    try {
      const productId = await invoke<number>('save_catalog_draft', { draft })
      const post = await invoke<import('../stores/store').MarketingPost>('generate_social_post', { productId })
      setToast('Item added to catalog!')
      removeAiMessage(index)
      const postMsg = {
        role: 'assistant' as const,
        text: '',
        social_post: post,
      }
      useAppStore.setState((state) => ({
        aiMessages: [...state.aiMessages, postMsg as any],
      }))
    } catch (err) {
      const msg = String(err)
      if (msg.includes('Duplicate item found')) {
        setToast('Warning: This item is already in your catalog!')
      } else {
        setToast(`Failed: ${err}`)
      }
    } finally {
      setGeneratingIndex(null)
    }
  }

  const handleGeneratePostForExisting = async (index: number, itemId: string) => {
    setGeneratingIndex(index)
    try {
      const post = await invoke<import('../stores/store').MarketingPost>('generate_social_post', { productId: parseInt(itemId) })
      const postMsg = {
        role: 'assistant' as const,
        text: '',
        social_post: post,
      }
      useAppStore.setState((state) => ({
        aiMessages: [...state.aiMessages, postMsg as any],
      }))
    } catch (err) {
      setToast(`Failed to generate: ${err}`)
    } finally {
      setGeneratingIndex(null)
    }
  }

  const copyToClipboard = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text)
      setToast('Copied to clipboard!')
    } catch {
      setToast('Failed to copy')
    }
  }

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault()
    const text = inputText.trim()
    if (!text && !pendingImage) return

    const prompt = text || 'Process this image for cataloging'
    sendAiMessage(prompt, pendingImage || undefined)
    setInputText('')
    setPendingImage(null)
    setPendingImageName('')
  }

  const handleImageSelect = () => {
    fileInputRef.current?.click()
    setShowMenu(false)
  }

  const handleFileChange = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0]
    if (!file) return
    if (file.type.startsWith('image/')) {
      const reader = new FileReader()
      reader.onload = (ev) => {
        const base64 = (ev.target?.result as string)?.split(',')[1]
        if (base64) {
          setPendingImage(base64)
          setPendingImageName(file.name)
        }
      }
      reader.readAsDataURL(file)
    } else if (file.type === 'application/pdf' || file.type.includes('text')) {
      setInputText(`[File: ${file.name}] Please analyze this document and extract any product information.`)
    }
    e.target.value = ''
  }

  const handlePasteLink = () => {
    const link = prompt('Paste product link (Facebook, Instagram, TikTok, or brand URL):')
    if (link && link.trim()) {
      sendAiMessage(`Analyze this product link and create a product draft: ${link.trim()}`)
    }
    setShowMenu(false)
  }

  const handleDragOver = useCallback((e: React.DragEvent) => {
    e.preventDefault()
    setIsDragging(true)
  }, [])

  const handleDragLeave = useCallback(() => {
    setIsDragging(false)
  }, [])

  const handleDrop = useCallback((e: React.DragEvent) => {
    e.preventDefault()
    setIsDragging(false)
    const files = e.dataTransfer.files
    if (files.length > 0) {
      const file = files[0]
      if (file.type.startsWith('image/')) {
        const reader = new FileReader()
        reader.onload = (ev) => {
          const base64 = (ev.target?.result as string)?.split(',')[1]
          if (base64) {
            setPendingImage(base64)
            setPendingImageName(file.name)
          }
        }
        reader.readAsDataURL(file)
      }
    }
  }, [])

  const handlePaste = useCallback((e: React.ClipboardEvent) => {
    const items = e.clipboardData?.items
    if (!items) return
    for (let i = 0; i < items.length; i++) {
      const item = items[i]
      if (item.type.startsWith('image/')) {
        const file = item.getAsFile()
        if (file) {
          e.preventDefault()
          const reader = new FileReader()
          reader.onload = (ev) => {
            const base64 = (ev.target?.result as string)?.split(',')[1]
            if (base64) {
              setPendingImage(base64)
              setPendingImageName('Pasted image')
            }
          }
          reader.readAsDataURL(file)
          return
        }
      }
    }
  }, [])

  // Resize handler
  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault()
    isResizing.current = true
    const startX = e.clientX
    const startWidth = panelRef.current?.getBoundingClientRect().width || 0
    const parentWidth = panelRef.current?.parentElement?.getBoundingClientRect().width || window.innerWidth

    const handleMouseMove = (ev: MouseEvent) => {
      if (!isResizing.current) return
      const delta = startX - ev.clientX
      const newWidthPx = startWidth + delta
      const newPercent = (newWidthPx / parentWidth) * 100
      const clamped = Math.max(25, Math.min(60, newPercent))
      setAiWorkspaceWidth(Math.round(clamped))
    }

    const handleMouseUp = () => {
      isResizing.current = false
      document.removeEventListener('mousemove', handleMouseMove)
      document.removeEventListener('mouseup', handleMouseUp)
    }

    document.addEventListener('mousemove', handleMouseMove)
    document.addEventListener('mouseup', handleMouseUp)
  }, [setAiWorkspaceWidth])

  const clearPendingImage = () => {
    setPendingImage(null)
    setPendingImageName('')
  }

  if (!showAiAssistant) return null

  const activeDrafts = aiProductDrafts

  return (
    <aside
      ref={panelRef}
      style={{ width: `${aiWorkspaceWidth}%` }}
      className="bg-slate-900/60 border-l border-gray-800/60 flex flex-col shrink-0 relative min-w-[25%] max-w-[60%]"
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
      onPaste={handlePaste}
    >
      {/* Resize Handle */}
      <div
        onMouseDown={handleMouseDown}
        className="absolute left-0 top-0 bottom-0 w-1.5 cursor-col-resize hover:bg-violet-500/50 active:bg-violet-500 transition-colors z-10 group"
      >
        <div className="absolute left-0.5 top-1/2 -translate-y-1/2 opacity-0 group-hover:opacity-100 transition-opacity">
          <GripVertical size={12} className="text-violet-400" />
        </div>
      </div>

      {/* Header */}
      <div className="flex items-center justify-between p-3 border-b border-gray-800/60 shrink-0">
        <h2 className="text-sm font-semibold text-white flex items-center space-x-2">
          <Sparkles size={16} className="text-violet-500" />
          <span>AI Workspace</span>
        </h2>
        <div className="flex items-center space-x-1">
          <button onClick={clearAiChat} className="p-1 text-gray-500 hover:text-gray-300 transition-colors" title="Clear chat">
            <Trash2 size={14} />
          </button>
          <button onClick={() => setVectorAssistant(false)} className="p-1 text-gray-500 hover:text-gray-300 transition-colors">
            <X size={16} />
          </button>
        </div>
      </div>

      {/* Messages + Drafts */}
      <div className="flex-1 overflow-y-auto p-3 space-y-3">
        {aiMessages.map((msg, i) => (
          <div key={i}>
            <div
              dir="ltr"
              className={`text-sm ${msg.role === 'user' ? 'text-right' : 'text-left'}`}
            >
              <span
                className={`inline-block px-3 py-2 rounded-xl max-w-[95%] ${
                  msg.role === 'user'
                    ? 'bg-violet-600/20 text-violet-200 border border-violet-500/10'
                    : 'bg-slate-800/60 text-gray-300 border border-gray-800'
                }`}
              >
                <span className="text-[10px] block text-gray-500 mb-1">
                  {msg.role === 'user' ? 'You' : 'AI'}
                </span>
                <FormattedMessage text={msg.text} />
                {msg.image_data && (
                  <div className="mt-1.5">
                    <img
                      src={`data:image/jpeg;base64,${msg.image_data}`}
                      alt="Uploaded"
                      className="w-16 h-16 rounded-lg object-contain border border-gray-700/50"
                    />
                  </div>
                )}
              </span>
            </div>
            {msg.role === 'assistant' && msg.product_draft && msg.confidence !== undefined && !msg.fast_path_data && (
              <div className="mt-2">
                <ProductDraftCard
                  draft={msg.product_draft}
                  confidence={msg.confidence}
                  missingFields={msg.missing_fields || []}
                  index={activeDrafts.findIndex(d => d.draft === msg.product_draft)}
                />
              </div>
            )}
            {msg.role === 'assistant' && msg.fast_path_data && (
              <div className="mt-2">
                {msg.fast_path_data.type === 'LocalMatchFound' && (
                  <div className="bg-emerald-900/30 border border-emerald-700/30 rounded-lg px-3 py-2 text-xs text-emerald-300">
                    <div className="flex items-start space-x-2">
                      {msg.image_data && (
                        <img src={`data:image/jpeg;base64,${msg.image_data}`} alt="" className="w-10 h-10 rounded object-contain shrink-0 border border-emerald-700/30" />
                      )}
                      <div className="flex-1 min-w-0">
                        <span className="font-semibold">Item already exists:</span>{' '}
                        <span>{msg.fast_path_data.data.title}</span>
                        {msg.fast_path_data.data.design_code && (
                          <span className="text-emerald-400/70 ml-1">
                            ({msg.fast_path_data.data.design_code})
                          </span>
                        )}
                      </div>
                    </div>
                    <div className="mt-2 pt-2 border-t border-emerald-800/20">
                      <button
                        onClick={() => handleGeneratePostForExisting(i, (msg.fast_path_data!.data as import('../stores/store').LocalMatchResult).item_id)}
                        disabled={generatingIndex === i}
                        className="flex items-center space-x-1 px-2.5 py-1 bg-emerald-700 hover:bg-emerald-600 disabled:opacity-50 text-emerald-200 rounded text-[11px] font-medium transition-colors"
                      >
                        <Sparkle size={12} />
                        <span>{generatingIndex === i ? 'Generating...' : 'Generate Post'}</span>
                      </button>
                    </div>
                  </div>
                )}
                {msg.fast_path_data.type === 'NewCatalogDraft' && (
                  <div className="bg-violet-900/30 border border-violet-700/30 rounded-lg px-3 py-2 text-xs space-y-1">
                    <div className="flex items-start space-x-2 mb-1">
                      {/* Show web image if available, otherwise fallback to uploaded image.
                          WebImageWithFallback handles broken/404 web URLs by silently
                          falling back to the user-uploaded image_data. */}
                      {msg.fast_path_data.data.best_image_url ? (
                        <WebImageWithFallback
                          src={msg.fast_path_data.data.best_image_url}
                          fallbackBase64={msg.image_data}
                          className="w-10 h-10 rounded object-contain shrink-0 border border-violet-700/30"
                        />
                      ) : msg.image_data ? (
                        <img src={`data:image/jpeg;base64,${msg.image_data}`} alt="Product thumbnail" className="w-10 h-10 rounded object-contain shrink-0 border border-violet-700/30" />
                      ) : null}
                      <div className="text-violet-300 font-semibold">Catalog Draft</div>
                    </div>
                    <div className="text-gray-300">
                      <span className="text-gray-500">Title:</span>{' '}
                      {msg.fast_path_data.data.title}
                    </div>
                    {msg.fast_path_data.data.brand && (
                      <div className="text-gray-300">
                        <span className="text-gray-500">Brand:</span>{' '}
                        {msg.fast_path_data.data.brand}
                      </div>
                    )}
                    {msg.fast_path_data.data.fabric && (
                      <div className="text-gray-300">
                        <span className="text-gray-500">Fabric:</span>{' '}
                        {msg.fast_path_data.data.fabric}
                      </div>
                    )}
                    {msg.fast_path_data.data.design_code && (
                      <div className="text-gray-300">
                        <span className="text-gray-500">Design Code:</span>{' '}
                        {msg.fast_path_data.data.design_code}
                      </div>
                    )}
                    {msg.fast_path_data.data.notes && (
                      <div className="text-gray-400 italic mt-1">
                        {msg.fast_path_data.data.notes}
                      </div>
                    )}
                    {msg.fast_path_data.data.web_evidence_count && (
                      <div className="text-cyan-400/80 text-[10px] mt-1.5 border-t border-violet-800/30 pt-1">
                        Web Evidence: Found {msg.fast_path_data.data.web_evidence_count} matching result{msg.fast_path_data.data.web_evidence_count !== 1 ? 's' : ''} from internet search
                      </div>
                    )}
                    <div className="flex items-center space-x-2 mt-2 pt-2 border-t border-violet-800/20">
                      <button
                        onClick={() => handleAddToCatalog(i, msg.fast_path_data!.data)}
                        disabled={savingIndex === i}
                        className="flex items-center space-x-1 px-2.5 py-1 bg-emerald-600 hover:bg-emerald-500 disabled:bg-emerald-800 text-white rounded text-[11px] font-medium transition-colors"
                      >
                        <Check size={12} />
                        <span>{savingIndex === i ? 'Saving...' : 'Add to Catalog'}</span>
                      </button>
                      <button
                        onClick={() => handleSaveAndGenerate(i, msg.fast_path_data!.data)}
                        disabled={savingIndex === i || generatingIndex === i}
                        className="flex items-center space-x-1 px-2.5 py-1 bg-violet-600 hover:bg-violet-500 disabled:bg-violet-800 text-white rounded text-[11px] font-medium transition-colors"
                      >
                        <Sparkle size={12} />
                        <span>{generatingIndex === i ? 'Generating...' : 'Save & Generate Post'}</span>
                      </button>
                      <button
                        onClick={() => handleDiscardDraft(i)}
                        disabled={savingIndex === i || generatingIndex === i}
                        className="flex items-center space-x-1 px-2.5 py-1 bg-red-600/60 hover:bg-red-500/80 disabled:opacity-50 text-red-200 rounded text-[11px] font-medium transition-colors"
                      >
                        <Ban size={12} />
                        <span>Discard</span>
                      </button>
                    </div>
              </div>
            )}
            {msg.role === 'assistant' && (msg as any).social_post && (
              <div className="mt-2">
                <div className="bg-gradient-to-r from-pink-900/30 to-violet-900/30 border border-pink-700/30 rounded-xl px-4 py-3 space-y-3 text-xs">
                  <div className="flex items-start space-x-2">
                    {msg.image_data && (
                      <img src={`data:image/jpeg;base64,${msg.image_data}`} alt="" className="w-10 h-10 rounded object-contain shrink-0 border border-pink-700/30" />
                    )}
                    <div className="text-pink-300 font-semibold flex items-center space-x-1.5">
                      <Sparkle size={14} />
                      <span>Social Media Post</span>
                    </div>
                  </div>
                  <div>
                    <div className="flex items-center justify-between mb-1">
                      <span className="text-gray-500 uppercase text-[10px] font-semibold">Short Caption (WhatsApp)</span>
                      <button onClick={() => copyToClipboard((msg as any).social_post.short_caption)} className="text-pink-400/70 hover:text-pink-300 transition-colors">
                        <Copy size={12} />
                      </button>
                    </div>
                    <p className="text-gray-200 bg-black/20 rounded-lg px-3 py-2 leading-relaxed">{(msg as any).social_post.short_caption}</p>
                  </div>
                  <div>
                    <div className="flex items-center justify-between mb-1">
                      <span className="text-gray-500 uppercase text-[10px] font-semibold">Long Caption (Instagram/Facebook)</span>
                      <button onClick={() => copyToClipboard((msg as any).social_post.long_caption)} className="text-pink-400/70 hover:text-pink-300 transition-colors">
                        <Copy size={12} />
                      </button>
                    </div>
                    <p className="text-gray-200 bg-black/20 rounded-lg px-3 py-2 leading-relaxed">{(msg as any).social_post.long_caption}</p>
                  </div>
                  <div>
                    <span className="text-gray-500 uppercase text-[10px] font-semibold block mb-1">Hashtags</span>
                    <div className="flex flex-wrap gap-1">
                      {(msg as any).social_post.hashtags.map((tag: string, ti: number) => (
                        <span key={ti} className="text-pink-400 bg-pink-900/20 px-2 py-0.5 rounded-full text-[10px]">{tag}</span>
                      ))}
                    </div>
                  </div>
                  <div className="pt-2 border-t border-pink-800/20">
                    <button
                      onClick={async () => {
                        const fullText = `${(msg as any).social_post.short_caption}\n\n${(msg as any).social_post.hashtags.join(' ')}`;
                        try {
                          await shareToPlatform('whatsapp', fullText);
                        } catch (err) {
                          console.error('[AiWorkspace] WhatsApp share failed:', err);
                          alert(`Could not open WhatsApp. Error: ${err}`);
                        }
                      }}
                      className="flex items-center justify-center space-x-2 w-full px-3 py-2 bg-green-600 hover:bg-green-500 text-white rounded-lg text-xs font-medium transition-colors"
                    >
                      <svg viewBox="0 0 24 24" fill="currentColor" className="w-4 h-4"><path d="M17.472 14.382c-.297-.149-1.758-.867-2.03-.967-.273-.099-.471-.148-.67.15-.197.297-.767.966-.94 1.164-.173.199-.347.223-.644.075-.297-.15-1.255-.463-2.39-1.475-.883-.788-1.48-1.761-1.653-2.059-.173-.297-.018-.458.13-.606.134-.133.298-.347.446-.52.149-.174.198-.298.298-.497.099-.198.05-.371-.025-.52-.075-.149-.669-1.612-.916-2.207-.242-.579-.487-.5-.669-.51-.173-.008-.371-.01-.57-.01-.198 0-.52.074-.792.372-.272.297-1.04 1.016-1.04 2.479 0 1.462 1.065 2.875 1.213 3.074.149.198 2.096 3.2 5.077 4.487.709.306 1.262.489 1.694.625.712.227 1.36.195 1.871.118.571-.085 1.758-.719 2.006-1.413.248-.694.248-1.289.173-1.413-.074-.124-.272-.198-.57-.347m-5.421 7.403h-.004a9.87 9.87 0 01-5.031-1.378l-.361-.214-3.741.982.998-3.648-.235-.374a9.86 9.86 0 01-1.51-5.26c.001-5.45 4.436-9.884 9.888-9.884 2.64 0 5.122 1.03 6.988 2.898a9.825 9.825 0 012.893 6.994c-.003 5.45-4.437 9.884-9.885 9.884m8.413-18.297A11.815 11.815 0 0012.05 0C5.495 0 .16 5.335.157 11.892c0 2.096.547 4.142 1.588 5.945L.057 24l6.305-1.654a11.882 11.882 0 005.683 1.448h.005c6.554 0 11.89-5.335 11.893-11.893a11.821 11.821 0 00-3.48-8.413z"/></svg>
                      <span>Share to WhatsApp</span>
                    </button>
                  </div>
                </div>
              </div>
            )}
          </div>
            )}
          </div>
        ))}
        {isAiLoading && (
          <div className="text-left text-sm">
            <span className="inline-block px-3 py-2 rounded-xl bg-slate-800/60 text-gray-400 border border-gray-800">
              <span className="flex items-center space-x-2">
                <span className="w-1.5 h-1.5 bg-violet-400 rounded-full animate-pulse" />
                <span>{aiMessages.length > 0 && aiMessages[aiMessages.length - 1].image_data ? 'AI is analyzing the image...' : 'Thinking...'}</span>
              </span>
            </span>
          </div>
        )}
        <div ref={messagesEndRef} />
      </div>

      {/* Toast notification */}
      {toast && (
        <div className={`absolute bottom-20 left-1/2 -translate-x-1/2 z-30 bg-slate-950 border text-xs px-4 py-2 rounded-lg shadow-xl whitespace-nowrap ${
          toast.startsWith('Warning') ? 'border-amber-700/50 text-amber-300' : 'border-emerald-700/50 text-emerald-300'
        }`}>
          {toast}
        </div>
      )}

      {/* Drag overlay */}
      {isDragging && (
        <div className="absolute inset-0 z-20 bg-violet-900/20 border-2 border-dashed border-violet-500/50 rounded-xl flex items-center justify-center pointer-events-none">
          <div className="text-violet-400 text-center">
            <Upload size={32} className="mx-auto mb-2" />
            <p className="text-sm font-medium">Drop image or file here</p>
          </div>
        </div>
      )}

      {/* Input */}
      <form onSubmit={handleSubmit} className="p-2 border-t border-gray-800/60 shrink-0 space-y-2">
        {/* Drafts summary */}
        {activeDrafts.length > 0 && (
          <div className="flex items-center space-x-1 text-[10px] text-violet-400">
            <Sparkles size={10} />
            <span>{activeDrafts.length} product draft{activeDrafts.length > 1 ? 's' : ''} pending review</span>
          </div>
        )}

        {/* Pending image preview */}
        {pendingImage && (
          <div className="flex items-center space-x-2 bg-slate-950 border border-gray-800 rounded-lg px-2 py-1.5">
            <div className="w-10 h-10 rounded overflow-hidden bg-slate-900 shrink-0">
              <img
                src={`data:image/jpeg;base64,${pendingImage}`}
                alt="Preview"
                className="w-full h-full object-contain"
              />
            </div>
            <span className="text-xs text-gray-400 truncate flex-1">{pendingImageName}</span>
            <button type="button" onClick={clearPendingImage} className="p-0.5 text-gray-500 hover:text-red-400 transition-colors">
              <X size={12} />
            </button>
          </div>
        )}

        <div className="flex items-center space-x-1.5">
          <div className="relative">
            <button
              type="button"
              onClick={() => setShowMenu(!showMenu)}
              className="p-2 bg-slate-800 hover:bg-slate-700 text-gray-400 rounded-lg transition-colors"
            >
              <Plus size={16} />
            </button>
            {showMenu && (
              <div className="absolute bottom-full left-0 mb-1 w-44 bg-slate-800 border border-gray-700 rounded-xl overflow-hidden shadow-xl z-30">
                <button type="button" onClick={handleImageSelect} className="w-full flex items-center space-x-2 px-3 py-2 text-xs text-gray-300 hover:bg-slate-700 transition-colors">
                  <Image size={14} className="text-violet-400" />
                  <span>Upload Image</span>
                </button>
                <button type="button" onClick={handlePasteLink} className="w-full flex items-center space-x-2 px-3 py-2 text-xs text-gray-300 hover:bg-slate-700 transition-colors">
                  <Link2 size={14} className="text-blue-400" />
                  <span>Paste Link</span>
                </button>
                <button type="button" onClick={() => { sendAiMessage('Show me a summary of my business today.'); setShowMenu(false) }} className="w-full flex items-center space-x-2 px-3 py-2 text-xs text-gray-300 hover:bg-slate-700 transition-colors">
                  <FileText size={14} className="text-green-400" />
                  <span>Business Summary</span>
                </button>
              </div>
            )}
          </div>
          <input
            type="text"
            value={inputText}
            onChange={e => setInputText(e.target.value)}
            placeholder={pendingImage ? 'Add instructions for this image...' : 'Ask AI, paste image (Ctrl+V)...'}
            className="flex-1 bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 placeholder-gray-600 focus:outline-none focus:border-violet-500"
          />
          <button
            type="submit"
            disabled={isAiLoading || (!inputText.trim() && !pendingImage)}
            className="p-2 bg-violet-600 hover:bg-violet-700 text-white rounded-lg transition-colors disabled:opacity-50"
          >
            <Send size={16} />
          </button>
        </div>
      </form>

      {/* Hidden file input */}
      <input
        ref={fileInputRef}
        type="file"
        accept="image/*,.pdf"
        className="hidden"
        onChange={handleFileChange}
      />
    </aside>
  )
}
