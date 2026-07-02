//! v0.15.0: Public Catalog Publishing Module
//!
//! Handles exporting public product data to a separate GitHub repo
//! (a-collection-catalog) which is served as a PWA via GitHub Pages.
//!
//! Architecture:
//! - HO (private Tauri app) is the single source of truth
//! - Catalog repo (public) only contains frontend + generated data/
//! - Publishing uses GitHub Contents API (atomic file-level updates)
//! - Only allowlist fields are exported (cost, supplier, profit stay private)
//!
//! Publishing flow:
//! 1. export_catalog_json() — build catalog.json with allowlist fields
//! 2. generate_webp_images() — resize + convert images to WebP (400px + 800px)
//! 3. preview_catalog() — return stats (product count, image count, total size)
//! 4. publish_catalog() — PUT catalog.json + images via GitHub Contents API,
//!    DELETE orphan images (products no longer in catalog)

use rusqlite::Connection;
use serde::{Serialize, Deserialize};
use crate::catalog;
use crate::utils;
use std::collections::HashMap;

// ============================================================
// PUBLIC TYPES
// ============================================================

/// Public-facing product representation. Only allowlist fields —
/// cost_price, supplier_id, profit_status, etc. are NEVER included.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PublicProduct {
    pub id: i64,
    pub name: String,
    pub sku: Option<String>,
    pub sale_price: f64,
    pub retail_price: Option<f64>,
    pub category: Option<String>,
    pub color: Option<String>,
    pub fabric: Option<String>,
    pub season: Option<String>,
    pub description: Option<String>,
    pub images: Vec<String>,
    pub availability: String,
}

/// Top-level catalog.json structure
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CatalogJson {
    pub brand: String,
    pub whatsapp_number: String,
    pub version: String,
    pub published_at: String,
    pub products: Vec<PublicProduct>,
}

/// Stats returned by preview_catalog() for the UI confirmation modal
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CatalogPreview {
    pub product_count: usize,
    pub image_count: usize,
    pub total_size_bytes: usize,
    pub total_size_display: String,
    pub brand: String,
    pub whatsapp_number: String,
    pub repo: String,
    pub catalog_url: String,
    /// v0.16.0: Validation warnings — problems that won't block publish
    /// but the user should be aware of (missing images, empty fields, etc.)
    pub warnings: Vec<CatalogWarning>,
    /// v0.16.0: Validation errors — problems that should be fixed before
    /// publishing. UI shows them prominently.
    pub errors: Vec<CatalogError>,
}

/// v0.16.0: A non-blocking warning about a product that will be published
/// but has incomplete data. Shown in preview modal as yellow warning.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CatalogWarning {
    pub product_id: i64,
    pub product_name: String,
    pub issue: String,
    pub severity: String,  // "warning" | "info"
}

/// v0.16.0: A blocking error — publish cannot proceed. Currently we don't
/// block publish (user can always override), but if errors exist the UI
/// shows them prominently and requires explicit confirmation.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CatalogError {
    pub product_id: i64,
    pub product_name: String,
    pub issue: String,
}

/// Result of a publish operation
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PublishResult {
    pub success: bool,
    pub products_published: usize,
    pub images_uploaded: usize,
    pub images_deleted: usize,
    pub catalog_url: String,
    pub errors: Vec<String>,
}

// ============================================================
// EXPORT: Build catalog.json from SQLite
// ============================================================

