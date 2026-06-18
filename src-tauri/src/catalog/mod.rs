use serde::{Serialize, Deserialize};
use rusqlite::{Connection, params};
use std::path::Path;
use std::fs;
use image::{ImageReader, imageops::FilterType};
use csv::{ReaderBuilder, WriterBuilder};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Product {
    pub id: Option<i64>,
    pub sku: String,
    pub name: String,
    pub category: Option<String>,
    pub color: Option<String>,
    pub design: Option<String>,
    pub season: Option<String>,
    pub cost_price: f64,
    pub sale_price: f64,
    pub purchase_price: f64,
    pub description: Option<String>,
    pub tags: Option<String>,
    pub stock_quantity: i64,
    pub status: String,
    pub images: String,
    pub supplier_id: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProductLocationStock {
    pub location_id: i64,
    pub location_name: String,
    pub quantity: i64,
}

pub fn get_all_products(conn: &Connection) -> Result<Vec<Product>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, COALESCE(sku,''), name, category, color, design, season,
                cost_price, sale_price, COALESCE(purchase_price, cost_price),
                description, tags, stock_quantity, status, images, supplier_id, created_at, updated_at
         FROM products ORDER BY id DESC"
    )?;
    let product_iter = stmt.query_map([], |row| {
        Ok(Product {
            id: Some(row.get(0)?),
            sku: row.get(1)?,
            name: row.get(2)?,
            category: row.get(3)?,
            color: row.get(4)?,
            design: row.get(5)?,
            season: row.get(6)?,
            cost_price: row.get(7)?,
            sale_price: row.get(8)?,
            purchase_price: row.get(9)?,
            description: row.get(10)?,
            tags: row.get(11)?,
            stock_quantity: row.get(12)?,
            status: row.get(13)?,
            images: row.get(14)?,
            supplier_id: row.get(15)?,
            created_at: row.get(16)?,
            updated_at: row.get(17)?,
        })
    })?;
    let mut products = Vec::new();
    for p in product_iter { products.push(p?); }
    Ok(products)
}

pub fn get_product_by_id(conn: &Connection, id: i64) -> Result<Product, rusqlite::Error> {
    conn.query_row(
        "SELECT id, COALESCE(sku,''), name, category, color, design, season,
                cost_price, sale_price, COALESCE(purchase_price, cost_price),
                description, tags, stock_quantity, status, images, supplier_id, created_at, updated_at
         FROM products WHERE id = ?1",
        [id],
        |row| {
            Ok(Product {
                id: Some(row.get(0)?),
                sku: row.get(1)?,
                name: row.get(2)?,
                category: row.get(3)?,
                color: row.get(4)?,
                design: row.get(5)?,
                season: row.get(6)?,
                cost_price: row.get(7)?,
                sale_price: row.get(8)?,
                purchase_price: row.get(9)?,
                description: row.get(10)?,
                tags: row.get(11)?,
                stock_quantity: row.get(12)?,
                status: row.get(13)?,
                images: row.get(14)?,
                supplier_id: row.get(15)?,
                created_at: row.get(16)?,
                updated_at: row.get(17)?,
            })
        },
    )
}

pub fn get_product_locations(conn: &Connection, product_id: i64) -> Result<Vec<ProductLocationStock>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT pl.location_id, l.name, pl.quantity
         FROM product_locations pl JOIN locations l ON l.id = pl.location_id
         WHERE pl.product_id = ?1 AND l.is_active = 1
         ORDER BY l.name"
    )?;
    let rows = stmt.query_map([product_id], |row| {
        Ok(ProductLocationStock {
            location_id: row.get(0)?,
            location_name: row.get(1)?,
            quantity: row.get(2)?,
        })
    })?;
    let mut result = Vec::new();
    for r in rows { result.push(r?); }
    Ok(result)
}

pub fn upsert_product_location(conn: &Connection, product_id: i64, location_id: i64, quantity: i64) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT INTO product_locations (product_id, location_id, quantity) VALUES (?1, ?2, ?3)
         ON CONFLICT(product_id, location_id) DO UPDATE SET quantity = ?3",
        params![product_id, location_id, quantity],
    )?;
    // Update total stock_quantity
    let total: i64 = conn.query_row(
        "SELECT COALESCE(SUM(quantity),0) FROM product_locations WHERE product_id = ?1",
        [product_id],
        |r| r.get(0),
    )?;
    conn.execute("UPDATE products SET stock_quantity = ?1 WHERE id = ?2", params![total, product_id])?;
    Ok(())
}

