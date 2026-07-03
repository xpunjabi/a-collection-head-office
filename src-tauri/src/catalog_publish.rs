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
// v0.17.0: STATIC PRODUCT PAGE GENERATION
// ============================================================

/// v0.17.0: Generate a static HTML page for a single product. This page
/// has proper Open Graph meta tags (for FB/WhatsApp link previews) and a
/// tiny redirect script that opens the SPA with this product in the modal.
///
/// When someone shares a product URL on FB/WhatsApp:
/// 1. The platform's crawler fetches this HTML file
/// 2. Crawler reads OG meta tags → shows rich preview (title, image, desc)
/// 3. When a real user clicks the link, the redirect script runs
/// 4. Browser navigates to ../#/SKU → SPA loads → product opens in modal
///
/// This is the ONLY way to get per-product OG previews on GitHub Pages
/// (static hosting, no server-side rendering).
pub fn generate_product_page(
    product: &PublicProduct,
    catalog: &CatalogJson,
    base_url: &str,
) -> String {
    let slug = product.sku.as_deref().unwrap_or("product");
    // v0.17.2: Use sanitized slug for the OG `og:url` meta tag and the
    // file on disk (kept in sync with upload loop). Raw `slug` (original SKU)
    // is still used in the redirect hash so the SPA can match it against
    // p.sku (which contains the original `#`, spaces, etc.).
    // Example: SKU "D#26" → safe_slug "D-26" (file + og:url), redirect hash "D#26".
    let safe_slug = sanitize_slug(slug);
    let title = html_escape(&product.name);
    let description = product.description
        .as_ref()
        .map(|d| html_escape(&d.chars().take(200).collect::<String>()))
        .unwrap_or_else(|| format!("{} — Rs. {}", product.name, format_price(product.sale_price)));
    // v0.17.3: Trim trailing slash from base_url to avoid double-slash in
    // og:url and og:image meta tags. catalog_url is constructed with a
    // trailing slash (https://owner.github.io/repo/), so without trimming
    // we'd get "https://owner.github.io/repo//products/..." which FB/WhatsApp
    // crawlers may treat as a distinct (broken) URL.
    let base = base_url.trim_end_matches('/');
    let image_url = if !product.images.is_empty() {
        format!("{}/data/images/{}", base, product.images[0])
    } else {
        format!("{}/icon-512.png", base)
    };
    let product_url = format!("{}/products/{}.html", base, safe_slug);
    let brand = html_escape(&catalog.brand);

    format!(r##"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>{title} — {brand}</title>

  <!-- Open Graph for Facebook, WhatsApp, Messenger -->
  <meta property="og:title" content="{title}" />
  <meta property="og:description" content="{description}" />
  <meta property="og:image" content="{image_url}" />
  <meta property="og:image:width" content="400" />
  <meta property="og:image:height" content="400" />
  <meta property="og:url" content="{product_url}" />
  <meta property="og:type" content="product" />
  <meta property="og:site_name" content="{brand}" />

  <!-- Twitter Card -->
  <meta name="twitter:card" content="summary_large_image" />
  <meta name="twitter:title" content="{title}" />
  <meta name="twitter:description" content="{description}" />
  <meta name="twitter:image" content="{image_url}" />

  <!-- Theme -->
  <meta name="theme-color" content="#000000" />
</head>
<body>
  <p>Redirecting to {brand}…</p>
  <p>Product: {title}</p>
  <script>
    // Redirect to the SPA with hash routing — opens this product in modal
    window.location.replace('../#/{slug}');
  </script>
  <noscript>
    <a href="../#/{slug}">Click here to view {title}</a>
  </noscript>
</body>
</html>"##)
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
     .replace('<', "&lt;")
     .replace('>', "&gt;")
     .replace('"', "&quot;")
     .replace('\'', "&#x27;")
}

fn format_price(n: f64) -> String {
    format!("{:.0}", n)
}

/// v0.17.2: Sanitize SKU into a URL-safe filename slug.
/// v0.17.3: Collapse consecutive hyphens (MULTI###HASH → MULTI-HASH, not MULTI---HASH).
///
/// SKUs like "D#26", "DS#10", "VOLUME#48 DS#14" contain characters
/// (`#`, spaces, etc.) that break URL handling:
///   - `#` is a URL fragment identifier → browser ignores everything after it
///   - spaces break URL parsing (need %20 encoding)
///   - other special chars can cause GitHub Pages 404s
///
/// This function:
///   1. Replaces every non-alphanumeric/non-hyphen character with a hyphen
///   2. Collapses consecutive hyphens into one (v0.17.3 fix)
///   3. Trims leading/trailing hyphens
///
/// Examples (v0.17.3 behavior):
///   "D#26"              → "D-26"
///   "DS#10"             → "DS-10"
///   "VOLUME#48 DS#14"   → "VOLUME-48-DS-14"
///   "MULTI###HASH"      → "MULTI-HASH"     (was "MULTI---HASH" in v0.17.2)
///   "AB##CD"            → "AB-CD"          (was "AB--CD" in v0.17.2)
///   "AH-2026-V29-DS10"  → "AH-2026-V29-DS10"  (already safe, unchanged)
///   "DE-LA65-P30634"    → "DE-LA65-P30634"    (already safe, unchanged)
///
/// MUST be used in TWO places to keep them in sync:
///   1. generate_product_page() — for OG `og:url` meta tag + redirect hash
///   2. upload loop in publish_catalog() — for the actual filename on disk
///
/// If they diverge, FB/WhatsApp preview shows a URL that 404s when clicked.
///
/// MUST match catalog app.js sanitizeSku() exactly (case-sensitive).
/// Catalog app.js v0.17.3:
///   sku.replace(/[^a-zA-Z0-9-]/g, '-').replace(/-+/g, '-').replace(/^-+|-+$/g, '')
fn sanitize_slug(sku: &str) -> String {
    let sanitized: String = sku
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' { c } else { '-' })
        .collect();
    // v0.17.3: collapse consecutive hyphens (MULTI---HASH → MULTI-HASH).
    // std::replace doesn't handle variable-length runs in one pass, so loop.
    let collapsed = collapse_hyphens(&sanitized);
    collapsed.trim_matches('-').to_string()
}

/// v0.17.3: Helper — collapse runs of 2+ hyphens into a single hyphen.
fn collapse_hyphens(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_was_hyphen = false;
    for c in s.chars() {
        if c == '-' {
            if !prev_was_hyphen {
                out.push('-');
            }
            prev_was_hyphen = true;
        } else {
            out.push(c);
            prev_was_hyphen = false;
        }
    }
    out
}

// ============================================================
// v0.18.0: META CATALOG FEED (Google Merchant RSS XML)
// ============================================================

/// Default Google Product Category for clothing/apparel.
/// 1604 = Apparel & Accessories > Clothing
///
/// Defined as a constant so it can be changed in one place if Ali bhai
/// ever expands to other product types (shoes, accessories, etc.).
/// Future: could be promoted to a per-product field or settings.json.
const DEFAULT_GOOGLE_PRODUCT_CATEGORY: &str = "1604";

/// v0.18.0: XML escape — escapes the 5 mandatory XML characters.
/// Distinct from html_escape() because XML has stricter rules for
/// apostrophes (`'` → `&apos;` vs HTML `&#x27;`).
fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
     .replace('<', "&lt;")
     .replace('>', "&gt;")
     .replace('"', "&quot;")
     .replace('\'', "&apos;")
}

