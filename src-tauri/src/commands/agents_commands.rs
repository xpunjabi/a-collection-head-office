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

// ============================================================
// v0.29.0: MANUAL AGENT LEDGER ENTRIES + EDIT/DELETE
// ============================================================
//
// Bhai ki need: "abhi sirf 'receive cash' entry kar sakte hain. Maal
// bheja entry chaahiye (even if maal catalog mein nahi hai). Kitna advance
// tha, kitne ka maal chala gaya — direct amount entry ka save/edit/delete."
//
// Solution: Expose 3 thin wrapper commands around existing functions in
// agents/mod.rs:
//   - add_agent_manual_entry: creates a balance_adjustment entry with
//     arbitrary amount + notes. Used for:
//       * Maal value (jo catalog mein nahi) — amount = maal ki qeemat
//       * Advance payment (cash given to agent) — amount = -advance
//       * Any other correction
//   - update_agent_ledger_entry: thin wrapper around agents::update_ledger_entry
//   - delete_agent_ledger_entry: thin wrapper around agents::delete_ledger_entry
//     + recalculates product stock if entry was stock-related.
//
// Existing receive_agent_cash + adjust_agent_balance are kept for backward
// compatibility. New UI uses add_agent_manual_entry for flexibility.

/// Add a manual ledger entry for an agent. Useful when:
/// - Maal bheja but catalog mein product nahi hai (amount = maal value)
/// - Cash advance diya agent ko (amount = -advance, reduces agent's debt to HO)
/// - Any other correction
///
/// Uses entry_type='balance_adjustment' under the hood (existing schema).
/// Notes are mandatory for audit trail.
#[tauri::command]
pub async fn add_agent_manual_entry(
    state: State<'_, DbState>,
    agent_id: i64,
    amount: f64,
    notes: String,
    entry_date: Option<String>,
) -> Result<i64, String> {
    if notes.trim().is_empty() {
        return Err("Notes are mandatory for manual ledger entries.".to_string());
    }
    if amount == 0.0 {
        return Err("Amount cannot be zero.".to_string());
    }
    let conn = state.0.lock().await;

    // Validate agent exists
    let _: i64 = conn.query_row(
        "SELECT id FROM agents WHERE id = ?1",
        rusqlite::params![agent_id],
        |r| r.get(0),
    ).map_err(|e| format!("Agent not found: {}", e))?;

    // Use existing append_ledger_entry with entry_type='balance_adjustment'.
    // The function stores -amount (because adjust_agent_balance uses this
    // convention: positive input = agent owes more, stored as negative).
    // We replicate that here for consistency.
    let stored_amount = -amount;
    let entry_id = if let Some(date) = entry_date {
        // Custom date — use direct INSERT to override entry_date
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO agent_ledger_entries (agent_id, product_id, entry_type, qty, unit_price, amount, reference_code, notes, entry_date, created_at, updated_at)
             VALUES (?1, NULL, 'balance_adjustment', 0, 0.0, ?2, '', ?3, ?4, ?5, ?5)",
            rusqlite::params![agent_id, stored_amount, &notes, &date, &now],
        ).map_err(|e| format!("Failed to insert entry: {}", e))?;
        conn.last_insert_rowid()
    } else {
        agents::append_ledger_entry(
            &conn, agent_id, None, "balance_adjustment", 0, 0.0, stored_amount,
            None, Some(&notes),
        ).map_err(|e| format!("Failed to insert entry: {}", e))?
    };

    Ok(entry_id)
}

