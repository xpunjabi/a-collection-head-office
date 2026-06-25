import { useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { useAppStore, AgentSummary, AgentLedgerEntry } from '../stores/store'
import {
  Plus, X, User, Package, Trash2,
  ArrowDownToLine, ArrowUpFromLine, Banknote, Scale, History
} from 'lucide-react'

export default function AgentsPage() {
  const { products, fetchProducts } = useAppStore()
  const [agents, setAgents] = useState<AgentSummary[]>([])
  const [selectedAgent, setSelectedAgent] = useState<AgentSummary | null>(null)
  const [ledger, setLedger] = useState<AgentLedgerEntry[]>([])

  // Add/Edit agent modal state
  const [showModal, setShowModal] = useState(false)
  const [editAgent, setEditAgent] = useState<AgentSummary | null>(null)
  const [name, setName] = useState('')
  const [phone, setPhone] = useState('')
  const [city, setCity] = useState('')
  const [area, setArea] = useState('')
  const [addressNotes, setAddressNotes] = useState('')
  const [notes, setNotes] = useState('')

  // Action modal state (send/return/sell/cash/adjust)
  const [actionModal, setActionModal] = useState<null | 'send' | 'return' | 'sell' | 'cash' | 'adjust'>(null)
  const [actionProductId, setActionProductId] = useState<number | ''>('')
  const [actionQty, setActionQty] = useState(1)
  const [actionPrice, setActionPrice] = useState(0)
  const [actionAmount, setActionAmount] = useState(0)
  const [actionNotes, setActionNotes] = useState('')
  const [actionLoading, setActionLoading] = useState(false)

  useEffect(() => {
    fetchProducts()
    loadAgents()
  }, [])

  const loadAgents = async () => {
    try {
      const data: AgentSummary[] = await invoke('get_agents')
      setAgents(data)
    } catch (err) { console.error('Failed to load agents:', err) }
  }

  const loadAgentDetail = async (a: AgentSummary) => {
    setSelectedAgent(a)
    try {
      const entries: AgentLedgerEntry[] = await invoke('get_agent_ledger', { agentId: a.agent.id, limit: 30 })
      setLedger(entries)
    } catch (err) {
      console.error('Failed to load ledger:', err)
      setLedger([])
    }
  }

  const handleOpenAdd = () => {
    setEditAgent(null)
    setName(''); setPhone(''); setCity(''); setArea(''); setAddressNotes(''); setNotes('')
    setShowModal(true)
  }

  const handleOpenEdit = (a: AgentSummary) => {
    setEditAgent(a)
    setName(a.agent.name)
    setPhone(a.agent.phone || '')
    setCity(a.agent.city || '')
    setArea(a.agent.area || '')
    setAddressNotes(a.agent.address_notes || '')
    setNotes(a.agent.notes || '')
    setShowModal(true)
  }

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!name.trim()) { alert('Name is required.'); return }
    try {
      if (editAgent?.agent.id) {
        await invoke('update_agent', {
          id: editAgent.agent.id, name,
          phone: phone || null,
          city: city || null,
          area: area || null,
          addressNotes: addressNotes || null,
          notes: notes || null,
          isActive: editAgent.agent.is_active,
        })
      } else {
        await invoke('add_agent', {
          name,
          phone: phone || null,
          city: city || null,
          area: area || null,
          addressNotes: addressNotes || null,
          notes: notes || null,
        })
      }
      setShowModal(false)
      await loadAgents()
    } catch (err) {
      alert(`Error: ${err}`)
    }
  }

  const handleToggleActive = async (a: AgentSummary) => {
    if (!a.agent.id) return
    try {
      await invoke('update_agent', {
        id: a.agent.id, name: a.agent.name,
        phone: a.agent.phone || null,
        city: a.agent.city || null,
        area: a.agent.area || null,
        addressNotes: a.agent.address_notes || null,
        notes: a.agent.notes || null,
        isActive: !a.agent.is_active,
      })
      await loadAgents()
    } catch (err) { alert(err) }
  }

  const handleDeleteAgent = async (a: AgentSummary) => {
    if (!a.agent.id) return
    const hasStock = a.current_stock_units > 0
    const hasBalance = a.outstanding_balance > 0
    let warning = `Delete agent "${a.agent.name}"?`
    if (hasStock) warning += `\n\n⚠️ This agent has ${a.current_stock_units} units in stock. Deleting will also delete all ledger entries.`
    if (hasBalance) warning += `\n\n⚠️ This agent has Rs. ${a.outstanding_balance.toFixed(0)} outstanding balance. Deleting will lose this record.`
    warning += '\n\nThis CANNOT be undone.'
    if (!confirm(warning)) return
    try {
      await invoke('delete_agent', { id: a.agent.id })
      setSelectedAgent(null)
      await loadAgents()
    } catch (err) { alert(`Error: ${err}`) }
  }

  const openAction = (kind: 'send' | 'return' | 'sell' | 'cash' | 'adjust') => {
    setActionModal(kind)
    setActionProductId('')
    setActionQty(1)
    setActionPrice(0)
    setActionAmount(0)
    setActionNotes('')
  }

  const handleActionSubmit = async () => {
    if (!selectedAgent?.agent.id) return
    setActionLoading(true)
    try {
      const agentId = selectedAgent.agent.id
      const product_id = actionProductId === '' ? null : Number(actionProductId)
      const notesVal = actionNotes.trim() || null

      switch (actionModal) {
        case 'send':
          if (!product_id) { alert('Select a product.'); setActionLoading(false); return }
          await invoke('send_stock_to_agent', {
            agentId, productId: product_id, qty: actionQty, unitPrice: actionPrice, notes: notesVal
          })
          break
        case 'return':
          if (!product_id) { alert('Select a product.'); setActionLoading(false); return }
          await invoke('return_stock_from_agent', {
            agentId, productId: product_id, qty: actionQty, unitPrice: actionPrice, notes: notesVal
          })
          break
        case 'sell':
          if (!product_id) { alert('Select a product.'); setActionLoading(false); return }
          await invoke('report_agent_sale', {
            agentId, productId: product_id, qty: actionQty, unitPrice: actionPrice, notes: notesVal
          })
          break
        case 'cash':
          await invoke('receive_agent_cash', { agentId, amount: actionAmount, notes: notesVal })
          break
        case 'adjust':
          if (!actionNotes.trim()) { alert('Notes are mandatory for adjustments.'); setActionLoading(false); return }
          await invoke('adjust_agent_balance', { agentId, amount: actionAmount, notes: actionNotes })
          break
      }
      setActionModal(null)
      // Refresh agent detail + list
      await loadAgents()
      const updated = (await invoke('get_agents') as AgentSummary[]).find(a => a.agent.id === agentId)
      if (updated) await loadAgentDetail(updated)
      await fetchProducts()
    } catch (err) {
      alert(`Error: ${err}`)
    } finally {
      setActionLoading(false)
    }
  }

  // ---- Helper render functions ----
  const fmtMoney = (n: number) => `Rs. ${n.toFixed(0)}`
  const fmtDate = (iso?: string | null) => iso ? new Date(iso).toLocaleDateString('en-PK', { day: '2-digit', month: 'short', year: 'numeric' }) : '—'

  type ActionKind = 'send' | 'return' | 'sell' | 'cash' | 'adjust'
  const actionModalTitle: Record<ActionKind, string> = {
    send: 'Send Stock to Agent',
    return: 'Return Stock from Agent',
    sell: 'Report Sale by Agent',
    cash: 'Receive Cash from Agent',
    adjust: 'Adjust Agent Balance',
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold tracking-tight text-white font-display">Agents</h1>
          <p className="text-sm text-gray-400 mt-1">Manage agents who receive stock, sell it, and remit cash. Agents replace the old Locations concept.</p>
        </div>
        <button onClick={handleOpenAdd} className="flex items-center space-x-1 px-4 py-2 bg-violet-600 hover:bg-violet-700 text-white rounded-lg text-sm font-medium">
          <Plus size={16} /><span>Add Agent</span>
        </button>
      </div>

      {/* Agent list (left) + detail (right) split */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Agents list */}
        <div className="lg:col-span-1 space-y-3">
          {agents.length === 0 ? (
            <p className="text-sm text-gray-500 text-center py-8">No agents yet. Click "Add Agent" to create your first.</p>
          ) : agents.map(a => (
            <button
              key={a.agent.id}
              onClick={() => loadAgentDetail(a)}
              className={`w-full text-left glass-card p-4 border transition-colors ${selectedAgent?.agent.id === a.agent.id ? 'border-violet-500 bg-violet-900/20' : 'border-gray-800 hover:border-violet-500/50'} ${a.agent.is_active ? '' : 'opacity-60'}`}
            >
              <div className="flex items-start justify-between">
                <div className="flex items-center space-x-2">
                  <User size={16} className="text-violet-400" />
                  <div>
                    <h3 className="text-sm font-semibold text-white">{a.agent.name}</h3>
                    <p className="text-[11px] text-gray-500">{a.agent.city || 'No city'} • {a.agent.agent_code}</p>
                  </div>
                </div>
                <span className={`text-[10px] px-1.5 py-0.5 rounded ${a.outstanding_balance > 0 ? 'bg-amber-900/40 text-amber-300' : 'bg-emerald-900/40 text-emerald-300'}`}>
                  {a.outstanding_balance > 0 ? `Owes ${fmtMoney(a.outstanding_balance)}` : 'Settled'}
                </span>
              </div>
              <div className="mt-2 grid grid-cols-3 gap-2 text-[10px]">
                <div className="bg-slate-950/50 rounded p-1.5">
                  <div className="text-gray-500">Stock</div>
                  <div className="text-white font-semibold">{a.current_stock_units} pcs</div>
                </div>
                <div className="bg-slate-950/50 rounded p-1.5">
                  <div className="text-gray-500">Cash Received</div>
                  <div className="text-emerald-400 font-semibold">{fmtMoney(a.total_cash_received)}</div>
                </div>
                <div className="bg-slate-950/50 rounded p-1.5">
                  <div className="text-gray-500">Outstanding</div>
                  <div className={a.outstanding_balance > 0 ? 'text-amber-400 font-semibold' : 'text-emerald-400 font-semibold'}>{fmtMoney(a.outstanding_balance)}</div>
                </div>
              </div>
            </button>
          ))}
        </div>

        {/* Agent detail */}
        <div className="lg:col-span-2">
          {selectedAgent ? (
            <div className="glass-card p-5 space-y-4">
              <div className="flex items-start justify-between">
                <div>
                  <h2 className="text-xl font-bold text-white">{selectedAgent.agent.name}</h2>
                  <p className="text-xs text-gray-500 mt-1">
                    {selectedAgent.agent.city || 'No city'} {selectedAgent.agent.area ? `• ${selectedAgent.agent.area}` : ''}
                    {selectedAgent.agent.phone ? ` • ${selectedAgent.agent.phone}` : ''}
                  </p>
                  {selectedAgent.agent.notes && (
                    <p className="text-xs text-gray-400 mt-2 italic">{selectedAgent.agent.notes}</p>
                  )}
                </div>
                <div className="flex items-center space-x-3">
                  <button onClick={() => handleOpenEdit(selectedAgent)} className="text-xs text-gray-400 hover:text-violet-400">Edit</button>
                  <button onClick={() => handleToggleActive(selectedAgent)} className="text-xs text-gray-500 hover:text-gray-300">
                    {selectedAgent.agent.is_active ? 'Deactivate' : 'Activate'}
                  </button>
                  <button onClick={() => handleDeleteAgent(selectedAgent)} className="flex items-center space-x-1 text-xs text-red-400 hover:text-red-300">
                    <Trash2 size={12} /><span>Delete</span>
                  </button>
                </div>
              </div>

              {/* Quick action buttons */}
              <div>
                <label className="block text-xs font-semibold uppercase text-gray-400 mb-2">Quick Actions</label>
                <div className="grid grid-cols-2 md:grid-cols-3 gap-2">
                  <button onClick={() => openAction('send')} className="flex items-center space-x-1 px-3 py-2 bg-emerald-600/10 hover:bg-emerald-600/20 text-emerald-400 border border-emerald-500/20 rounded-lg text-xs font-medium">
                    <ArrowUpFromLine size={12} /><span>Send Stock</span>
                  </button>
                  <button onClick={() => openAction('return')} className="flex items-center space-x-1 px-3 py-2 bg-blue-600/10 hover:bg-blue-600/20 text-blue-400 border border-blue-500/20 rounded-lg text-xs font-medium">
                    <ArrowDownToLine size={12} /><span>Return Stock</span>
                  </button>
                  <button onClick={() => openAction('sell')} className="flex items-center space-x-1 px-3 py-2 bg-violet-600/10 hover:bg-violet-600/20 text-violet-400 border border-violet-500/20 rounded-lg text-xs font-medium">
                    <Package size={12} /><span>Report Sale</span>
                  </button>
                  <button onClick={() => openAction('cash')} className="flex items-center space-x-1 px-3 py-2 bg-green-600/10 hover:bg-green-600/20 text-green-400 border border-green-500/20 rounded-lg text-xs font-medium">
                    <Banknote size={12} /><span>Receive Cash</span>
                  </button>
                  <button onClick={() => openAction('adjust')} className="flex items-center space-x-1 px-3 py-2 bg-amber-600/10 hover:bg-amber-600/20 text-amber-400 border border-amber-500/20 rounded-lg text-xs font-medium">
                    <Scale size={12} /><span>Adjust Balance</span>
                  </button>
                </div>
              </div>

              {/* Recent ledger */}
              <div>
                <label className="block text-xs font-semibold uppercase text-gray-400 mb-2 flex items-center">
                  <History size={12} className="mr-1" />Recent Ledger Activity
                </label>
                <div className="overflow-x-auto">
                  <table className="w-full text-xs">
                    <thead>
                      <tr className="text-gray-500 border-b border-gray-800">
                        <th className="text-left py-2 px-2">Date</th>
                        <th className="text-left py-2 px-2">Type</th>
                        <th className="text-right py-2 px-2">Qty</th>
                        <th className="text-right py-2 px-2">Amount</th>
                        <th className="text-left py-2 px-2">Notes</th>
                      </tr>
                    </thead>
                    <tbody className="divide-y divide-gray-800">
                      {ledger.length === 0 ? (
                        <tr><td colSpan={5} className="py-6 text-center text-gray-500">No ledger entries yet.</td></tr>
                      ) : ledger.map(e => (
                        <tr key={e.id} className="text-gray-300">
                          <td className="py-2 px-2 text-gray-500">{fmtDate(e.entry_date)}</td>
                          <td className="py-2 px-2">
                            <span className={`px-1.5 py-0.5 rounded text-[10px] font-medium ${
                              e.entry_type === 'stock_sent' ? 'bg-emerald-900/40 text-emerald-300' :
                              e.entry_type === 'stock_returned' ? 'bg-blue-900/40 text-blue-300' :
                              e.entry_type === 'sale_reported' ? 'bg-violet-900/40 text-violet-300' :
                              e.entry_type === 'cash_received' ? 'bg-green-900/40 text-green-300' :
                              'bg-amber-900/40 text-amber-300'
                            }`}>{e.entry_type.replace('_', ' ')}</span>
                          </td>
                          <td className="py-2 px-2 text-right">{e.qty > 0 ? e.qty : '—'}</td>
                          <td className="py-2 px-2 text-right">{e.amount !== 0 ? fmtMoney(e.amount) : '—'}</td>
                          <td className="py-2 px-2 text-gray-500 max-w-[200px] truncate">{e.notes || '—'}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </div>
            </div>
          ) : (
            <div className="glass-card p-8 text-center">
              <User size={32} className="mx-auto text-gray-700 mb-2" />
              <p className="text-sm text-gray-500">Select an agent to view details, manage stock, and track balances.</p>
            </div>
          )}
        </div>
      </div>

      {/* Add/Edit Agent Modal */}
      {showModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm p-4">
          <div className="bg-slate-900 border border-gray-800 rounded-2xl w-full max-w-md overflow-hidden shadow-2xl">
            <div className="flex items-center justify-between p-4 border-b border-gray-800 bg-slate-950/40">
              <h3 className="text-lg font-bold text-white font-display">{editAgent ? 'Edit Agent' : 'Add Agent'}</h3>
              <button onClick={() => setShowModal(false)} className="text-gray-400 hover:text-white"><X size={20} /></button>
            </div>
            <form onSubmit={handleSave} className="p-5 space-y-3">
              <div>
                <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">Name *</label>
                <input type="text" required value={name} onChange={e => setName(e.target.value)}
                  placeholder="Raza Ahmad"
                  className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500" />
              </div>
              <div className="grid grid-cols-2 gap-3">
                <div>
                  <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">Phone</label>
                  <input type="text" value={phone} onChange={e => setPhone(e.target.value)}
                    placeholder="+92 300 1234567"
                    className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500" />
                </div>
                <div>
                  <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">City</label>
                  <input type="text" value={city} onChange={e => setCity(e.target.value)}
                    placeholder="Narowal"
                    className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500" />
                </div>
              </div>
              <div>
                <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">Area</label>
                <input type="text" value={area} onChange={e => setArea(e.target.value)}
                  placeholder="Main Bazar"
                  className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500" />
              </div>
              <div>
                <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">Address Notes</label>
                <input type="text" value={addressNotes} onChange={e => setAddressNotes(e.target.value)}
                  placeholder="Near Civil Hospital"
                  className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500" />
              </div>
              <div>
                <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">Notes</label>
                <textarea value={notes} onChange={e => setNotes(e.target.value)} rows={2}
                  placeholder="Trustworthy, pays on time, etc."
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

      {/* Action Modal (send/return/sell/cash/adjust) */}
      {actionModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm p-4">
          <div className="bg-slate-900 border border-gray-800 rounded-2xl w-full max-w-md overflow-hidden shadow-2xl">
            <div className="flex items-center justify-between p-4 border-b border-gray-800 bg-slate-950/40">
              <h3 className="text-lg font-bold text-white font-display">{actionModal ? actionModalTitle[actionModal] : ''}</h3>
              <button onClick={() => setActionModal(null)} className="text-gray-400 hover:text-white"><X size={20} /></button>
            </div>
            <div className="p-5 space-y-3">
              {(actionModal === 'send' || actionModal === 'return' || actionModal === 'sell') && (
                <>
                  <div>
                    <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">Product *</label>
                    <select
                      value={actionProductId}
                      onChange={e => setActionProductId(e.target.value ? Number(e.target.value) : '')}
                      className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500"
                    >
                      <option value="">-- Select Product --</option>
                      {products.map(p => (
                        <option key={p.id} value={p.id}>{p.name} ({p.sku}) — HO: {p.stock_quantity}</option>
                      ))}
                    </select>
                  </div>
                  <div className="grid grid-cols-2 gap-3">
                    <div>
                      <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">Quantity *</label>
                      <input type="number" min={1} value={actionQty} onChange={e => setActionQty(Math.max(1, Number(e.target.value)))}
                        className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500" />
                    </div>
                    <div>
                      <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">Unit Price (Rs.)</label>
                      <input type="number" min={0} step={0.01} value={actionPrice} onChange={e => setActionPrice(Number(e.target.value))}
                        className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500" />
                    </div>
                  </div>
                </>
              )}
              {(actionModal === 'cash' || actionModal === 'adjust') && (
                <div>
                  <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">
                    {actionModal === 'cash' ? 'Amount Received (Rs.) *' : 'Adjustment Amount (Rs.) *'}
                  </label>
                  <input type="number" step={0.01} value={actionAmount} onChange={e => setActionAmount(Number(e.target.value))}
                    placeholder={actionModal === 'adjust' ? 'Positive = agent owes more, Negative = less' : '0'}
                    className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500" />
                  {actionModal === 'adjust' && (
                    <p className="text-[10px] text-gray-500 mt-1">Positive = agent owes more. Negative = agent owes less (e.g., write-off).</p>
                  )}
                </div>
              )}
              <div>
                <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">
                  Notes {actionModal === 'adjust' ? '* (mandatory)' : '(optional)'}
                </label>
                <textarea value={actionNotes} onChange={e => setActionNotes(e.target.value)} rows={2}
                  placeholder={actionModal === 'adjust' ? 'Explain why this adjustment is being made.' : 'Optional context for this entry.'}
                  className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500" />
              </div>
              {selectedAgent && (actionModal === 'cash' || actionModal === 'adjust') && (
                <div className="bg-slate-950/50 border border-gray-800 rounded-lg p-3 text-xs text-gray-400">
                  Current outstanding balance: <span className={selectedAgent.outstanding_balance > 0 ? 'text-amber-400 font-semibold' : 'text-emerald-400 font-semibold'}>{fmtMoney(selectedAgent.outstanding_balance)}</span>
                </div>
              )}
              <div className="flex justify-end space-x-2 pt-3 border-t border-gray-800">
                <button type="button" onClick={() => setActionModal(null)} disabled={actionLoading}
                  className="px-4 py-2 bg-slate-800 hover:bg-slate-700 text-gray-200 rounded-lg text-sm disabled:opacity-50">Cancel</button>
                <button type="button" onClick={handleActionSubmit} disabled={actionLoading}
                  className="px-4 py-2 bg-violet-600 hover:bg-violet-700 text-white rounded-lg text-sm font-medium disabled:opacity-50">
                  {actionLoading ? 'Saving...' : 'Confirm'}
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}