/// v0.18.0: Generate Meta-compatible product feed in Google Merchant RSS XML format.
///
/// Spec: https://support.google.com/merchants/answer/160589
/// Compatible with: Meta Commerce Manager, Google Merchant Center, Pinterest.
///
/// Architecture:
/// - Pure function (no I/O) — takes CatalogJson, returns XML string
/// - Reuses sanitize_slug() for product URLs (consistency with v0.17.x)
/// - Reuses base_url.trim_end_matches('/') for clean URLs (v0.17.3 fix)
/// - Separate `<g:additional_image_link>` element per additional image
///   (per ChatGPT recommendation — more standard than comma-separated)
///
/// Field mapping (HO PublicProduct → Google/Meta feed):
///   g:id                    ← product.id           (SQLite integer, NEVER changes)
///   g:title                 ← product.name
///   g:description           ← product.description  (fallback: name + price)
///   g:link                  ← catalog_url + products/<safe_slug>.html
///   g:image_link            ← catalog_url + data/images/<images[0]>
///   g:additional_image_link ← one element per remaining image (max 10 total)
///   g:price                 ← sale_price + " PKR"  (2 decimal places)
///   g:availability          ← "in stock" if availability == "available", else "out of stock"
///   g:brand                 ← catalog.brand
///   g:condition             ← "new" (hardcoded — Ali sells only new stock)
///   g:product_type          ← product.category     (HO's internal category)
///   g:google_product_category ← DEFAULT_GOOGLE_PRODUCT_CATEGORY constant
///   g:custom_label_0        ← product.color
///   g:custom_label_1        ← product.fabric
///   g:custom_label_2        ← product.season
///   g:mpn                   ← sanitize_slug(product.sku)  (Manufacturer Part Number)
///   g:identifier_exists     ← "no" (no GTIN/MPN globally registered)
///
/// Future fields (easy to add — just append inside <item>):
///   g:sale_price, g:color, g:size, g:material, g:gender, g:age_group,
///   g:pattern, g:shipping, g:availability_date, g:custom_label_3, g:custom_label_4
///
/// Layout:
/// - `feed.xml` at catalog repo ROOT (URL: https://owner.github.io/repo/feed.xml)
/// - Public, raw — Meta Commerce Manager fetches this URL daily
fn generate_meta_feed(catalog: &CatalogJson, base_url: &str) -> String {
    let base = base_url.trim_end_matches('/');
    let brand = xml_escape(&catalog.brand);

    // RFC 822 date for last_build_date (e.g. "Wed, 03 Jul 2026 11:30:00 +0000")
    let last_build_date = match chrono::DateTime::parse_from_rfc3339(&catalog.published_at) {
        Ok(dt) => dt.format("%a, %d %b %Y %H:%M:%S %z").to_string(),
        Err(_) => catalog.published_at.clone(),
    };

    let mut xml = String::with_capacity(8192);
    xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str("<rss version=\"2.0\" xmlns:g=\"http://base.google.com/ns/1.0\">\n");
    xml.push_str("  <channel>\n");
    xml.push_str(&format!("    <title>{} — Product Catalog</title>\n", brand));
    xml.push_str(&format!("    <link>{}/</link>\n", base));
    xml.push_str(&format!(
        "    <description>Latest products from {} — order via WhatsApp {}</description>\n",
        brand, catalog.whatsapp_number
    ));
    xml.push_str(&format!("    <last_build_date>{}</last_build_date>\n", xml_escape(&last_build_date)));

    for product in &catalog.products {
        let title = xml_escape(&product.name);
        let description_owned = product.description
            .as_ref()
            .map(|d| d.clone())
            .unwrap_or_else(|| format!("{} — Rs. {}", product.name, format_price(product.sale_price)));
        let description = xml_escape(&description_owned);
        let product_id = product.id;

        // Product URL — points to static product page (v0.17.0+)
        let slug_raw = product.sku.as_deref().unwrap_or("product");
        let safe_slug = sanitize_slug(slug_raw);
        let product_url = if safe_slug.is_empty() {
            format!("{}/", base)
        } else {
            format!("{}/products/{}.html", base, safe_slug)
        };

        // Main image + additional images (absolute HTTPS URLs)
        let (image_link, additional_images): (Option<String>, Vec<String>) =
            if product.images.is_empty() {
                (None, Vec::new())
            } else {
                let main = format!("{}/data/images/{}", base, product.images[0]);
                let rest: Vec<String> = product.images.iter()
                    .skip(1)
                    .take(10) // Google Merchant max 10 additional images
                    .map(|img| format!("{}/data/images/{}", base, img))
                    .collect();
                (Some(main), rest)
            };

        // Availability mapping
        let availability = if product.availability.eq_ignore_ascii_case("available") {
            "in stock"
        } else {
            "out of stock"
        };

        // Price (2 decimal places + PKR currency)
        let price = format!("{:.2} PKR", product.sale_price);

        // Optional fields (only emit if non-empty)
        let product_type = product.category.as_ref().filter(|s| !s.is_empty());
        let custom_label_0 = product.color.as_ref().filter(|s| !s.is_empty());
        let custom_label_1 = product.fabric.as_ref().filter(|s| !s.is_empty());
        let custom_label_2 = product.season.as_ref().filter(|s| !s.is_empty());

        // Build <item>
        xml.push_str("    <item>\n");
        xml.push_str(&format!("      <g:id>{}</g:id>\n", product_id));
        xml.push_str(&format!("      <g:title>{}</g:title>\n", title));
        xml.push_str(&format!("      <g:description>{}</g:description>\n", description));
        xml.push_str(&format!("      <g:link>{}</g:link>\n", xml_escape(&product_url)));
        if let Some(img) = image_link {
            xml.push_str(&format!("      <g:image_link>{}</g:image_link>\n", xml_escape(&img)));
        }
        // Separate element per additional image (per ChatGPT recommendation)
        for img in &additional_images {
            xml.push_str(&format!("      <g:additional_image_link>{}</g:additional_image_link>\n", xml_escape(img)));
        }
        xml.push_str(&format!("      <g:price>{}</g:price>\n", price));
        xml.push_str(&format!("      <g:availability>{}</g:availability>\n", availability));
        xml.push_str(&format!("      <g:brand>{}</g:brand>\n", brand));
        xml.push_str("      <g:condition>new</g:condition>\n");
        if let Some(pt) = product_type {
            xml.push_str(&format!("      <g:product_type>{}</g:product_type>\n", xml_escape(pt)));
        }
        xml.push_str(&format!("      <g:google_product_category>{}</g:google_product_category>\n", DEFAULT_GOOGLE_PRODUCT_CATEGORY));
        if let Some(cl) = custom_label_0 {
            xml.push_str(&format!("      <g:custom_label_0>{}</g:custom_label_0>\n", xml_escape(cl)));
        }
        if let Some(cl) = custom_label_1 {
            xml.push_str(&format!("      <g:custom_label_1>{}</g:custom_label_1>\n", xml_escape(cl)));
        }
        if let Some(cl) = custom_label_2 {
            xml.push_str(&format!("      <g:custom_label_2>{}</g:custom_label_2>\n", xml_escape(cl)));
        }
        if !safe_slug.is_empty() {
            xml.push_str(&format!("      <g:mpn>{}</g:mpn>\n", xml_escape(&safe_slug)));
        }
        xml.push_str("      <g:identifier_exists>no</g:identifier_exists>\n");
        xml.push_str("    </item>\n");
    }

    xml.push_str("  </channel>\n");
    xml.push_str("</rss>\n");
    xml
}

