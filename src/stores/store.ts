import { create } from 'zustand'
import { invoke } from '@tauri-apps/api/core'

export interface Product {
  id?: number;
  sku: string;
  name: string;
  category?: string;
  color?: string;
  design?: string;
  season?: string;
  cost_price: number;
  sale_price: number;
  purchase_price: number;
  description?: string;
  tags?: string;
  stock_quantity: number;
  status: string;
  images: string;
  supplier_id?: number;
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

export interface ProductDraft {
  name?: string;
  sku?: string;
  category?: string;
  brand?: string;
  fabric?: string;
  color?: string;
  design?: string;
  season?: string;
  cost_price?: number;
  sale_price?: number;
  retail_price?: number;
  description?: string;
  tags?: string[];
  keywords?: string[];
  hashtags?: string[];
  images?: string[];
}

export interface LocalMatchResult {
  item_id: string;
  title: string;
  design_code?: string;
  confidence: number;
}

export interface CatalogDraft {
  title: string;
  brand?: string;
  fabric?: string;
  design_code?: string;
  notes?: string;
  web_evidence_count?: number;
  web_evidence_snippets?: string[];
  best_image_url?: string;
}

export type AssistantResult =
  | { type: "LocalMatchFound"; data: LocalMatchResult }
  | { type: "NewCatalogDraft"; data: CatalogDraft };

export interface AiResponse {
  text: string;
  detected_action?: string;
  action_data?: any;
  product_draft?: ProductDraft;
  confidence?: number;
  missing_fields?: string[];
  suggested_actions?: string[];
  fast_path_data?: AssistantResult;
}

export interface MarketingContent {
  platform: string;
  content: string;
  caption_type: string;
}

export interface MarketingPost {
  short_caption: string;
  long_caption: string;
  hashtags: string[];
}

export interface WorkspaceAsset {
  id: string;
  name: string;
  path?: string;
  data?: string;
  mime: string;
  type: 'image' | 'document' | 'link';
  source_url?: string;
}

interface AppState {
  // Navigation & UI
  currentTab: string;
  setCurrentTab: (tab: string) => void;
  showAiAssistant: boolean;
  setVectorAssistant: (show: boolean) => void;
  aiWorkspaceWidth: number;
  setAiWorkspaceWidth: (width: number) => void;

  // Products
  products: Product[];
  isLoadingProducts: boolean;
  fetchProducts: () => Promise<void>;
  addProduct: (product: Product) => Promise<number>;
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

  // AI Product Drafts
  aiProductDrafts: { draft: ProductDraft; confidence: number; missingFields: string[]; suggestedActions: string[] }[];
  addAiProductDraft: (draft: ProductDraft, confidence: number, missingFields: string[], suggestedActions: string[]) => void;
  removeAiProductDraft: (index: number) => void;
  updateAiProductDraft: (index: number, draft: ProductDraft) => void;
  addDraftToCatalog: (draft: ProductDraft) => Promise<number>;
  marketingForProduct: (productId: number) => Promise<void>;

  // Workspace assets
  workspaceAssets: WorkspaceAsset[];
  addWorkspaceAsset: (asset: WorkspaceAsset) => void;
  removeWorkspaceAsset: (id: string) => void;
  clearWorkspaceAssets: () => void;

