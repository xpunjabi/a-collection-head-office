use serde::{Serialize, Deserialize};
use rusqlite::{Connection, params};

/// An agent is a person at a place who receives stock from Head Office,
/// sells it, and remits cash. Agents replace the old locations concept
/// (which was place-only, no person info).
///
/// Computed fields (current_stock_value, current_outstanding_balance,
/// total_cash_received, total_stock_units_in_hand) are NOT stored — they
/// are derived on-the-fly from the agent_ledger_entries table. This keeps
/// the agents table as a pure master record and the ledger as the single
/// source of truth for all movement.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Agent {
    pub id: Option<i64>,
    pub agent_code: String,
    pub name: String,
    pub phone: Option<String>,
    pub city: Option<String>,
    pub area: Option<String>,
    pub address_notes: Option<String>,
    pub notes: Option<String>,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// Agent summary with computed financial/stock fields. Returned by
/// get_agent_summary() which joins the agents table with aggregations
/// over agent_ledger_entries.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AgentSummary {
    pub agent: Agent,
    /// Total units currently held by this agent (stock_sent - stock_returned - sale_reported)
    pub current_stock_units: i64,
    /// Total cash value of stock currently held (qty * unit_price at send time)
    pub current_stock_value: f64,
    /// Total cash received from this agent across all time
    pub total_cash_received: f64,
    /// Outstanding balance = (stock sent value) - (cash received) - (stock returned value)
    /// Positive = agent owes Head Office. Negative = Head Office owes agent.
    pub outstanding_balance: f64,
    /// ISO timestamp of the most recent cash_received entry (None if never)
    pub last_settlement_at: Option<String>,
}

/// A single ledger entry recording stock or cash movement between Head
/// Office and an agent. entry_type determines the meaning of qty/amount:
///
/// - stock_sent:        qty > 0, amount = qty * unit_price (value sent)
/// - stock_returned:    qty > 0, amount = qty * unit_price (value returned)
/// - sale_reported:     qty > 0, amount = qty * unit_price (value sold)
/// - cash_received:     qty = 0, amount > 0 (cash received)
/// - balance_adjustment: qty = 0, amount can be + or - (manual correction)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AgentLedgerEntry {
    pub id: Option<i64>,
    pub agent_id: i64,
    pub product_id: Option<i64>,
    pub entry_type: String,
    pub qty: i64,
    pub unit_price: f64,
    pub amount: f64,
    pub reference_code: Option<String>,
    pub notes: Option<String>,
    pub entry_date: String,
    pub created_at: String,
    pub updated_at: String,
}

pub fn get_all_agents(conn: &Connection) -> Result<Vec<Agent>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, agent_code, name, phone, city, area, address_notes, notes, is_active, created_at, updated_at
         FROM agents ORDER BY is_active DESC, name ASC"
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(Agent {
            id: Some(row.get(0)?),
            agent_code: row.get(1)?,
            name: row.get(2)?,
            phone: row.get(3)?,
            city: row.get(4)?,
            area: row.get(5)?,
            address_notes: row.get(6)?,
            notes: row.get(7)?,
            is_active: row.get::<_, i64>(8)? != 0,
            created_at: row.get(9)?,
            updated_at: row.get(10)?,
        })
    })?;
    let mut result = Vec::new();
    for r in rows { result.push(r?); }
    Ok(result)
}

pub fn get_agent_by_id(conn: &Connection, id: i64) -> Result<Agent, rusqlite::Error> {
    conn.query_row(
        "SELECT id, agent_code, name, phone, city, area, address_notes, notes, is_active, created_at, updated_at
         FROM agents WHERE id = ?1",
        params![id],
        |row| {
            Ok(Agent {
                id: Some(row.get(0)?),
                agent_code: row.get(1)?,
                name: row.get(2)?,
                phone: row.get(3)?,
                city: row.get(4)?,
                area: row.get(5)?,
                address_notes: row.get(6)?,
                notes: row.get(7)?,
                is_active: row.get::<_, i64>(8)? != 0,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
            })
        },
    )
}

/// Generate a unique agent_code. Format: "AGT-<timestamp_nanos>".
fn generate_agent_code() -> String {
    let ts = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
    format!("AGT-{}", ts)
}