/// Build the public CatalogJson from the local SQLite database.
/// Only active products are exported. Allowlist fields only.
/// v0.16.3: Deduplicates by name (case-insensitive) — if two products have
/// the same name, only the first one (by id, newest) is published.
pub fn build_catalog_json(
    conn: &Connection,
    brand: &str,
    whatsapp_number: &str,
) -> Result<CatalogJson, String> {
    let products = catalog::get_all_products(conn).map_err(|e| e.to_string())?;

    // v0.16.3: Track seen names to prevent duplicates on the public catalog.
    // Key = lowercase name, value = (). If we've seen this name, skip.
    let mut seen_names: std::collections::HashSet<String> = std::collections::HashSet::new();

    let public_products: Vec<PublicProduct> = products
        .into_iter()
        .filter(|p| p.status == "active")  // Only publish active products
        .filter(|p| {
            // v0.16.3: Dedup by name (case-insensitive)
            let key = p.name.trim().to_lowercase();
            if key.is_empty() { return false; }
            seen_names.insert(key)
        })
        .map(|p| {
            let images: Vec<String> = serde_json::from_str(&p.images)
                .unwrap_or_default();

            let availability = if p.profit_status.as_deref() == Some("sold_out") {
                "sold_out".to_string()
            } else {
                "available".to_string()
            };

            PublicProduct {
                id: p.id.unwrap_or(0),
                name: p.name,
                sku: if p.sku.is_empty() { None } else { Some(p.sku) },
                sale_price: p.sale_price,
                retail_price: p.retail_price.filter(|&r| r > 0.0),
                category: p.category,
                color: p.color,
                fabric: p.fabric,
                season: p.season,
                description: p.description,
                images,
                availability,
            }
        })
        .collect();

    let now = chrono::Utc::now().to_rfc3339();

    Ok(CatalogJson {
        brand: brand.to_string(),
        whatsapp_number: whatsapp_number.to_string(),
        version: now.clone(),
        published_at: now,
        products: public_products,
    })
}

// ============================================================
// IMAGE: Generate WebP versions for catalog
// ============================================================

