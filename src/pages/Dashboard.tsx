import { useEffect, useState } from 'react'
import { useAppStore } from '../stores/store'
import { invoke } from '@tauri-apps/api/core'
import { 
  Package, 
  Layers, 
  AlertTriangle, 
  Users, 
  Plus, 
  ShoppingCart, 
  TrendingUp,
  FileText
} from 'lucide-react'
import { 
  XAxis, 
  YAxis, 
  CartesianGrid, 
  Tooltip, 
  ResponsiveContainer, 
  AreaChart,
  Area
} from 'recharts'

interface DashboardStats {
  totalProducts: number;
  totalStock: number;
  lowStockCount: number;
  totalCustomers: number;
}

export default function Dashboard() {
  const { products, customers, setCurrentTab, fetchProducts, fetchCustomers } = useAppStore()
  const [stats, setStats] = useState<DashboardStats>({
    totalProducts: 0,
    totalStock: 0,
    lowStockCount: 0,
    totalCustomers: 0
  })

  // Load chart data
  const [chartData, setChartData] = useState([
    { name: 'Jan', Sales: 4000, Profit: 2400 },
    { name: 'Feb', Sales: 3000, Profit: 1398 },
    { name: 'Mar', Sales: 2000, Profit: 9800 },
    { name: 'Apr', Sales: 2780, Profit: 3908 },
    { name: 'May', Sales: 1890, Profit: 4800 },
    { name: 'Jun', Sales: 2390, Profit: 3800 },
  ])

  useEffect(() => {
    fetchProducts()
    fetchCustomers()
  }, [])

  useEffect(() => {
    // Calculate stats
    const totalProducts = products.length
    const totalStock = products.reduce((acc, p) => acc + p.stock_quantity, 0)
    const lowStockCount = products.filter(p => p.stock_quantity <= 5).length
    const totalCustomers = customers.length

    setStats({
      totalProducts,
      totalStock,
      lowStockCount,
      totalCustomers
    })

    // Fetch actual sales stats from db for chart if possible
    invoke('get_sales_report', { start_date: '2026-01-01', end_date: '2026-12-31' })
      .then((res: any) => {
        if (res && res.total_sales > 0) {
          // If we have sales, mock a monthly distribution based on it
          const salesVal = res.total_sales;
          const profitVal = res.total_profit;
          setChartData([
            { name: 'Jan', Sales: salesVal * 0.1, Profit: profitVal * 0.1 },
            { name: 'Feb', Sales: salesVal * 0.15, Profit: profitVal * 0.15 },
            { name: 'Mar', Sales: salesVal * 0.12, Profit: profitVal * 0.12 },
            { name: 'Apr', Sales: salesVal * 0.2, Profit: profitVal * 0.2 },
            { name: 'May', Sales: salesVal * 0.18, Profit: profitVal * 0.18 },
            { name: 'Jun', Sales: salesVal * 0.25, Profit: profitVal * 0.25 },
          ])
        }
      })
      .catch(console.error)
  }, [products, customers])

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold tracking-tight text-white font-display">Dashboard</h1>
        <p className="text-sm text-gray-400 mt-1">Overview of your clothing business operations.</p>
      </div>

      {/* Stats Grid */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
        <div className="glass-card p-5 flex items-center space-x-4">
          <div className="p-3 bg-violet-600/20 text-violet-400 rounded-lg">
            <Package size={24} />
          </div>
          <div>
            <p className="text-xs text-gray-400 uppercase font-medium">Total Products</p>
            <p className="text-2xl font-bold text-white">{stats.totalProducts}</p>
          </div>
        </div>

        <div className="glass-card p-5 flex items-center space-x-4">
          <div className="p-3 bg-emerald-600/20 text-emerald-400 rounded-lg">
            <Layers size={24} />
          </div>
          <div>
            <p className="text-xs text-gray-400 uppercase font-medium">Active Stock</p>
            <p className="text-2xl font-bold text-white">{stats.totalStock} units</p>
          </div>
        </div>

        <div className="glass-card p-5 flex items-center space-x-4 border-red-500/20">
          <div className="p-3 bg-red-600/20 text-red-400 rounded-lg">
            <AlertTriangle size={24} />
          </div>
          <div>
            <p className="text-xs text-gray-400 uppercase font-medium">Low Stock Items</p>
            <p className="text-2xl font-bold text-white">{stats.lowStockCount}</p>
          </div>
        </div>

        <div className="glass-card p-5 flex items-center space-x-4">
          <div className="p-3 bg-cyan-600/20 text-cyan-400 rounded-lg">
            <Users size={24} />
          </div>
          <div>
            <p className="text-xs text-gray-400 uppercase font-medium">Active Customers</p>
            <p className="text-2xl font-bold text-white">{stats.totalCustomers}</p>
          </div>
        </div>
      </div>

      {/* Chart Section */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <div className="glass-card p-5 lg:col-span-2">
          <h2 className="text-lg font-semibold text-white mb-4 flex items-center">
            <TrendingUp className="mr-2 text-violet-500" size={20} /> Sales & Profit Trend
          </h2>
          <div className="h-64 w-full">
            <ResponsiveContainer width="100%" height="100%">
              <AreaChart data={chartData} margin={{ top: 10, right: 10, left: 0, bottom: 0 }}>
                <defs>
                  <linearGradient id="colorSales" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="5%" stopColor="#8b5cf6" stopOpacity={0.4}/>
                    <stop offset="95%" stopColor="#8b5cf6" stopOpacity={0}/>
                  </linearGradient>
                  <linearGradient id="colorProfit" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="5%" stopColor="#10b981" stopOpacity={0.4}/>
                    <stop offset="95%" stopColor="#10b981" stopOpacity={0}/>
                  </linearGradient>
                </defs>
                <CartesianGrid strokeDasharray="3 3" stroke="rgba(255, 255, 255, 0.05)" />
                <XAxis dataKey="name" stroke="#9ca3af" fontSize={11} />
                <YAxis stroke="#9ca3af" fontSize={11} />
                <Tooltip 
                  contentStyle={{ backgroundColor: '#111827', borderColor: 'rgba(255,255,255,0.08)', color: '#fff' }}
                />
                <Area type="monotone" dataKey="Sales" stroke="#8b5cf6" fillOpacity={1} fill="url(#colorSales)" />
                <Area type="monotone" dataKey="Profit" stroke="#10b981" fillOpacity={1} fill="url(#colorProfit)" />
              </AreaChart>
            </ResponsiveContainer>
          </div>
        </div>

        {/* Quick Actions */}
        <div className="glass-card p-5 flex flex-col justify-between">
          <div>
            <h2 className="text-lg font-semibold text-white mb-4">Quick Actions</h2>
            <div className="space-y-2">
              <button 
                onClick={() => setCurrentTab('catalog')}
                className="w-full flex items-center justify-between p-3 bg-violet-600 hover:bg-violet-700 text-white rounded-lg transition-colors glow-btn text-sm font-medium"
              >
                <span>Add New Product</span>
                <Plus size={18} />
              </button>
              <button 
                onClick={() => setCurrentTab('customers')}
                className="w-full flex items-center justify-between p-3 bg-slate-800 hover:bg-slate-700 text-gray-200 rounded-lg border border-gray-700 transition-colors text-sm font-medium"
              >
                <span>New Sales Order</span>
                <ShoppingCart size={18} />
              </button>
              <button 
                onClick={() => setCurrentTab('reports')}
                className="w-full flex items-center justify-between p-3 bg-slate-800 hover:bg-slate-700 text-gray-200 rounded-lg border border-gray-700 transition-colors text-sm font-medium"
              >
                <span>Generate Business Report</span>
                <FileText size={18} />
              </button>
            </div>
          </div>
          
          <div className="mt-6 p-4 bg-violet-950/20 border border-violet-500/10 rounded-lg text-xs text-violet-300">
            <strong>AI Suggestion:</strong> Your inventory shows {stats.lowStockCount} items at low stock levels. Try generating purchase orders or running promotions on dead stock.
          </div>
        </div>
      </div>
    </div>
  )
}
