import { useState } from 'react'
import { Clipboard, ClipboardCheck } from 'lucide-react'

interface Props {
  text: string
}

export default function FormattedMessage({ text }: Props) {
  // Split by code blocks: ```lang ... ```
  const parts: { type: 'text' | 'code'; lang?: string; content: string }[] = []
  let remaining = text

  while (remaining.length > 0) {
    const codeStart = remaining.indexOf('```')
    if (codeStart === -1) {
      parts.push({ type: 'text', content: remaining })
      break
    }
    // Push text before code block
    if (codeStart > 0) {
      parts.push({ type: 'text', content: remaining.slice(0, codeStart) })
    }
    // Find the end of the code block
    const afterOpen = remaining.slice(codeStart + 3)
    const lineEnd = afterOpen.indexOf('\n')
    const lang = lineEnd > 0 ? afterOpen.slice(0, lineEnd).trim() : ''
    const codeContentStart = lineEnd > 0 ? lineEnd + 1 : 0
    const codeEnd = afterOpen.indexOf('```', codeContentStart)
    if (codeEnd === -1) {
      // No closing ```, treat rest as code
      parts.push({ type: 'code', lang, content: afterOpen.slice(codeContentStart) })
      break
    }
    parts.push({ type: 'code', lang, content: afterOpen.slice(codeContentStart, codeEnd) })
    remaining = afterOpen.slice(codeEnd + 3)
  }

  return (
    <span className="whitespace-pre-wrap text-sm leading-relaxed select-text">
      {parts.map((part, i) => {
        if (part.type === 'text') {
          return <span key={i}>{part.content}</span>
        }
        return <CodeBlock key={i} lang={part.lang} content={part.content} />
      })}
    </span>
  )
}

function CodeBlock({ lang, content }: { lang?: string; content: string }) {
  const [copied, setCopied] = useState(false)

  const handleCopy = () => {
    navigator.clipboard.writeText(content)
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }

  return (
    <div className="my-2 bg-slate-950 border border-gray-700 rounded-lg overflow-hidden select-text">
      {lang && (
        <div className="flex items-center justify-between px-3 py-1 bg-slate-900 border-b border-gray-700">
          <span className="text-[10px] text-gray-500 uppercase">{lang}</span>
          <button
            onClick={handleCopy}
            className="flex items-center space-x-1 text-[10px] text-gray-400 hover:text-white transition-colors"
          >
            {copied ? (
              <><ClipboardCheck size={10} className="text-green-400" /><span className="text-green-400">Copied!</span></>
            ) : (
              <><Clipboard size={10} /><span>Copy</span></>
            )}
          </button>
        </div>
      )}
      <pre className="p-3 overflow-x-auto text-[11px] leading-relaxed text-gray-300 font-mono whitespace-pre-wrap m-0 select-text">
        {content}
      </pre>
    </div>
  )
}
