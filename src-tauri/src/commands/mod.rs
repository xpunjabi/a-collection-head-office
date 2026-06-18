use crate::catalog::{self, Product, ProductLocationStock};
use crate::inventory::{self, InventorySummary, LowStockItem, DeadStockItem, BestSellerItem};
use crate::customers::{self, Customer, OrderItemInput, OrderHistory};
use crate::reports::{self, SalesReport, InventoryReport, CustomerSummaryReport};
use crate::locations::{self, Location};
use crate::ai::{self, AiResponse, KnowledgeEntry};
use crate::utils;
use std::sync::Mutex;
use std::path::Path;
use rusqlite::Connection;
use tauri::State;

pub struct DbState(pub Mutex<Connection>);

fn set_setting_val(conn: &Connection, key: &str, value: &str) -> Result<(), rusqlite::Error> {
    conn.execute("INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2);", [key, value])?;
    Ok(())
}

fn get_setting_val(conn: &Connection, key: &str) -> Result<String, rusqlite::Error> {
    conn.query_row("SELECT value FROM settings WHERE key = ?1", [key], |row| row.get(0))
}

// ==================== CATALOG ====================

#[tauri::command]
pub async fn get_products(state: State<'_, DbState>) -> Result<Vec<Product>, String> {
    let conn = state.0.lock().unwrap();
    catalog::get_all_products(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_product(state: State<'_, DbState>, id: i64) -> Result<Product, String> {
    let conn = state.0.lock().unwrap();
    catalog::get_product_by_id(&conn, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_product(state: State<'_, DbState>, product: Product) -> Result<i64, String> {
    let conn = state.0.lock().unwrap();
    catalog::add_product(&conn, &product).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_product(state: State<'_, DbState>, product: Product) -> Result<(), String> {
    let conn = state.0.lock().unwrap();
    catalog::update_product(&conn, &product).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_product(state: State<'_, DbState>, id: i64) -> Result<(), String> {
    let conn = state.0.lock().unwrap();
    catalog::delete_product(&conn, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_product_locations(state: State<'_, DbState>, product_id: i64) -> Result<Vec<ProductLocationStock>, String> {
    let conn = state.0.lock().unwrap();
    catalog::get_product_locations(&conn, product_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn upsert_product_location(state: State<'_, DbState>, product_id: i64, location_id: i64, quantity: i64) -> Result<(), String> {
    let conn = state.0.lock().unwrap();
    catalog::upsert_product_location(&conn, product_id, location_id, quantity).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn search_products_by_color(state: State<'_, DbState>, color: String) -> Result<Vec<Product>, String> {
    let conn = state.0.lock().unwrap();
    catalog::search_by_color(&conn, &color).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn export_products_csv(state: State<'_, DbState>) -> Result<String, String> {
    let conn = state.0.lock().unwrap();
    catalog::export_to_csv(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn import_products_csv(state: State<'_, DbState>, csv_content: String) -> Result<(), String> {
    let conn = state.0.lock().unwrap();
    catalog::import_from_csv(&conn, &csv_content).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn upload_product_image(src_path: String, format_type: String) -> Result<String, String> {
    let src = Path::new(&src_path);
    if !src.exists() {
        return Err("Source image file does not exist.".to_string());
    }
    let images_dir = utils::get_images_dir();
    catalog::process_and_save_image(src, &images_dir, &format_type).map_err(|e| e.to_string())
}

// ==================== LOCATIONS ====================

#[tauri::command]
pub async fn get_locations(state: State<'_, DbState>) -> Result<Vec<Location>, String> {
    let conn = state.0.lock().unwrap();
    locations::get_all_locations(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_location(state: State<'_, DbState>, name: String, address: String) -> Result<i64, String> {
    let conn = state.0.lock().unwrap();
    locations::add_location(&conn, &name, &address).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_location(state: State<'_, DbState>, id: i64, name: String, address: String, is_active: bool) -> Result<(), String> {
    let conn = state.0.lock().unwrap();
    locations::update_location(&conn, id, &name, &address, is_active).map_err(|e| e.to_string())
}

// ==================== INVENTORY ====================

#[tauri::command]
pub async fn get_inventory_summary(state: State<'_, DbState>) -> Result<InventorySummary, String> {
    let conn = state.0.lock().unwrap();
    inventory::get_inventory_summary(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_low_stock(state: State<'_, DbState>, threshold: i64) -> Result<Vec<LowStockItem>, String> {
    let conn = state.0.lock().unwrap();
    inventory::get_low_stock_items(&conn, threshold).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_dead_stock(state: State<'_, DbState>, days_limit: i64) -> Result<Vec<DeadStockItem>, String> {
    let conn = state.0.lock().unwrap();
    inventory::get_dead_stock_items(&conn, days_limit).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_best_sellers(state: State<'_, DbState>, limit: i64) -> Result<Vec<BestSellerItem>, String> {
    let conn = state.0.lock().unwrap();
    inventory::get_best_sellers(&conn, limit).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn adjust_stock(state: State<'_, DbState>, product_id: i64, adjustment: i64) -> Result<(), String> {
    let conn = state.0.lock().unwrap();
    inventory::adjust_stock(&conn, product_id, adjustment).map_err(|e| e.to_string())
}

// ==================== CUSTOMERS ====================

#[tauri::command]
pub async fn get_customers(state: State<'_, DbState>) -> Result<Vec<Customer>, String> {
    let conn = state.0.lock().unwrap();
    customers::get_all_customers(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_customer(state: State<'_, DbState>, customer: Customer) -> Result<i64, String> {
    let conn = state.0.lock().unwrap();
    customers::add_customer(&conn, &customer).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_customer(state: State<'_, DbState>, customer: Customer) -> Result<(), String> {
    let conn = state.0.lock().unwrap();
    customers::update_customer(&conn, &customer).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_customer(state: State<'_, DbState>, id: i64) -> Result<(), String> {
    let conn = state.0.lock().unwrap();
    customers::delete_customer(&conn, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_order(state: State<'_, DbState>, customer_id: i64, items: Vec<OrderItemInput>) -> Result<i64, String> {
    let mut conn = state.0.lock().unwrap();
    customers::create_order(&mut conn, customer_id, items).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_customer_history(state: State<'_, DbState>, customer_id: i64) -> Result<Vec<OrderHistory>, String> {
    let conn = state.0.lock().unwrap();
    customers::get_customer_purchase_history(&conn, customer_id).map_err(|e| e.to_string())
}

// ==================== REPORTS ====================

#[tauri::command]
pub async fn get_sales_report(state: State<'_, DbState>, start_date: String, end_date: String) -> Result<SalesReport, String> {
    let conn = state.0.lock().unwrap();
    reports::generate_sales_report(&conn, &start_date, &end_date).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_inventory_report(state: State<'_, DbState>) -> Result<InventoryReport, String> {
    let conn = state.0.lock().unwrap();
    reports::generate_inventory_report(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_customer_report(state: State<'_, DbState>) -> Result<CustomerSummaryReport, String> {
    let conn = state.0.lock().unwrap();
    reports::generate_customer_report(&conn).map_err(|e| e.to_string())
}

// ==================== AI ====================

#[tauri::command]
pub async fn ask_ai(state: State<'_, DbState>, prompt: String) -> Result<AiResponse, String> {
    let local_result = {
        let conn = state.0.lock().unwrap();
        ai::try_local_intent(&conn, &prompt)
    };
    if let Some(response) = local_result { return Ok(response); }

    let (provider, api_key, model) = {
        let conn = state.0.lock().unwrap();
        ai::get_ai_config(&conn)?
    };
    if api_key.is_empty() && provider != "local" {
        return Err("AI API key is missing. Please configure it in Settings.".to_string());
    }

    let (system_prompt, _) = {
        let conn = state.0.lock().unwrap();
        (ai::build_system_prompt(&conn, &prompt)?, ())
    };

    let response_text = ai::call_ai_provider(&provider, &api_key, &model, &system_prompt, &prompt).await?;
    {
        let conn = state.0.lock().unwrap();
        ai::log_request(&conn, &prompt, &response_text, &provider)?;
    }
    Ok(AiResponse { text: response_text, detected_action: None, action_data: None })
}

#[tauri::command]
pub async fn get_knowledge(state: State<'_, DbState>) -> Result<Vec<KnowledgeEntry>, String> {
    let conn = state.0.lock().unwrap();
    ai::get_all_knowledge(&conn)
}

#[tauri::command]
pub async fn save_knowledge(state: State<'_, DbState>, topic: String, content: String, source: String) -> Result<(), String> {
    let conn = state.0.lock().unwrap();
    ai::save_knowledge(&conn, &topic, &content, &source)
}

#[tauri::command]
pub async fn delete_knowledge(state: State<'_, DbState>, id: i64) -> Result<(), String> {
    let conn = state.0.lock().unwrap();
    ai::delete_knowledge(&conn, id)
}

// ==================== SETTINGS ====================

#[tauri::command]
pub async fn get_settings(state: State<'_, DbState>) -> Result<std::collections::HashMap<String, String>, String> {
    let conn = state.0.lock().unwrap();
    let mut stmt = conn.prepare("SELECT key, value FROM settings").map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |row| {
        let k: String = row.get(0)?;
        let v: String = row.get(1)?;
        Ok((k, v))
    }).map_err(|e| e.to_string())?;
    let mut map = std::collections::HashMap::new();
    for row in rows { if let Ok((k, v)) = row { map.insert(k, v); } }
    Ok(map)
}

#[tauri::command]
pub async fn update_setting(state: State<'_, DbState>, key: String, value: String) -> Result<(), String> {
    let conn = state.0.lock().unwrap();
    set_setting_val(&conn, &key, &value).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn backup_database_now(state: State<'_, DbState>) -> Result<String, String> {
    let conn = state.0.lock().unwrap();
    let backup_path = get_setting_val(&conn, "backup_path").map_err(|e| e.to_string())?;
    if backup_path.is_empty() { return Err("Backup path is not configured.".to_string()); }
    let backup_dir = Path::new(&backup_path);
    if !backup_dir.exists() { return Err("Backup path does not exist.".to_string()); }
    let db_src = utils::get_db_path();
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let dest = backup_dir.join(format!("manual_backup_{}.db", timestamp));
    std::fs::copy(db_src, &dest).map_err(|e| format!("Failed to copy: {}", e))?;
    Ok(dest.to_string_lossy().to_string())
}
