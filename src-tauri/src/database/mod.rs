use rusqlite::{Connection, Result};
use std::path::Path;
use std::fs;

pub fn init_db<P: AsRef<Path>>(db_path: P) -> Result<Connection> {
    if let Some(parent) = db_path.as_ref().parent() {
        fs::create_dir_all(parent).unwrap_or_default();
    }

    let mut conn = Connection::open(db_path)?;
    run_migrations(&mut conn)?;
    Ok(conn)
}

fn run_migrations(conn: &mut Connection) -> Result<()> {
    conn.execute("PRAGMA foreign_keys = ON;", [])?;

    // Existing tables (from before)
    conn.execute("CREATE TABLE IF NOT EXISTS products (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        sku TEXT NOT NULL UNIQUE,
        name TEXT NOT NULL,
        category TEXT,
        cost_price REAL NOT NULL,
        sale_price REAL NOT NULL,
        description TEXT,
        tags TEXT,
        stock_quantity INTEGER NOT NULL DEFAULT 0,
        status TEXT NOT NULL DEFAULT 'active',
        images TEXT NOT NULL DEFAULT '[]',
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL
    );", [])?;

    conn.execute("CREATE TABLE IF NOT EXISTS customers (
        id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL,
        phone TEXT, location TEXT, notes TEXT, created_at TEXT NOT NULL
    );", [])?;

    conn.execute("CREATE TABLE IF NOT EXISTS orders (
        id INTEGER PRIMARY KEY AUTOINCREMENT, customer_id INTEGER NOT NULL,
        total_amount REAL NOT NULL, profit REAL NOT NULL, order_date TEXT NOT NULL,
        FOREIGN KEY(customer_id) REFERENCES customers(id) ON DELETE CASCADE
    );", [])?;

    conn.execute("CREATE TABLE IF NOT EXISTS order_items (
        id INTEGER PRIMARY KEY AUTOINCREMENT, order_id INTEGER NOT NULL,
        product_id INTEGER NOT NULL, quantity INTEGER NOT NULL,
        sale_price REAL NOT NULL, cost_price REAL NOT NULL,
        FOREIGN KEY(order_id) REFERENCES orders(id) ON DELETE CASCADE,
        FOREIGN KEY(product_id) REFERENCES products(id) ON DELETE RESTRICT
    );", [])?;

    conn.execute("CREATE TABLE IF NOT EXISTS social_posts (
        id INTEGER PRIMARY KEY AUTOINCREMENT, product_id INTEGER,
        platform TEXT NOT NULL, content TEXT NOT NULL, scheduled_time TEXT,
        status TEXT NOT NULL DEFAULT 'draft', post_url TEXT, created_at TEXT NOT NULL,
        FOREIGN KEY(product_id) REFERENCES products(id) ON DELETE SET NULL
    );", [])?;

    conn.execute("CREATE TABLE IF NOT EXISTS automations (
        id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL UNIQUE,
        schedule_type TEXT NOT NULL, last_run TEXT, active INTEGER NOT NULL DEFAULT 1
    );", [])?;

    conn.execute("CREATE TABLE IF NOT EXISTS ai_logs (
        id INTEGER PRIMARY KEY AUTOINCREMENT, prompt TEXT NOT NULL,
        response TEXT NOT NULL, provider TEXT NOT NULL, created_at TEXT NOT NULL
    );", [])?;

    conn.execute("CREATE TABLE IF NOT EXISTS ai_knowledge (
        id INTEGER PRIMARY KEY AUTOINCREMENT, topic TEXT NOT NULL,
        content TEXT NOT NULL, source TEXT NOT NULL DEFAULT 'manual',
        created_at TEXT NOT NULL, updated_at TEXT NOT NULL
    );", [])?;

    conn.execute("CREATE TABLE IF NOT EXISTS business_memory (
        id INTEGER PRIMARY KEY AUTOINCREMENT, category TEXT NOT NULL,
        insight TEXT NOT NULL, confidence REAL NOT NULL DEFAULT 1.0,
        created_at TEXT NOT NULL, last_used_at TEXT NOT NULL,
        usage_count INTEGER NOT NULL DEFAULT 1
    );", [])?;

    conn.execute("CREATE TABLE IF NOT EXISTS settings (
        key TEXT PRIMARY KEY, value TEXT NOT NULL
    );", [])?;

    // === NEW TABLES (v0.3.0) ===

    conn.execute("CREATE TABLE IF NOT EXISTS locations (
        id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL UNIQUE,
        address TEXT, is_active INTEGER NOT NULL DEFAULT 1, created_at TEXT NOT NULL
    );", [])?;

    conn.execute("CREATE TABLE IF NOT EXISTS suppliers (
        id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL,
        contact TEXT, city TEXT, notes TEXT, created_at TEXT NOT NULL
    );", [])?;

    conn.execute("CREATE TABLE IF NOT EXISTS product_locations (
        id INTEGER PRIMARY KEY AUTOINCREMENT, product_id INTEGER NOT NULL,
        location_id INTEGER NOT NULL, quantity INTEGER NOT NULL DEFAULT 0,
        FOREIGN KEY(product_id) REFERENCES products(id) ON DELETE CASCADE,
        FOREIGN KEY(location_id) REFERENCES locations(id) ON DELETE CASCADE,
        UNIQUE(product_id, location_id)
    );", [])?;

    // === MIGRATIONS for existing columns ===
    add_col_if_missing(conn, "products", "product_code", "TEXT DEFAULT ''")?;
    add_col_if_missing(conn, "products", "color", "TEXT DEFAULT ''")?;
    add_col_if_missing(conn, "products", "design", "TEXT DEFAULT ''")?;
    add_col_if_missing(conn, "products", "season", "TEXT DEFAULT ''")?;
    add_col_if_missing(conn, "products", "supplier_id", "INTEGER DEFAULT NULL")?;
    add_col_if_missing(conn, "products", "purchase_price", "REAL DEFAULT 0.0")?;

    seed_initial_data(conn)?;
    ensure_business_profile(conn)?;
    seed_locations(conn)?;
    Ok(())
}

