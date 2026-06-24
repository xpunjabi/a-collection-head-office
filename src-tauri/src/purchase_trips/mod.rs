use serde::{Serialize, Deserialize};
use rusqlite::{Connection, params};

/// A purchase trip — a buying expedition to Faisalabad (or other source city)
/// where stock is purchased and brought back to Head Office. Trip expenses
/// (travel, transport, food, loading, misc) are allocated proportionally
/// across all items purchased on the trip to compute landed_unit_cost.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PurchaseTrip {
    pub id: Option<i64>,
    pub trip_code: String,
    pub trip_date: String,
    pub source_city: String,
    pub supplier_notes: Option<String>,
    pub travel_cost: f64,
    pub transport_cost: f64,
    pub food_cost: f64,
    pub loading_cost: f64,
    pub misc_cost: f64,
    pub created_at: String,
    pub updated_at: String,
}

impl PurchaseTrip {
    /// Sum of all expense fields. This is the total trip overhead that
    /// gets allocated across items proportionally by purchase cost.
    pub fn total_trip_expense(&self) -> f64 {
        self.travel_cost + self.transport_cost + self.food_cost
            + self.loading_cost + self.misc_cost
    }
}

/// A single item purchased on a trip. Linked to a product (which may be
/// created at trip-item-add time). The expense_allocation_amount is this
/// item's proportional share of the trip's total expense, computed as:
///   (item.total_purchase_cost / sum(all items' total_purchase_cost)) * trip.total_trip_expense
///
/// landed_unit_cost = (total_purchase_cost + expense_allocation_amount) / qty_purchased
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PurchaseTripItem {
    pub id: Option<i64>,
    pub trip_id: i64,
    pub product_id: Option<i64>,
    pub qty_purchased: i64,
    pub unit_purchase_cost: f64,
    pub total_purchase_cost: f64,
    pub expense_allocation_amount: f64,
    pub landed_unit_cost: f64,
    pub created_at: String,
    pub updated_at: String,
}

/// Trip summary with computed totals + item count. Used by the trip list.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PurchaseTripSummary {
    pub trip: PurchaseTrip,
    pub item_count: i64,
    pub total_purchase_cost: f64,
    pub total_landed_cost: f64,
}

pub fn get_all_purchase_trips(conn: &Connection) -> Result<Vec<PurchaseTripSummary>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT t.id, t.trip_code, t.trip_date, t.source_city, t.supplier_notes,
                t.travel_cost, t.transport_cost, t.food_cost, t.loading_cost, t.misc_cost,
                t.created_at, t.updated_at,
                COUNT(i.id) AS item_count,
                COALESCE(SUM(i.total_purchase_cost), 0.0) AS total_purchase_cost,
                COALESCE(SUM(i.total_purchase_cost + i.expense_allocation_amount), 0.0) AS total_landed_cost
         FROM purchase_trips t
         LEFT JOIN purchase_trip_items i ON i.trip_id = t.id
         GROUP BY t.id
         ORDER BY t.trip_date DESC, t.id DESC"
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(PurchaseTripSummary {
            trip: PurchaseTrip {
                id: Some(row.get(0)?),
                trip_code: row.get(1)?,
                trip_date: row.get(2)?,
                source_city: row.get(3)?,
                supplier_notes: row.get(4)?,
                travel_cost: row.get(5)?,
                transport_cost: row.get(6)?,
                food_cost: row.get(7)?,
                loading_cost: row.get(8)?,
                misc_cost: row.get(9)?,
                created_at: row.get(10)?,
                updated_at: row.get(11)?,
            },
            item_count: row.get(12)?,
            total_purchase_cost: row.get(13)?,
            total_landed_cost: row.get(14)?,
        })
    })?;
    let mut result = Vec::new();
    for r in rows { result.push(r?); }
    Ok(result)
}

