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
  Sparkles,
  MessageSquare
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
import AiWorkspace from './components/AiWorkspace'

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
    aiProductDrafts,
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
        <div className="p-3 border-t border-gray-800/60 space-y-2">
          <button
            onClick={() => setVectorAssistant(!showAiAssistant)}
            className="w-full flex items-center justify-between px-3 py-2 rounded-lg text-xs bg-violet-600/10 text-violet-400 border border-violet-500/10 hover:bg-violet-600/20 transition-colors"
          >
            <span className="flex items-center space-x-1.5">
              <Sparkles size={14} />
              <span>AI Workspace</span>
            </span>
            <div className="flex items-center space-x-1">
              {aiProductDrafts.length > 0 && (
                <span className="bg-violet-500 text-white text-[9px] rounded-full w-4 h-4 flex items-center justify-center font-bold">
                  {aiProductDrafts.length}
                </span>
              )}
              <ChevronLeft size={14} className={`transition-transform ${showAiAssistant ? 'rotate-180' : ''}`} />
            </div>
          </button>
          {!showAiAssistant && (
            <button
              onClick={() => setVectorAssistant(true)}
              className="w-full flex items-center space-x-2 px-3 py-2 rounded-lg text-xs text-gray-500 hover:text-gray-300 hover:bg-slate-800/50 transition-colors"
            >
              <MessageSquare size={12} />
              <span>Open AI Workspace</span>
            </button>
          )}
        </div>
      </aside>

      {/* Main Content */}
      <main className="flex-1 overflow-y-auto p-6 bg-[#030712]">
        {renderPage()}
      </main>

      {/* AI Workspace */}
      <AiWorkspace />
    </div>
  )
}

export default App
