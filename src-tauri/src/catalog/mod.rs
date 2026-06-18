use serde::{Serialize, Deserialize};
use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};
use std::fs;
use image::{ImageReader, imageops::FilterType};
use csv::{ReaderBuilder, WriterBuilder};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Product {
    pub id: Option<i64>,
    pub sku: String,
    pub name: String,
    pub category: Option<String>,
    pub cost_price: f64,
    pub sale_price: f64,
    pub description: Option<String>,
    pub tags: Option<String>,
    pub stock_quantity: i64,
    pub status: String,
    pub images: String, // Stringified JSON array of local image paths/names
    pub created_at: String,
    pub updated_at: String,
}

pub fn get_all_products(conn: &Connection) -> Result<Vec<Product>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, sku, name, category, cost_price, sale_price, description, tags, stock_quantity, status, images, created_at, updated_at 
         FROM products ORDER BY id DESC"
    )?;
    
    let product_iter = stmt.query_map([], |row| {
        Ok(Product {
            id: Some(row.get(0)?),
            sku: row.get(1)?,
            name: row.get(2)?,
            category: row.get(3)?,
            cost_price: row.get(4)?,
            sale_price: row.get(5)?,
            description: row.get(6)?,
            tags: row.get(7)?,
            stock_quantity: row.get(8)?,
            status: row.get(9)?,
            images: row.get(10)?,
            created_at: row.get(11)?,
            updated_at: row.get(12)?,
        })
    })?;

    let mut products = Vec::new();
    for product in product_iter {
        products.push(product?);
    }
    Ok(products)
}

pub fn add_product(conn: &Connection, product: &Product) -> Result<i64, rusqlite::Error> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO products (sku, name, category, cost_price, sale_price, description, tags, stock_quantity, status, images, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        (
            &product.sku,
            &product.name,
            &product.category,
            product.cost_price,
            product.sale_price,
            &product.description,
            &product.tags,
            product.stock_quantity,
            &product.status,
            &product.images,
            &now,
            &now
        ),
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update_product(conn: &Connection, product: &Product) -> Result<(), rusqlite::Error> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE products SET sku = ?1, name = ?2, category = ?3, cost_price = ?4, sale_price = ?5, 
         description = ?6, tags = ?7, stock_quantity = ?8, status = ?9, images = ?10, updated_at = ?11 
         WHERE id = ?12",
        (
            &product.sku,
            &product.name,
            &product.category,
            product.cost_price,
            product.sale_price,
            &product.description,
            &product.tags,
            product.stock_quantity,
            &product.status,
            &product.images,
            &now,
            product.id
        ),
    )?;
    Ok(())
}

pub fn delete_product(conn: &Connection, id: i64) -> Result<(), rusqlite::Error> {
    conn.execute("DELETE FROM products WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn export_to_csv(conn: &Connection) -> Result<String, Box<dyn std::error::Error>> {
    let products = get_all_products(conn)?;
    let mut wtr = WriterBuilder::new().from_writer(vec![]);
    
    // Write headers
    wtr.write_record(&["SKU", "Name", "Category", "Cost Price", "Sale Price", "Description", "Tags", "Stock Quantity", "Status"])?;
    
    for p in products {
        wtr.write_record(&[
            p.sku,
            p.name,
            p.category.unwrap_or_default(),
            p.cost_price.to_string(),
            p.sale_price.to_string(),
            p.description.unwrap_or_default(),
            p.tags.unwrap_or_default(),
            p.stock_quantity.to_string(),
            p.status,
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
        if record.len() < 9 { continue; }
        
        let sku = record[0].to_string();
        let name = record[1].to_string();
        let category = if record[2].is_empty() { None } else { Some(record[2].to_string()) };
        let cost_price: f64 = record[3].parse().unwrap_or(0.0);
        let sale_price: f64 = record[4].parse().unwrap_or(0.0);
        let description = if record[5].is_empty() { None } else { Some(record[5].to_string()) };
        let tags = if record[6].is_empty() { None } else { Some(record[6].to_string()) };
        let stock_quantity: i64 = record[7].parse().unwrap_or(0);
        let status = record[8].to_string();

        conn.execute(
            "INSERT OR REPLACE INTO products (sku, name, category, cost_price, sale_price, description, tags, stock_quantity, status, images, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, '[]', ?10, ?11)",
            (
                &sku,
                &name,
                &category,
                cost_price,
                sale_price,
                &description,
                &tags,
                stock_quantity,
                &status,
                &now,
                &now
            ),
        )?;
    }
    Ok(())
}

pub fn process_and_save_image(
    src_path: &Path,
    app_images_dir: &Path,
    format_type: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    fs::create_dir_all(app_images_dir)?;
    
    // Generate unique file name
    let uuid_str = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0).to_string();
    let file_name = format!("{}_{}.jpg", uuid_str, format_type);
    let dest_path = app_images_dir.join(&file_name);
    
    // Open image
    let img = ImageReader::open(src_path)?.decode()?;
    
    // Target dimensions based on social media recommendations
    let (width, height) = match format_type {
        "instagram" => (1080, 1080),  // Square 1:1
        "facebook" => (1200, 630),   // Shared image 1.91:1
        "whatsapp" => (800, 800),     // WhatsApp status/catalog
        "thumbnail" => (200, 200),    // Thumbnail
        _ => (1080, 1080),            // Default square
    };
    
    // Resize image (fill/cover style resize or thumbnail)
    let resized = if format_type == "thumbnail" {
        img.resize_to_fill(width, height, FilterType::Lanczos3)
    } else {
        // Normal social media sizing
        img.resize(width, height, FilterType::Lanczos3)
    };
    
    // Save as JPEG with compression (implied by image save wrapper)
    resized.save(&dest_path)?;
    
    Ok(file_name)
}
