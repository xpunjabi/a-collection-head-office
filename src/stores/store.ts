import { create } from 'zustand'
import { invoke } from '@tauri-apps/api/core'

export interface Product {
  id?: number;
  sku: string;
  name: string;
  category?: string;
  cost_price: number;
  sale_price: number;
  description?: string;
  tags?: string;
  stock_quantity: number;
  status: string;
  images: string; // JSON array string
  created_at?: string;
  updated_at?: string;
}

export interface Customer {
  id?: number;
  name: string;
  phone?: string;
  location?: string;
  notes?: string;
  created_at?: string;
}

export interface OrderItemInput {
  product_id: number;
  quantity: number;
}

export interface OrderItemDetail {
  product_name: string;
  sku: string;
  quantity: number;
  sale_price: number;
}

export interface OrderHistory {
  order_id: number;
  order_date: string;
  total_amount: number;
  profit: number;
  items: OrderItemDetail[];
}

export interface AiResponse {
  text: string;
  detected_action?: string;
  action_data?: any;
}

interface AppState {
  // Navigation & UI
  currentTab: string;
  setCurrentTab: (tab: string) => void;
  showAiAssistant: boolean;
  setVectorAssistant: (show: boolean) => void;

  // Products
  products: Product[];
  isLoadingProducts: boolean;
  fetchProducts: () => Promise<void>;
  addProduct: (product: Product) => Promise<void>;
  updateProduct: (product: Product) => Promise<void>;
  deleteProduct: (id: number) => Promise<void>;
  exportProductsCsv: () => Promise<string>;
  importProductsCsv: (csvContent: string) => Promise<void>;
  uploadProductImage: (srcPath: string, formatType: string) => Promise<string>;

  // Customers
  customers: Customer[];
  isLoadingCustomers: boolean;
  fetchCustomers: () => Promise<void>;
  addCustomer: (customer: Customer) => Promise<void>;
  updateCustomer: (customer: Customer) => Promise<void>;
  deleteCustomer: (id: number) => Promise<void>;
  createOrder: (customerId: number, items: OrderItemInput[]) => Promise<number>;
  getCustomerHistory: (customerId: number) => Promise<OrderHistory[]>;

  // Cart (for placing orders)
  cart: { product: Product; quantity: number }[];
  addToCart: (product: Product, quantity: number) => void;
  removeFromCart: (productId: number) => void;
  clearCart: () => void;

  // Settings
  settings: Record<string, string>;
  fetchSettings: () => Promise<void>;
  updateSetting: (key: string, value: string) => Promise<void>;
  backupDatabaseNow: () => Promise<string>;

  // AI Assistant Chat
  aiMessages: { role: 'user' | 'assistant'; text: string; action?: string }[];
  isAiLoading: boolean;
  sendAiMessage: (prompt: string) => Promise<void>;
  clearAiChat: () => void;
}