/// v0.18.0: Basic XML well-formedness validation.
///
/// Checks:
/// - Starts with `<?xml` declaration
/// - Has `<rss` root element with proper closing `</rss>`
/// - Has `<channel>` with closing `</channel>`
/// - Number of `<item>` opens == number of `</item>` closes
///
/// NOT a full XML parser — just sanity checks to prevent uploading
/// a catastrophically broken feed. If Meta/Google rejects a feed
/// that passes these checks, we'll add more validation later.
///
/// Returns Ok(()) if valid, Err(message) if not.
fn validate_feed_xml(xml: &str) -> Result<(), String> {
    if !xml.starts_with("<?xml") {
        return Err("Feed XML missing <?xml declaration".to_string());
    }
    if !xml.contains("<rss") || !xml.contains("</rss>") {
        return Err("Feed XML missing <rss> root element".to_string());
    }
    if !xml.contains("<channel>") || !xml.contains("</channel>") {
        return Err("Feed XML missing <channel> element".to_string());
    }
    let open_items = xml.matches("<item>").count();
    let close_items = xml.matches("</item>").count();
    if open_items != close_items {
        return Err(format!(
            "Feed XML item tag mismatch: {} opens vs {} closes",
            open_items, close_items
        ));
    }
    Ok(())
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

    // v0.17.0: Upload static product pages (for OG link previews).
    // Each product gets its own HTML file at products/<slug>.html with
    // proper OG meta tags. FB/WhatsApp crawlers read these tags directly
    // (they don't execute JS), so shared product links show rich previews.
    let mut product_pages_uploaded = 0usize;
    let mut uploaded_product_slugs: std::collections::HashSet<String> = std::collections::HashSet::new();

    for product in &catalog.products {
        let slug = product.sku.as_deref().unwrap_or("product");
        // v0.17.2: Use shared sanitize_slug() helper (kept in sync with
        // generate_product_page's og:url meta tag). Diverging here would
        // cause FB/WhatsApp preview URL to 404 when clicked.
        let safe_slug = sanitize_slug(slug);
        if safe_slug.is_empty() { continue; }

        let html = generate_product_page(product, &catalog, &catalog_url);
        let html_b64 = base64::engine::general_purpose::STANDARD.encode(html.as_bytes());
        let path = format!("products/{}.html", safe_slug);
        let msg = format!("Update product page: {}", safe_slug);
        match upload_file(&client, &api_base, github_token, &path, &msg, &html_b64).await {
            Ok(_) => {
                product_pages_uploaded += 1;
                uploaded_product_slugs.insert(format!("{}.html", safe_slug));
            },
            Err(e) => errors.push(format!("product page {}: {}", safe_slug, e)),
        }
    }

    // v0.17.0: Delete orphan product pages (products no longer in catalog)
    if let Ok(existing_pages) = list_repo_directory(&client, repo, github_token, "products").await {
        for file in existing_pages {
            if !uploaded_product_slugs.contains(&file.name) {
                if let Err(e) = delete_file(
                    &client, &api_base, github_token,
                    &format!("products/{}", file.name),
                    &format!("Delete orphan product page: {}", file.name),
                    &file.sha,
                ).await {
                    errors.push(format!("delete page {}: {}", file.name, e));
                }
            }
        }
    }

    let _ = product_pages_uploaded;  // Could add to PublishResult if needed

    // v0.18.0: Generate + upload Meta Catalog Feed (Google Merchant RSS XML).
    // Completely additive — no existing flow modified. If feed generation or
    // upload fails, errors[] is pushed but publish succeeds (feed is non-critical).
    // Feed is uploaded to repo ROOT as feed.xml — public URL:
    //   https://<owner>.github.io/<repo>/feed.xml
    // Meta Commerce Manager fetches this URL on a schedule (daily by default).
    let feed_xml = generate_meta_feed(catalog, &catalog_url);
    match validate_feed_xml(&feed_xml) {
        Ok(()) => {
            let feed_b64 = base64::engine::general_purpose::STANDARD.encode(feed_xml.as_bytes());
            let feed_path = "feed.xml";
            let feed_msg = format!(
                "Update Meta Catalog feed — {} products, {}",
                products_published, catalog.version
            );
            if let Err(e) = upload_file(
                &client, &api_base, github_token, feed_path, &feed_msg, &feed_b64,
            ).await {
                errors.push(format!("feed.xml: {}", e));
            }
        },
        Err(e) => {
            // Validation failed — don't upload broken feed, just log error
            errors.push(format!("feed.xml validation: {}", e));
        }
    }

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