pub fn add_product(conn: &Connection, product: &Product) -> Result<i64, rusqlite::Error> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO products (sku, name, category, color, design, season, cost_price, sale_price, purchase_price, description, tags, stock_quantity, status, images, supplier_id, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?16)",
        rusqlite::params![
            &product.sku, &product.name, &product.category,
            &product.color, &product.design, &product.season,
            product.cost_price, product.sale_price, product.purchase_price,
            &product.description, &product.tags, product.stock_quantity,
            &product.status, &product.images, product.supplier_id, &now,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update_product(conn: &Connection, product: &Product) -> Result<(), rusqlite::Error> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE products SET sku=?1, name=?2, category=?3, color=?4, design=?5, season=?6,
         cost_price=?7, sale_price=?8, purchase_price=?9, description=?10, tags=?11, stock_quantity=?12,
         status=?13, images=?14, supplier_id=?15, updated_at=?16 WHERE id=?17",
        rusqlite::params![
            &product.sku, &product.name, &product.category,
            &product.color, &product.design, &product.season,
            product.cost_price, product.sale_price, product.purchase_price,
            &product.description, &product.tags, product.stock_quantity,
            &product.status, &product.images, product.supplier_id,
            &now, product.id,
        ],
    )?;
    Ok(())
}

pub fn delete_product(conn: &Connection, id: i64) -> Result<(), rusqlite::Error> {
    conn.execute("DELETE FROM product_locations WHERE product_id = ?1", params![id])?;
    conn.execute("DELETE FROM products WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn export_to_csv(conn: &Connection) -> Result<String, Box<dyn std::error::Error>> {
    let products = get_all_products(conn)?;
    let mut wtr = WriterBuilder::new().from_writer(vec![]);
    wtr.write_record(&["Product Code", "Name", "Category", "Color", "Design", "Season",
        "Cost Price", "Sale Price", "Description", "Tags", "Stock", "Status"])?;
    for p in products {
        wtr.write_record(&[
            p.sku, p.name, p.category.unwrap_or_default(),
            p.color.unwrap_or_default(), p.design.unwrap_or_default(),
            p.season.unwrap_or_default(), p.cost_price.to_string(),
            p.sale_price.to_string(), p.description.unwrap_or_default(),
            p.tags.unwrap_or_default(), p.stock_quantity.to_string(), p.status,
        ])?;
    }
    let data = String::from_utf8(wtr.into_inner()?)?;
    Ok(data)
}

pub fn import_from_csv(conn: &Connection, csv_content: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut rdr = ReaderBuilder::new().from_reader(csv_content.as_bytes());
    let now = chrono::Utc::now().to_rfc3339();
    for result in rdr.records() {
        let record = result?;
        if record.len() < 12 { continue; }
        conn.execute(
            "INSERT INTO products (sku, name, category, color, design, season, cost_price, sale_price, purchase_price, description, tags, stock_quantity, status, images, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?7, ?9, ?10, ?11, ?12, '[]', ?13, ?13)",
            rusqlite::params![&record[0], &record[1], &record[2], &record[3], &record[4], &record[5],
             record[6].parse::<f64>().unwrap_or(0.0), record[7].parse::<f64>().unwrap_or(0.0),
             &record[8], &record[9], record[10].parse::<i64>().unwrap_or(0), &record[11], &now],
        )?;
    }
    Ok(())
}

pub fn process_and_save_image(src_path: &Path, app_images_dir: &Path, format_type: &str) -> Result<String, Box<dyn std::error::Error>> {
    fs::create_dir_all(app_images_dir)?;
    let uuid_str = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0).to_string();
    let file_name = format!("{}_{}.jpg", uuid_str, format_type);
    let dest_path = app_images_dir.join(&file_name);
    let img = ImageReader::open(src_path)?.decode()?;
    let (width, height) = match format_type {
        "instagram" => (1080, 1080),
        "facebook" => (1200, 630),
        "whatsapp" => (800, 800),
        "thumbnail" => (200, 200),
        _ => (1080, 1080),
    };
    let resized = if format_type == "thumbnail" {
        img.resize_to_fill(width, height, FilterType::Lanczos3)
    } else {
        img.resize(width, height, FilterType::Lanczos3)
    };
    resized.save(&dest_path)?;
    Ok(file_name)
}

pub fn search_by_color(conn: &Connection, color: &str) -> Result<Vec<Product>, rusqlite::Error> {
    let search = format!("%{}%", color);
    let mut stmt = conn.prepare(
        "SELECT id, COALESCE(sku,''), name, category, color, design, season,
                cost_price, sale_price, COALESCE(purchase_price, cost_price),
                description, tags, stock_quantity, status, images, supplier_id, created_at, updated_at
         FROM products WHERE color LIKE ?1 AND status='active' ORDER BY name"
    )?;
    let rows = stmt.query_map([&search], |row| {
        Ok(Product {
            id: Some(row.get(0)?), sku: row.get(1)?, name: row.get(2)?,
            category: row.get(3)?, color: row.get(4)?, design: row.get(5)?, season: row.get(6)?,
            cost_price: row.get(7)?, sale_price: row.get(8)?, purchase_price: row.get(9)?,
            description: row.get(10)?, tags: row.get(11)?, stock_quantity: row.get(12)?,
            status: row.get(13)?, images: row.get(14)?, supplier_id: row.get(15)?,
            created_at: row.get(16)?, updated_at: row.get(17)?,
        })
    })?;
    let mut products = Vec::new();
    for p in rows { products.push(p?); }
    Ok(products)
}
