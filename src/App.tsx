import { useAppStore } from './stores/store'
import {
  LayoutDashboard,
  BookOpen,
  Share2,
  Users,
  Package,
  Bot,
  FileText,
  Settings as SettingsIcon,
  MapPin,
  ChevronLeft,
  MessageSquare,
  Send,
  X
} from 'lucide-react'

import Dashboard from './pages/Dashboard'
import Catalog from './pages/Catalog'
import SocialHub from './pages/SocialHub'
import Customers from './pages/Customers'
import Inventory from './pages/Inventory'
import Automation from './pages/Automation'
import Reports from './pages/Reports'
import SettingsPage from './pages/Settings'
import LocationsPage from './pages/Locations'

const tabs = [
  { id: 'dashboard', label: 'Dashboard', icon: LayoutDashboard },
  { id: 'catalog', label: 'Catalog', icon: BookOpen },
  { id: 'social', label: 'Social Hub', icon: Share2 },
  { id: 'customers', label: 'Customers', icon: Users },
  { id: 'inventory', label: 'Inventory', icon: Package },
  { id: 'locations', label: 'Locations', icon: MapPin },
  { id: 'automation', label: 'Automation', icon: Bot },
  { id: 'reports', label: 'Reports', icon: FileText },
  { id: 'settings', label: 'Settings', icon: SettingsIcon },
]

function App() {
  const {
    currentTab,
    setCurrentTab,
    showAiAssistant,
    setVectorAssistant,
    aiMessages,
    isAiLoading,
    sendAiMessage
  } = useAppStore()

  const renderPage = () => {
    switch (currentTab) {
      case 'dashboard': return <Dashboard />
      case 'catalog': return <Catalog />
      case 'social': return <SocialHub />
      case 'customers': return <Customers />
      case 'inventory': return <Inventory />
      case 'automation': return <Automation />
      case 'locations': return <LocationsPage />
      case 'reports': return <Reports />
      case 'settings': return <SettingsPage />
      default: return <Dashboard />
    }
  }

  const handleAiSubmit = (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault()
    const form = e.currentTarget
    const input = form.elements.namedItem('ai-input') as HTMLInputElement
    if (input.value.trim()) {
      sendAiMessage(input.value.trim())
      input.value = ''
    }
  }

  return (
    <div className="h-screen flex overflow-hidden bg-[#030712]">
      {/* Sidebar */}
      <aside className="w-56 bg-slate-900/60 border-r border-gray-800/60 flex flex-col shrink-0">
        <div className="p-4 border-b border-gray-800/60">
          <h1 className="text-lg font-bold text-white font-display tracking-tight">A Collection</h1>
          <p className="text-[10px] text-gray-500 uppercase tracking-wider mt-0.5">Head Office</p>
        </div>
        <nav className="flex-1 overflow-y-auto p-2 space-y-1">
          {tabs.map((tab) => {
            const Icon = tab.icon
            return (
              <button
                key={tab.id}
                onClick={() => setCurrentTab(tab.id)}
                className={`w-full flex items-center space-x-2.5 px-3 py-2.5 rounded-lg text-sm transition-all ${
                  currentTab === tab.id
                    ? 'bg-violet-600/15 text-violet-400 border border-violet-500/20'
                    : 'text-gray-400 hover:text-gray-200 hover:bg-slate-800/50 border border-transparent'
                }`}
              >
                <Icon size={17} />
                <span>{tab.label}</span>
              </button>
            )
          })}
        </nav>
        <div className="p-3 border-t border-gray-800/60">
          <button
            onClick={() => setVectorAssistant(!showAiAssistant)}
            className="w-full flex items-center justify-between px-3 py-2 rounded-lg text-xs bg-violet-600/10 text-violet-400 border border-violet-500/10 hover:bg-violet-600/20 transition-colors"
          >
            <span className="flex items-center space-x-1.5">
              <MessageSquare size={14} />
              <span>AI Assistant</span>
            </span>
            <ChevronLeft size={14} className={`transition-transform ${showAiAssistant ? 'rotate-180' : ''}`} />
          </button>
        </div>
      </aside>

      {/* Main Content */}
      <main className="flex-1 overflow-y-auto p-6 bg-[#030712]">
        {renderPage()}
      </main>

      {/* AI Assistant Panel */}
      {showAiAssistant && (
        <aside className="w-80 bg-slate-900/60 border-l border-gray-800/60 flex flex-col shrink-0">
          <div className="flex items-center justify-between p-4 border-b border-gray-800/60">
            <h2 className="text-sm font-semibold text-white flex items-center space-x-2">
              <MessageSquare size={16} className="text-violet-500" />
              <span>AI Assistant</span>
            </h2>
            <button
              onClick={() => setVectorAssistant(false)}
              className="text-gray-500 hover:text-gray-300 transition-colors"
            >
              <X size={16} />
            </button>
          </div>

          <div className="flex-1 overflow-y-auto p-4 space-y-3">
            {aiMessages.map((msg, i) => (
              <div
                key={i}
                className={`text-sm ${msg.role === 'user' ? 'text-right' : 'text-left'}`}
              >
                <span
                  className={`inline-block px-3 py-2 rounded-xl max-w-[90%] ${
                    msg.role === 'user'
                      ? 'bg-violet-600/20 text-violet-200 border border-violet-500/10'
                      : 'bg-slate-800/60 text-gray-300 border border-gray-800'
                  }`}
                >
                  <span className="text-[10px] block text-gray-500 mb-1">
                    {msg.role === 'user' ? 'You' : 'AI'}
                  </span>
                  <span className="whitespace-pre-wrap">{msg.text}</span>
                </span>
              </div>
            ))}
            {isAiLoading && (
              <div className="text-left text-sm">
                <span className="inline-block px-3 py-2 rounded-xl bg-slate-800/60 text-gray-400 border border-gray-800">
                  Thinking...
                </span>
              </div>
            )}
          </div>

          <form onSubmit={handleAiSubmit} className="p-3 border-t border-gray-800/60 flex space-x-2">
            <input
              name="ai-input"
              type="text"
              placeholder="Ask AI anything..."
              className="flex-1 bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 placeholder-gray-600 focus:outline-none focus:border-violet-500"
            />
            <button
              type="submit"
              disabled={isAiLoading}
              className="p-2 bg-violet-600 hover:bg-violet-700 text-white rounded-lg transition-colors disabled:opacity-50"
            >
              <Send size={16} />
            </button>
          </form>
        </aside>
      )}
    </div>
  )
}

export default App