pub fn add_agent(
    conn: &Connection,
    name: &str,
    phone: Option<&str>,
    city: Option<&str>,
    area: Option<&str>,
    address_notes: Option<&str>,
    notes: Option<&str>,
) -> Result<i64, rusqlite::Error> {
    let now = chrono::Utc::now().to_rfc3339();
    let agent_code = generate_agent_code();
    conn.execute(
        "INSERT INTO agents (agent_code, name, phone, city, area, address_notes, notes, is_active, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1, ?8, ?9)",
        params![
            &agent_code,
            name,
            phone.unwrap_or(""),
            city.unwrap_or(""),
            area.unwrap_or(""),
            address_notes.unwrap_or(""),
            notes.unwrap_or(""),
            &now,
            &now,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update_agent(
    conn: &Connection,
    id: i64,
    name: &str,
    phone: Option<&str>,
    city: Option<&str>,
    area: Option<&str>,
    address_notes: Option<&str>,
    notes: Option<&str>,
    is_active: bool,
) -> Result<(), rusqlite::Error> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE agents SET name=?1, phone=?2, city=?3, area=?4, address_notes=?5, notes=?6, is_active=?7, updated_at=?8 WHERE id=?9",
        params![
            name,
            phone.unwrap_or(""),
            city.unwrap_or(""),
            area.unwrap_or(""),
            address_notes.unwrap_or(""),
            notes.unwrap_or(""),
            is_active as i64,
            &now,
            id,
        ],
    )?;
    Ok(())
}

pub fn delete_agent(conn: &Connection, id: i64) -> Result<(), rusqlite::Error> {
    conn.execute("DELETE FROM agents WHERE id = ?1", params![id])?;
    Ok(())
}

/// Compute the full summary for an agent: stock held, cash received,
/// outstanding balance, last settlement date. All values are derived
/// from agent_ledger_entries (single source of truth).
pub fn get_agent_summary(conn: &Connection, agent_id: i64) -> Result<AgentSummary, rusqlite::Error> {
    let agent = get_agent_by_id(conn, agent_id)?;

    let (current_stock_units, total_stock_value_sent, total_stock_value_returned, total_cash_received, last_cash_at): (i64, f64, f64, f64, Option<String>) = conn.query_row(
        "SELECT
            COALESCE(SUM(CASE WHEN entry_type = 'stock_sent' THEN qty ELSE 0 END) -
                     SUM(CASE WHEN entry_type = 'stock_returned' THEN qty ELSE 0 END) -
                     SUM(CASE WHEN entry_type = 'sale_reported' THEN qty ELSE 0 END), 0) AS current_stock_units,
            COALESCE(SUM(CASE WHEN entry_type = 'stock_sent' THEN amount ELSE 0 END), 0.0) AS total_stock_value_sent,
            COALESCE(SUM(CASE WHEN entry_type = 'stock_returned' THEN amount ELSE 0 END), 0.0) AS total_stock_value_returned,
            COALESCE(SUM(CASE WHEN entry_type = 'cash_received' THEN amount ELSE 0 END), 0.0) AS total_cash_received,
            MAX(CASE WHEN entry_type = 'cash_received' THEN entry_date ELSE NULL END) AS last_cash_at
         FROM agent_ledger_entries WHERE agent_id = ?1",
        params![agent_id],
        |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        },
    )?;

    let outstanding_balance = total_stock_value_sent - total_stock_value_returned - total_cash_received;

    let current_stock_value = if current_stock_units > 0 {
        let total_qty_sent_minus_returned: i64 = conn.query_row(
            "SELECT COALESCE(SUM(CASE WHEN entry_type = 'stock_sent' THEN qty ELSE 0 END) -
                             SUM(CASE WHEN entry_type = 'stock_returned' THEN qty ELSE 0 END), 0)
             FROM agent_ledger_entries WHERE agent_id = ?1",
            params![agent_id],
            |r| r.get(0),
        ).unwrap_or(0);
        if total_qty_sent_minus_returned > 0 {
            (total_stock_value_sent - total_stock_value_returned) / total_qty_sent_minus_returned as f64 * current_stock_units as f64
        } else {
            0.0
        }
    } else {
        0.0
    };

    Ok(AgentSummary {
        agent,
        current_stock_units,
        current_stock_value,
        total_cash_received,
        outstanding_balance,
        last_settlement_at: last_cash_at,
    })
}

pub fn get_all_agent_summaries(conn: &Connection) -> Result<Vec<AgentSummary>, rusqlite::Error> {
    let agents = get_all_agents(conn)?;
    let mut summaries = Vec::new();
    for agent in agents {
        if let Some(id) = agent.id {
            match get_agent_summary(conn, id) {
                Ok(s) => summaries.push(s),
                Err(_) => continue,
            }
        }
    }
    Ok(summaries)
}

pub fn get_agent_ledger_entries(
    conn: &Connection,
    agent_id: i64,
    limit: i64,
) -> Result<Vec<AgentLedgerEntry>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, agent_id, product_id, entry_type, qty, unit_price, amount, reference_code, notes, entry_date, created_at, updated_at
         FROM agent_ledger_entries
         WHERE agent_id = ?1
         ORDER BY entry_date DESC, id DESC
         LIMIT ?2"
    )?;
    let rows = stmt.query_map(params![agent_id, limit], |row| {
        Ok(AgentLedgerEntry {
            id: Some(row.get(0)?),
            agent_id: row.get(1)?,
            product_id: row.get(2)?,
            entry_type: row.get(3)?,
            qty: row.get(4)?,
            unit_price: row.get(5)?,
            amount: row.get(6)?,
            reference_code: row.get(7)?,
            notes: row.get(8)?,
            entry_date: row.get(9)?,
            created_at: row.get(10)?,
            updated_at: row.get(11)?,
        })
    })?;
    let mut result = Vec::new();
    for r in rows { result.push(r?); }
    Ok(result)
}