export const useAppStore = create<AppState>((set, get) => ({
  // Navigation & UI Defaults
  currentTab: 'dashboard',
  setCurrentTab: (tab) => set({ currentTab: tab }),
  showAiAssistant: true,
  setVectorAssistant: (show) => set({ showAiAssistant: show }),

  // Products
  products: [],
  isLoadingProducts: false,
  fetchProducts: async () => {
    set({ isLoadingProducts: true });
    try {
      const products: Product[] = await invoke('get_products');
      set({ products, isLoadingProducts: false });
    } catch (err) {
      console.error(err);
      set({ isLoadingProducts: false });
    }
  },
  addProduct: async (product) => {
    try {
      await invoke('add_product', { product });
      await get().fetchProducts();
    } catch (err) {
      throw new Error(String(err));
    }
  },
  updateProduct: async (product) => {
    try {
      await invoke('update_product', { product });
      await get().fetchProducts();
    } catch (err) {
      throw new Error(String(err));
    }
  },
  deleteProduct: async (id) => {
    try {
      await invoke('delete_product', { id });
      await get().fetchProducts();
    } catch (err) {
      throw new Error(String(err));
    }
  },
  exportProductsCsv: async () => {
    try {
      return await invoke<string>('export_products_csv');
    } catch (err) {
      throw new Error(String(err));
    }
  },
  importProductsCsv: async (csvContent) => {
    try {
      await invoke('import_products_csv', { csvContent });
      await get().fetchProducts();
    } catch (err) {
      throw new Error(String(err));
    }
  },
  uploadProductImage: async (srcPath, formatType) => {
    try {
      return await invoke<string>('upload_product_image', { srcPath, formatType });
    } catch (err) {
      throw new Error(String(err));
    }
  },

  // Customers
  customers: [],
  isLoadingCustomers: false,
  fetchCustomers: async () => {
    set({ isLoadingCustomers: true });
    try {
      const customers: Customer[] = await invoke('get_customers');
      set({ customers, isLoadingCustomers: false });
    } catch (err) {
      console.error(err);
      set({ isLoadingCustomers: false });
    }
  },
  addCustomer: async (customer) => {
    try {
      await invoke('add_customer', { customer });
      await get().fetchCustomers();
    } catch (err) {
      throw new Error(String(err));
    }
  },
  updateCustomer: async (customer) => {
    try {
      await invoke('update_customer', { customer });
      await get().fetchCustomers();
    } catch (err) {
      throw new Error(String(err));
    }
  },
  deleteCustomer: async (id) => {
    try {
      await invoke('delete_customer', { id });
      await get().fetchCustomers();
    } catch (err) {
      throw new Error(String(err));
    }
  },
  createOrder: async (customerId, items) => {
    try {
      const orderId = await invoke<number>('create_order', { customerId, items });
      await get().fetchProducts(); // Refresh stock
      return orderId;
    } catch (err) {
      throw new Error(String(err));
    }
  },
  getCustomerHistory: async (customerId) => {
    try {
      return await invoke<OrderHistory[]>('get_customer_history', { customerId });
    } catch (err) {
      throw new Error(String(err));
    }
  },

  // Cart
  cart: [],
  addToCart: (product, quantity) => {
    const cart = get().cart;
    const existing = cart.find((item) => item.product.id === product.id);
    if (existing) {
      set({
        cart: cart.map((item) =>
          item.product.id === product.id
            ? { ...item, quantity: item.quantity + quantity }
            : item
        ),
      });
    } else {
      set({ cart: [...cart, { product, quantity }] });
    }
  },
  removeFromCart: (productId) => {
    set({ cart: get().cart.filter((item) => item.product.id !== productId) });
  },
  clearCart: () => set({ cart: [] }),

  // Settings
  settings: {},
  fetchSettings: async () => {
    try {
      const settings: Record<string, string> = await invoke('get_settings');
      set({ settings });
    } catch (err) {
      console.error(err);
    }
  },
  updateSetting: async (key, value) => {
    try {
      await invoke('update_setting', { key, value });
      await get().fetchSettings();
    } catch (err) {
      throw new Error(String(err));
    }
  },
  backupDatabaseNow: async () => {
    try {
      return await invoke<string>('backup_database_now');
    } catch (err) {
      throw new Error(String(err));
    }
  },

  // AI Assistant Chat
  aiMessages: [
    {
      role: 'assistant',
      text: 'Hello! I am your AI Business Assistant. How can I help you manage your clothing shop today? Ask me to: \n- "Show low stock items"\n- "Generate Facebook post for product"\n- "Suggest promotional campaigns"',
    },
  ],
  isAiLoading: false,
  sendAiMessage: async (prompt) => {
    if (!prompt.trim()) return;

    // Add user message to chat
    set((state) => ({
      aiMessages: [...state.aiMessages, { role: 'user', text: prompt }],
      isAiLoading: true,
    }));

    try {
      const response: AiResponse = await invoke('ask_ai', { prompt });

      // Add assistant response
      set((state) => ({
        aiMessages: [
          ...state.aiMessages,
          {
            role: 'assistant',
            text: response.text,
            action: response.detected_action,
          },
        ],
        isAiLoading: false,
      }));

      // If a local action occurred that modified data or requires data reload, trigger refreshes
      if (response.detected_action === 'low_stock') {
        await get().fetchProducts();
      }
    } catch (err) {
      set((state) => ({
        aiMessages: [
          ...state.aiMessages,
          {
            role: 'assistant',
            text: `Error: ${String(err)}`,
          },
        ],
        isAiLoading: false,
      }));
    }
  },
  clearAiChat: () =>
    set({
      aiMessages: [
        {
          role: 'assistant',
          text: 'Chat history cleared. How can I help you now?',
        },
      ],
    }),
}));
