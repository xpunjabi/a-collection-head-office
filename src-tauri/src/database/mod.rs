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
    // Enable foreign keys
    conn.execute("PRAGMA foreign_keys = ON;", [])?;

    // Products table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS products (
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
        );",
        [],
    )?;

    // Customers table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS customers (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            phone TEXT,
            location TEXT,
            notes TEXT,
            created_at TEXT NOT NULL
        );",
        [],
    )?;

    // Orders table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS orders (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            customer_id INTEGER NOT NULL,
            total_amount REAL NOT NULL,
            profit REAL NOT NULL,
            order_date TEXT NOT NULL,
            FOREIGN KEY(customer_id) REFERENCES customers(id) ON DELETE CASCADE
        );",
        [],
    )?;

    // Order Items table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS order_items (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            order_id INTEGER NOT NULL,
            product_id INTEGER NOT NULL,
            quantity INTEGER NOT NULL,
            sale_price REAL NOT NULL,
            cost_price REAL NOT NULL,
            FOREIGN KEY(order_id) REFERENCES orders(id) ON DELETE CASCADE,
            FOREIGN KEY(product_id) REFERENCES products(id) ON DELETE RESTRICT
        );",
        [],
    )?;

    // Social Posts table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS social_posts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            product_id INTEGER,
            platform TEXT NOT NULL,
            content TEXT NOT NULL,
            scheduled_time TEXT,
            status TEXT NOT NULL DEFAULT 'draft',
            post_url TEXT,
            created_at TEXT NOT NULL,
            FOREIGN KEY(product_id) REFERENCES products(id) ON DELETE SET NULL
        );",
        [],
    )?;

    // Automations table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS automations (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            schedule_type TEXT NOT NULL,
            last_run TEXT,
            active INTEGER NOT NULL DEFAULT 1
        );",
        [],
    )?;

    // AI Logs table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS ai_logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            prompt TEXT NOT NULL,
            response TEXT NOT NULL,
            provider TEXT NOT NULL,
            created_at TEXT NOT NULL
        );",
        [],
    )?;

    // Settings table (key-value store for preferences)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );",
        [],
    )?;

    seed_initial_data(conn)?;

    Ok(())
}

fn seed_initial_data(conn: &mut Connection) -> Result<()> {
    // Seed default settings if not exists
    let settings_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM settings",
        [],
        |row| row.get(0),
    )?;

    if settings_count == 0 {
        let default_settings = [
            ("theme", "dark"),
            ("ai_provider", "gemini"),
            ("ai_api_key", ""),
            ("ai_model", "gemini-1.5-flash"),
            ("backup_path", ""),
            ("backup_interval_days", "7")
        ];

        for (k, v) in default_settings.iter() {
            conn.execute(
                "INSERT INTO settings (key, value) VALUES (?1, ?2);",
                [k, v],
            )?;
        }
    }

    // Seed default automations if not exists
    let aut_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM automations",
        [],
        |row| row.get(0),
    )?;

    if aut_count == 0 {
        let automations = [
            ("Database Backup", "daily"),
            ("Weekly Performance Report", "weekly"),
            ("Low Stock Reminder", "daily"),
            ("Dead Stock Audit", "monthly")
        ];

        for (name, sched) in automations.iter() {
            conn.execute(
                "INSERT INTO automations (name, schedule_type, active) VALUES (?1, ?2, 1);",
                [name, sched],
            )?;
        }
    }

    // Seed dummy product data for preview if table is empty
    let prod_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM products",
        [],
        |row| row.get(0),
    )?;

    if prod_count == 0 {
        let dummy_products = [
            ("SKU-KURTA-001", "Designer Linen Kurta", "Kurta", 12.50, 29.99, "Embroidered designer kurta in premium linen.", "linen,kurta,summer", 45, "active", "[]"),
            ("SKU-SHIRT-002", "Casual Cotton Shirt", "Shirt", 8.00, 19.99, "Classic fit casual cotton shirt for daily wear.", "cotton,shirt,casual", 12, "active", "[]"),
            ("SKU-TROUS-003", "Slim Fit Denim", "Trouser", 15.00, 39.99, "Stretchable slim fit blue denim jeans.", "denim,pants,casual", 5, "active", "[]"),
            ("SKU-JACKET-004", "Leather Bomber Jacket", "Jacket", 45.00, 99.99, "Premium faux leather black bomber jacket.", "leather,jacket,winter", 2, "active", "[]")
        ];

        let now = chrono::Utc::now().to_rfc3339();
        for (sku, name, cat, cost, sale, desc, tags, qty, status, img) in dummy_products.iter() {
            conn.execute(
                "INSERT INTO products (sku, name, category, cost_price, sale_price, description, tags, stock_quantity, status, images, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12);",
                (sku, name, cat, cost, sale, desc, tags, qty, status, img, &now, &now),
            )?;
        }
    }

    // Seed dummy customer if table is empty
    let cust_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM customers",
        [],
        |row| row.get(0),
    )?;

    if cust_count == 0 {
        let dummy_customers = [
            ("Ahmad Khan", "+923001234567", "Lahore, Pakistan", "Regular customer, prefers medium sizes."),
            ("Sara Ahmed", "+923219876543", "Karachi, Pakistan", "Interested in formal wear and lawn collections.")
        ];
        let now = chrono::Utc::now().to_rfc3339();
        for (name, phone, loc, notes) in dummy_customers.iter() {
            conn.execute(
                "INSERT INTO customers (name, phone, location, notes, created_at) VALUES (?1, ?2, ?3, ?4, ?5);",
                (name, phone, loc, notes, &now),
            )?;
        }
    }

    Ok(())
}
