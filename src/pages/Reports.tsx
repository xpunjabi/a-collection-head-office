import React, { useState } from 'react'
import { useAppStore } from '../stores/store'
import { invoke } from '@tauri-apps/api/core'
import { 
  FileText, 
  Calendar, 
  Download, 
  TrendingUp, 
  Layers, 
  Users,
  Printer
} from 'lucide-react'

export default function Reports() {
  const { settings } = useAppStore()
  
  // Date Filters
  const [startDate, setStartDate] = useState('2026-01-01')
  const [endDate, setEndDate] = useState('2026-12-31')

  // Report States
  const [salesReport, setSalesReport] = useState<any | null>(null)
  const [inventoryReport, setInventoryReport] = useState<any | null>(null)
  const [customerReport, setCustomerReport] = useState<any | null>(null)
  const [activeReportType, setActiveReportType] = useState<'sales' | 'inventory' | 'customer'>('sales')
  const [isGenerating, setIsGenerating] = useState(false)

  const handleGenerateSalesReport = async () => {
    setIsGenerating(true)
    try {
      const res = await invoke('get_sales_report', { start_date: startDate, end_date: endDate })
      setSalesReport(res)
      setActiveReportType('sales')
    } catch (err) {
      alert(`Failed to generate sales report: ${err}`)
    } finally {
      setIsGenerating(false)
    }
  }

  const handleGenerateInventoryReport = async () => {
    setIsGenerating(true)
    try {
      const res = await invoke('get_inventory_report')
      setInventoryReport(res)
      setActiveReportType('inventory')
    } catch (err) {
      alert(`Failed to generate inventory report: ${err}`)
    } finally {
      setIsGenerating(false)
    }
  }

  const handleGenerateCustomerReport = async () => {
    setIsGenerating(true)
    try {
      const res = await invoke('get_customer_report')
      setCustomerReport(res)
      setActiveReportType('customer')
    } catch (err) {
      alert(`Failed to generate customer report: ${err}`)
    } finally {
      setIsGenerating(false)
    }
  }

  const handlePrint = () => {
    window.print()
  }

  const handleExportCsv = () => {
    let csvContent = 'data:text/csv;charset=utf-8,'
    
    if (activeReportType === 'sales' && salesReport) {
      csvContent += 'Sales Report\n'
      csvContent += `Start Date,${salesReport.start_date}\n`
      csvContent += `End Date,${salesReport.end_date}\n`
      csvContent += `Total Sales ($),${salesReport.total_sales.toFixed(2)}\n`
      csvContent += `Total Profit ($),${salesReport.total_profit.toFixed(2)}\n`
      csvContent += `Total Orders,${salesReport.total_orders}\n`
      csvContent += `Average Order Value ($),${salesReport.avg_order_value.toFixed(2)}\n`
    } else if (activeReportType === 'inventory' && inventoryReport) {
      csvContent += 'Inventory Report\n'
      csvContent += `Total Items,${inventoryReport.total_items}\n`
      csvContent += `Total Cost ($),${inventoryReport.total_cost.toFixed(2)}\n`
      csvContent += `Total Retail ($),${inventoryReport.total_retail.toFixed(2)}\n\n`
      csvContent += 'Category,Product Count,Stock Quantity,Cost Value ($),Retail Value ($)\n'
      inventoryReport.category_summaries.forEach((c: any) => {
        csvContent += `${c.category},${c.count},${c.total_stock},${c.cost_value.toFixed(2)},${c.retail_value.toFixed(2)}\n`
      })
    } else if (activeReportType === 'customer' && customerReport) {
      csvContent += 'Customer Summary Report\n'
      csvContent += `Total Customers,${customerReport.total_customers}\n`
      csvContent += `Total Orders,${customerReport.total_orders}\n`
      csvContent += `Total Spent ($),${customerReport.total_spent.toFixed(2)}\n\n`
      csvContent += 'Customer ID,Name,Phone,Total Spent ($),Orders Count\n'
      customerReport.top_customers.forEach((c: any) => {
        csvContent += `${c.customer_id},${c.name},${c.phone || ''},${c.total_spent.toFixed(2)},${c.orders_count}\n`
      })
    } else {
      alert('Generate a report first.')
      return
    }

    const encodedUri = encodeURI(csvContent)
    const link = document.createElement('a')
    link.setAttribute('href', encodedUri)
    link.setAttribute('download', `${activeReportType}_report_${Date.now()}.csv`)
    document.body.appendChild(link)
    link.click()
    document.body.removeChild(link)
  }

  return (
    <div className="space-y-6">
      <div className="flex flex-col md:flex-row md:items-center md:justify-between gap-4">
        <div>
          <h1 className="text-3xl font-bold tracking-tight text-white font-display">Business Reports</h1>
          <p className="text-sm text-gray-400 mt-1">Compile and export sales, inventory, and customer reports.</p>
        </div>
        <div className="flex items-center space-x-2">
          <button 
            onClick={handlePrint}
            className="flex items-center space-x-1 px-3 py-2 bg-slate-800 hover:bg-slate-700 text-gray-200 border border-gray-700 rounded-lg text-sm transition-colors"
          >
            <Printer size={16} />
            <span>Print Report</span>
          </button>
          <button 
            onClick={handleExportCsv}
            className="flex items-center space-x-1 px-3 py-2 bg-violet-600 hover:bg-violet-700 text-white rounded-lg text-sm font-medium transition-colors"
          >
            <Download size={16} />
            <span>Export CSV</span>
          </button>
        </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-4 gap-6">
        {/* Controls Panel */}
        <div className="glass-card p-5 space-y-4 lg:col-span-1">
          <h2 className="text-lg font-semibold text-white">Generate Options</h2>
          
          <div className="space-y-3">
            <button 
              onClick={handleGenerateSalesReport}
              disabled={isGenerating}
              className={`w-full flex items-center space-x-2 p-3 rounded-lg text-sm transition-all border ${
                activeReportType === 'sales' && salesReport 
                  ? 'bg-violet-600 border-violet-500 text-white font-semibold' 
                  : 'bg-slate-950 border-gray-800 text-gray-300 hover:border-gray-700'
              }`}
            >
              <TrendingUp size={16} />
              <span>Sales Report</span>
            </button>
            <button 
              onClick={handleGenerateInventoryReport}
              disabled={isGenerating}
              className={`w-full flex items-center space-x-2 p-3 rounded-lg text-sm transition-all border ${
                activeReportType === 'inventory' && inventoryReport 
                  ? 'bg-violet-600 border-violet-500 text-white font-semibold' 
                  : 'bg-slate-950 border-gray-800 text-gray-300 hover:border-gray-700'
              }`}
            >
              <Layers size={16} />
              <span>Inventory Valuation</span>
            </button>
            <button 
              onClick={handleGenerateCustomerReport}
              disabled={isGenerating}
              className={`w-full flex items-center space-x-2 p-3 rounded-lg text-sm transition-all border ${
                activeReportType === 'customer' && customerReport 
                  ? 'bg-violet-600 border-violet-500 text-white font-semibold' 
                  : 'bg-slate-950 border-gray-800 text-gray-300 hover:border-gray-700'
              }`}
            >
              <Users size={16} />
              <span>Customer Summary</span>
            </button>
          </div>

          {activeReportType === 'sales' && (
            <div className="space-y-3 pt-4 border-t border-gray-800">
              <p className="text-xs font-semibold uppercase text-gray-400">Date Filters</p>
              <div>
                <label className="block text-[10px] text-gray-500 mb-1">Start Date</label>
                <input 
                  type="date" 
                  value={startDate}
                  onChange={(e) => setStartDate(e.target.value)}
                  className="w-full bg-slate-950 border border-gray-850 rounded px-2 py-1.5 text-xs text-gray-200"
                />
              </div>
              <div>
                <label className="block text-[10px] text-gray-500 mb-1">End Date</label>
                <input 
                  type="date" 
                  value={endDate}
                  onChange={(e) => setEndDate(e.target.value)}
                  className="w-full bg-slate-950 border border-gray-850 rounded px-2 py-1.5 text-xs text-gray-200"
                />
              </div>
            </div>
          )}
        </div>

        {/* Report Display Area */}
        <div className="glass-card p-6 lg:col-span-3 min-h-[400px]">
          {activeReportType === 'sales' && salesReport && (
            <div id="print-area" className="space-y-6">
              <div className="border-b border-gray-850 pb-4">
                <h2 className="text-xl font-bold text-white">Sales Performance Report</h2>
                <p className="text-xs text-gray-400">Duration: {startDate} to {endDate}</p>
              </div>

              <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
                <div className="bg-slate-950 p-4 border border-gray-850 rounded-lg">
                  <span className="text-[10px] uppercase text-gray-500">Gross Sales</span>
                  <p className="text-xl font-bold text-violet-400 mt-1">${salesReport.total_sales.toFixed(2)}</p>
                </div>
                <div className="bg-slate-950 p-4 border border-gray-850 rounded-lg">
                  <span className="text-[10px] uppercase text-gray-500">Net Profit</span>
                  <p className="text-xl font-bold text-emerald-400 mt-1">${salesReport.total_profit.toFixed(2)}</p>
                </div>
                <div className="bg-slate-950 p-4 border border-gray-850 rounded-lg">
                  <span className="text-[10px] uppercase text-gray-500">Orders Completed</span>
                  <p className="text-xl font-bold text-white mt-1">{salesReport.total_orders}</p>
                </div>
                <div className="bg-slate-950 p-4 border border-gray-850 rounded-lg">
                  <span className="text-[10px] uppercase text-gray-500">Average Order Value</span>
                  <p className="text-xl font-bold text-cyan-400 mt-1">${salesReport.avg_order_value.toFixed(2)}</p>
                </div>
              </div>
            </div>
          )}

          {activeReportType === 'inventory' && inventoryReport && (
            <div id="print-area" className="space-y-6">
              <div className="border-b border-gray-850 pb-4">
                <h2 className="text-xl font-bold text-white">Inventory Valuation Report</h2>
                <p className="text-xs text-gray-400">Snapshot generated at: {new Date().toLocaleDateString()}</p>
              </div>

              <div className="grid grid-cols-3 gap-4">
                <div className="bg-slate-950 p-4 border border-gray-850 rounded-lg">
                  <span className="text-[10px] uppercase text-gray-500">Total Stock Pieces</span>
                  <p className="text-xl font-bold text-white mt-1">{inventoryReport.total_items} units</p>
                </div>
                <div className="bg-slate-950 p-4 border border-gray-850 rounded-lg">
                  <span className="text-[10px] uppercase text-gray-500">Asset Cost Value</span>
                  <p className="text-xl font-bold text-violet-400 mt-1">${inventoryReport.total_cost.toFixed(2)}</p>
                </div>
                <div className="bg-slate-950 p-4 border border-gray-850 rounded-lg">
                  <span className="text-[10px] uppercase text-gray-500">Retail Value</span>
                  <p className="text-xl font-bold text-cyan-400 mt-1">${inventoryReport.total_retail.toFixed(2)}</p>
                </div>
              </div>

              <div className="overflow-x-auto pt-4">
                <p className="text-xs font-semibold text-gray-400 mb-2 uppercase">Category Breakdown</p>
                <table className="w-full text-left text-xs">
                  <thead>
                    <tr className="border-b border-gray-800 text-gray-500 uppercase">
                      <th className="pb-2">Category</th>
                      <th className="pb-2 text-center">Products Count</th>
                      <th className="pb-2 text-center">Stock Quantity</th>
                      <th className="pb-2 text-right">Cost Value</th>
                      <th className="pb-2 text-right">Retail Value</th>
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-gray-900 text-gray-300">
                    {inventoryReport.category_summaries.map((cat: any) => (
                      <tr key={cat.category} className="hover:bg-slate-900/10">
                        <td className="py-2.5 font-semibold text-white">{cat.category}</td>
                        <td className="py-2.5 text-center">{cat.count}</td>
                        <td className="py-2.5 text-center font-bold">{cat.total_stock}</td>
                        <td className="py-2.5 text-right font-mono">${cat.cost_value.toFixed(2)}</td>
                        <td className="py-2.5 text-right font-mono text-violet-400">${cat.retail_value.toFixed(2)}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>
          )}

          {activeReportType === 'customer' && customerReport && (
            <div id="print-area" className="space-y-6">
              <div className="border-b border-gray-850 pb-4">
                <h2 className="text-xl font-bold text-white">Customer Summary Report</h2>
                <p className="text-xs text-gray-400">Total Customer Spendings</p>
              </div>

              <div className="grid grid-cols-3 gap-4">
                <div className="bg-slate-950 p-4 border border-gray-850 rounded-lg">
                  <span className="text-[10px] uppercase text-gray-500">Total Customers</span>
                  <p className="text-xl font-bold text-white mt-1">{customerReport.total_customers}</p>
                </div>
                <div className="bg-slate-950 p-4 border border-gray-850 rounded-lg">
                  <span className="text-[10px] uppercase text-gray-500">Total Orders Placed</span>
                  <p className="text-xl font-bold text-violet-400 mt-1">{customerReport.total_orders}</p>
                </div>
                <div className="bg-slate-950 p-4 border border-gray-850 rounded-lg">
                  <span className="text-[10px] uppercase text-gray-500">Total Gross Revenue</span>
                  <p className="text-xl font-bold text-emerald-400 mt-1">${customerReport.total_spent.toFixed(2)}</p>
                </div>
              </div>

              <div className="overflow-x-auto pt-4">
                <p className="text-xs font-semibold text-gray-400 mb-2 uppercase">Top Spenders</p>
                <table className="w-full text-left text-xs">
                  <thead>
                    <tr className="border-b border-gray-800 text-gray-500 uppercase">
                      <th className="pb-2">Customer Name</th>
                      <th className="pb-2">Phone</th>
                      <th className="pb-2 text-center">Orders Count</th>
                      <th className="pb-2 text-right">Total Spent</th>
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-gray-900 text-gray-300">
                    {customerReport.top_customers.map((c: any) => (
                      <tr key={c.customer_id} className="hover:bg-slate-900/10">
                        <td className="py-2.5 font-semibold text-white">{c.name}</td>
                        <td className="py-2.5 text-gray-400">{c.phone || '-'}</td>
                        <td className="py-2.5 text-center font-bold">{c.orders_count}</td>
                        <td className="py-2.5 text-right font-mono text-emerald-400 font-bold">${c.total_spent.toFixed(2)}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>
          )}

          {!salesReport && !inventoryReport && !customerReport && (
            <div className="flex flex-col items-center justify-center text-gray-500 h-64">
              <FileText size={40} className="text-gray-700 mb-3" />
              <p className="text-sm">Click any button on the left to compile a report.</p>
            </div>
          )}
        </div>
      </div>
    </div>
  )
}
