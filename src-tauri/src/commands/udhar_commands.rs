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
// // Customer Udhar/Credit (खाता): record_payment + balance_history
// ============================================================

// ============================================================
// v0.26.0: CUSTOMER UDHAR / CREDIT (खाता) TRACKING
// ============================================================

/// Record a payment from a customer against their outstanding balance.
/// Reduces customer.outstanding_balance by the payment amount and inserts
/// a row in customer_payments for audit/history.
/// Optionally link the payment to a specific sale_id.
#[tauri::command]
pub async fn record_customer_payment(
    state: State<'_, DbState>,
    customer_id: i64,
    amount: f64,
    notes: Option<String>,
    sale_id: Option<i64>,
) -> Result<(), String> {
    if amount <= 0.0 {
        return Err("Payment amount must be positive.".to_string());
    }
    let conn = state.0.lock().await;
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute("BEGIN IMMEDIATE", []).map_err(|e| e.to_string())?;

    // Check customer exists and has enough balance
    let current_balance: f64 = conn.query_row(
        "SELECT COALESCE(outstanding_balance, 0.0) FROM customers WHERE id = ?1",
        rusqlite::params![customer_id],
        |r| r.get(0),
    ).map_err(|e| {
        let _ = conn.execute("ROLLBACK", []);
        format!("Customer not found: {}", e)
    })?;

    if amount > current_balance {
        let _ = conn.execute("ROLLBACK", []);
        return Err(format!(
            "Payment (Rs. {:.0}) exceeds outstanding balance (Rs. {:.0}).",
            amount, current_balance
        ));
    }

    // Insert payment record
    if let Err(e) = conn.execute(
        "INSERT INTO customer_payments (customer_id, amount, payment_date, notes, sale_id, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![
            customer_id,
            amount,
            &now,
            notes.as_deref().unwrap_or(""),
            sale_id,
            &now,
            &now,
        ],
    ) {
        let _ = conn.execute("ROLLBACK", []);
        return Err(format!("Failed to insert payment: {}", e));
    }

    // Reduce customer's outstanding balance
    if let Err(e) = conn.execute(
        "UPDATE customers SET outstanding_balance = outstanding_balance - ?1, updated_at = ?2 WHERE id = ?3",
        rusqlite::params![amount, &now, customer_id],
    ) {
        let _ = conn.execute("ROLLBACK", []);
        return Err(format!("Failed to update balance: {}", e));
    }

    conn.execute("COMMIT", []).map_err(|e| {
        let _ = conn.execute("ROLLBACK", []);
        e.to_string()
    })?;

    Ok(())
}

/// Get a customer's full balance history (sales + payments) in chronological order.
/// Each entry includes: type (sale/payment), date, description, amount, balance_after.
/// Used by the customer detail modal to show the khata timeline.
#[tauri::command]
pub async fn get_customer_balance_history(
    state: State<'_, DbState>,
    customer_id: i64,
) -> Result<Vec<customers::BalanceHistoryEntry>, String> {
    let conn = state.0.lock().await;
    let now = chrono::Utc::now().to_rfc3339();

    // Fetch sales with balance for this customer
    let mut stmt = conn.prepare(
        "SELECT s.id, s.sale_date, p.name, s.qty, s.total_sale_amount, s.balance
         FROM sales s
         LEFT JOIN products p ON s.product_id = p.id
         WHERE s.customer_id = ?1
         ORDER BY s.sale_date ASC"
    ).map_err(|e| e.to_string())?;

    let sales: Vec<(i64, String, String, i64, f64, f64)> = stmt.query_map(
        [customer_id],
        |row| Ok((
            row.get(0)?,
            row.get(1)?,
            row.get::<_, Option<String>>(2)?.unwrap_or_default(),
            row.get(3)?,
            row.get(4)?,
            row.get(5)?,
        )),
    ).map_err(|e| e.to_string())?
    .filter_map(|r| r.ok())
    .collect();

    // Fetch payments for this customer
    let mut stmt2 = conn.prepare(
        "SELECT id, payment_date, amount, notes
         FROM customer_payments
         WHERE customer_id = ?1
         ORDER BY payment_date ASC"
    ).map_err(|e| e.to_string())?;

    let payments: Vec<(i64, String, f64, String)> = stmt2.query_map(
        [customer_id],
        |row| Ok((
            row.get(0)?,
            row.get(1)?,
            row.get(2)?,
            row.get::<_, Option<String>>(3)?.unwrap_or_default(),
        )),
    ).map_err(|e| e.to_string())?
    .filter_map(|r| r.ok())
    .collect();

    // Merge + sort by date, compute running balance
    let mut entries: Vec<customers::BalanceHistoryEntry> = Vec::new();
    let mut running_balance: f64 = 0.0;

    // Build a combined list of (date, type, description, amount_change)
    let mut combined: Vec<(String, String, String, f64, i64)> = Vec::new();
    for (id, date, pname, qty, total, balance) in &sales {
        let desc = format!("{} x{} (Rs. {:.0})", pname, qty, total);
        combined.push((date.clone(), "sale".to_string(), desc, *balance, *id));
    }
    for (id, date, amount, notes) in &payments {
        let desc = if notes.is_empty() { format!("Payment Rs. {:.0}", amount) } else { format!("Payment: {}", notes) };
        combined.push((date.clone(), "payment".to_string(), desc, -*amount, *id));
    }
    combined.sort_by(|a, b| a.0.cmp(&b.0));

    for (date, etype, desc, amount_change, id) in combined {
        running_balance += amount_change;
        if running_balance < 0.0 { running_balance = 0.0; } // safety
        entries.push(customers::BalanceHistoryEntry {
            id,
            entry_type: etype,
            date,
            description: desc,
            amount: amount_change,
            balance_after: running_balance,
        });
    }

    let _ = now; // suppress unused warning
    Ok(entries)
}
