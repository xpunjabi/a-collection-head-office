import { useEffect, useState } from 'react'
import { useAppStore } from '../stores/store'
import { open } from '@tauri-apps/plugin-dialog'
import { Shield, Key, Server, HardDrive, RefreshCw } from 'lucide-react'

export default function Settings() {
  const {
    settings,
    fetchSettings,
    updateSetting,
    backupDatabaseNow
  } = useAppStore()

  const [aiProvider, setAiProvider] = useState('gemini')
  const [apiKey, setApiKey] = useState('')
  const [aiModel, setAiModel] = useState('')
  const [backupPath, setBackupPath] = useState('')
  const [backupInterval, setBackupInterval] = useState('7')
  const [backupResult, setBackupResult] = useState('')
  const [isBackingUp, setIsBackingUp] = useState(false)

  useEffect(() => {
    fetchSettings()
  }, [])

  useEffect(() => {
    if (settings.ai_provider) setAiProvider(settings.ai_provider)
    if (settings.ai_api_key) setApiKey(settings.ai_api_key)
    if (settings.ai_model) setAiModel(settings.ai_model)
    if (settings.backup_path) setBackupPath(settings.backup_path)
    if (settings.backup_interval_days) setBackupInterval(settings.backup_interval_days)
  }, [settings])

  const handleSaveAiSettings = async () => {
    try {
      await updateSetting('ai_provider', aiProvider)
      await updateSetting('ai_api_key', apiKey)
      await updateSetting('ai_model', aiModel)
      alert('AI settings saved successfully!')
    } catch (err) {
      alert(`Failed to save AI settings: ${err}`)
    }
  }

  const handleSelectBackupPath = async () => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: 'Select Backup Folder'
      })
      if (selected && typeof selected === 'string') {
        setBackupPath(selected)
        await updateSetting('backup_path', selected)
      }
    } catch (err) {
      console.error(err)
    }
  }

  const handleSaveBackupSettings = async () => {
    try {
      await updateSetting('backup_path', backupPath)
      await updateSetting('backup_interval_days', backupInterval)
      alert('Backup settings saved!')
    } catch (err) {
      alert(`Failed to save backup settings: ${err}`)
    }
  }

  const handleBackupNow = async () => {
    setIsBackingUp(true)
    setBackupResult('')
    try {
      const dest = await backupDatabaseNow()
      setBackupResult(`Backup successful! Saved to:\n${dest}`)
    } catch (err) {
      setBackupResult(`Backup failed: ${err}`)
    } finally {
      setIsBackingUp(false)
    }
  }

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold tracking-tight text-white font-display">Settings</h1>
        <p className="text-sm text-gray-400 mt-1">Configure AI provider, backup preferences, and system options.</p>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* AI Configuration */}
        <div className="glass-card p-5 space-y-4">
          <h2 className="text-lg font-semibold text-white flex items-center">
            <Server className="mr-2 text-violet-500" size={20} /> AI Provider Settings
          </h2>

          <div>
            <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">AI Provider</label>
            <select
              value={aiProvider}
              onChange={(e) => setAiProvider(e.target.value)}
              className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500"
            >
              <option value="gemini">Gemini (Google)</option>
              <option value="openai">OpenAI</option>
              <option value="claude">Claude (Anthropic)</option>
              <option value="local">Local LLM (Ollama)</option>
            </select>
          </div>

          <div>
            <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">API Key</label>
            <div className="relative">
              <Key className="absolute left-3 top-2.5 text-gray-500" size={16} />
              <input
                type="password"
                value={apiKey}
                onChange={(e) => setApiKey(e.target.value)}
                placeholder="Enter your API key..."
                className="w-full bg-slate-950 border border-gray-800 rounded-lg pl-10 pr-4 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500"
              />
            </div>
            <p className="text-[10px] text-gray-600 mt-1">Stored securely in local database.</p>
          </div>

          <div>
            <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">Model Name</label>
            <input
              type="text"
              value={aiModel}
              onChange={(e) => setAiModel(e.target.value)}
              placeholder="E.g. gemini-1.5-flash, gpt-4o"
              className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500"
            />
          </div>

          <button
            onClick={handleSaveAiSettings}
            className="px-4 py-2 bg-violet-600 hover:bg-violet-700 text-white rounded-lg text-sm font-medium transition-colors"
          >
            Save AI Settings
          </button>
        </div>

        {/* Backup Configuration */}
        <div className="glass-card p-5 space-y-4">
          <h2 className="text-lg font-semibold text-white flex items-center">
            <Shield className="mr-2 text-violet-500" size={20} /> Database Backup
          </h2>

          <div>
            <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">Backup Location</label>
            <div className="flex space-x-2">
              <input
                type="text"
                value={backupPath}
                readOnly
                placeholder="Select a backup folder..."
                className="flex-1 bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-400 focus:outline-none"
              />
              <button
                onClick={handleSelectBackupPath}
                className="px-3 py-2 bg-slate-800 hover:bg-slate-700 text-gray-200 border border-gray-700 rounded-lg text-sm transition-colors"
              >
                <HardDrive size={16} />
              </button>
            </div>
            {/* Google Drive backup tip */}
            <div className="mt-2 bg-emerald-950/30 border border-emerald-800/40 rounded-lg p-3">
              <p className="text-xs text-emerald-300 font-semibold flex items-center">
                ☁️ Google Drive Auto-Backup Tip
              </p>
              <p className="text-xs text-gray-400 mt-1">
                Install <strong>Google Drive desktop app</strong> on your PC. It creates a folder like <code className="text-emerald-400">C:\Users\YourName\Google Drive</code>. Select that folder as your backup location above — your database will automatically sync to the cloud whenever a backup runs.
              </p>
              <p className="text-xs text-gray-500 mt-1">
                No extra app configuration needed — Drive syncs the file in the background.
              </p>
            </div>
          </div>

          <div>
            <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">Backup Interval (Days)</label>
            <select
              value={backupInterval}
              onChange={(e) => setBackupInterval(e.target.value)}
              className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500"
            >
              <option value="1">Every Day</option>
              <option value="3">Every 3 Days</option>
              <option value="7">Every Week</option>
              <option value="14">Every 2 Weeks</option>
              <option value="30">Every Month</option>
            </select>
          </div>

          <div className="flex space-x-2">
            <button
              onClick={handleSaveBackupSettings}
              className="px-4 py-2 bg-slate-800 hover:bg-slate-700 text-gray-200 border border-gray-700 rounded-lg text-sm transition-colors"
            >
              Save Backup Settings
            </button>
            <button
              onClick={handleBackupNow}
              disabled={isBackingUp}
              className="flex items-center space-x-1 px-4 py-2 bg-violet-600 hover:bg-violet-700 text-white rounded-lg text-sm font-medium transition-colors disabled:opacity-50"
            >
              <RefreshCw size={16} className={isBackingUp ? 'animate-spin' : ''} />
              <span>Backup Now</span>
            </button>
          </div>

          {backupResult && (
            <div className="bg-slate-950 border border-gray-800 rounded-lg p-3 text-xs font-mono text-gray-400 whitespace-pre-wrap">
              {backupResult}
            </div>
          )}
        </div>
      </div>
    </div>
  )
}