pub fn get_purchase_trip(conn: &Connection, id: i64) -> Result<(PurchaseTrip, Vec<PurchaseTripItem>), rusqlite::Error> {
    let trip = conn.query_row(
        "SELECT id, trip_code, trip_date, source_city, supplier_notes,
                travel_cost, transport_cost, food_cost, loading_cost, misc_cost,
                created_at, updated_at
         FROM purchase_trips WHERE id = ?1",
        params![id],
        |row| {
            Ok(PurchaseTrip {
                id: Some(row.get(0)?),
                trip_code: row.get(1)?,
                trip_date: row.get(2)?,
                source_city: row.get(3)?,
                supplier_notes: row.get(4)?,
                travel_cost: row.get(5)?,
                transport_cost: row.get(6)?,
                food_cost: row.get(7)?,
                loading_cost: row.get(8)?,
                misc_cost: row.get(9)?,
                created_at: row.get(10)?,
                updated_at: row.get(11)?,
            })
        },
    )?;

    let mut stmt = conn.prepare(
        "SELECT id, trip_id, product_id, qty_purchased, unit_purchase_cost,
                total_purchase_cost, expense_allocation_amount, landed_unit_cost,
                created_at, updated_at
         FROM purchase_trip_items WHERE trip_id = ?1 ORDER BY id"
    )?;
    let rows = stmt.query_map(params![id], |row| {
        Ok(PurchaseTripItem {
            id: Some(row.get(0)?),
            trip_id: row.get(1)?,
            product_id: row.get(2)?,
            qty_purchased: row.get(3)?,
            unit_purchase_cost: row.get(4)?,
            total_purchase_cost: row.get(5)?,
            expense_allocation_amount: row.get(6)?,
            landed_unit_cost: row.get(7)?,
            created_at: row.get(8)?,
            updated_at: row.get(9)?,
        })
    })?;
    let mut items = Vec::new();
    for r in rows { items.push(r?); }
    Ok((trip, items))
}

fn generate_trip_code() -> String {
    let ts = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
    format!("TRIP-{}", ts)
}

/// Create a new purchase trip. Returns the new trip ID.
pub fn create_purchase_trip(
    conn: &Connection,
    trip_date: &str,
    source_city: &str,
    supplier_notes: Option<&str>,
    travel_cost: f64,
    transport_cost: f64,
    food_cost: f64,
    loading_cost: f64,
    misc_cost: f64,
) -> Result<i64, rusqlite::Error> {
    let now = chrono::Utc::now().to_rfc3339();
    let trip_code = generate_trip_code();
    conn.execute(
        "INSERT INTO purchase_trips (trip_code, trip_date, source_city, supplier_notes,
            travel_cost, transport_cost, food_cost, loading_cost, misc_cost, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            &trip_code,
            trip_date,
            source_city,
            supplier_notes.unwrap_or(""),
            travel_cost,
            transport_cost,
            food_cost,
            loading_cost,
            misc_cost,
            &now,
            &now,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Update trip header fields (date, source, notes, expenses).
/// Does NOT touch items — use add_trip_item / remove_trip_item for those.
pub fn update_purchase_trip(
    conn: &Connection,
    id: i64,
    trip_date: &str,
    source_city: &str,
    supplier_notes: Option<&str>,
    travel_cost: f64,
    transport_cost: f64,
    food_cost: f64,
    loading_cost: f64,
    misc_cost: f64,
) -> Result<(), rusqlite::Error> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE purchase_trips SET trip_date=?1, source_city=?2, supplier_notes=?3,
            travel_cost=?4, transport_cost=?5, food_cost=?6, loading_cost=?7, misc_cost=?8,
            updated_at=?9 WHERE id=?10",
        params![
            trip_date,
            source_city,
            supplier_notes.unwrap_or(""),
            travel_cost,
            transport_cost,
            food_cost,
            loading_cost,
            misc_cost,
            &now,
            id,
        ],
    )?;
    Ok(())
}

pub fn delete_purchase_trip(conn: &Connection, id: i64) -> Result<(), rusqlite::Error> {
    // CASCADE delete will remove all purchase_trip_items for this trip.
    conn.execute("DELETE FROM purchase_trips WHERE id = ?1", params![id])?;
    Ok(())
}

