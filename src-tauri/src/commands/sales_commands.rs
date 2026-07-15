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
// // Sales: record_sale
// ============================================================

// ============================================================
// v0.12.5 — Sales Recording (Head Office records ALL sales)
// ============================================================

/// Record a sale. Works for both direct HO sales AND agent walk-in sales.
/// If agent_id is provided, it's an agent sale (reduces agent stock).
/// If agent_id is None, it's a direct HO sale (reduces HO stock).
///
/// Auto-updates:
/// - sales table entry created
/// - product stock reduced (HO or agent depending on sale type)
/// - product.qty_sold increased
/// - product.profit_status auto-recalculated
#[tauri::command]
pub async fn record_sale(
    state: State<'_, DbState>,
    product_id: i64,
    qty: i64,
    unit_sale_price: f64,
    sale_channel: String,
    agent_id: Option<i64>,
    customer_name: Option<String>,
    customer_phone: Option<String>,
    notes: Option<String>,
    amount_paid: Option<f64>,
    customer_id: Option<i64>,
) -> Result<i64, String> {
    if qty <= 0 {
        return Err("Quantity must be positive.".to_string());
    }
    let conn = state.0.lock().await;
    let now = chrono::Utc::now().to_rfc3339();
    let total = qty as f64 * unit_sale_price;

    // v0.14.4: Wrap the entire sale-recording flow in BEGIN IMMEDIATE / COMMIT.
    // record_sale writes to 2-3 tables (agent_ledger_entries, products,
    // sales) and does a stock-availability check before mutating. Without
    // a transaction, a crash between any two of these writes leaves the DB
    // inconsistent — e.g., the sales row exists but the product's
    // qty_sold was never incremented, or the ledger entry exists but the
    // sales row doesn't. BEGIN IMMEDIATE acquires a RESERVED lock so the
    // whole "check stock → insert ledger → update product → insert sale →
    // update profit_status" sequence is atomic.
    conn.execute("BEGIN IMMEDIATE", []).map_err(|e| e.to_string())?;

    // Helper closure to rollback on error and convert rusqlite::Error to String.
    // Used for the early-return paths below.
    macro_rules! try_or_rollback {
        ($expr:expr) => {
            match $expr {
                Ok(v) => v,
                Err(e) => {
                    let _ = conn.execute("ROLLBACK", []);
                    return Err(e);
                }
            }
        };
    }

    // If agent_id is provided, record as agent sale (reduces agent stock)
    if let Some(aid) = agent_id {
        // Validate agent has enough stock of this product
        let agent_qty: i64 = conn.query_row(
            "SELECT COALESCE(SUM(CASE WHEN entry_type = 'stock_sent' THEN qty ELSE 0 END) -
                              SUM(CASE WHEN entry_type = 'stock_returned' THEN qty ELSE 0 END) -
                              SUM(CASE WHEN entry_type = 'sale_reported' THEN qty ELSE 0 END), 0)
             FROM agent_ledger_entries WHERE agent_id = ?1 AND product_id = ?2",
            rusqlite::params![aid, product_id],
            |r| r.get(0),
        ).unwrap_or(0);
        if agent_qty < qty {
            let _ = conn.execute("ROLLBACK", []);
            return Err(format!(
                "Agent does not have enough stock. Agent has: {}, requested: {}.",
                agent_qty, qty
            ));
        }
        // Create agent ledger entry for the sale
        let amount = qty as f64 * unit_sale_price;
        try_or_rollback!(conn.execute(
            "INSERT INTO agent_ledger_entries (agent_id, product_id, entry_type, qty, unit_price, amount, reference_code, notes, entry_date, created_at, updated_at)
             VALUES (?1, ?2, 'sale_reported', ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![
                aid, product_id, qty, unit_sale_price, amount,
                format!("SALE-{}", now),
                notes.as_deref().unwrap_or(""),
                &now, &now, &now,
            ],
        ).map_err(|e| e.to_string()));
        // Reduce agent stock, increase sold
        try_or_rollback!(conn.execute(
            "UPDATE products SET qty_with_agents = MAX(0, qty_with_agents - ?1), qty_sold = qty_sold + ?2, updated_at = ?3 WHERE id = ?4",
            rusqlite::params![qty, qty, &now, product_id],
        ).map_err(|e| e.to_string()));
    } else {
        // Direct HO sale — validate HO has enough stock
        let ho_qty: i64 = match conn.query_row(
            "SELECT COALESCE(qty_in_head_office, stock_quantity, 0) FROM products WHERE id = ?1",
            rusqlite::params![product_id],
            |r| r.get(0),
        ) {
            Ok(v) => v,
            Err(e) => {
                let _ = conn.execute("ROLLBACK", []);
                return Err(format!("Product not found: {}", e));
            }
        };
        if ho_qty < qty {
            let _ = conn.execute("ROLLBACK", []);
            return Err(format!(
                "Insufficient stock in Head Office. Available: {}, requested: {}.",
                ho_qty, qty
            ));
        }
        // Reduce HO stock, increase sold
        try_or_rollback!(conn.execute(
            "UPDATE products SET qty_in_head_office = qty_in_head_office - ?1, stock_quantity = stock_quantity - ?2, qty_sold = qty_sold + ?3, updated_at = ?4 WHERE id = ?5",
            rusqlite::params![qty, qty, qty, &now, product_id],
        ).map_err(|e| e.to_string()));
    }

    // Create sales table entry
    // v0.26.0: Track amount_paid + balance for udhar/credit feature.
    // If amount_paid is None, default to full payment (backward compat).
    let paid = amount_paid.unwrap_or(total).min(total);
    let balance = (total - paid).max(0.0);
    try_or_rollback!(conn.execute(
        "INSERT INTO sales (product_id, sale_channel, sale_type, agent_id, qty, unit_sale_price, total_sale_amount, amount_paid, balance, customer_id, customer_name, customer_phone, notes, sale_date, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
        rusqlite::params![
            product_id,
            &sale_channel,
            if agent_id.is_some() { "agent_sale" } else { "direct_sale" },
            agent_id,
            qty,
            unit_sale_price,
            total,
            paid,
            balance,
            customer_id,
            customer_name.as_deref().unwrap_or(""),
            customer_phone.as_deref().unwrap_or(""),
            notes.as_deref().unwrap_or(""),
            &now,
            &now,
            &now,
        ],
    ).map_err(|e| e.to_string()));
    let sale_id = conn.last_insert_rowid();

    // Auto-update profit_status based on remaining stock
    let (ho_qty, agent_qty): (i64, i64) = conn.query_row(
        "SELECT COALESCE(qty_in_head_office, 0), COALESCE(qty_with_agents, 0) FROM products WHERE id = ?1",
        rusqlite::params![product_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    ).unwrap_or((0, 0));
    let new_status = if ho_qty == 0 && agent_qty == 0 {
        "sold_out"
    } else if ho_qty == 0 && agent_qty > 0 {
        "with_agent"
    } else {
        "in_head_office"
    };
    try_or_rollback!(conn.execute(
        "UPDATE products SET profit_status = ?1, updated_at = ?2 WHERE id = ?3",
        rusqlite::params![new_status, &now, product_id],
    ).map_err(|e| e.to_string()));

    // v0.26.0: If customer_id is provided and there's an unpaid balance,
    // increase the customer's outstanding_balance by the sale's balance.
    // This keeps the khata (credit) tracking in sync.
    if balance > 0.0 {
        if let Some(cid) = customer_id {
            try_or_rollback!(conn.execute(
                "UPDATE customers SET outstanding_balance = outstanding_balance + ?1, updated_at = ?2 WHERE id = ?3",
                rusqlite::params![balance, &now, cid],
            ).map_err(|e| e.to_string()));
        }
    }

    conn.execute("COMMIT", []).map_err(|e| {
        let _ = conn.execute("ROLLBACK", []);
        e.to_string()
    })?;

    Ok(sale_id)
}
