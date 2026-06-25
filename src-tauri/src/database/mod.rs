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
    run_migrations_impl(conn)
}

/// Public wrapper around run_migrations so the `init_database` Tauri command
/// can trigger a re-sync of sales_areas -> locations without an app restart.
pub fn run_migrations_public(conn: &mut Connection) -> Result<()> {
    run_migrations_impl(conn)
}

fn run_migrations_impl(conn: &mut Connection) -> Result<()> {
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

    // === NEW TABLES (v0.4.0 — AI Workspace) ===
    conn.execute("CREATE TABLE IF NOT EXISTS product_drafts (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        source_type TEXT NOT NULL DEFAULT 'manual',
        source_data TEXT,
        draft_data TEXT NOT NULL,
        confidence REAL DEFAULT 0.0,
        missing_fields TEXT DEFAULT '[]',
        status TEXT NOT NULL DEFAULT 'draft',
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL
    );", [])?;

    conn.execute("CREATE TABLE IF NOT EXISTS media_assets (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        original_name TEXT NOT NULL,
        stored_path TEXT NOT NULL,
        mime_type TEXT NOT NULL,
        file_size INTEGER NOT NULL DEFAULT 0,
        source_url TEXT,
        analysis_result TEXT,
        draft_id INTEGER,
        product_id INTEGER,
        created_at TEXT NOT NULL,
        FOREIGN KEY(draft_id) REFERENCES product_drafts(id) ON DELETE SET NULL,
        FOREIGN KEY(product_id) REFERENCES products(id) ON DELETE SET NULL
    );", [])?;

    add_col_if_missing(conn, "social_posts", "caption_type", "TEXT DEFAULT 'general'")?;
    add_col_if_missing(conn, "social_posts", "media_path", "TEXT")?;
    add_col_if_missing(conn, "social_posts", "draft_id", "INTEGER")?;
    // Issue #5 fix: store per-platform hashtags as JSON array string
    add_col_if_missing(conn, "social_posts", "hashtags", "TEXT")?;

    // === OLD NEW TABLES (v0.3.0) ===

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

    // ============================================================
    // v0.11.0 — Profit-Mode Refactor: Data Model Foundation
    // ============================================================
    // New tables for the profit-first operating system. Additive only —
    // no existing tables dropped, no existing data touched.

    // --- purchase_trips: Faisalabad buying trips with landed cost ---
    conn.execute("CREATE TABLE IF NOT EXISTS purchase_trips (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        trip_code TEXT NOT NULL UNIQUE,
        trip_date TEXT NOT NULL,
        source_city TEXT NOT NULL DEFAULT 'Faisalabad',
        supplier_notes TEXT,
        travel_cost REAL NOT NULL DEFAULT 0.0,
        transport_cost REAL NOT NULL DEFAULT 0.0,
        food_cost REAL NOT NULL DEFAULT 0.0,
        loading_cost REAL NOT NULL DEFAULT 0.0,
        misc_cost REAL NOT NULL DEFAULT 0.0,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL
    );", [])?;

    // --- purchase_trip_items: items purchased on a trip, with cost allocation ---
    conn.execute("CREATE TABLE IF NOT EXISTS purchase_trip_items (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        trip_id INTEGER NOT NULL,
        product_id INTEGER,
        qty_purchased INTEGER NOT NULL DEFAULT 0,
        unit_purchase_cost REAL NOT NULL DEFAULT 0.0,
        total_purchase_cost REAL NOT NULL DEFAULT 0.0,
        expense_allocation_amount REAL NOT NULL DEFAULT 0.0,
        landed_unit_cost REAL NOT NULL DEFAULT 0.0,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        FOREIGN KEY(trip_id) REFERENCES purchase_trips(id) ON DELETE CASCADE,
        FOREIGN KEY(product_id) REFERENCES products(id) ON DELETE SET NULL
    );", [])?;

    // --- agents: replaces locations concept (person + place unified) ---
    // One agent = one person at a place. Existing locations data is migrated
    // to agents by sync_locations_to_agents() below.
    conn.execute("CREATE TABLE IF NOT EXISTS agents (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        agent_code TEXT NOT NULL UNIQUE,
        name TEXT NOT NULL,
        phone TEXT,
        city TEXT,
        area TEXT,
        address_notes TEXT,
        notes TEXT,
        is_active INTEGER NOT NULL DEFAULT 1,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL
    );", [])?;

    // --- agent_ledger_entries: unified stock + cash movement log ---
    // This is THE single source of truth for agent stock and money flow.
    // entry_type enum: stock_sent | stock_returned | sale_reported |
    //                  cash_received | balance_adjustment
    conn.execute("CREATE TABLE IF NOT EXISTS agent_ledger_entries (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        agent_id INTEGER NOT NULL,
        product_id INTEGER,
        entry_type TEXT NOT NULL,
        qty INTEGER NOT NULL DEFAULT 0,
        unit_price REAL NOT NULL DEFAULT 0.0,
        amount REAL NOT NULL DEFAULT 0.0,
        reference_code TEXT,
        notes TEXT,
        entry_date TEXT NOT NULL,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        FOREIGN KEY(agent_id) REFERENCES agents(id) ON DELETE CASCADE,
        FOREIGN KEY(product_id) REFERENCES products(id) ON DELETE SET NULL
    );", [])?;

    // --- share_logs: social sharing audit trail ---
    // platform enum: whatsapp_status | whatsapp_direct | facebook |
    //                instagram | tiktok
    conn.execute("CREATE TABLE IF NOT EXISTS share_logs (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        product_id INTEGER,
        platform TEXT NOT NULL,
        share_angle TEXT,
        caption_text TEXT,
        shared_by TEXT,
        shared_at TEXT NOT NULL,
        notes TEXT,
        FOREIGN KEY(product_id) REFERENCES products(id) ON DELETE SET NULL
    );", [])?;

    // --- sales: replaces orders table (single sales concept) ---
    // sale_channel enum: head_office | whatsapp | facebook | instagram |
    //                    tiktok | agent
    conn.execute("CREATE TABLE IF NOT EXISTS sales (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        product_id INTEGER NOT NULL,
        sale_channel TEXT NOT NULL DEFAULT 'head_office',
        sale_type TEXT,
        agent_id INTEGER,
        qty INTEGER NOT NULL DEFAULT 1,
        unit_sale_price REAL NOT NULL DEFAULT 0.0,
        total_sale_amount REAL NOT NULL DEFAULT 0.0,
        customer_name TEXT,
        customer_phone TEXT,
        notes TEXT,
        sale_date TEXT NOT NULL,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        FOREIGN KEY(product_id) REFERENCES products(id) ON DELETE RESTRICT,
        FOREIGN KEY(agent_id) REFERENCES agents(id) ON DELETE SET NULL
    );", [])?;

    // --- products table: additive column extensions for profit-mode ---
    // Existing columns (sku, name, cost_price, sale_price, etc.) are kept
    // untouched. These new columns enable profit-mode features.
    add_col_if_missing(conn, "products", "source_trip_id", "INTEGER DEFAULT NULL")?;
    add_col_if_missing(conn, "products", "base_unit_cost", "REAL DEFAULT 0.0")?;
    add_col_if_missing(conn, "products", "landed_unit_cost", "REAL DEFAULT 0.0")?;
    add_col_if_missing(conn, "products", "retail_price", "REAL")?;
    add_col_if_missing(conn, "products", "discount_price", "REAL")?;
    add_col_if_missing(conn, "products", "size_info", "TEXT")?;
    add_col_if_missing(conn, "products", "brand", "TEXT")?;
    add_col_if_missing(conn, "products", "fabric", "TEXT")?;
    add_col_if_missing(conn, "products", "qty_with_agents", "INTEGER DEFAULT 0")?;
    add_col_if_missing(conn, "products", "qty_sold", "INTEGER DEFAULT 0")?;
    add_col_if_missing(conn, "products", "qty_reserved", "INTEGER DEFAULT 0")?;
    add_col_if_missing(conn, "products", "profit_status", "TEXT DEFAULT 'in_head_office'")?;
    // qty_in_head_office mirrors the legacy stock_quantity column but with
    // a clearer name in the profit-mode context. Backfilled from
    // stock_quantity on first migration; thereafter maintained by the
    // agent ledger functions (send_stock_to_agent, return_stock_from_agent).
    add_col_if_missing(conn, "products", "qty_in_head_office", "INTEGER DEFAULT 0")?;
    // Backfill qty_in_head_office from existing stock_quantity for legacy products.
    let _ = conn.execute(
        "UPDATE products SET qty_in_head_office = stock_quantity WHERE qty_in_head_office = 0 AND stock_quantity > 0",
        [],
    );
    // Backfill retail_price from existing sale_price for legacy products.
    let _ = conn.execute(
        "UPDATE products SET retail_price = sale_price WHERE retail_price IS NULL AND sale_price IS NOT NULL",
        [],
    );
    // Backfill base_unit_cost from existing purchase_price (or cost_price) for legacy products.
    let _ = conn.execute(
        "UPDATE products SET base_unit_cost = COALESCE(purchase_price, cost_price, 0.0) WHERE base_unit_cost = 0.0",
        [],
    );

    seed_initial_data(conn)?;
    ensure_business_profile(conn)?;
    // v0.12.7: locations table aur uske saare sync functions REMOVED.
    // Locations table DB mein exist karti hai (taake purana code break na
    // ho), lekin hum usme koi naya data insert nahi karte. Agents table
    // ab single source of truth hai.
    // seed_locations(conn)?;           // REMOVED — naye locations nahi banenge
    // sync_sales_areas_to_locations(conn)?;  // REMOVED
    // sync_locations_to_agents(conn)?;       // REMOVED
    // Sirf ek baar cleanup chalao taake purane duplicates (agar koi bache
    // hain) remove ho jayein.
    cleanup_duplicate_agents(conn)?;

    // ============================================================
    // v0.11.1 — Share Center enhancements
    // ============================================================
    // Add segment column to customers for bulk WhatsApp broadcasting.
    // segment values: 'women', 'girls', 'vip', 'agent', 'general', etc.
    // User-defined — stored as free-form text so they can create custom
    // segments beyond the defaults.
    add_col_if_missing(conn, "customers", "segment", "TEXT DEFAULT 'general'")?;
    add_col_if_missing(conn, "customers", "is_active", "INTEGER NOT NULL DEFAULT 1")?;
    // v0.12.6: Clean up duplicate agents (same name, different agent_code)
    // that were created before the name-check fix in sync_locations_to_agents.
    cleanup_duplicate_agents(conn)?;
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
        // Issue #6 fix: derive seed locations from DEFAULT_PROFILE.sales_areas
        // rather than hardcoding a separate 2-entry list. The business profile
        // already lists 4 sales_areas (Narowal, Shakargarh, Zafarwal, Nearby
        // Villages), and those should be the initial Location entries.
        if let Ok(profile) = serde_json::from_str::<serde_json::Value>(DEFAULT_PROFILE) {
            if let Some(areas) = profile["sales_areas"].as_array() {
                for area in areas {
                    if let Some(name) = area.as_str() {
                        let _ = conn.execute(
                            "INSERT OR IGNORE INTO locations (name, address, created_at) VALUES (?1, ?2, ?3)",
                            rusqlite::params![name, "", &now],
                        );
                    }
                }
                // Also add a "Head Office" entry as the operational base.
                let _ = conn.execute(
                    "INSERT OR IGNORE INTO locations (name, address, created_at) VALUES (?1, ?2, ?3)",
                    rusqlite::params!["Head Office", "Main Office", &now],
                );
                return Ok(());
            }
        }
        // Fallback if profile parsing failed for any reason
        let locs = [("Head Office", "Main Office"), ("Shakargarh Shop", "Shakargarh City")];
        for (name, addr) in &locs {
            conn.execute("INSERT INTO locations (name, address, created_at) VALUES (?1, ?2, ?3)", rusqlite::params![name, addr, &now])?;
        }
    }
    Ok(())
}