/// Update an existing agent ledger entry's amount + notes.
/// v0.31.0: ALL entry types are now editable (was: balance_adjustment only).
/// Bhai ki feedback: "1 lakh wasool entry edit/delete nahi ho rahi" — restriction
/// hata diya. For stock entries (stock_sent/stock_returned/sale_reported),
/// product stock is recalculated after edit via recalc_product_stock_from_ledger.
///
/// Amount sign convention (preserved per entry_type):
/// - cash_received: amount stored as +amount (positive cash received)
/// - balance_adjustment: amount stored as -amount (negative of user input)
/// - stock_*: amount = qty * unit_price (positive)
/// For simplicity, frontend passes the DISPLAY amount (signed by user);
/// we preserve the existing sign convention by reading current stored sign.
#[tauri::command]
pub async fn update_agent_ledger_entry(
    state: State<'_, DbState>,
    entry_id: i64,
    amount: f64,
    notes: String,
) -> Result<(), String> {
    if notes.trim().is_empty() {
        return Err("Notes are mandatory.".to_string());
    }
    if amount == 0.0 {
        return Err("Amount cannot be zero.".to_string());
    }
    let conn = state.0.lock().await;
    let now = chrono::Utc::now().to_rfc3339();

    // Fetch existing entry — v0.31.0: removed restriction to balance_adjustment only
    let (entry_type, product_id, current_amount): (String, Option<i64>, f64) = conn.query_row(
        "SELECT entry_type, product_id, amount FROM agent_ledger_entries WHERE id = ?1",
        rusqlite::params![entry_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    ).map_err(|e| format!("Entry not found: {}", e))?;

    // v0.31.0: Preserve sign convention for the entry_type being edited.
    // - For cash_received + stock_*: amount is stored as the user enters it (positive).
    // - For balance_adjustment: amount is stored as -amount (negative of user input).
    // Frontend sends DISPLAY amount (already sign-flipped for balance_adjustment),
    // so we re-flip it before storing.
    let stored_amount = if entry_type == "balance_adjustment" {
        -amount
    } else {
        // For cash_received + stock entries: preserve sign of original entry.
        // If original was negative (refund-like), keep negative. If positive, keep positive.
        if current_amount < 0.0 { -amount.abs() } else { amount.abs() }
    };

    conn.execute(
        "UPDATE agent_ledger_entries SET amount = ?1, notes = ?2, updated_at = ?3 WHERE id = ?4",
        rusqlite::params![stored_amount, &notes, &now, entry_id],
    ).map_err(|e| format!("Failed to update entry: {}", e))?;

    // v0.31.0: If entry was stock-related, recalc product stock
    if let Some(pid) = product_id {
        if entry_type == "stock_sent" || entry_type == "stock_returned" || entry_type == "sale_reported" {
            if let Err(e) = agents::recalc_product_stock_from_ledger(&conn, pid) {
                // Don't fail the whole operation — log warning but proceed
                eprintln!("Warning: failed to recalc product {} stock after entry edit: {}", pid, e);
            }
        }
    }

    Ok(())
}

/// Delete an agent ledger entry. v0.31.0: ALL entry types are now deletable.
/// For stock entries, product stock is recalculated after delete.
#[tauri::command]
pub async fn delete_agent_ledger_entry(
    state: State<'_, DbState>,
    entry_id: i64,
) -> Result<(), String> {
    let conn = state.0.lock().await;

    // Fetch entry — v0.31.0: removed restriction to balance_adjustment only
    let (entry_type, product_id): (String, Option<i64>) = conn.query_row(
        "SELECT entry_type, product_id FROM agent_ledger_entries WHERE id = ?1",
        rusqlite::params![entry_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    ).map_err(|e| format!("Entry not found: {}", e))?;

    agents::delete_ledger_entry(&conn, entry_id)
        .map_err(|e| format!("Failed to delete entry: {}", e))?;

    // v0.31.0: If entry was stock-related, recalc product stock
    if let Some(pid) = product_id {
        if entry_type == "stock_sent" || entry_type == "stock_returned" || entry_type == "sale_reported" {
            if let Err(e) = agents::recalc_product_stock_from_ledger(&conn, pid) {
                eprintln!("Warning: failed to recalc product {} stock after entry delete: {}", pid, e);
            }
        }
    }

    Ok(())
}
