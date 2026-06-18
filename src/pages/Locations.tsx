import { useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { MapPin, Plus, X } from 'lucide-react'

interface Location {
  id?: number;
  name: string;
  address?: string;
  is_active: boolean;
  created_at?: string;
}

export default function LocationsPage() {
  const [locations, setLocations] = useState<Location[]>([])
  const [showModal, setShowModal] = useState(false)
  const [editLoc, setEditLoc] = useState<Location | null>(null)
  const [name, setName] = useState('')
  const [address, setAddress] = useState('')

  useEffect(() => { load() }, [])

  const load = async () => {
    try {
      const locs: Location[] = await invoke('get_locations')
      setLocations(locs)
    } catch (err) { console.error(err) }
  }

  const handleOpenAdd = () => {
    setEditLoc(null); setName(''); setAddress(''); setShowModal(true)
  }

  const handleOpenEdit = (l: Location) => {
    setEditLoc(l); setName(l.name); setAddress(l.address || ''); setShowModal(true)
  }

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault()
    try {
      if (editLoc?.id) {
        await invoke('update_location', { id: editLoc.id, name, address, isActive: true })
      } else {
        await invoke('add_location', { name, address })
      }
      setShowModal(false)
      await load()
    } catch (err) { alert(`Error: ${err}`) }
  }

  const handleToggle = async (l: Location) => {
    if (!l.id) return
    try {
      await invoke('update_location', { id: l.id, name: l.name, address: l.address || '', isActive: !l.is_active })
      await load()
    } catch (err) { alert(err) }
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold tracking-tight text-white font-display">Locations</h1>
          <p className="text-sm text-gray-400 mt-1">Manage stock locations (shops, agents, office).</p>
        </div>
        <button onClick={handleOpenAdd} className="flex items-center space-x-1 px-4 py-2 bg-violet-600 hover:bg-violet-700 text-white rounded-lg text-sm font-medium">
          <Plus size={16} /><span>Add Location</span>
        </button>
      </div>

      <div className="grid gap-3 md:grid-cols-2 lg:grid-cols-3">
        {locations.map(l => (
          <div key={l.id} className={`glass-card p-4 border ${l.is_active ? 'border-gray-800' : 'border-red-900/30 opacity-60'}`}>
            <div className="flex items-start justify-between">
              <div className="flex items-center space-x-2">
                <MapPin size={18} className="text-violet-400" />
                <div>
                  <h3 className="text-sm font-semibold text-white">{l.name}</h3>
                  {l.address && <p className="text-xs text-gray-500">{l.address}</p>}
                </div>
              </div>
              <button onClick={() => handleOpenEdit(l)} className="text-xs text-gray-400 hover:text-violet-400">Edit</button>
            </div>
            <div className="mt-2 flex items-center justify-between">
              <span className={`text-xs ${l.is_active ? 'text-emerald-400' : 'text-red-400'}`}>
                {l.is_active ? 'Active' : 'Inactive'}
              </span>
              <button onClick={() => handleToggle(l)} className="text-xs text-gray-500 hover:text-gray-300">
                {l.is_active ? 'Deactivate' : 'Activate'}
              </button>
            </div>
          </div>
        ))}
        {locations.length === 0 && (
          <p className="text-sm text-gray-500 col-span-full text-center py-8">No locations found. Add your first location (Head Office, Shop, etc).</p>
        )}
      </div>

      {showModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm p-4">
          <div className="bg-slate-900 border border-gray-800 rounded-2xl w-full max-w-md overflow-hidden shadow-2xl">
            <div className="flex items-center justify-between p-4 border-b border-gray-800 bg-slate-950/40">
              <h3 className="text-lg font-bold text-white font-display">{editLoc ? 'Edit Location' : 'Add Location'}</h3>
              <button onClick={() => setShowModal(false)} className="text-gray-400 hover:text-white"><X size={20} /></button>
            </div>
            <form onSubmit={handleSave} className="p-5 space-y-4">
              <div>
                <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">Name *</label>
                <input type="text" required value={name} onChange={e => setName(e.target.value)}
                  placeholder="Shakargarh Shop"
                  className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500" />
              </div>
              <div>
                <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">Address</label>
                <input type="text" value={address} onChange={e => setAddress(e.target.value)}
                  placeholder="Main Bazar, Shakargarh"
                  className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500" />
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