/// Add an item to a trip. This creates the purchase_trip_items row AND
/// updates the linked product's stock (qty_in_head_office += qty_purchased)
/// and cost fields (base_unit_cost, landed_unit_cost, source_trip_id).
///
/// NOTE: expense_allocation_amount and landed_unit_cost are computed by
/// recalculate_trip_allocations() which is called after this function
/// (or after all items are added). Initially they are set to 0.
pub fn add_trip_item(
    conn: &Connection,
    trip_id: i64,
    product_id: i64,
    qty_purchased: i64,
    unit_purchase_cost: f64,
) -> Result<i64, rusqlite::Error> {
    if qty_purchased <= 0 {
        return Err(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CONSTRAINT),
            Some("qty_purchased must be positive".to_string()),
        ));
    }
    let now = chrono::Utc::now().to_rfc3339();
    let total_purchase_cost = qty_purchased as f64 * unit_purchase_cost;
    conn.execute(
        "INSERT INTO purchase_trip_items (trip_id, product_id, qty_purchased, unit_purchase_cost,
            total_purchase_cost, expense_allocation_amount, landed_unit_cost, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 0.0, 0.0, ?6, ?7)",
        params![
            trip_id,
            product_id,
            qty_purchased,
            unit_purchase_cost,
            total_purchase_cost,
            &now,
            &now,
        ],
    )?;
    let item_id = conn.last_insert_rowid();

    // Update the linked product: increase Head Office stock, set cost fields,
    // link to this trip.
    conn.execute(
        "UPDATE products SET
            qty_in_head_office = qty_in_head_office + ?1,
            stock_quantity = stock_quantity + ?1,
            base_unit_cost = ?2,
            source_trip_id = ?3,
            updated_at = ?4
         WHERE id = ?5",
        params![
            qty_purchased,
            unit_purchase_cost,
            trip_id,
            &now,
            product_id,
        ],
    )?;

    Ok(item_id)
}

pub fn remove_trip_item(conn: &Connection, item_id: i64) -> Result<(), rusqlite::Error> {
    // Before deleting, reverse the stock addition on the linked product.
    let (product_id, qty_purchased): (Option<i64>, i64) = conn.query_row(
        "SELECT product_id, qty_purchased FROM purchase_trip_items WHERE id = ?1",
        params![item_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    ).unwrap_or((None, 0));

    if let Some(pid) = product_id {
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE products SET
                qty_in_head_office = MAX(0, qty_in_head_office - ?1),
                stock_quantity = MAX(0, stock_quantity - ?1),
                updated_at = ?2
             WHERE id = ?3",
            params![qty_purchased, &now, pid],
        )?;
    }

    conn.execute("DELETE FROM purchase_trip_items WHERE id = ?1", params![item_id])?;
    Ok(())
}

/// Recalculate expense_allocation_amount and landed_unit_cost for ALL items
/// on a trip. Called after items are added/removed or after trip expenses
/// change. Uses proportional allocation by total_purchase_cost.
pub fn recalculate_trip_allocations(conn: &Connection, trip_id: i64) -> Result<(), rusqlite::Error> {
    // Get trip expenses
    let (travel, transport, food, loading, misc): (f64, f64, f64, f64, f64) = conn.query_row(
        "SELECT travel_cost, transport_cost, food_cost, loading_cost, misc_cost
         FROM purchase_trips WHERE id = ?1",
        params![trip_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
    )?;
    let total_trip_expense = travel + transport + food + loading + misc;

    // Get sum of all items' total_purchase_cost
    let sum_purchase: f64 = conn.query_row(
        "SELECT COALESCE(SUM(total_purchase_cost), 0.0) FROM purchase_trip_items WHERE trip_id = ?1",
        params![trip_id],
        |row| row.get(0),
    ).unwrap_or(0.0);

    if sum_purchase <= 0.0 {
        // No items or all zero-cost — set allocations to 0
        conn.execute(
            "UPDATE purchase_trip_items SET expense_allocation_amount = 0.0, landed_unit_cost = 0.0 WHERE trip_id = ?1",
            params![trip_id],
        )?;
        return Ok(());
    }

    // Update each item with its proportional allocation
    let now = chrono::Utc::now().to_rfc3339();
    let mut stmt = conn.prepare(
        "SELECT id, total_purchase_cost, qty_purchased, product_id FROM purchase_trip_items WHERE trip_id = ?1"
    )?;
    let items: Vec<(i64, f64, i64, Option<i64>)> = stmt.query_map(params![trip_id], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
    })?.filter_map(|r| r.ok()).collect();

    for (item_id, total_purchase_cost, qty, product_id) in items {
        let allocation = (total_purchase_cost / sum_purchase) * total_trip_expense;
        let landed_unit_cost = if qty > 0 {
            (total_purchase_cost + allocation) / qty as f64
        } else {
            0.0
        };
        conn.execute(
            "UPDATE purchase_trip_items SET expense_allocation_amount = ?1, landed_unit_cost = ?2, updated_at = ?3 WHERE id = ?4",
            params![allocation, landed_unit_cost, &now, item_id],
        )?;
        // Also update the linked product's landed_unit_cost
        if let Some(pid) = product_id {
            conn.execute(
                "UPDATE products SET landed_unit_cost = ?1, updated_at = ?2 WHERE id = ?3",
                params![landed_unit_cost, &now, pid],
            )?;
        }
    }

    Ok(())
}
