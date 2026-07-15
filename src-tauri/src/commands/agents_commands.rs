//! Auto-extracted from commands/mod.rs during v0.27.0 refactor.
//! See worklog.md Task ID: refactor-1 for context.
//!
//! Behavior unchanged — only file structure modified.

use crate::catalog::{self, Product};
use crate::inventory::{self, InventorySummary, LowStockItem, DeadStockItem, BestSellerItem};
use crate::customers::{self, Customer, OrderItemInput, OrderHistory};
use crate::reports::{self, SalesReport, InventoryReport, CustomerSummaryReport};
use crate::agents::{self, AgentSummary, AgentLedgerEntry};
use crate::purchase_trips::{self, PurchaseTripSummary};
use crate::adapters::duckduckgo::{self, WebEvidence};
use crate::ai::{self, AiResponse};
use crate::utils;
use crate::commands::{DbState, set_setting_val, get_setting_val};
use tauri::async_runtime::Mutex;
use std::path::Path;
use rusqlite::{Connection, params};
use tauri::State;

// ============================================================
// // Agents: CRUD + ledger + stock transfer + sale + cash + balance
// ============================================================

// ============================================================
// v0.11.0 — Agents (replaces Locations as primary stock-movement entity)
// ============================================================

#[tauri::command]
pub async fn get_agents(state: State<'_, DbState>) -> Result<Vec<AgentSummary>, String> {
    let conn = state.0.lock().await;
    agents::get_all_agent_summaries(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_agent(
    state: State<'_, DbState>,
    name: String,
    phone: Option<String>,
    city: Option<String>,
    area: Option<String>,
    address_notes: Option<String>,
    notes: Option<String>,
) -> Result<i64, String> {
    let conn = state.0.lock().await;
    agents::add_agent(
        &conn, &name,
        phone.as_deref(), city.as_deref(), area.as_deref(),
        address_notes.as_deref(), notes.as_deref(),
    ).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_agent(
    state: State<'_, DbState>,
    id: i64,
    name: String,
    phone: Option<String>,
    city: Option<String>,
    area: Option<String>,
    address_notes: Option<String>,
    notes: Option<String>,
    is_active: bool,
) -> Result<(), String> {
    let conn = state.0.lock().await;
    agents::update_agent(
        &conn, id, &name,
        phone.as_deref(), city.as_deref(), area.as_deref(),
        address_notes.as_deref(), notes.as_deref(), is_active,
    ).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_agent(state: State<'_, DbState>, id: i64) -> Result<(), String> {
    let conn = state.0.lock().await;
    agents::delete_agent(&conn, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_agent_ledger(
    state: State<'_, DbState>,
    agent_id: i64,
    limit: Option<i64>,
) -> Result<Vec<AgentLedgerEntry>, String> {
    let conn = state.0.lock().await;
    let limit = limit.unwrap_or(50);
    agents::get_agent_ledger_entries(&conn, agent_id, limit).map_err(|e| e.to_string())
}

/// v0.14.10: Get current stock held by each agent for a specific product.
/// Used by the Catalog form's "Agent Stock (initial allocation)" section
/// so that editing a product shows the ACTUAL current allocation per
/// agent (not zeroed-out). Returns Vec of {agent_id, agent_name, quantity}.
#[tauri::command]
pub async fn get_product_agent_stock(
    state: State<'_, DbState>,
    product_id: i64,
) -> Result<Vec<serde_json::Value>, String> {
    let conn = state.0.lock().await;
    let rows = agents::get_product_stock_by_agents(&conn, product_id).map_err(|e| e.to_string())?;
    Ok(rows.into_iter().map(|(id, name, qty)| serde_json::json!({
        "agent_id": id,
        "agent_name": name,
        "quantity": qty,
    })).collect())
}

/// Send stock from Head Office to an agent.
/// Validates that the product has enough qty_in_head_office before sending.
#[tauri::command]
pub async fn send_stock_to_agent(
    state: State<'_, DbState>,
    agent_id: i64,
    product_id: i64,
    qty: i64,
    unit_price: f64,
    notes: Option<String>,
) -> Result<i64, String> {
    if qty <= 0 {
        return Err("Quantity must be positive.".to_string());
    }
    let conn = state.0.lock().await;
    // v0.14.4: Wrap the read-check-write in BEGIN IMMEDIATE / COMMIT.
    // The Mutex already serializes Tauri commands, but rusqlite's autocommit
    // mode means the read (ho_qty check) and the write (ledger insert +
    // product update) are separate transactions. If the app crashes between
    // them, the ledger entry is committed but the product's
    // qty_in_head_office is not decremented — leading to phantom stock.
    // BEGIN IMMEDIATE acquires a RESERVED lock at the start, ensuring the
    // read+check+write trio is atomic.
    conn.execute("BEGIN IMMEDIATE", []).map_err(|e| e.to_string())?;

    // Validate: product must have enough stock in Head Office.
    let ho_qty: i64 = conn.query_row(
        "SELECT COALESCE(qty_in_head_office, stock_quantity, 0) FROM products WHERE id = ?1",
        rusqlite::params![product_id],
        |r| r.get(0),
    ).map_err(|e| {
        let _ = conn.execute("ROLLBACK", []);
        format!("Product not found: {}", e)
    })?;
    if ho_qty < qty {
        let _ = conn.execute("ROLLBACK", []);
        return Err(format!(
            "Insufficient stock in Head Office. Available: {}, requested: {}.",
            ho_qty, qty
        ));
    }
    let result = agents::send_stock_to_agent(
        &conn, agent_id, product_id, qty, unit_price, notes.as_deref(),
    ).map_err(|e| {
        let _ = conn.execute("ROLLBACK", []);
        e.to_string()
    });
    conn.execute("COMMIT", []).map_err(|e| e.to_string())?;
    result
}

/// Return stock from an agent back to Head Office.
/// Validates that the agent has enough stock of this product.
#[tauri::command]
pub async fn return_stock_from_agent(
    state: State<'_, DbState>,
    agent_id: i64,
    product_id: i64,
    qty: i64,
    unit_price: f64,
    notes: Option<String>,
) -> Result<i64, String> {
    if qty <= 0 {
        return Err("Quantity must be positive.".to_string());
    }
    let conn = state.0.lock().await;
    // v0.14.4: Wrap in BEGIN IMMEDIATE / COMMIT (see send_stock_to_agent
    // comment for full rationale).
    conn.execute("BEGIN IMMEDIATE", []).map_err(|e| e.to_string())?;

    // Validate: agent must have enough stock of this product.
    let agent_qty: i64 = conn.query_row(
        "SELECT COALESCE(SUM(CASE WHEN entry_type = 'stock_sent' THEN qty ELSE 0 END) -
                          SUM(CASE WHEN entry_type = 'stock_returned' THEN qty ELSE 0 END) -
                          SUM(CASE WHEN entry_type = 'sale_reported' THEN qty ELSE 0 END), 0)
         FROM agent_ledger_entries WHERE agent_id = ?1 AND product_id = ?2",
        rusqlite::params![agent_id, product_id],
        |r| r.get(0),
    ).unwrap_or(0);
    if agent_qty < qty {
        let _ = conn.execute("ROLLBACK", []);
        return Err(format!(
            "Agent does not have enough stock of this product. Agent has: {}, requested: {}.",
            agent_qty, qty
        ));
    }
    let result = agents::return_stock_from_agent(
        &conn, agent_id, product_id, qty, unit_price, notes.as_deref(),
    ).map_err(|e| {
        let _ = conn.execute("ROLLBACK", []);
        e.to_string()
    });
    conn.execute("COMMIT", []).map_err(|e| e.to_string())?;
    result
}

/// Agent reports a sale (stock sold by agent to end customer).
#[tauri::command]
pub async fn report_agent_sale(
    state: State<'_, DbState>,
    agent_id: i64,
    product_id: i64,
    qty: i64,
    unit_price: f64,
    notes: Option<String>,
) -> Result<i64, String> {
    if qty <= 0 {
        return Err("Quantity must be positive.".to_string());
    }
    let conn = state.0.lock().await;
    agents::report_agent_sale(
        &conn, agent_id, product_id, qty, unit_price, notes.as_deref(),
    ).map_err(|e| e.to_string())
}

/// Record cash received from an agent.
#[tauri::command]
pub async fn receive_agent_cash(
    state: State<'_, DbState>,
    agent_id: i64,
    amount: f64,
    notes: Option<String>,
) -> Result<i64, String> {
    if amount <= 0.0 {
        return Err("Amount must be positive.".to_string());
    }
    let conn = state.0.lock().await;
    agents::receive_agent_cash(&conn, agent_id, amount, notes.as_deref())
        .map_err(|e| e.to_string())
}

/// Manual balance adjustment. Notes are MANDATORY.
#[tauri::command]
pub async fn adjust_agent_balance(
    state: State<'_, DbState>,
    agent_id: i64,
    amount: f64,
    notes: String,
) -> Result<i64, String> {
    if notes.trim().is_empty() {
        return Err("Notes are mandatory for balance adjustments.".to_string());
    }
    let conn = state.0.lock().await;
    agents::adjust_agent_balance(&conn, agent_id, amount, &notes)
        .map_err(|e| e.to_string())
}