/// Append a ledger entry. This is the ONLY way stock or cash should move
/// between Head Office and an agent.
pub fn append_ledger_entry(
    conn: &Connection,
    agent_id: i64,
    product_id: Option<i64>,
    entry_type: &str,
    qty: i64,
    unit_price: f64,
    amount: f64,
    reference_code: Option<&str>,
    notes: Option<&str>,
) -> Result<i64, rusqlite::Error> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO agent_ledger_entries (agent_id, product_id, entry_type, qty, unit_price, amount, reference_code, notes, entry_date, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            agent_id,
            product_id,
            entry_type,
            qty,
            unit_price,
            amount,
            reference_code.unwrap_or(""),
            notes.unwrap_or(""),
            &now,
            &now,
            &now,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn send_stock_to_agent(
    conn: &Connection,
    agent_id: i64,
    product_id: i64,
    qty: i64,
    unit_price: f64,
    notes: Option<&str>,
) -> Result<i64, rusqlite::Error> {
    if qty <= 0 {
        return Err(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CONSTRAINT),
            Some("qty must be positive for stock_sent".to_string()),
        ));
    }
    let amount = qty as f64 * unit_price;
    let entry_id = append_ledger_entry(
        conn, agent_id, Some(product_id), "stock_sent", qty, unit_price, amount, None, notes,
    )?;
    conn.execute(
        "UPDATE products SET qty_in_head_office = qty_in_head_office - ?1, qty_with_agents = qty_with_agents + ?2, updated_at = ?3 WHERE id = ?4",
        params![qty, qty, chrono::Utc::now().to_rfc3339(), product_id],
    )?;
    Ok(entry_id)
}

pub fn return_stock_from_agent(
    conn: &Connection,
    agent_id: i64,
    product_id: i64,
    qty: i64,
    unit_price: f64,
    notes: Option<&str>,
) -> Result<i64, rusqlite::Error> {
    if qty <= 0 {
        return Err(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CONSTRAINT),
            Some("qty must be positive for stock_returned".to_string()),
        ));
    }
    let amount = qty as f64 * unit_price;
    let entry_id = append_ledger_entry(
        conn, agent_id, Some(product_id), "stock_returned", qty, unit_price, amount, None, notes,
    )?;
    conn.execute(
        "UPDATE products SET qty_in_head_office = qty_in_head_office + ?1, qty_with_agents = qty_with_agents - ?2, updated_at = ?3 WHERE id = ?4",
        params![qty, qty, chrono::Utc::now().to_rfc3339(), product_id],
    )?;
    Ok(entry_id)
}

pub fn report_agent_sale(
    conn: &Connection,
    agent_id: i64,
    product_id: i64,
    qty: i64,
    unit_price: f64,
    notes: Option<&str>,
) -> Result<i64, rusqlite::Error> {
    if qty <= 0 {
        return Err(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CONSTRAINT),
            Some("qty must be positive for sale_reported".to_string()),
        ));
    }
    let amount = qty as f64 * unit_price;
    let entry_id = append_ledger_entry(
        conn, agent_id, Some(product_id), "sale_reported", qty, unit_price, amount, None, notes,
    )?;
    conn.execute(
        "UPDATE products SET qty_with_agents = qty_with_agents - ?1, qty_sold = qty_sold + ?2, updated_at = ?3 WHERE id = ?4",
        params![qty, qty, chrono::Utc::now().to_rfc3339(), product_id],
    )?;
    Ok(entry_id)
}

pub fn receive_agent_cash(
    conn: &Connection,
    agent_id: i64,
    amount: f64,
    notes: Option<&str>,
) -> Result<i64, rusqlite::Error> {
    if amount <= 0.0 {
        return Err(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CONSTRAINT),
            Some("amount must be positive for cash_received".to_string()),
        ));
    }
    append_ledger_entry(
        conn, agent_id, None, "cash_received", 0, 0.0, amount, None, notes,
    )
}

pub fn adjust_agent_balance(
    conn: &Connection,
    agent_id: i64,
    amount: f64,
    notes: &str,
) -> Result<i64, rusqlite::Error> {
    if notes.trim().is_empty() {
        return Err(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CONSTRAINT),
            Some("notes are mandatory for balance_adjustment".to_string()),
        ));
    }
    let stored_amount = -amount;
    append_ledger_entry(
        conn, agent_id, None, "balance_adjustment", 0, 0.0, stored_amount, None, Some(notes),
    )
}