fn add_col_if_missing(conn: &Connection, table: &str, col: &str, col_def: &str) -> Result<()> {
    let exists: i64 = conn.query_row(
        &format!("SELECT COUNT(*) FROM pragma_table_info('{}') WHERE name='{}'", table, col),
        [],
        |r| r.get(0),
    ).unwrap_or(0);
    if exists == 0 {
        conn.execute(&format!("ALTER TABLE {} ADD COLUMN {} {}", table, col, col_def), [])?;
    }
    Ok(())
}

const DEFAULT_PROFILE: &str = r#"{
  "business_name": "A Collection",
  "industry": "Ladies & Gents Clothing Retail",
  "currency": "PKR",
  "owner": "Ali",
  "purchase_city": "Faisalabad",
  "facebook_page": "https://www.facebook.com/profile.php?id=61589997236061",
  "whatsapp_channel": "https://whatsapp.com/channel/0029VbCcUycLNSaChZf2WJ2H",
  "whatsapp_number": "+923420830995",
  "sales_areas": ["Narowal", "Shakargarh", "Zafarwal", "Nearby Villages"],
  "sales_channels": ["Facebook", "WhatsApp", "Door To Door"],
  "collections": ["Summer", "Winter", "Eid Special", "Festive"],
  "target_customers": {
    "gender": "Female & Male",
    "income_group": "Middle Income",
    "preferred_products": [
      "3 Piece Suits", "2 Piece Suits", "Lawn", "Cotton",
      "Printed Designs", "Embroidery", "Cut Piece",
      "Gents Cotton", "Gents Washing Wear"
    ]
  },
  "business_goals": [
    "Increase Profit", "Increase Sales", "Reduce Dead Stock",
    "Improve Customer Retention", "Improve Marketing",
    "Auto-generate Social Media Posts"
  ],
  "assistant_roles": [
    "Inventory Manager", "Sales Analyst", "Marketing Assistant",
    "Business Advisor", "Purchase Planner", "Social Media Manager",
    "Product Photographer Assistant"
  ]
}"#;

fn ensure_business_profile(conn: &Connection) -> Result<()> {
    let val: Result<String, _> = conn.query_row(
        "SELECT value FROM settings WHERE key = 'business_profile'", [], |row| row.get(0),
    );
    match val {
        Ok(existing) => {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&existing) {
                if json.get("currency").is_none() || json.get("facebook_page").is_none() {
                    conn.execute("UPDATE settings SET value = ?1 WHERE key = 'business_profile'", [DEFAULT_PROFILE])?;
                }
            }
        }
        Err(_) => {
            conn.execute("INSERT OR IGNORE INTO settings (key, value) VALUES ('business_profile', ?1)", [DEFAULT_PROFILE])?;
        }
    }
    Ok(())
}