/// Issue #6 fix: one-time migration that syncs `business_profile.sales_areas`
/// into the `locations` table. Any sales_area name not already in `locations`
/// is inserted (preserving existing rows). This runs on every startup via
/// run_migrations but only inserts missing rows — idempotent.
fn sync_sales_areas_to_locations(conn: &Connection) -> Result<()> {
    let profile_val: Result<String, _> = conn.query_row(
        "SELECT value FROM settings WHERE key = 'business_profile'", [], |row| row.get(0),
    );
    let profile_str = match profile_val {
        Ok(s) => s,
        Err(_) => return Ok(()), // no business_profile setting yet — skip
    };
    let profile: serde_json::Value = match serde_json::from_str(&profile_str) {
        Ok(v) => v,
        Err(_) => return Ok(()),
    };
    let areas = match profile["sales_areas"].as_array() {
        Some(a) => a,
        None => return Ok(()),
    };
    let now = chrono::Utc::now().to_rfc3339();
    for area in areas {
        if let Some(name) = area.as_str() {
            // INSERT OR IGNORE relies on the UNIQUE constraint on locations.name
            // to skip rows that already exist. This makes the sync fully
            // idempotent — safe to run on every startup.
            let _ = conn.execute(
                "INSERT OR IGNORE INTO locations (name, address, created_at) VALUES (?1, ?2, ?3)",
                rusqlite::params![name, "", &now],
            );
        }
    }
    Ok(())
}