  // AI Assistant Chat
  aiMessages: { role: 'user' | 'assistant'; text: string; action?: string; product_draft?: ProductDraft; confidence?: number; missing_fields?: string[]; suggested_actions?: string[]; fast_path_data?: AssistantResult; social_post?: MarketingPost; image_data?: string }[];
  isAiLoading: boolean;
  sendAiMessage: (prompt: string, imageData?: string) => Promise<void>;
  clearAiChat: () => void;
  removeAiMessage: (index: number) => void;
}

export const useAppStore = create<AppState>((set, get) => ({
  // Navigation & UI Defaults
  currentTab: 'dashboard',
  setCurrentTab: (tab) => set({ currentTab: tab }),
  showAiAssistant: true,
  setVectorAssistant: (show) => set({ showAiAssistant: show }),
  aiWorkspaceWidth: 35,
  setAiWorkspaceWidth: (width) => set({ aiWorkspaceWidth: width }),

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
      const id: number = await invoke('add_product', { product });
      await get().fetchProducts();
      return id;
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

  // AI Product Drafts
  aiProductDrafts: [],
  addAiProductDraft: (draft, confidence, missingFields, suggestedActions) => set((state) => {
    // Deduplicate by SKU (preferred) or name (fallback). If a draft with the
    // same SKU already exists in the in-memory list, replace it instead of
    // pushing a duplicate. This is a defensive layer — the backend
    // `ask_ai` command in v0.9.5+ should already short-circuit duplicate
    // drafts, but this guard prevents any future regression from surfacing
    // as a duplicate UI card.
    const sku = (draft.sku || '').trim().toLowerCase();
    const name = (draft.name || '').trim().toLowerCase();
    const key = sku || name;
    let list = [...state.aiProductDrafts];
    if (key) {
      list = list.filter((d) => {
        const dSku = (d.draft.sku || '').trim().toLowerCase();
        const dName = (d.draft.name || '').trim().toLowerCase();
        return (dSku || dName) !== key;
      });
    }
    list.push({ draft, confidence, missingFields, suggestedActions });
    return { aiProductDrafts: list };
  }),
  removeAiProductDraft: (index) => set((state) => ({
    aiProductDrafts: state.aiProductDrafts.filter((_, i) => i !== index),
  })),
  updateAiProductDraft: (index, draft) => set((state) => ({
    aiProductDrafts: state.aiProductDrafts.map((d, i) => i === index ? { ...d, draft } : d),
  })),
  addDraftToCatalog: async (draft) => {
    try {
      const id: number = await invoke('save_product_draft_to_catalog', { draft });
      await get().fetchProducts();
      // Auto-generate marketing content
      try {
        await get().marketingForProduct(id);
      } catch {}
      return id;
    } catch (err) {
      throw new Error(String(err));
    }
  },
  marketingForProduct: async (productId) => {
    try {
      await invoke<MarketingContent[]>('generate_marketing', { productId });
    } catch (err) {
      console.error('Marketing generation failed:', err);
    }
  },

  // Workspace assets
  workspaceAssets: [],
  addWorkspaceAsset: (asset) => set((state) => ({
    workspaceAssets: [...state.workspaceAssets, asset],
  })),
  removeWorkspaceAsset: (id) => set((state) => ({
    workspaceAssets: state.workspaceAssets.filter((a) => a.id !== id),
  })),
  clearWorkspaceAssets: () => set({ workspaceAssets: [] }),

  // AI Assistant Chat
  aiMessages: [
    {
      role: 'assistant',
      text: '👋 Hello! I am your AI Business Assistant.\n\nI can help you with:\n• **Product Intake** — Drop an image, link, or paste product code\n• **Catalog Management** — Create and edit product drafts\n• **Marketing Content** — Generate posts for Facebook, WhatsApp, Instagram\n• **Inventory Insights** — Check stock, low items, dead stock\n• **Business Advice** — Purchasing decisions, sales analysis\n\nTry sending a product image or paste a product link!',
    },
  ],
  isAiLoading: false,
  sendAiMessage: async (prompt, imageData) => {
    if (!prompt.trim() && !imageData) return;

    // Add user message to chat
    const userMsg: any = { role: 'user', text: prompt || (imageData ? '[Image uploaded]' : ''), image_data: imageData || undefined };
    set((state) => ({
      aiMessages: [...state.aiMessages, userMsg],
      isAiLoading: true,
    }));

    try {
      const response: AiResponse = await invoke('ask_ai', { prompt, imageData: imageData || null });

      // If we sent an image and got a product draft, save the image locally
      if (imageData && response.product_draft) {
        try {
          const savedName = await invoke<string>('save_base64_image', {
            base64Data: imageData,
            formatType: 'thumbnail',
          })
          response.product_draft.images = [savedName]
        } catch (err) {
          console.error('Failed to save image from AI:', err)
        }
      }

      const assistantMsg: any = {
        role: 'assistant',
        text: response.text,
        action: response.detected_action,
        product_draft: response.product_draft,
        confidence: response.confidence,
        missing_fields: response.missing_fields,
        suggested_actions: response.suggested_actions,
        fast_path_data: response.fast_path_data,
        image_data: imageData || undefined,
      };

      set((state) => ({
        aiMessages: [...state.aiMessages, assistantMsg],
        isAiLoading: false,
      }));

      // Add draft to drafts store if detected
      if (response.product_draft && response.confidence) {
        get().addAiProductDraft(
          response.product_draft,
          response.confidence,
          response.missing_fields || [],
          response.suggested_actions || []
        );
      }

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
          text: 'Chat history cleared. I\'m ready to help with product intake, marketing, or business questions.',
        },
      ],
    }),
  removeAiMessage: (index) => set((state) => ({
    aiMessages: state.aiMessages.filter((_, i) => i !== index),
  })),
}));
