import { useRef, useState, useCallback, useEffect } from 'react'
import { useAppStore } from '../stores/store'
import ProductDraftCard from './ProductDraftCard'
import {
  Send, X, Plus, Image, Link2, FileText, Upload,
  Trash2, GripVertical, Sparkles
} from 'lucide-react'

export default function AiWorkspace() {
  const {
    aiMessages, isAiLoading, sendAiMessage, clearAiChat,
    showAiAssistant, setVectorAssistant,
    aiWorkspaceWidth, setAiWorkspaceWidth,
    aiProductDrafts
  } = useAppStore()

  const [inputText, setInputText] = useState('')
  const [showMenu, setShowMenu] = useState(false)
  const [isDragging, setIsDragging] = useState(false)
  const messagesEndRef = useRef<HTMLDivElement>(null)
  const fileInputRef = useRef<HTMLInputElement>(null)
  const panelRef = useRef<HTMLDivElement>(null)
  const isResizing = useRef(false)

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' })
  }, [aiMessages])

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault()
    if (inputText.trim()) {
      sendAiMessage(inputText.trim())
      setInputText('')
    }
  }

  const handleImageSelect = () => {
    fileInputRef.current?.click()
    setShowMenu(false)
  }

  const handleFileChange = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0]
    if (!file) return
    const reader = new FileReader()
    reader.onload = async (ev) => {
      const base64 = (ev.target?.result as string)?.split(',')[1]
      if (base64) {
        sendAiMessage(`[Image: ${file.name}]`, base64)
      }
    }
    reader.readAsDataURL(file)
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
            sendAiMessage(`[Image: ${file.name}]`, base64)
          }
        }
        reader.readAsDataURL(file)
      } else if (file.type === 'application/pdf' || file.type.includes('text')) {
        sendAiMessage(`[File: ${file.name}] Please analyze this document and extract any product information.`)
      }
    }
  }, [sendAiMessage])

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
              sendAiMessage('[Pasted image]', base64)
            }
          }
          reader.readAsDataURL(file)
          return
        }
      }
    }
  }, [sendAiMessage])

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

  if (!showAiAssistant) return null

  // Collect all active drafts from messages
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
            <div className={`text-sm ${msg.role === 'user' ? 'text-right' : 'text-left'}`}>
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
                <span className="whitespace-pre-wrap text-xs leading-relaxed">{msg.text}</span>
              </span>
            </div>
            {/* Product Draft Card shown after AI response */}
            {msg.role === 'assistant' && msg.product_draft && msg.confidence !== undefined && (
              <div className="mt-2">
                <ProductDraftCard
                  draft={msg.product_draft}
                  confidence={msg.confidence}
                  missingFields={msg.missing_fields || []}
                  index={activeDrafts.findIndex(d => d.draft === msg.product_draft)}
                />
              </div>
            )}
          </div>
        ))}
        {isAiLoading && (
          <div className="text-left text-sm">
            <span className="inline-block px-3 py-2 rounded-xl bg-slate-800/60 text-gray-400 border border-gray-800">
              <span className="flex items-center space-x-2">
                <span className="w-1.5 h-1.5 bg-violet-400 rounded-full animate-pulse" />
                <span>Thinking...</span>
              </span>
            </span>
          </div>
        )}
        <div ref={messagesEndRef} />
      </div>

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
            placeholder="Ask AI, paste image (Ctrl+V), or drop link..."
            className="flex-1 bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 placeholder-gray-600 focus:outline-none focus:border-violet-500"
          />
          <button
            type="submit"
            disabled={isAiLoading || (!inputText.trim())}
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