/// v0.11.0: One-time (idempotent) migration that creates an agent entry for
/// each existing location. This bridges the old locations table to the new
/// agents table so users don't lose their existing location data.
///
/// Each location becomes an agent with:
///   - name = location name (e.g., "Narowal", "Shakargarh")
///   - agent_code = "LOC-<id>" (derived from location id, stable)
///   - city = location name
///   - is_active = same as location.is_active
///
/// Safe to run on every startup — INSERT OR IGNORE skips agents that already
/// exist (matched by agent_code).
fn sync_locations_to_agents(conn: &Connection) -> Result<()> {
    let now = chrono::Utc::now().to_rfc3339();
    // Select all locations and insert a matching agent for each (if not exists).
    let mut stmt = match conn.prepare("SELECT id, name, address, is_active FROM locations") {
        Ok(s) => s,
        Err(_) => return Ok(()), // locations table might not exist yet — skip
    };
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, i64>(3)?,
        ))
    });
    let rows = match rows {
        Ok(r) => r,
        Err(_) => return Ok(()),
    };
    for row in rows {
        if let Ok((loc_id, name, address, is_active)) = row {
            // v0.12.6 fix: Check if an agent with the same name already
            // exists BEFORE inserting. Previously, INSERT OR IGNORE only
            // checked agent_code uniqueness — so if the user manually added
            // an agent named "Narowal" (agent_code "AGT-XXX"), and a
            // location named "Narowal" also existed (agent_code "LOC-Y"),
            // both would be inserted → duplicate agents by name.
            let existing_count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM agents WHERE LOWER(name) = LOWER(?1)",
                rusqlite::params![&name],
                |r| r.get(0),
            ).unwrap_or(0);
            if existing_count > 0 {
                // Agent with same name already exists — skip to avoid duplicate
                continue;
            }
            let agent_code = format!("LOC-{}", loc_id);
            let _ = conn.execute(
                "INSERT OR IGNORE INTO agents (agent_code, name, phone, city, area, address_notes, notes, is_active, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                rusqlite::params![
                    &agent_code,
                    &name,
                    "", // phone — empty, user can fill later
                    &name, // city = location name
                    "", // area — empty
                    address.unwrap_or_default(),
                    "Migrated from locations table",
                    is_active,
                    &now,
                    &now,
                ],
            );
        }
    }
    Ok(())
}

