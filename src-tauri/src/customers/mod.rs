use serde::{Serialize, Deserialize};
use rusqlite::{Connection, params};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Customer {
    pub id: Option<i64>,
    pub name: String,
    pub phone: Option<String>,
    pub location: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OrderItemInput {
    pub product_id: i64,
    pub quantity: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OrderHistory {
    pub order_id: i64,
    pub order_date: String,
    pub total_amount: f64,
    pub profit: f64,
    pub items: Vec<OrderItemDetail>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OrderItemDetail {
    pub product_name: String,
    pub sku: String,
    pub quantity: i64,
    pub sale_price: f64,
}

pub fn get_all_customers(conn: &Connection) -> Result<Vec<Customer>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, name, phone, location, notes, created_at FROM customers ORDER BY name ASC"
    )?;
    
    let customer_iter = stmt.query_map([], |row| {
        Ok(Customer {
            id: Some(row.get(0)?),
            name: row.get(1)?,
            phone: row.get(2)?,
            location: row.get(3)?,
            notes: row.get(4)?,
            created_at: row.get(5)?,
        })
    })?;

    let mut customers = Vec::new();
    for customer in customer_iter {
        customers.push(customer?);
    }
    Ok(customers)
}

pub fn add_customer(conn: &Connection, customer: &Customer) -> Result<i64, rusqlite::Error> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO customers (name, phone, location, notes, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        (
            &customer.name,
            &customer.phone,
            &customer.location,
            &customer.notes,
            &now
        ),
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update_customer(conn: &Connection, customer: &Customer) -> Result<(), rusqlite::Error> {
    conn.execute(
        "UPDATE customers SET name = ?1, phone = ?2, location = ?3, notes = ?4 WHERE id = ?5",
        (
            &customer.name,
            &customer.phone,
            &customer.location,
            &customer.notes,
            customer.id
        ),
    )?;
    Ok(())
}

pub fn delete_customer(conn: &Connection, id: i64) -> Result<(), rusqlite::Error> {
    conn.execute("DELETE FROM customers WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn create_order(conn: &mut Connection, customer_id: i64, items: Vec<OrderItemInput>) -> Result<i64, Box<dyn std::error::Error>> {
    let tx = conn.transaction()?;
    
    let mut total_amount = 0.0;
    let mut total_cost = 0.0;
    let now = chrono::Utc::now().to_rfc3339();
    
    // 1. Insert Order placeholder (will update later with correct totals)
    tx.execute(
        "INSERT INTO orders (customer_id, total_amount, profit, order_date) VALUES (?1, 0.0, 0.0, ?2)",
        params![customer_id, &now],
    )?;
    let order_id = tx.last_insert_rowid();

    // 2. Loop items, calculate prices, decrease stock
    for item in items {
        let (cost_price, sale_price, stock_qty): (f64, f64, i64) = tx.query_row(
            "SELECT cost_price, sale_price, stock_quantity FROM products WHERE id = ?1",
            params![item.product_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
        
        if stock_qty < item.quantity {
            return Err(format!("Insufficient stock for product ID: {}", item.product_id).into());
        }
        
        // Update product stock
        tx.execute(
            "UPDATE products SET stock_quantity = stock_quantity - ?1, updated_at = ?2 WHERE id = ?3",
            params![item.quantity, &now, item.product_id],
        )?;

        // Insert order item
        tx.execute(
            "INSERT INTO order_items (order_id, product_id, quantity, sale_price, cost_price) 
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                order_id,
                item.product_id,
                item.quantity,
                sale_price,
                cost_price
            ],
        )?;

        total_amount += sale_price * (item.quantity as f64);
        total_cost += cost_price * (item.quantity as f64);
    }

    let profit = total_amount - total_cost;

    // 3. Update order with actual totals
    tx.execute(
        "UPDATE orders SET total_amount = ?1, profit = ?2 WHERE id = ?3",
        params![total_amount, profit, order_id],
    )?;

    tx.commit()?;
    Ok(order_id)
}

pub fn get_customer_purchase_history(conn: &Connection, customer_id: i64) -> Result<Vec<OrderHistory>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, order_date, total_amount, profit FROM orders WHERE customer_id = ?1 ORDER BY order_date DESC"
    )?;

    let order_iter = stmt.query_map(params![customer_id], |row| {
        let order_id: i64 = row.get(0)?;
        let order_date: String = row.get(1)?;
        let total_amount: f64 = row.get(2)?;
        let profit: f64 = row.get(3)?;
        Ok((order_id, order_date, total_amount, profit))
    })?;

    let mut history = Vec::new();
    for row in order_iter {
        let (order_id, order_date, total_amount, profit) = row?;
        
        // Get items for this order
        let mut item_stmt = conn.prepare(
            "SELECT p.name, p.sku, oi.quantity, oi.sale_price 
             FROM order_items oi
             JOIN products p ON oi.product_id = p.id
             WHERE oi.order_id = ?1"
        )?;
        
        let item_iter = item_stmt.query_map(params![order_id], |i_row| {
            Ok(OrderItemDetail {
                product_name: i_row.get(0)?,
                sku: i_row.get(1)?,
                quantity: i_row.get(2)?,
                sale_price: i_row.get(3)?,
            })
        })?;
        
        let mut items = Vec::new();
        for item in item_iter {
            items.push(item?);
        }

        history.push(OrderHistory {
            order_id,
            order_date,
            total_amount,
            profit,
            items,
        });
    }

    Ok(history)
}
