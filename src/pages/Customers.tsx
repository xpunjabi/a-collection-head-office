import React, { useEffect, useState } from 'react'
import { useAppStore, Customer, OrderHistory } from '../stores/store'
import { 
  Search, 
  UserPlus, 
  Phone, 
  Calendar, 
  ShoppingBag, 
  User, 
  Trash2,
  X
} from 'lucide-react'

export default function Customers() {
  const { 
    customers, 
    products,
    fetchCustomers, 
    fetchProducts,
    addCustomer, 
    updateCustomer, 
    deleteCustomer,
    cart,
    addToCart,
    removeFromCart,
    clearCart,
    createOrder,
    getCustomerHistory
  } = useAppStore()

  const [searchTerm, setSearchTerm] = useState('')
  const [selectedCustomerId, setSelectedCustomerId] = useState<number | null>(null)
  const [purchaseHistory, setPurchaseHistory] = useState<OrderHistory[]>([])
  
  // Customer Modals
  const [showCustModal, setShowCustModal] = useState(false)
  const [editCustomer, setEditCustomer] = useState<Customer | null>(null)
  const [custName, setCustName] = useState('')
  const [custPhone, setCustPhone] = useState('')
  const [custLocation, setCustLocation] = useState('')
  const [custNotes, setCustNotes] = useState('')

  // Order Placement Modal
  const [showOrderModal, setShowOrderModal] = useState(false)
  const [orderSearch, setOrderSearch] = useState('')

  useEffect(() => {
    fetchCustomers()
    fetchProducts()
  }, [])

  useEffect(() => {
    if (selectedCustomerId) {
      loadHistory(selectedCustomerId)
    } else {
      setPurchaseHistory([])
    }
  }, [selectedCustomerId])

  const loadHistory = async (id: number) => {
    try {
      const history = await getCustomerHistory(id)
      setPurchaseHistory(history)
    } catch (err) {
      console.error(err)
    }
  }

  const handleOpenAddCust = () => {
    setEditCustomer(null)
    setCustName('')
    setCustPhone('')
    setCustLocation('')
    setCustNotes('')
    setShowCustModal(true)
  }

  const handleOpenEditCust = (c: Customer) => {
    setEditCustomer(c)
    setCustName(c.name)
    setCustPhone(c.phone || '')
    setCustLocation(c.location || '')
    setCustNotes(c.notes || '')
    setShowCustModal(true)
  }

  const handleSaveCustomer = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!custName) return

    const data: Customer = {
      id: editCustomer?.id,
      name: custName,
      phone: custPhone,
      location: custLocation,
      notes: custNotes
    }

    try {
      if (editCustomer) {
        await updateCustomer(data)
      } else {
        await addCustomer(data)
      }
      setShowCustModal(false)
    } catch (err) {
      alert(`Error saving customer: ${err}`)
    }
  }

  const handleDeleteCustomer = async (id: number) => {
    if (confirm('Are you sure you want to delete this customer? This will also delete their order history.')) {
      try {
        await deleteCustomer(id)
        if (selectedCustomerId === id) setSelectedCustomerId(null)
      } catch (err) {
        alert(err)
      }
    }
  }

  const handlePlaceOrder = async () => {
    if (!selectedCustomerId) return
    if (cart.length === 0) {
      alert('Your cart is empty. Please add products first.')
      return
    }

    const items = cart.map(item => ({
      product_id: item.product.id!,
      quantity: item.quantity
    }))

    try {
      await createOrder(selectedCustomerId, items)
      clearCart()
      setShowOrderModal(false)
      loadHistory(selectedCustomerId)
      alert('Order placed successfully! Stock levels updated.')
    } catch (err) {
      alert(`Failed to place order: ${err}`)
    }
  }

  // Filters
  const filteredCustomers = customers.filter(c => 
    c.name.toLowerCase().includes(searchTerm.toLowerCase()) || 
    (c.phone && c.phone.includes(searchTerm)) ||
    (c.location && c.location.toLowerCase().includes(searchTerm.toLowerCase()))
  )

  const activeCustomer = customers.find(c => c.id === selectedCustomerId)

  // Products filter for order modal
  const filteredProductsForOrder = products.filter(p => 
    p.status === 'active' && 
    p.stock_quantity > 0 &&
    (p.name.toLowerCase().includes(orderSearch.toLowerCase()) || p.sku.toLowerCase().includes(orderSearch.toLowerCase()))
  )

  const totalCartValue = cart.reduce((acc, item) => acc + item.product.sale_price * item.quantity, 0)

  return (
    <div className="space-y-6">
      <div className="flex flex-col md:flex-row md:items-center md:justify-between gap-4">
        <div>
          <h1 className="text-3xl font-bold tracking-tight text-white font-display">Customer Management</h1>
          <p className="text-sm text-gray-400 mt-1">Manage customer profiles and place sales orders.</p>
        </div>
        <button 
          onClick={handleOpenAddCust}
          className="flex items-center space-x-1 px-4 py-2 bg-violet-600 hover:bg-violet-700 text-white rounded-lg text-sm font-medium transition-colors self-start"
        >
          <UserPlus size={16} />
          <span>New Customer</span>
        </button>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Customers List (Left 1/3) */}
        <div className="glass-card p-5 flex flex-col h-[550px]">
          <div className="relative mb-4">
            <Search className="absolute left-3 top-2.5 text-gray-500" size={18} />
            <input 
              type="text"
              placeholder="Search customers..."
              value={searchTerm}
              onChange={(e) => setSearchTerm(e.target.value)}
              className="w-full bg-slate-950 border border-gray-800 rounded-lg pl-10 pr-4 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500 transition-colors"
            />
          </div>

          <div className="flex-1 overflow-y-auto space-y-2 pr-1">
            {filteredCustomers.length === 0 ? (
              <p className="text-sm text-gray-500 text-center py-8">No customers found.</p>
            ) : (
              filteredCustomers.map(c => (
                <div 
                  key={c.id}
                  onClick={() => c.id && setSelectedCustomerId(c.id)}
                  className={`p-3 rounded-lg border cursor-pointer transition-all flex justify-between items-center ${
                    selectedCustomerId === c.id 
                      ? 'bg-violet-600/10 border-violet-500/50' 
                      : 'bg-slate-950 border-gray-800 hover:border-gray-700'
                  }`}
                >
                  <div className="space-y-1">
                    <p className="text-sm font-semibold text-white">{c.name}</p>
                    <p className="text-xs text-gray-400 flex items-center"><Phone size={10} className="mr-1" />{c.phone || '-'}</p>
                  </div>
                  <button 
                    onClick={(e) => { e.stopPropagation(); c.id && handleDeleteCustomer(c.id); }}
                    className="text-gray-500 hover:text-red-400 transition-colors p-1"
                  >
                    <Trash2 size={14} />
                  </button>
                </div>
              ))
            )}
          </div>
        </div>

        {/* Customer Details & Order History (Right 2/3) */}
        <div className="lg:col-span-2 space-y-6">
          {activeCustomer ? (
            <>
              {/* Customer Profile Card */}
              <div className="glass-card p-5 space-y-4">
                <div className="flex justify-between items-start">
                  <div className="flex items-center space-x-3">
                    <div className="w-12 h-12 bg-violet-600/10 text-violet-400 rounded-full flex items-center justify-center">
                      <User size={24} />
                    </div>
                    <div>
                      <h2 className="text-xl font-bold text-white">{activeCustomer.name}</h2>
                      <p className="text-xs text-gray-400">Created: {new Date(activeCustomer.created_at || '').toLocaleDateString()}</p>
                    </div>
                  </div>
                  <div className="flex space-x-2">
                    <button 
                      onClick={() => handleOpenEditCust(activeCustomer)}
                      className="px-3 py-1.5 bg-slate-800 hover:bg-slate-700 border border-gray-700 text-gray-300 rounded-lg text-xs font-medium transition-colors"
                    >
                      Edit Profile
                    </button>
                    <button 
                      onClick={() => setShowOrderModal(true)}
                      className="px-3 py-1.5 bg-violet-600 hover:bg-violet-700 text-white rounded-lg text-xs font-medium transition-colors glow-btn"
                    >
                      New Order
                    </button>
                  </div>
                </div>

                <div className="grid grid-cols-2 gap-4 text-sm border-t border-gray-800 pt-4">
                  <div className="space-y-1">
                    <p className="text-xs text-gray-400 uppercase font-semibold">Phone</p>
                    <p className="text-gray-200">{activeCustomer.phone || 'N/A'}</p>
                  </div>
                  <div className="space-y-1">
                    <p className="text-xs text-gray-400 uppercase font-semibold">Location</p>
                    <p className="text-gray-200">{activeCustomer.location || 'N/A'}</p>
                  </div>
                </div>

                {activeCustomer.notes && (
                  <div className="bg-slate-950 p-3 rounded-lg border border-gray-800 text-xs text-gray-400">
                    <strong>Notes:</strong> {activeCustomer.notes}
                  </div>
                )}
              </div>

              {/* Purchase History */}
              <div className="glass-card p-5">
                <h3 className="text-lg font-semibold text-white mb-4 flex items-center">
                  <ShoppingBag className="mr-2 text-violet-500" size={18} /> Purchase History
                </h3>
                
                <div className="space-y-3 overflow-y-auto max-h-[250px] pr-1">
                  {purchaseHistory.length === 0 ? (
                    <p className="text-sm text-gray-500 text-center py-6">No previous orders found for this customer.</p>
                  ) : (
                    purchaseHistory.map(order => (
                      <div key={order.order_id} className="p-4 bg-slate-950 border border-gray-800 rounded-xl space-y-3">
                        <div className="flex justify-between items-center text-xs">
                          <div className="space-y-0.5">
                            <p className="text-gray-400">Order ID: #{order.order_id}</p>
                            <p className="text-gray-500 flex items-center"><Calendar size={10} className="mr-1" />{new Date(order.order_date).toLocaleString()}</p>
                          </div>
                          <div className="text-right">
                            <p className="text-sm font-bold text-violet-400">${order.total_amount.toFixed(2)}</p>
                            <p className="text-[10px] text-emerald-400">Est. Profit: ${order.profit.toFixed(2)}</p>
                          </div>
                        </div>

                        <div className="border-t border-gray-900 pt-2 space-y-1">
                          {order.items.map((item, idx) => (
                            <div key={idx} className="flex justify-between text-xs text-gray-400">
                              <span>{item.product_name} <span className="text-gray-600">x{item.quantity}</span></span>
                              <span className="font-mono">${(item.sale_price * item.quantity).toFixed(2)}</span>
                            </div>
                          ))}
                        </div>
                      </div>
                    ))
                  )}
                </div>
              </div>
            </>
          ) : (
            <div className="glass-card p-12 text-center text-gray-500 flex flex-col items-center justify-center h-[350px]">
              <User size={48} className="text-gray-700 mb-4" />
              <p className="text-sm">Select a customer from the list to view profile, order history, and place new orders.</p>
            </div>
          )}
        </div>
      </div>

      {/* Customer Add/Edit Modal */}
      {showCustModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm p-4">
          <div className="bg-slate-900 border border-gray-800 rounded-2xl w-full max-w-md overflow-hidden shadow-2xl animate-in fade-in zoom-in-95 duration-150">
            <div className="flex items-center justify-between p-4 border-b border-gray-800 bg-slate-950/40">
              <h3 className="text-lg font-bold text-white font-display">
                {editCustomer ? 'Edit Customer' : 'Add New Customer'}
              </h3>
              <button onClick={() => setShowCustModal(false)} className="text-gray-400 hover:text-white transition-colors">
                <X size={20} />
              </button>
            </div>

            <form onSubmit={handleSaveCustomer} className="p-6 space-y-4">
              <div>
                <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">Full Name *</label>
                <input 
                  type="text" required value={custName} onChange={(e) => setCustName(e.target.value)}
                  className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500"
                  placeholder="E.g. Yasir Ali"
                />
              </div>

              <div>
                <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">Phone Number</label>
                <input 
                  type="text" value={custPhone} onChange={(e) => setCustPhone(e.target.value)}
                  className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500"
                  placeholder="E.g. +923001234567"
                />
              </div>

              <div>
                <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">Location / Address</label>
                <input 
                  type="text" value={custLocation} onChange={(e) => setCustLocation(e.target.value)}
                  className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500"
                  placeholder="E.g. Lahore, Pakistan"
                />
              </div>

              <div>
                <label className="block text-xs font-semibold uppercase text-gray-400 mb-1">Notes</label>
                <textarea 
                  value={custNotes} onChange={(e) => setCustNotes(e.target.value)} rows={3}
                  className="w-full bg-slate-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-violet-500"
                  placeholder="Preferences, sizing, favorite categories..."
                />
              </div>

              <div className="flex justify-end space-x-2 pt-4 border-t border-gray-800">
                <button
                  type="button" onClick={() => setShowCustModal(false)}
                  className="px-4 py-2 bg-slate-800 hover:bg-slate-700 text-gray-200 rounded-lg text-sm"
                >
                  Cancel
                </button>
                <button
                  type="submit"
                  className="px-4 py-2 bg-violet-600 hover:bg-violet-700 text-white rounded-lg text-sm font-medium"
                >
                  Save Customer
                </button>
              </div>
            </form>
          </div>
        </div>
      )}

      {/* Order Placement Modal */}
      {showOrderModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm p-4">
          <div className="bg-slate-900 border border-gray-800 rounded-2xl w-full max-w-2xl overflow-hidden shadow-2xl animate-in fade-in zoom-in-95 duration-150">
            <div className="flex items-center justify-between p-4 border-b border-gray-800 bg-slate-950/40">
              <h3 className="text-lg font-bold text-white font-display">
                Place Sales Order for {activeCustomer?.name}
              </h3>
              <button onClick={() => setShowOrderModal(false)} className="text-gray-400 hover:text-white transition-colors">
                <X size={20} />
              </button>
            </div>

            <div className="grid grid-cols-1 md:grid-cols-2 gap-4 p-6">
              {/* Product Selector */}
              <div className="space-y-3 flex flex-col h-[350px]">
                <p className="text-xs font-semibold uppercase text-gray-400">Select Products</p>
                <div className="relative">
                  <Search className="absolute left-2.5 top-2 text-gray-500" size={14} />
                  <input 
                    type="text"
                    placeholder="Search product to add..."
                    value={orderSearch}
                    onChange={(e) => setOrderSearch(e.target.value)}
                    className="w-full bg-slate-950 border border-gray-800 rounded-lg pl-8 pr-3 py-1.5 text-xs text-gray-200 focus:outline-none focus:border-violet-500"
                  />
                </div>

                <div className="flex-1 overflow-y-auto space-y-2 pr-1">
                  {filteredProductsForOrder.map(product => (
                    <div key={product.id} className="p-2 bg-slate-950 border border-gray-850 rounded-lg flex justify-between items-center text-xs">
                      <div>
                        <p className="font-semibold text-white">{product.name}</p>
                        <p className="text-[10px] text-gray-500">SKU: {product.sku} | Stock: {product.stock_quantity}</p>
                      </div>
                      <div className="flex items-center space-x-2">
                        <span className="font-mono text-violet-400 font-bold">${product.sale_price.toFixed(2)}</span>
                        <button
                          onClick={() => addToCart(product, 1)}
                          className="bg-violet-600 hover:bg-violet-700 text-white rounded px-2 py-1 font-medium transition-colors"
                        >
                          Add
                        </button>
                      </div>
                    </div>
                  ))}
                </div>
              </div>

              {/* Order Cart */}
              <div className="space-y-3 flex flex-col h-[350px] border-l border-gray-800 pl-4">
                <p className="text-xs font-semibold uppercase text-gray-400">Order Cart</p>
                <div className="flex-1 overflow-y-auto space-y-2 pr-1">
                  {cart.length === 0 ? (
                    <p className="text-xs text-gray-500 text-center py-12">No items in cart.</p>
                  ) : (
                    cart.map(item => (
                      <div key={item.product.id} className="p-2 bg-slate-950 border border-gray-850 rounded-lg flex justify-between items-center text-xs">
                        <div>
                          <p className="font-semibold text-white">{item.product.name}</p>
                          <p className="text-[10px] text-gray-500">${item.product.sale_price.toFixed(2)} x {item.quantity}</p>
                        </div>
                        <button
                          onClick={() => item.product.id && removeFromCart(item.product.id)}
                          className="text-gray-500 hover:text-red-400 p-1"
                        >
                          <Trash2 size={12} />
                        </button>
                      </div>
                    ))
                  )}
                </div>

                {/* Subtotal */}
                <div className="border-t border-gray-800 pt-3 space-y-2">
                  <div className="flex justify-between items-center text-sm font-bold text-white">
                    <span>Total Amount:</span>
                    <span className="font-mono text-violet-400">${totalCartValue.toFixed(2)}</span>
                  </div>
                  <div className="flex space-x-2">
                    <button
                      onClick={clearCart}
                      className="flex-1 py-2 bg-slate-800 hover:bg-slate-700 text-gray-300 rounded-lg text-xs"
                    >
                      Clear
                    </button>
                    <button
                      onClick={handlePlaceOrder}
                      disabled={cart.length === 0}
                      className="flex-1 py-2 bg-violet-600 hover:bg-violet-700 text-white rounded-lg text-xs font-medium disabled:opacity-50"
                    >
                      Confirm Order
                    </button>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}