/// v0.12.6: Clean up duplicate agents that have the same name (case-insensitive).
/// Keeps the agent with the LOWEST id (first created), deletes the rest.
/// Ledger entries cascade-delete with the agent (ON DELETE CASCADE).
///
/// This runs on every startup but is a no-op if no duplicates exist.
fn cleanup_duplicate_agents(conn: &Connection) -> Result<()> {
    // Find all agent names that appear more than once (case-insensitive)
    let mut stmt = conn.prepare(
        "SELECT LOWER(name) AS lname, COUNT(*) as cnt
         FROM agents
         GROUP BY LOWER(name)
         HAVING cnt > 1"
    )?;
    let dupes: Vec<String> = stmt.query_map([], |row| {
        row.get::<_, String>(0)
    })?.filter_map(|r| r.ok()).collect();

    for lname in dupes {
        // Delete all agents with this name EXCEPT the one with the lowest id
        // (the first-created one). Ledger entries cascade-delete.
        conn.execute(
            "DELETE FROM agents WHERE LOWER(name) = ?1 AND id NOT IN (
                SELECT MIN(id) FROM agents WHERE LOWER(name) = ?1
            )",
            rusqlite::params![&lname],
        )?;
    }

    Ok(())
}

fn seed_initial_data(conn: &mut Connection) -> Result<()> {
    let settings_count: i64 = conn.query_row("SELECT COUNT(*) FROM settings", [], |r| r.get(0))?;
    if settings_count == 0 {
        let default_settings = [
            ("theme", "dark"), ("ai_provider", "gemini"), ("ai_api_key", ""),
            ("ai_model", "gemini-2.0-flash"), ("backup_path", ""), ("backup_interval_days", "7"),
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
