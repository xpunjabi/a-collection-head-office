use serde::{Serialize, Deserialize};
use rusqlite::{Connection, params};

#[derive(Debug, Serialize, Deserialize)]
pub struct SalesReport {
    pub total_sales: f64,
    pub total_profit: f64,
    pub total_orders: i64,
    pub avg_order_value: f64,
    pub start_date: String,
    pub end_date: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CategorySummary {
    pub category: String,
    pub count: i64,
    pub total_stock: i64,
    pub cost_value: f64,
    pub retail_value: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InventoryReport {
    pub total_items: i64,
    pub total_cost: f64,
    pub total_retail: f64,
    pub category_summaries: Vec<CategorySummary>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TopCustomer {
    pub customer_id: i64,
    pub name: String,
    pub phone: Option<String>,
    pub total_spent: f64,
    pub orders_count: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CustomerSummaryReport {
    pub total_customers: i64,
    pub total_orders: i64,
    pub total_spent: f64,
    pub top_customers: Vec<TopCustomer>,
}

pub fn generate_sales_report(
    conn: &Connection,
    start_date: &str,
    end_date: &str,
) -> Result<SalesReport, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT COUNT(*), SUM(total_amount), SUM(profit) 
         FROM orders 
         WHERE order_date BETWEEN ?1 AND ?2"
    )?;

    let report = stmt.query_row(params![start_date, end_date], |row| {
        let total_orders: i64 = row.get(0)?;
        let total_sales: f64 = row.get(1).unwrap_or(0.0);
        let total_profit: f64 = row.get(2).unwrap_or(0.0);
        let avg_order_value = if total_orders > 0 { total_sales / (total_orders as f64) } else { 0.0 };

        Ok(SalesReport {
            total_sales,
            total_profit,
            total_orders,
            avg_order_value,
            start_date: start_date.to_string(),
            end_date: end_date.to_string(),
        })
    })?;

    Ok(report)
}

pub fn generate_inventory_report(conn: &Connection) -> Result<InventoryReport, rusqlite::Error> {
    // 1. Get totals
    let mut stmt = conn.prepare(
        "SELECT SUM(stock_quantity), SUM(stock_quantity * cost_price), SUM(stock_quantity * sale_price) 
         FROM products WHERE status = 'active'"
    )?;

    let (total_items, total_cost, total_retail) = stmt.query_row([], |row| {
        let items: i64 = row.get(0).unwrap_or(0);
        let cost: f64 = row.get(1).unwrap_or(0.0);
        let retail: f64 = row.get(2).unwrap_or(0.0);
        Ok((items, cost, retail))
    })?;

    // 2. Get category summaries
    let mut cat_stmt = conn.prepare(
        "SELECT 
            COALESCE(category, 'Uncategorized') as cat, 
            COUNT(*), 
            SUM(stock_quantity), 
            SUM(stock_quantity * cost_price), 
            SUM(stock_quantity * sale_price)
         FROM products 
         WHERE status = 'active'
         GROUP BY cat"
    )?;

    let cat_iter = cat_stmt.query_map([], |row| {
        Ok(CategorySummary {
            category: row.get(0)?,
            count: row.get(1)?,
            total_stock: row.get(2).unwrap_or(0),
            cost_value: row.get(3).unwrap_or(0.0),
            retail_value: row.get(4).unwrap_or(0.0),
        })
    })?;

    let mut category_summaries = Vec::new();
    for cat in cat_iter {
        category_summaries.push(cat?);
    }

    Ok(InventoryReport {
        total_items,
        total_cost,
        total_retail,
        category_summaries,
    })
}

pub fn generate_customer_report(conn: &Connection) -> Result<CustomerSummaryReport, rusqlite::Error> {
    // Total customers
    let total_customers: i64 = conn.query_row("SELECT COUNT(*) FROM customers", [], |row| row.get(0))?;
    
    // Total orders and spent
    let (total_orders, total_spent) = conn.query_row(
        "SELECT COUNT(*), SUM(total_amount) FROM orders",
        [],
        |row| Ok((row.get(0).unwrap_or(0), row.get(1).unwrap_or(0.0)))
    )?;

    // Top customers
    let mut top_stmt = conn.prepare(
        "SELECT c.id, c.name, c.phone, SUM(o.total_amount) as spent, COUNT(o.id) as orders_count 
         FROM customers c
         JOIN orders o ON c.id = o.customer_id
         GROUP BY c.id
         ORDER BY spent DESC
         LIMIT 5"
    )?;

    let top_iter = top_stmt.query_map([], |row| {
        Ok(TopCustomer {
            customer_id: row.get(0)?,
            name: row.get(1)?,
            phone: row.get(2)?,
            total_spent: row.get(3).unwrap_or(0.0),
            orders_count: row.get(4).unwrap_or(0),
        })
    })?;

    let mut top_customers = Vec::new();
    for tc in top_iter {
        top_customers.push(tc?);
    }

    Ok(CustomerSummaryReport {
        total_customers,
        total_orders,
        total_spent,
        top_customers,
    })
}
