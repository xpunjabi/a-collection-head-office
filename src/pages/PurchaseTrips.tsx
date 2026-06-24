import { useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { useAppStore } from '../stores/store'
import {
  Plus, X, Calendar, MapPin, Truck, Utensils, Package, Wallet,
  Trash2, Edit3, Calculator, ArrowLeft
} from 'lucide-react'

interface PurchaseTripItem {
  id: number
  trip_id: number
  product_id: number | null
  product_name: string
  qty_purchased: number
  unit_purchase_cost: number
  total_purchase_cost: number
  expense_allocation_amount: number
  landed_unit_cost: number
}

interface PurchaseTrip {
  id: number
  trip_code: string
  trip_date: string
  source_city: string
  supplier_notes: string | null
  travel_cost: number
  transport_cost: number
  food_cost: number
  loading_cost: number
  misc_cost: number
  created_at: string
  updated_at: string
}

interface PurchaseTripSummary {
  trip: PurchaseTrip
  item_count: number
  total_purchase_cost: number
  total_landed_cost: number
}

interface TripDetail {
  trip: PurchaseTrip
  items: PurchaseTripItem[]
}

export default function PurchaseTripsPage() {
  const { products, fetchProducts } = useAppStore()
  const [trips, setTrips] = useState<PurchaseTripSummary[]>([])
  const [selectedTrip, setSelectedTrip] = useState<TripDetail | null>(null)

  // Trip form state
  const [showTripModal, setShowTripModal] = useState(false)
  const [editTripId, setEditTripId] = useState<number | null>(null)
  const [tripDate, setTripDate] = useState(new Date().toISOString().split('T')[0])
  const [sourceCity, setSourceCity] = useState('Faisalabad')
  const [supplierNotes, setSupplierNotes] = useState('')
  const [travelCost, setTravelCost] = useState(0)
  const [transportCost, setTransportCost] = useState(0)
  const [foodCost, setFoodCost] = useState(0)
  const [loadingCost, setLoadingCost] = useState(0)
  const [miscCost, setMiscCost] = useState(0)

  // Item form state
  const [showItemModal, setShowItemModal] = useState(false)
  const [itemProductId, setItemProductId] = useState<number | ''>('')
  const [itemQty, setItemQty] = useState(1)
  const [itemUnitCost, setItemUnitCost] = useState(0)
  const [itemSaving, setItemSaving] = useState(false)

  useEffect(() => {
    fetchProducts()
    loadTrips()
  }, [])

  const loadTrips = async () => {
    try {
      const data: PurchaseTripSummary[] = await invoke('get_purchase_trips')
      setTrips(data)
    } catch (err) { console.error('Failed to load trips:', err) }
  }

  const loadTripDetail = async (id: number) => {
    try {
      const detail: TripDetail = await invoke('get_purchase_trip', { id })
      setSelectedTrip(detail)
    } catch (err) {
      console.error('Failed to load trip detail:', err)
      alert(`Error: ${err}`)
    }
  }

  const handleOpenAddTrip = () => {
    setEditTripId(null)
    setTripDate(new Date().toISOString().split('T')[0])
    setSourceCity('Faisalabad')
    setSupplierNotes('')
    setTravelCost(0); setTransportCost(0); setFoodCost(0); setLoadingCost(0); setMiscCost(0)
    setShowTripModal(true)
  }

  const handleOpenEditTrip = (t: PurchaseTripSummary) => {
    setEditTripId(t.trip.id)
    setTripDate(t.trip.trip_date.split('T')[0])
    setSourceCity(t.trip.source_city)
    setSupplierNotes(t.trip.supplier_notes || '')
    setTravelCost(t.trip.travel_cost)
    setTransportCost(t.trip.transport_cost)
    setFoodCost(t.trip.food_cost)
    setLoadingCost(t.trip.loading_cost)
    setMiscCost(t.trip.misc_cost)
    setShowTripModal(true)
  }

  const handleSaveTrip = async () => {
    if (!tripDate) { alert('Trip date is required.'); return }
    try {
      if (editTripId) {
        await invoke('update_purchase_trip', {
          id: editTripId,
          tripDate,
          sourceCity,
          supplierNotes: supplierNotes || null,
          travelCost, transportCost, foodCost, loadingCost, miscCost,
        })
      } else {
        const newId = await invoke<number>('create_purchase_trip', {
          tripDate,
          sourceCity: sourceCity || null,
          supplierNotes: supplierNotes || null,
          travelCost, transportCost, foodCost, loadingCost, miscCost,
        })
        // Immediately open the new trip for adding items
        setShowTripModal(false)
        await loadTrips()
        await loadTripDetail(newId)
        return
      }
      setShowTripModal(false)
      await loadTrips()
      if (selectedTrip) await loadTripDetail(selectedTrip.trip.id)
    } catch (err) {
      alert(`Error: ${err}`)
    }
  }

  const handleDeleteTrip = async (id: number) => {
    if (!confirm('Delete this trip? All items will be removed and stock reversed. This cannot be undone.')) return
    try {
      await invoke('delete_purchase_trip', { id })
      if (selectedTrip?.trip.id === id) setSelectedTrip(null)
      await loadTrips()
      await fetchProducts()
    } catch (err) { alert(`Error: ${err}`) }
  }

  const handleAddItem = async () => {
    if (!selectedTrip) return
    if (!itemProductId) { alert('Select a product.'); return }
    if (itemQty <= 0) { alert('Quantity must be positive.'); return }
    setItemSaving(true)
    try {
      await invoke('add_trip_item', {
        tripId: selectedTrip.trip.id,
        productId: Number(itemProductId),
        qtyPurchased: itemQty,
        unitPurchaseCost: itemUnitCost,
      })
      setShowItemModal(false)
      setItemProductId(''); setItemQty(1); setItemUnitCost(0)
      await loadTripDetail(selectedTrip.trip.id)
      await fetchProducts()
      await loadTrips()
    } catch (err) {
      alert(`Error: ${err}`)
    } finally {
      setItemSaving(false)
    }
  }

  const handleRemoveItem = async (itemId: number) => {
    if (!selectedTrip) return
    if (!confirm('Remove this item? Stock will be reversed.')) return
    try {
      await invoke('remove_trip_item', { itemId })
      await loadTripDetail(selectedTrip.trip.id)
      await fetchProducts()
      await loadTrips()
    } catch (err) { alert(`Error: ${err}`) }
  }

  const fmtMoney = (n: number) => `Rs. ${n.toFixed(2)}`
  const fmtDate = (iso: string) => new Date(iso).toLocaleDateString('en-PK', { day: '2-digit', month: 'short', year: 'numeric' })

  const totalTripExpense = travelCost + transportCost + foodCost + loadingCost + miscCost

  // ---- TRIP DETAIL VIEW ----
  if (selectedTrip) {
    const t = selectedTrip.trip
    const tripExpense = t.travel_cost + t.transport_cost + t.food_cost + t.loading_cost + t.misc_cost
    const totalPurchase = selectedTrip.items.reduce((s, i) => s + i.total_purchase_cost, 0)
    const totalLanded = selectedTrip.items.reduce((s, i) => s + i.total_purchase_cost + i.expense_allocation_amount, 0)

    return (
      <div className="space-y-6">
        <div className="flex items-center justify-between">
          <div className="flex items-center space-x-3">
            <button onClick={() => setSelectedTrip(null)} className="p-2 text-gray-400 hover:text-white">
              <ArrowLeft size={20} />
            </button>
            <div>
              <h1 className="text-2xl font-bold text-white font-display">{t.trip_code}</h1>
              <p className="text-sm text-gray-400">{fmtDate(t.trip_date)} • {t.source_city}</p>
            </div>
          </div>
          <div className="flex items-center space-x-2">
            <button onClick={() => handleOpenEditTrip({ trip: t, item_count: selectedTrip.items.length, total_purchase_cost: totalPurchase, total_landed_cost: totalLanded })}
              className="flex items-center space-x-1 px-3 py-1.5 bg-slate-800 hover:bg-slate-700 text-gray-200 rounded-lg text-xs">
              <Edit3 size={12} /><span>Edit Trip</span>
            </button>
            <button onClick={() => handleDeleteTrip(t.id)}
              className="flex items-center space-x-1 px-3 py-1.5 bg-red-600/20 hover:bg-red-600/40 text-red-300 rounded-lg text-xs">
              <Trash2 size={12} /><span>Delete</span>
            </button>
          </div>
        </div>

        {/* Trip expenses summary */}
        <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-6 gap-3">
          {[
            { label: 'Travel', value: t.travel_cost, icon: MapPin },
            { label: 'Transport', value: t.transport_cost, icon: Truck },
            { label: 'Food', value: t.food_cost, icon: Utensils },
            { label: 'Loading', value: t.loading_cost, icon: Package },
            { label: 'Misc', value: t.misc_cost, icon: Wallet },
            { label: 'Total Expense', value: tripExpense, icon: Calculator, highlight: true },
          ].map(card => (
            <div key={card.label} className={`glass-card p-3 ${card.highlight ? 'border-violet-500/30' : ''}`}>
              <div className="flex items-center space-x-1 text-[10px] text-gray-500 uppercase">
                <card.icon size={10} />
                <span>{card.label}</span>
              </div>
              <div className={`text-sm font-semibold mt-1 ${card.highlight ? 'text-violet-400' : 'text-white'}`}>
                {fmtMoney(card.value)}
              </div>
            </div>
          ))}
        </div>

        {/* Items table */}
        <div className="glass-card overflow-hidden">
          <div className="flex items-center justify-between p-4 border-b border-gray-800">
            <h2 className="text-sm font-semibold text-white">Items Purchased ({selectedTrip.items.length})</h2>
            <button onClick={() => setShowItemModal(true)}
              className="flex items-center space-x-1 px-3 py-1.5 bg-violet-600 hover:bg-violet-700 text-white rounded-lg text-xs font-medium">
              <Plus size={12} /><span>Add Item</span>
            </button>
          </div>
          <div className="overflow-x-auto">
            <table className="w-full text-xs">
              <thead>
                <tr className="border-b border-gray-800 text-gray-500">
                  <th className="text-left py-2 px-3">Product</th>
                  <th className="text-right py-2 px-3">Qty</th>
                  <th className="text-right py-2 px-3">Unit Cost</th>
                  <th className="text-right py-2 px-3">Total Purchase</th>
                  <th className="text-right py-2 px-3">Expense Alloc.</th>
                  <th className="text-right py-2 px-3">Landed Unit Cost</th>
                  <th className="text-center py-2 px-3">Actions</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-gray-800">
                {selectedTrip.items.length === 0 ? (
                  <tr><td colSpan={7} className="py-8 text-center text-gray-500">No items yet. Click "Add Item" to record purchases.</td></tr>
                ) : selectedTrip.items.map(item => (
                  <tr key={item.id} className="text-gray-300">
                    <td className="py-2 px-3">{item.product_name}</td>
                    <td className="py-2 px-3 text-right">{item.qty_purchased}</td>
                    <td className="py-2 px-3 text-right">{fmtMoney(item.unit_purchase_cost)}</td>
                    <td className="py-2 px-3 text-right">{fmtMoney(item.total_purchase_cost)}</td>
                    <td className="py-2 px-3 text-right text-amber-400">{fmtMoney(item.expense_allocation_amount)}</td>
                    <td className="py-2 px-3 text-right text-violet-400 font-semibold">{fmtMoney(item.landed_unit_cost)}</td>
                    <td className="py-2 px-3 text-center">
                      <button onClick={() => handleRemoveItem(item.id)}
                        className="p-1 text-gray-500 hover:text-red-400"><Trash2 size={12} /></button>
                    </td>
                  </tr>
                ))}
              </tbody>
              {selectedTrip.items.length > 0 && (
                <tfoot>
                  <tr className="border-t border-gray-800 text-white font-semibold">
                    <td className="py-2 px-3">TOTALS</td>
                    <td className="py-2 px-3 text-right">{selectedTrip.items.reduce((s, i) => s + i.qty_purchased, 0)}</td>
                    <td className="py-2 px-3"></td>
                    <td className="py-2 px-3 text-right">{fmtMoney(totalPurchase)}</td>
                    <td className="py-2 px-3 text-right text-amber-400">{fmtMoney(tripExpense)}</td>
                    <td className="py-2 px-3 text-right text-violet-400">{fmtMoney(totalLanded)}</td>
                    <td></td>
                  </tr>
                </tfoot>
              )}
            </table>
          </div>
        </div>

        {t.supplier_notes && (
          <div className="glass-card p-4">
            <h3 className="text-xs font-semibold uppercase text-gray-400 mb-1">Supplier Notes</h3>
            <p className="text-sm text-gray-300">{t.supplier_notes}</p>
          </div>
        )}

        {/* Add Item Modal */}
        {showItemModal && (
          <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm p-4">
            <div className="bg-slate-900 border border-gray-800 rounded-2xl w-full max-w-md overflow-hidden shadow-2xl">
              <div className="flex items-center justify-between p-4 border-b border-gray-800 bg-slate-950/40">
                <h3 className="text-lg font-bold text-white">Add Item to Trip</h3>
                <button onClick={() => setShowItemModal(false)} className="text-gray-400 hover:text-white"><X size={20} /></button>
              </div>
              <div className="p-5 space-y-3">
                <div>
                  <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">Product *</label>
                  <select
                    value={itemProductId}
                    onChange={e => setItemProductId(e.target.value ? Number(e.target.value) : '')}
                    className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500"
                  >
                    <option value="">-- Select Product --</option>
                    {products.map(p => (
                      <option key={p.id} value={p.id}>{p.name} ({p.sku})</option>
                    ))}
                  </select>
                  <p className="text-[10px] text-gray-500 mt-1">Stock will be added to Head Office automatically.</p>
                </div>
                <div className="grid grid-cols-2 gap-3">
                  <div>
                    <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">Quantity *</label>
                    <input type="number" min={1} value={itemQty} onChange={e => setItemQty(Math.max(1, Number(e.target.value)))}
                      className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500" />
                  </div>
                  <div>
                    <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">Unit Purchase Cost (Rs.) *</label>
                    <input type="number" min={0} step={0.01} value={itemUnitCost} onChange={e => setItemUnitCost(Number(e.target.value))}
                      className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500" />
                  </div>
                </div>
                <div className="bg-slate-950/50 border border-gray-800 rounded-lg p-3 text-xs text-gray-400">
                  <div className="flex justify-between">
                    <span>Total Purchase Cost:</span>
                    <span className="text-white font-semibold">{fmtMoney(itemQty * itemUnitCost)}</span>
                  </div>
                  <p className="mt-1 text-[10px]">Landed unit cost will be calculated after adding (based on trip expense allocation).</p>
                </div>
                <div className="flex justify-end space-x-2 pt-3 border-t border-gray-800">
                  <button type="button" onClick={() => setShowItemModal(false)} disabled={itemSaving}
                    className="px-4 py-2 bg-slate-800 hover:bg-slate-700 text-gray-200 rounded-lg text-sm disabled:opacity-50">Cancel</button>
                  <button type="button" onClick={handleAddItem} disabled={itemSaving}
                    className="px-4 py-2 bg-violet-600 hover:bg-violet-700 text-white rounded-lg text-sm font-medium disabled:opacity-50">
                    {itemSaving ? 'Adding...' : 'Add Item'}
                  </button>
                </div>
              </div>
            </div>
          </div>
        )}

        {/* Trip Edit Modal (reused for add) */}
        {showTripModal && (
          <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm p-4">
            <div className="bg-slate-900 border border-gray-800 rounded-2xl w-full max-w-lg overflow-hidden shadow-2xl max-h-[90vh] flex flex-col">
              <div className="flex items-center justify-between p-4 border-b border-gray-800 bg-slate-950/40 shrink-0">
                <h3 className="text-lg font-bold text-white">{editTripId ? 'Edit Trip' : 'New Purchase Trip'}</h3>
                <button onClick={() => setShowTripModal(false)} className="text-gray-400 hover:text-white"><X size={20} /></button>
              </div>
              <div className="p-5 space-y-3 overflow-y-auto">
                <div className="grid grid-cols-2 gap-3">
                  <div>
                    <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">Trip Date *</label>
                    <input type="date" value={tripDate} onChange={e => setTripDate(e.target.value)}
                      className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500" />
                  </div>
                  <div>
                    <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">Source City</label>
                    <input type="text" value={sourceCity} onChange={e => setSourceCity(e.target.value)}
                      className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500" />
                  </div>
                </div>
                <div>
                  <label className="block text-xs font-semibold uppercase text-gray-400 mb-2">Trip Expenses (Rs.)</label>
                  <div className="grid grid-cols-2 md:grid-cols-3 gap-2">
                    {[
                      { label: 'Travel', val: travelCost, set: setTravelCost, icon: MapPin },
                      { label: 'Transport', val: transportCost, set: setTransportCost, icon: Truck },
                      { label: 'Food', val: foodCost, set: setFoodCost, icon: Utensils },
                      { label: 'Loading', val: loadingCost, set: setLoadingCost, icon: Package },
                      { label: 'Misc', val: miscCost, set: setMiscCost, icon: Wallet },
                    ].map(f => (
                      <div key={f.label}>
                        <div className="flex items-center space-x-1 text-[10px] text-gray-500 mb-0.5">
                          <f.icon size={10} /><span>{f.label}</span>
                        </div>
                        <input type="number" min={0} step={0.01} value={f.val} onChange={e => f.set(Number(e.target.value))}
                          className="w-full bg-slate-950 border border-gray-800 rounded-lg px-2 py-1.5 text-sm text-gray-200 focus:outline-none focus:border-violet-500" />
                      </div>
                    ))}
                    <div className="bg-violet-900/20 border border-violet-500/30 rounded-lg p-2">
                      <div className="text-[10px] text-violet-400 mb-0.5">Total Expense</div>
                      <div className="text-sm font-semibold text-violet-300">{fmtMoney(totalTripExpense)}</div>
                    </div>
                  </div>
                </div>
                <div>
                  <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">Supplier Notes</label>
                  <textarea value={supplierNotes} onChange={e => setSupplierNotes(e.target.value)} rows={2}
                    placeholder="Supplier name, contact, deals, etc."
                    className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500" />
                </div>
              </div>
              <div className="flex justify-end space-x-2 p-4 border-t border-gray-800 bg-slate-950/40 shrink-0">
                <button type="button" onClick={() => setShowTripModal(false)}
                  className="px-4 py-2 bg-slate-800 hover:bg-slate-700 text-gray-200 rounded-lg text-sm">Cancel</button>
                <button type="button" onClick={handleSaveTrip}
                  className="px-4 py-2 bg-violet-600 hover:bg-violet-700 text-white rounded-lg text-sm font-medium">
                  {editTripId ? 'Update Trip' : 'Create & Add Items'}
                </button>
              </div>
            </div>
          </div>
        )}
      </div>
    )
  }

  // ---- TRIP LIST VIEW ----
  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold tracking-tight text-white font-display">Purchase Trips</h1>
          <p className="text-sm text-gray-400 mt-1">Record Faisalabad buying trips. Expenses are allocated proportionally to compute landed unit cost.</p>
        </div>
        <button onClick={handleOpenAddTrip}
          className="flex items-center space-x-1 px-4 py-2 bg-violet-600 hover:bg-violet-700 text-white rounded-lg text-sm font-medium">
          <Plus size={16} /><span>New Trip</span>
        </button>
      </div>

      {trips.length === 0 ? (
        <div className="glass-card p-8 text-center">
          <Truck size={32} className="mx-auto text-gray-700 mb-2" />
          <p className="text-sm text-gray-500">No purchase trips yet. Click "New Trip" to record your first Faisalabad buying trip.</p>
        </div>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          {trips.map(t => (
            <button key={t.trip.id} onClick={() => loadTripDetail(t.trip.id)}
              className="text-left glass-card p-4 border border-gray-800 hover:border-violet-500/50 transition-colors">
              <div className="flex items-start justify-between">
                <div>
                  <h3 className="text-sm font-semibold text-white">{t.trip.trip_code}</h3>
                  <p className="text-[11px] text-gray-500 flex items-center mt-0.5">
                    <Calendar size={10} className="mr-1" />{fmtDate(t.trip.trip_date)}
                    <MapPin size={10} className="ml-2 mr-1" />{t.trip.source_city}
                  </p>
                </div>
                <span className="text-[10px] px-1.5 py-0.5 rounded bg-slate-800 text-gray-400">
                  {t.item_count} items
                </span>
              </div>
              <div className="mt-3 grid grid-cols-3 gap-2 text-[10px]">
                <div className="bg-slate-950/50 rounded p-1.5">
                  <div className="text-gray-500">Purchase</div>
                  <div className="text-white font-semibold">Rs. {t.total_purchase_cost.toFixed(0)}</div>
                </div>
                <div className="bg-slate-950/50 rounded p-1.5">
                  <div className="text-gray-500">Expense</div>
                  <div className="text-amber-400 font-semibold">Rs. {(t.trip.travel_cost + t.trip.transport_cost + t.trip.food_cost + t.trip.loading_cost + t.trip.misc_cost).toFixed(0)}</div>
                </div>
                <div className="bg-slate-950/50 rounded p-1.5">
                  <div className="text-gray-500">Landed</div>
                  <div className="text-violet-400 font-semibold">Rs. {t.total_landed_cost.toFixed(0)}</div>
                </div>
              </div>
            </button>
          ))}
        </div>
      )}

      {/* Trip Modal (for list view add) */}
      {showTripModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm p-4">
          <div className="bg-slate-900 border border-gray-800 rounded-2xl w-full max-w-lg overflow-hidden shadow-2xl max-h-[90vh] flex flex-col">
            <div className="flex items-center justify-between p-4 border-b border-gray-800 bg-slate-950/40 shrink-0">
              <h3 className="text-lg font-bold text-white">{editTripId ? 'Edit Trip' : 'New Purchase Trip'}</h3>
              <button onClick={() => setShowTripModal(false)} className="text-gray-400 hover:text-white"><X size={20} /></button>
            </div>
            <div className="p-5 space-y-3 overflow-y-auto">
              <div className="grid grid-cols-2 gap-3">
                <div>
                  <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">Trip Date *</label>
                  <input type="date" value={tripDate} onChange={e => setTripDate(e.target.value)}
                    className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500" />
                </div>
                <div>
                  <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">Source City</label>
                  <input type="text" value={sourceCity} onChange={e => setSourceCity(e.target.value)}
                    className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500" />
                </div>
              </div>
              <div>
                <label className="block text-xs font-semibold uppercase text-gray-400 mb-2">Trip Expenses (Rs.)</label>
                <div className="grid grid-cols-2 md:grid-cols-3 gap-2">
                  {[
                    { label: 'Travel', val: travelCost, set: setTravelCost, icon: MapPin },
                    { label: 'Transport', val: transportCost, set: setTransportCost, icon: Truck },
                    { label: 'Food', val: foodCost, set: setFoodCost, icon: Utensils },
                    { label: 'Loading', val: loadingCost, set: setLoadingCost, icon: Package },
                    { label: 'Misc', val: miscCost, set: setMiscCost, icon: Wallet },
                  ].map(f => (
                    <div key={f.label}>
                      <div className="flex items-center space-x-1 text-[10px] text-gray-500 mb-0.5">
                        <f.icon size={10} /><span>{f.label}</span>
                      </div>
                      <input type="number" min={0} step={0.01} value={f.val} onChange={e => f.set(Number(e.target.value))}
                        className="w-full bg-slate-950 border border-gray-800 rounded-lg px-2 py-1.5 text-sm text-gray-200 focus:outline-none focus:border-violet-500" />
                    </div>
                  ))}
                  <div className="bg-violet-900/20 border border-violet-500/30 rounded-lg p-2">
                    <div className="text-[10px] text-violet-400 mb-0.5">Total Expense</div>
                    <div className="text-sm font-semibold text-violet-300">{fmtMoney(totalTripExpense)}</div>
                  </div>
                </div>
              </div>
              <div>
                <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">Supplier Notes</label>
                <textarea value={supplierNotes} onChange={e => setSupplierNotes(e.target.value)} rows={2}
                  placeholder="Supplier name, contact, deals, etc."
                  className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500" />
              </div>
            </div>
            <div className="flex justify-end space-x-2 p-4 border-t border-gray-800 bg-slate-950/40 shrink-0">
              <button type="button" onClick={() => setShowTripModal(false)}
                className="px-4 py-2 bg-slate-800 hover:bg-slate-700 text-gray-200 rounded-lg text-sm">Cancel</button>
              <button type="button" onClick={handleSaveTrip}
                className="px-4 py-2 bg-violet-600 hover:bg-violet-700 text-white rounded-lg text-sm font-medium">
                {editTripId ? 'Update Trip' : 'Create & Add Items'}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}
