import { useEffect, useState } from 'react'
import { useAppStore } from '../stores/store'
import { invoke } from '@tauri-apps/api/core'
import { 
  AlertTriangle, 
  Flame, 
  TrendingUp, 
  Plus, 
  Minus
} from 'lucide-react'

interface SummaryData {
  total_products: number;
  total_stock: number;
  total_cost_value: number;
  total_retail_value: number;
  potential_profit: number;
}

export default function Inventory() {
  const { fetchProducts } = useAppStore()
  const [summary, setSummary] = useState<SummaryData>({
    total_products: 0,
    total_stock: 0,
    total_cost_value: 0.0,
    total_retail_value: 0.0,
    potential_profit: 0.0
  })

  const [lowStockList, setLowStockList] = useState<any[]>([])
  const [deadStockList, setDeadStockList] = useState<any[]>([])
  const [bestSellers, setBestSellers] = useState<any[]>([])
  const [deadStockDays, setDeadStockDays] = useState(30)
  const [activeTab, setActiveTab] = useState<'low' | 'dead' | 'best'>('low')

  useEffect(() => {
    loadInventoryData()
  }, [])

  const loadInventoryData = async () => {
    try {
      await fetchProducts()
      const sumData: SummaryData = await invoke('get_inventory_summary')
      setSummary(sumData)

      const lowData: any[] = await invoke('get_low_stock', { threshold: 5 })
      setLowStockList(lowData)

      const deadData: any[] = await invoke('get_dead_stock', { days_limit: Number(deadStockDays) })
      setDeadStockList(deadData)

      const bestData: any[] = await invoke('get_best_sellers', { limit: 10 })
      setBestSellers(bestData)
    } catch (err) {
      console.error(err)
    }
  }

  const handleAdjustStock = async (productId: number, adjustment: number) => {
    try {
      await invoke('adjust_stock', { productId, adjustment })
      await loadInventoryData()
    } catch (err) {
      alert(`Adjustment failed: ${err}`)
    }
  }

  useEffect(() => {
    // Reload dead stock whenever days filter changes
    invoke('get_dead_stock', { days_limit: Number(deadStockDays) })
      .then((res: any) => setDeadStockList(res))
      .catch(console.error)
  }, [deadStockDays])

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold tracking-tight text-white font-display">Inventory Analytics</h1>
        <p className="text-sm text-gray-400 mt-1">Monitor stock health, profit margins, and dead stock.</p>
      </div>

      {/* Summary Cards */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
        <div className="glass-card p-5">
          <p className="text-xs text-gray-400 uppercase font-semibold">Total Stock</p>
          <p className="text-2xl font-bold text-white mt-1">{summary.total_stock} pcs</p>
        </div>
        <div className="glass-card p-5">
          <p className="text-xs text-gray-400 uppercase font-semibold">Asset Cost Value</p>
          <p className="text-2xl font-bold text-violet-400 mt-1">${summary.total_cost_value.toFixed(2)}</p>
        </div>
        <div className="glass-card p-5">
          <p className="text-xs text-gray-400 uppercase font-semibold">Estimated Retail Value</p>
          <p className="text-2xl font-bold text-cyan-400 mt-1">${summary.total_retail_value.toFixed(2)}</p>
        </div>
        <div className="glass-card p-5">
          <p className="text-xs text-gray-400 uppercase font-semibold">Potential Profit</p>
          <p className="text-2xl font-bold text-emerald-400 mt-1">${summary.potential_profit.toFixed(2)}</p>
        </div>
      </div>

      {/* Tabbable Lists */}
      <div className="glass-card">
        {/* Navigation Tabs */}
        <div className="flex border-b border-gray-800 bg-slate-900/40">
          <button
            onClick={() => setActiveTab('low')}
            className={`px-6 py-4 text-sm font-medium border-b-2 transition-all flex items-center space-x-2 ${
              activeTab === 'low' 
                ? 'border-violet-500 text-violet-400 bg-slate-950/20' 
                : 'border-transparent text-gray-400 hover:text-gray-200'
            }`}
          >
            <AlertTriangle size={16} />
            <span>Low Stock Alerts ({lowStockList.length})</span>
          </button>
          <button
            onClick={() => setActiveTab('dead')}
            className={`px-6 py-4 text-sm font-medium border-b-2 transition-all flex items-center space-x-2 ${
              activeTab === 'dead' 
                ? 'border-violet-500 text-violet-400 bg-slate-950/20' 
                : 'border-transparent text-gray-400 hover:text-gray-200'
            }`}
          >
            <Flame size={16} />
            <span>Dead Stock Audit ({deadStockList.length})</span>
          </button>
          <button
            onClick={() => setActiveTab('best')}
            className={`px-6 py-4 text-sm font-medium border-b-2 transition-all flex items-center space-x-2 ${
              activeTab === 'best' 
                ? 'border-violet-500 text-violet-400 bg-slate-950/20' 
                : 'border-transparent text-gray-400 hover:text-gray-200'
            }`}
          >
            <TrendingUp size={16} />
            <span>Best Sellers ({bestSellers.length})</span>
          </button>
        </div>

        {/* Tab Panels */}
        <div className="p-6">
          {activeTab === 'low' && (
            <div className="space-y-4">
              <p className="text-xs text-gray-400">Products with stock at or below critical threshold (5 units):</p>
              <div className="overflow-x-auto">
                <table className="w-full text-left">
                  <thead>
                    <tr className="border-b border-gray-800 text-xs font-semibold text-gray-500 uppercase">
                      <th className="pb-3">Product Name</th>
                      <th className="pb-3">SKU</th>
                      <th className="pb-3">Category</th>
                      <th className="pb-3 text-center">Current Stock</th>
                      <th className="pb-3 text-center">Adjust Stock</th>
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-gray-800 text-sm text-gray-300">
                    {lowStockList.length === 0 ? (
                      <tr>
                        <td colSpan={5} className="py-6 text-center text-gray-500">All products are healthy! No low stock warnings.</td>
                      </tr>
                    ) : (
                      lowStockList.map(item => (
                        <tr key={item.id} className="hover:bg-slate-900/10">
                          <td className="py-3 font-semibold text-white">{item.name}</td>
                          <td className="py-3 font-mono text-xs">{item.sku}</td>
                          <td className="py-3">{item.category || '-'}</td>
                          <td className="py-3 text-center text-red-400 font-bold">{item.stock_quantity}</td>
                          <td className="py-3 text-center">
                            <div className="flex items-center justify-center space-x-2">
                              <button 
                                onClick={() => handleAdjustStock(item.id, -1)}
                                className="p-1 bg-slate-800 hover:bg-slate-700 text-gray-400 rounded transition-colors"
                              >
                                <Minus size={12} />
                              </button>
                              <button 
                                onClick={() => handleAdjustStock(item.id, 5)}
                                className="px-2 py-0.5 bg-violet-600 hover:bg-violet-700 text-white rounded text-xs transition-colors"
                              >
                                +5
                              </button>
                              <button 
                                onClick={() => handleAdjustStock(item.id, 1)}
                                className="p-1 bg-slate-800 hover:bg-slate-700 text-gray-400 rounded transition-colors"
                              >
                                <Plus size={12} />
                              </button>
                            </div>
                          </td>
                        </tr>
                      ))
                    )}
                  </tbody>
                </table>
              </div>
            </div>
          )}

          {activeTab === 'dead' && (
            <div className="space-y-4">
              <div className="flex items-center justify-between">
                <p className="text-xs text-gray-400">Products in stock with 0 sales in the chosen duration:</p>
                <div className="flex items-center space-x-2">
                  <span className="text-xs text-gray-400">Cutoff:</span>
                  <select 
                    value={deadStockDays}
                    onChange={(e) => setDeadStockDays(Number(e.target.value))}
                    className="bg-slate-950 border border-gray-800 rounded px-2 py-1 text-xs text-gray-200 focus:outline-none"
                  >
                    <option value={15}>15 Days</option>
                    <option value={30}>30 Days</option>
                    <option value={60}>60 Days</option>
                    <option value={90}>90 Days</option>
                  </select>
                </div>
              </div>
              <div className="overflow-x-auto">
                <table className="w-full text-left">
                  <thead>
                    <tr className="border-b border-gray-800 text-xs font-semibold text-gray-500 uppercase">
                      <th className="pb-3">Product Name</th>
                      <th className="pb-3">SKU</th>
                      <th className="pb-3">Category</th>
                      <th className="pb-3">Added Date</th>
                      <th className="pb-3 text-center">In-Stock Quantity</th>
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-gray-800 text-sm text-gray-300">
                    {deadStockList.length === 0 ? (
                      <tr>
                        <td colSpan={5} className="py-6 text-center text-gray-500">No dead stock detected! All items are active.</td>
                      </tr>
                    ) : (
                      deadStockList.map(item => (
                        <tr key={item.id} className="hover:bg-slate-900/10">
                          <td className="py-3 font-semibold text-white">{item.name}</td>
                          <td className="py-3 font-mono text-xs">{item.sku}</td>
                          <td className="py-3">{item.category || '-'}</td>
                          <td className="py-3 text-xs text-gray-400">{new Date(item.created_at).toLocaleDateString()}</td>
                          <td className="py-3 text-center font-bold text-amber-500">{item.stock_quantity}</td>
                        </tr>
                      ))
                    )}
                  </tbody>
                </table>
              </div>
            </div>
          )}

          {activeTab === 'best' && (
            <div className="space-y-4">
              <p className="text-xs text-gray-400">Top selling items sorted by quantity sold:</p>
              <div className="overflow-x-auto">
                <table className="w-full text-left">
                  <thead>
                    <tr className="border-b border-gray-800 text-xs font-semibold text-gray-500 uppercase">
                      <th className="pb-3">Product Name</th>
                      <th className="pb-3">SKU</th>
                      <th className="pb-3 text-center">Quantity Sold</th>
                      <th className="pb-3 text-right">Revenue</th>
                      <th className="pb-3 text-right">Gross Profit</th>
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-gray-800 text-sm text-gray-300">
                    {bestSellers.length === 0 ? (
                      <tr>
                        <td colSpan={5} className="py-6 text-center text-gray-500">No sales completed yet. Place orders to generate best seller metrics.</td>
                      </tr>
                    ) : (
                      bestSellers.map(item => (
                        <tr key={item.product_id} className="hover:bg-slate-900/10">
                          <td className="py-3 font-semibold text-white">{item.name}</td>
                          <td className="py-3 font-mono text-xs">{item.sku}</td>
                          <td className="py-3 text-center font-bold text-cyan-400">{item.quantity_sold} pcs</td>
                          <td className="py-3 text-right font-mono text-gray-300">${item.total_revenue.toFixed(2)}</td>
                          <td className="py-3 text-right font-mono text-emerald-400 font-semibold">${item.total_profit.toFixed(2)}</td>
                        </tr>
                      ))
                    )}
                  </tbody>
                </table>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  )
}