/// Generate WebP images for the catalog. For each product image:
/// - Generate a 400px version (for grid display)
/// - Image is saved as <original_filename_without_ext>.webp
///
/// Returns a map of original_filename → webp_filename.
/// WebP is 30% smaller than JPEG at equivalent quality.
pub fn generate_webp_images(
    conn: &Connection,
) -> Result<HashMap<String, String>, String> {
    use image::ImageReader;

    let products = catalog::get_all_products(conn).map_err(|e| e.to_string())?;
    let images_dir = utils::get_images_dir();
    let mut mapping: HashMap<String, String> = HashMap::new();

    for product in &products {
        if product.status != "active" { continue; }
        let images: Vec<String> = serde_json::from_str(&product.images).unwrap_or_default();
        for original_filename in images {
            if original_filename.is_empty() { continue; }
            // Skip if already processed
            if mapping.contains_key(&original_filename) { continue; }

            let src_path = images_dir.join(&original_filename);
            if !src_path.exists() {
                continue;
            }

            // Generate catalog image filename: <stem>_catalog.jpg
            let stem = std::path::Path::new(&original_filename)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("image");
            let final_filename = format!("{}_catalog.jpg", stem);
            let final_path = images_dir.join(&final_filename);

            // Load + resize to 400px max (preserves aspect ratio)
            match ImageReader::open(&src_path) {
                Ok(reader) => match reader.decode() {
                    Ok(img) => {
                        let resized = img.resize(400, 400, image::imageops::FilterType::Lanczos3);
                        match resized.save_with_format(&final_path, image::ImageFormat::Jpeg) {
                            Ok(_) => {
                                mapping.insert(original_filename, final_filename);
                            }
                            Err(e) => {
                                eprintln!("[catalog_publish] Failed to save {}: {}", final_filename, e);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("[catalog_publish] Failed to decode {}: {}", original_filename, e);
                    }
                },
                Err(e) => {
                    eprintln!("[catalog_publish] Failed to open {}: {}", original_filename, e);
                }
            }
        }
    }

    Ok(mapping)
}

// ============================================================
// PREVIEW: Build stats for confirmation modal
// ============================================================

/// Build a preview summary of what will be published.
/// Called by the UI before the user confirms the publish action.
/// v0.16.0: Now also runs validation and includes warnings/errors.
pub fn build_preview(
    conn: &Connection,
    brand: &str,
    whatsapp_number: &str,
    repo: &str,
) -> Result<CatalogPreview, String> {
    let catalog = build_catalog_json(conn, brand, whatsapp_number)?;
    let images_dir = utils::get_images_dir();

    // Count unique images across all products + calculate total size
    let mut unique_images: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut total_size: u64 = 0;

    for product in &catalog.products {
        for img in &product.images {
            if unique_images.insert(img.clone()) {
                let path = images_dir.join(img);
                if let Ok(meta) = std::fs::metadata(&path) {
                    total_size += meta.len();
                }
            }
        }
    }

    // Add catalog.json size estimate (~500 bytes per product)
    total_size += (catalog.products.len() as u64) * 500 + 200;

    let size_display = if total_size < 1024 {
        format!("{} B", total_size)
    } else if total_size < 1024 * 1024 {
        format!("{:.1} KB", total_size as f64 / 1024.0)
    } else {
        format!("{:.2} MB", total_size as f64 / (1024.0 * 1024.0))
    };

    // Build catalog URL: https://<owner>.github.io/<repo-name>/
    let (owner, repo_name) = match repo.split_once('/') {
        Some((o, r)) => (o, r),
        None => ("xpunjabi", "a-collection-catalog"),
    };
    let catalog_url = format!("https://{}.github.io/{}/", owner, repo_name);

    // v0.16.0: Run validation
    let (warnings, errors) = validate_catalog(&catalog, whatsapp_number, &images_dir);

    Ok(CatalogPreview {
        product_count: catalog.products.len(),
        image_count: unique_images.len(),
        total_size_bytes: total_size as usize,
        total_size_display: size_display,
        brand: brand.to_string(),
        whatsapp_number: whatsapp_number.to_string(),
        repo: repo.to_string(),
        catalog_url,
        warnings,
        errors,
    })
}

/// v0.16.0: Validate catalog before publishing. Returns (warnings, errors).
/// Warnings are non-blocking (missing images, empty optional fields).
/// Errors are blocking (empty name, zero price, duplicate SKU).
///
/// We don't actually block publish on errors — the UI shows them and lets
/// the user override. But this gives them a chance to fix issues first.
fn validate_catalog(
    catalog: &CatalogJson,
    whatsapp_number: &str,
    images_dir: &std::path::Path,
) -> (Vec<CatalogWarning>, Vec<CatalogError>) {
    let mut warnings = Vec::new();
    let mut errors = Vec::new();

    // Global validation: WhatsApp number
    let clean_wa = whatsapp_number.replace(|c: char| !c.is_ascii_digit(), "");
    if clean_wa.is_empty() {
        errors.push(CatalogError {
            product_id: 0,
            product_name: "(global)".to_string(),
            issue: "WhatsApp number is empty — customers won't be able to order. Set it in Settings → Catalog.".to_string(),
        });
    } else if clean_wa.len() < 10 || clean_wa.len() > 15 {
        warnings.push(CatalogWarning {
            product_id: 0,
            product_name: "(global)".to_string(),
            issue: format!("WhatsApp number '{}' looks unusual ({} digits). Expected 10-15 digits with country code.", clean_wa, clean_wa.len()),
            severity: "warning".to_string(),
        });
    }

    // Track SKUs for duplicate detection
    let mut seen_skus: std::collections::HashMap<String, i64> = std::collections::HashMap::new();

    for product in &catalog.products {
        let name = &product.name;
        // v0.16.1: Use a fresh clone for each push to avoid "use of moved value"
        let display_name = || -> String {
            if name.is_empty() { "(unnamed product)".to_string() } else { name.clone() }
        };

        // ERROR: Empty name
        if name.trim().is_empty() {
            errors.push(CatalogError {
                product_id: product.id,
                product_name: display_name(),
                issue: "Product name is empty.".to_string(),
            });
        }

        // ERROR: Zero or negative sale price
        if product.sale_price <= 0.0 {
            errors.push(CatalogError {
                product_id: product.id,
                product_name: display_name(),
                issue: "Sale price is 0 or negative. Customers won't know the cost.".to_string(),
            });
        }

        // ERROR: Duplicate SKU
        if let Some(sku) = &product.sku {
            if !sku.trim().is_empty() {
                if let Some(_prev_id) = seen_skus.get(sku) {
                    errors.push(CatalogError {
                        product_id: product.id,
                        product_name: display_name(),
                        issue: format!("Duplicate SKU '{}'. Each product must have a unique SKU.", sku),
                    });
                } else {
                    seen_skus.insert(sku.clone(), product.id);
                }
            }
        }

        // WARNING: No images
        if product.images.is_empty() {
            warnings.push(CatalogWarning {
                product_id: product.id,
                product_name: display_name(),
                issue: "No product image — will show placeholder on catalog.".to_string(),
                severity: "warning".to_string(),
            });
        } else {
            // WARNING: Image file missing on disk
            for img in &product.images {
                if !img.is_empty() {
                    let path = images_dir.join(img);
                    if !path.exists() {
                        warnings.push(CatalogWarning {
                            product_id: product.id,
                            product_name: display_name(),
                            issue: format!("Image file '{}' not found on disk.", img),
                            severity: "warning".to_string(),
                        });
                    }
                }
            }
        }

        // WARNING: No category
        if product.category.as_ref().map_or(true, |c| c.trim().is_empty()) {
            warnings.push(CatalogWarning {
                product_id: product.id,
                product_name: display_name(),
                issue: "No category set — customers can't filter by category.".to_string(),
                severity: "info".to_string(),
            });
        }

        // INFO: No description
        if product.description.as_ref().map_or(true, |d| d.trim().is_empty()) {
            warnings.push(CatalogWarning {
                product_id: product.id,
                product_name: display_name(),
                issue: "No description — customers see less product info.".to_string(),
                severity: "info".to_string(),
            });
        }

        // WARNING: retail_price < sale_price (looks like a markup, probably wrong)
        if let Some(retail) = product.retail_price {
            if retail > 0.0 && retail < product.sale_price {
                warnings.push(CatalogWarning {
                    product_id: product.id,
                    product_name: display_name(),
                    issue: format!("Retail price (Rs. {:.0}) is less than sale price (Rs. {:.0}). Discount will show negative.", retail, product.sale_price),
                    severity: "warning".to_string(),
                });
            }
        }
    }

    (warnings, errors)
}

// ============================================================
// PUBLISH: Upload to GitHub via Contents API
// ============================================================

/// Upload catalog.json + all images to GitHub via Contents API.
/// v0.15.2: This is the ASYNC-ONLY part of the publish flow. The sync
/// data prep (build_catalog_json, generate_webp_images) is done BEFORE
/// calling this function, while the DB lock is still held. This function
/// does NO database access — it only does HTTP calls to GitHub.
///
/// This separation is required because rusqlite::Connection is not Send,
/// so we can't hold the Mutex<Connection> across .await points.
///
/// GitHub Contents API:
///   GET  /repos/{owner}/{repo}/contents/{path}  → returns SHA if file exists
///   PUT  /repos/{owner}/{repo}/contents/{path}  → create (no SHA) or update (with SHA)
///   DELETE /repos/{owner}/{repo}/contents/{path}  → delete (requires SHA)
pub async fn upload_to_github(
    catalog: &CatalogJson,
    image_mapping: &HashMap<String, String>,
    repo: &str,
    github_token: &str,
) -> Result<PublishResult, String> {
    use base64::Engine as _;

    let mut errors: Vec<String> = Vec::new();
    let mut images_uploaded = 0usize;
    let mut images_deleted = 0usize;
    let products_published = catalog.products.len();

    // Build HTTP client
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("a-collection-head-office/0.15.2")
        .build()
        .map_err(|e| format!("HTTP client build failed: {}", e))?;

    let api_base = format!("https://api.github.com/repos/{}/contents", repo);

    // Upload catalog.json
    let catalog_json_str = serde_json::to_string_pretty(catalog)
        .map_err(|e| format!("JSON serialize failed: {}", e))?;
    let catalog_b64 = base64::engine::general_purpose::STANDARD.encode(catalog_json_str.as_bytes());

    let catalog_path = "data/catalog.json";
    if let Err(e) = upload_file(
        &client, &api_base, github_token, catalog_path,
        &format!("Update catalog.json — {} products, {}", products_published, catalog.version),
        &catalog_b64,
    ).await {
        errors.push(format!("catalog.json: {}", e));
    }

    // Upload all catalog images
    let images_dir = utils::get_images_dir();
    let mut uploaded_image_filenames: std::collections::HashSet<String> = std::collections::HashSet::new();

    for (_original_filename, catalog_filename) in image_mapping {
        let src_path = images_dir.join(catalog_filename);
        match std::fs::read(&src_path) {
            Ok(bytes) => {
                let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                let path = format!("data/images/{}", catalog_filename);
                let msg = format!("Update image: {}", catalog_filename);
                match upload_file(&client, &api_base, github_token, &path, &msg, &b64).await {
                    Ok(_) => {
                        images_uploaded += 1;
                        uploaded_image_filenames.insert(catalog_filename.clone());
                    },
                    Err(e) => errors.push(format!("image {}: {}", catalog_filename, e)),
                }
            },
            Err(e) => errors.push(format!("read {}: {}", catalog_filename, e)),
        }
    }

    // Delete orphan images (files in data/images/ that aren't in current catalog)
    if let Ok(existing_files) = list_repo_directory(&client, repo, github_token, "data/images").await {
        for file in existing_files {
            if !uploaded_image_filenames.contains(&file.name) {
                if let Err(e) = delete_file(
                    &client, &api_base, github_token,
                    &format!("data/images/{}", file.name),
                    &format!("Delete orphan image: {}", file.name),
                    &file.sha,
                ).await {
                    errors.push(format!("delete {}: {}", file.name, e));
                } else {
                    images_deleted += 1;
                }
            }
        }
    }

    // Build catalog URL
    let (owner, repo_name) = match repo.split_once('/') {
        Some((o, r)) => (o, r),
        None => ("xpunjabi", "a-collection-catalog"),
    };
    let catalog_url = format!("https://{}.github.io/{}/", owner, repo_name);

    let success = errors.is_empty();
    Ok(PublishResult {
        success,
        products_published,
        images_uploaded,
        images_deleted,
        catalog_url,
        errors,
    })
}

// ============================================================
// GITHUB API HELPERS
// ============================================================

#[derive(Debug, Deserialize)]
struct GitHubContent {
    name: String,
    sha: String,
}

/// Upload a file via Contents API PUT.
/// First GETs the existing file to fetch its SHA (needed for updates).
/// If file doesn't exist (404), creates it without SHA.
async fn upload_file(
    client: &reqwest::Client,
    api_base: &str,
    token: &str,
    path: &str,
    message: &str,
    content_b64: &str,
) -> Result<(), String> {
    // Get existing SHA (if file exists)
    let get_url = format!("{}/{}", api_base, path);
    let existing_sha: Option<String> = match client
        .get(&get_url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
    {
        Ok(resp) => {
            if resp.status() == reqwest::StatusCode::OK {
                match resp.json::<serde_json::Value>().await {
                    Ok(json) => json.get("sha").and_then(|s| s.as_str()).map(|s| s.to_string()),
                    Err(_) => None,
                }
            } else {
                None
            }
        }
        Err(_) => None,
    };

    // Build PUT body
    let mut body = serde_json::json!({
        "message": message,
        "content": content_b64,
    });
    if let Some(sha) = existing_sha {
        body["sha"] = serde_json::Value::String(sha);
    }

    // PUT request
    let resp = client
        .put(&get_url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Accept", "application/vnd.github+json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("PUT failed: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("HTTP {}: {}", status, body));
    }

    Ok(())
}

/// List files in a directory of the repo. Returns Vec of (name, sha).
async fn list_repo_directory(
    client: &reqwest::Client,
    repo: &str,
    token: &str,
    path: &str,
) -> Result<Vec<GitHubContent>, String> {
    let url = format!("https://api.github.com/repos/{}/contents/{}", repo, path);
    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| format!("GET listing failed: {}", e))?;

    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(Vec::new());  // Directory doesn't exist yet
    }
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }

    let json: serde_json::Value = resp.json().await.map_err(|e| format!("JSON parse: {}", e))?;
    let files = json.as_array().ok_or("Expected array response")?;
    Ok(files.iter().filter_map(|f| {
        let name = f.get("name")?.as_str()?.to_string();
        let sha = f.get("sha")?.as_str()?.to_string();
        // Only return files (not subdirectories)
        if f.get("type").and_then(|t| t.as_str()) == Some("file") {
            Some(GitHubContent { name, sha })
        } else {
            None
        }
    }).collect())
}

/// Delete a file via Contents API DELETE.
async fn delete_file(
    client: &reqwest::Client,
    api_base: &str,
    token: &str,
    path: &str,
    message: &str,
    sha: &str,
) -> Result<(), String> {
    let url = format!("{}/{}", api_base, path);
    let body = serde_json::json!({
        "message": message,
        "sha": sha,
    });

    let resp = client
        .delete(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Accept", "application/vnd.github+json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("DELETE failed: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("HTTP {}: {}", status, body));
    }

    Ok(())
}

// ============================================================
// PUBLISH HISTORY: Log + Query
// ============================================================

use rusqlite::params;

/// v0.16.0: A row in the catalog_publish_history table.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PublishHistoryEntry {
    pub id: i64,
    pub published_at: String,
    pub duration_ms: i64,
    pub products_count: i64,
    pub images_uploaded: i64,
    pub images_deleted: i64,
    pub success: bool,
    pub catalog_version: Option<String>,
    pub error_message: Option<String>,
    pub warnings_count: i64,
    pub errors_count: i64,
}

/// v0.16.0: Insert a publish history entry. Called after every publish
/// attempt (success or failure).
pub fn log_publish_history(
    conn: &Connection,
    duration_ms: i64,
    products_count: i64,
    images_uploaded: i64,
    images_deleted: i64,
    success: bool,
    catalog_version: Option<&str>,
    error_message: Option<&str>,
    warnings_count: i64,
    errors_count: i64,
) -> Result<i64, rusqlite::Error> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO catalog_publish_history
         (published_at, duration_ms, products_count, images_uploaded, images_deleted,
          success, catalog_version, error_message, warnings_count, errors_count)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            now, duration_ms, products_count, images_uploaded, images_deleted,
            success as i64, catalog_version, error_message, warnings_count, errors_count
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

/// v0.16.0: Get the last N publish history entries (most recent first).
pub fn get_publish_history(
    conn: &Connection,
    limit: i64,
) -> Result<Vec<PublishHistoryEntry>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, published_at, duration_ms, products_count, images_uploaded,
                images_deleted, success, catalog_version, error_message,
                warnings_count, errors_count
         FROM catalog_publish_history
         ORDER BY published_at DESC
         LIMIT ?1"
    )?;
    let rows = stmt.query_map(params![limit], |row| {
        Ok(PublishHistoryEntry {
            id: row.get(0)?,
            published_at: row.get(1)?,
            duration_ms: row.get(2)?,
            products_count: row.get(3)?,
            images_uploaded: row.get(4)?,
            images_deleted: row.get(5)?,
            success: row.get::<_, i64>(6)? != 0,
            catalog_version: row.get(7)?,
            error_message: row.get(8)?,
            warnings_count: row.get(9)?,
            errors_count: row.get(10)?,
        })
    })?;
    let mut result = Vec::new();
    for r in rows { result.push(r?); }
    Ok(result)
}