fn seed_locations(conn: &Connection) -> Result<()> {
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM locations", [], |r| r.get(0)).unwrap_or(0);
    if count == 0 {
        let now = chrono::Utc::now().to_rfc3339();
        let locs = [("Head Office", "Main Office"), ("Shakargarh Shop", "Shakargarh City")];
        for (name, addr) in &locs {
            conn.execute("INSERT INTO locations (name, address, created_at) VALUES (?1, ?2, ?3)", rusqlite::params![name, addr, &now])?;
        }
    }
    Ok(())
}

fn seed_initial_data(conn: &mut Connection) -> Result<()> {
    let settings_count: i64 = conn.query_row("SELECT COUNT(*) FROM settings", [], |r| r.get(0))?;
    if settings_count == 0 {
        let default_settings = [
            ("theme", "dark"), ("ai_provider", "gemini"), ("ai_api_key", ""),
            ("ai_model", "gemini-1.5-flash"), ("backup_path", ""), ("backup_interval_days", "7"),
            ("business_profile", DEFAULT_PROFILE),
        ];
        for (k, v) in default_settings.iter() {
            conn.execute("INSERT INTO settings (key, value) VALUES (?1, ?2);", [k, v])?;
        }
    }

    let aut_count: i64 = conn.query_row("SELECT COUNT(*) FROM automations", [], |r| r.get(0))?;
    if aut_count == 0 {
        let automations = [("Database Backup", "daily"), ("Weekly Performance Report", "weekly"),
            ("Low Stock Reminder", "daily"), ("Dead Stock Audit", "monthly")];
        for (name, sched) in automations.iter() {
            conn.execute("INSERT INTO automations (name, schedule_type, active) VALUES (?1, ?2, 1);", [name, sched])?;
        }
    }

    let prod_count: i64 = conn.query_row("SELECT COUNT(*) FROM products", [], |r| r.get(0))?;
    if prod_count == 0 {
        let now = chrono::Utc::now().to_rfc3339();
        let dummy = [
            ("AC-2026-001", "Designer Linen Kurta", "3 Piece", 12.50, 29.99, "Premium linen kurta", "linen,kurta", 45, "active", "[]", "Bottle Green", "Embroidered", "summer", ""),
            ("AC-2026-002", "Casual Cotton Shirt", "Gents Cotton", 8.00, 19.99, "Classic fit cotton shirt", "cotton,shirt", 12, "active", "[]", "White", "Plain", "summer", ""),
            ("AC-2026-003", "Slim Fit Denim", "Gents Washing", 15.00, 39.99, "Blue denim jeans", "denim,pants", 5, "active", "[]", "Blue", "Slim Fit", "winter", ""),
            ("AC-2026-004", "Embroidered Lawn Suit", "3 Piece Lawn", 22.00, 49.99, "Beautiful embroidered lawn suit", "lawn,embroidered", 2, "active", "[]", "Red", "Digital Print", "summer", ""),
        ];
        for (code, name, cat, cost, sale, desc, tags, qty, status, img, color, design, season, _) in dummy.iter() {
            conn.execute(
                "INSERT INTO products (sku, product_code, name, category, color, design, season, cost_price, sale_price, purchase_price, description, tags, stock_quantity, status, images, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?8, ?10, ?11, ?12, ?13, ?14, ?15, ?15)",
                (code, code, name, cat, color, design, season, cost, sale, desc, tags, qty, status, img, &now),
            )?;
        }
    }

    let cust_count: i64 = conn.query_row("SELECT COUNT(*) FROM customers", [], |r| r.get(0))?;
    if cust_count == 0 {
        let now = chrono::Utc::now().to_rfc3339();
        let customers = [
            ("Ahmad Khan", "+923001234567", "Narowal, Pakistan", "Regular customer, prefers medium sizes."),
            ("Sara Ahmed", "+923219876543", "Shakargarh, Pakistan", "Interested in lawn collections."),
        ];
        for (name, phone, loc, notes) in customers.iter() {
            conn.execute("INSERT INTO customers (name, phone, location, notes, created_at) VALUES (?1, ?2, ?3, ?4, ?5);", (name, phone, loc, notes, &now))?;
        }
    }

    Ok(())
}
