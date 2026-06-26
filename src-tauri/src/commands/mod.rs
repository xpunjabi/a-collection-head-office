use crate::catalog::{self, Product, ProductLocationStock};
use crate::inventory::{self, InventorySummary, LowStockItem, DeadStockItem, BestSellerItem};
use crate::customers::{self, Customer, OrderItemInput, OrderHistory};
use crate::reports::{self, SalesReport, InventoryReport, CustomerSummaryReport};
use crate::locations::{self, Location};
use crate::agents::{self, AgentSummary, AgentLedgerEntry};
use crate::purchase_trips::{self, PurchaseTripSummary};
use crate::adapters::duckduckgo::{self, WebEvidence};
use crate::ai::{self, AiResponse, KnowledgeEntry};
use crate::utils;
use tauri::async_runtime::Mutex;
use std::path::Path;
use rusqlite::Connection;
use tauri::State;

/// Database state shared across all Tauri commands.
///
/// Uses `tauri::async_runtime::Mutex` (which is `tokio::sync::Mutex` under the
/// hood) instead of `std::sync::Mutex`. This is critical because:
///
/// 1. **No deadlock across `.await`** — `std::sync::Mutex` is not `Send` when
///    held across `.await` points, which would fail to compile under Tauri's
///    async command model. `tokio::sync::Mutex` is `Send` and safe to hold
///    across awaits.
///
/// 2. **No runtime blocking** — When a command needs to await (e.g., a 45s
///    Gemini API call), other commands can still acquire the lock if needed
///    (though in practice they shouldn't — see pattern below).
///
/// 3. **Pattern discipline** — Even with an async mutex, the codebase follows
///    the scoped-block pattern: acquire lock only for the duration of the
///    synchronous DB operation, then release before any `.await`. This means
///    long AI calls do NOT hold the DB lock, preventing UI freezes.
///
/// Usage:
/// ```ignore
/// let conn = state.0.lock().await;
/// // do synchronous rusqlite work here
/// // drop(conn) — implicit when block ends
/// // .await calls happen AFTER the lock is released
/// ```
pub struct DbState(pub Mutex<Connection>);

fn set_setting_val(conn: &Connection, key: &str, value: &str) -> Result<(), rusqlite::Error> {
    conn.execute("INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2);", [key, value])?;
    Ok(())
}

fn get_setting_val(conn: &Connection, key: &str) -> Result<String, rusqlite::Error> {
    conn.query_row("SELECT value FROM settings WHERE key = ?1", [key], |row| row.get(0))
}

// ==================== CATALOG ====================

#[tauri::command]
pub async fn get_products(state: State<'_, DbState>) -> Result<Vec<Product>, String> {
    let conn = state.0.lock().await;
    catalog::get_all_products(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_product(state: State<'_, DbState>, id: i64) -> Result<Product, String> {
    let conn = state.0.lock().await;
    catalog::get_product_by_id(&conn, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_product(state: State<'_, DbState>, product: Product) -> Result<i64, String> {
    let conn = state.0.lock().await;
    catalog::add_product(&conn, &product).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_product(state: State<'_, DbState>, product: Product) -> Result<(), String> {
    let conn = state.0.lock().await;
    catalog::update_product(&conn, &product).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_product(state: State<'_, DbState>, id: i64) -> Result<(), String> {
    let conn = state.0.lock().await;
    catalog::delete_product(&conn, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_product_locations(state: State<'_, DbState>, product_id: i64) -> Result<Vec<ProductLocationStock>, String> {
    let conn = state.0.lock().await;
    catalog::get_product_locations(&conn, product_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn upsert_product_location(state: State<'_, DbState>, product_id: i64, location_id: i64, quantity: i64) -> Result<(), String> {
    let conn = state.0.lock().await;
    catalog::upsert_product_location(&conn, product_id, location_id, quantity).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn search_products_by_color(state: State<'_, DbState>, color: String) -> Result<Vec<Product>, String> {
    let conn = state.0.lock().await;
    catalog::search_by_color(&conn, &color).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn export_products_csv(state: State<'_, DbState>) -> Result<String, String> {
    let conn = state.0.lock().await;
    catalog::export_to_csv(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn import_products_csv(state: State<'_, DbState>, csv_content: String) -> Result<(), String> {
    let conn = state.0.lock().await;
    catalog::import_from_csv(&conn, &csv_content).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn upload_product_image(src_path: String, format_type: String) -> Result<String, String> {
    let src = Path::new(&src_path);
    if !src.exists() {
        return Err("Source image file does not exist.".to_string());
    }
    let images_dir = utils::get_images_dir();
    catalog::process_and_save_image(src, &images_dir, &format_type).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_image_as_base64(filename: String) -> Result<String, String> {
    let images_dir = utils::get_images_dir();
    let path = images_dir.join(&filename);
    if !path.exists() {
        return Err(format!("Image not found: {}", filename));
    }
    let bytes = std::fs::read(&path).map_err(|e| format!("Failed to read image: {}", e))?;
    let lower = filename.to_lowercase();
    let mime = if lower.ends_with(".png") {
        "image/png"
    } else if lower.ends_with(".webp") {
        "image/webp"
    } else if lower.ends_with(".gif") {
        "image/gif"
    } else {
        "image/jpeg"
    };
    use base64::Engine as _;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Ok(format!("data:{};base64,{}", mime, b64))
}

#[tauri::command]
pub async fn save_base64_image(base64_data: String, format_type: String) -> Result<String, String> {
    use base64::Engine as _;
    let raw = base64::engine::general_purpose::STANDARD
        .decode(&base64_data)
        .map_err(|e| format!("Base64 decode error: {}", e))?;
    let images_dir = utils::get_images_dir();
    catalog::process_and_save_image_bytes(&raw, &images_dir, &format_type)
        .map_err(|e| e.to_string())
}

// ==================== LOCATIONS ====================

#[tauri::command]
pub async fn get_locations(state: State<'_, DbState>) -> Result<Vec<Location>, String> {
    let conn = state.0.lock().await;
    locations::get_all_locations(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_location(state: State<'_, DbState>, name: String, address: String) -> Result<i64, String> {
    let conn = state.0.lock().await;
    locations::add_location(&conn, &name, &address).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_location(state: State<'_, DbState>, id: i64, name: String, address: String, is_active: bool) -> Result<(), String> {
    let conn = state.0.lock().await;
    locations::update_location(&conn, id, &name, &address, is_active).map_err(|e| e.to_string())
}

// ==================== INVENTORY ====================

#[tauri::command]
pub async fn get_inventory_summary(state: State<'_, DbState>) -> Result<InventorySummary, String> {
    let conn = state.0.lock().await;
    inventory::get_inventory_summary(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_low_stock(state: State<'_, DbState>, threshold: i64) -> Result<Vec<LowStockItem>, String> {
    let conn = state.0.lock().await;
    inventory::get_low_stock_items(&conn, threshold).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_dead_stock(state: State<'_, DbState>, days_limit: i64) -> Result<Vec<DeadStockItem>, String> {
    let conn = state.0.lock().await;
    inventory::get_dead_stock_items(&conn, days_limit).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_best_sellers(state: State<'_, DbState>, limit: i64) -> Result<Vec<BestSellerItem>, String> {
    let conn = state.0.lock().await;
    inventory::get_best_sellers(&conn, limit).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn adjust_stock(state: State<'_, DbState>, product_id: i64, adjustment: i64) -> Result<(), String> {
    let conn = state.0.lock().await;
    inventory::adjust_stock(&conn, product_id, adjustment).map_err(|e| e.to_string())
}

// ==================== CUSTOMERS ====================

#[tauri::command]
pub async fn get_customers(state: State<'_, DbState>) -> Result<Vec<Customer>, String> {
    let conn = state.0.lock().await;
    customers::get_all_customers(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_customer(state: State<'_, DbState>, customer: Customer) -> Result<i64, String> {
    let conn = state.0.lock().await;
    customers::add_customer(&conn, &customer).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_customer(state: State<'_, DbState>, customer: Customer) -> Result<(), String> {
    let conn = state.0.lock().await;
    customers::update_customer(&conn, &customer).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_customer(state: State<'_, DbState>, id: i64) -> Result<(), String> {
    let conn = state.0.lock().await;
    customers::delete_customer(&conn, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_order(state: State<'_, DbState>, customer_id: i64, items: Vec<OrderItemInput>) -> Result<i64, String> {
    let mut conn = state.0.lock().await;
    customers::create_order(&mut conn, customer_id, items).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_customer_history(state: State<'_, DbState>, customer_id: i64) -> Result<Vec<OrderHistory>, String> {
    let conn = state.0.lock().await;
    customers::get_customer_purchase_history(&conn, customer_id).map_err(|e| e.to_string())
}

// ==================== REPORTS ====================

#[tauri::command]
pub async fn get_sales_report(state: State<'_, DbState>, start_date: String, end_date: String) -> Result<SalesReport, String> {
    let conn = state.0.lock().await;
    reports::generate_sales_report(&conn, &start_date, &end_date).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_inventory_report(state: State<'_, DbState>) -> Result<InventoryReport, String> {
    let conn = state.0.lock().await;
    reports::generate_inventory_report(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_customer_report(state: State<'_, DbState>) -> Result<CustomerSummaryReport, String> {
    let conn = state.0.lock().await;
    reports::generate_customer_report(&conn).map_err(|e| e.to_string())
}

// ==================== AI ====================

#[tauri::command]
pub async fn ask_ai(
    state: State<'_, DbState>,
    prompt: String,
    image_data: Option<String>,
    history: Option<Vec<ai::ChatMessage>>,
) -> Result<AiResponse, String> {
    println!("[ask_ai] instruction='{}' has_image={}", prompt, image_data.is_some());

    let extraction = if let Some(ref b64) = image_data {
        use base64::Engine as _;
        if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(b64) {
            match crate::ai::ingestion::extract_local_data(&bytes) {
                Ok(result) => {
                    println!("[Local Extraction] qr={:?} ocr={:?}", result.qr_data, result.ocr_text);
                    Some(result)
                }
                Err(e) => {
                    println!("[Local Extraction] Error: {}", e);
                    None
                }
            }
        } else {
            None
        }
    } else {
        None
    };

    let mut fast_path_data: Option<ai::AssistantResult> = None;

    if let Some(ref extraction) = extraction {
        let match_result = {
            let conn = state.0.lock().await;
            crate::ai::local_match::check_local_catalog(&conn, &extraction.qr_data, &extraction.ocr_text)
        };
        match match_result {
            Ok(Some(mr)) => {
                println!("[Local Match] Found: id={} title={} confidence={}", mr.item_id, mr.title, mr.confidence);
                fast_path_data = Some(ai::AssistantResult::LocalMatchFound(mr));
            }
            Ok(None) => {
                println!("[Local Match] No match found. Proceeding to web evidence + AI draft.");
                // Capture provider along with api_key + model so we can pass
                // it to catalog_composer. Previously cfg.0 (provider) was
                // discarded, causing catalog_composer to silently use
                // hardcoded "gemini" — meaning OpenAI/Claude/Ollama users
                // would get a Gemini API call (which fails without a Gemini
                // API key).
                let (provider, api_key, model) = {
                    let conn = state.0.lock().await;
                    let cfg = ai::get_ai_config(&conn)?;
                    (cfg.0.clone(), cfg.1.clone(), cfg.2.clone())
                };

                // Build search query from extraction text
                let search_query = extraction.ocr_text.as_deref()
                    .or(extraction.qr_data.as_deref())
                    .unwrap_or("")
                    .to_string();

                // Fetch web evidence via DuckDuckGo (free, no API key needed)
                let web_evidence: Option<WebEvidence> = if !search_query.is_empty() {
                    match duckduckgo::fetch_web_evidence(&search_query).await {
                        Ok(evidence) => {
                            println!("[Web Evidence] Found {} results", evidence.result_count);
                            Some(evidence)
                        }
                        Err(e) => {
                            println!("[Web Evidence] DuckDuckGo error: {}. Continuing with OCR text only.", e);
                            None
                        }
                    }
                } else {
                    None
                };

                match crate::ai::catalog_composer::generate_catalog_draft(
                    extraction, &Some(prompt.clone()), &provider, &api_key, &model, &web_evidence, image_data.as_deref()
                ).await {
                    Ok(draft) => {
                        println!("[AI Draft] title={} brand={:?} fabric={:?} design_code={:?} web_count={:?}",
                            draft.title, draft.brand, draft.fabric, draft.design_code, draft.web_evidence_count);
                        fast_path_data = Some(ai::AssistantResult::NewCatalogDraft(draft));
                    }
                    Err(e) => {
                        println!("[AI Draft] Error: {}", e);
                    }
                }
            }
            Err(e) => {
                println!("[Local Match] Error: {}", e);
            }
        }
    }

    let local_result = {
        let conn = state.0.lock().await;
        ai::try_local_intent(&conn, &prompt)
    };
    if let Some(response) = local_result { return Ok(response); }

    // SHORT-CIRCUIT: if the fast path already produced a CatalogDraft (or a
    // LocalMatchFound), DO NOT run the second AI call. The previous behavior
    // was to ALWAYS run a fallback `call_ai_provider` that re-prompted Gemini
    // in "Product Intake Mode" and parsed another draft from its text response,
    // which caused the duplicate-draft UX bug (frontend rendered BOTH
    // fast_path_data AND product_draft for the same image).
    //
    // We only fall through to the second AI call when the fast path did not
    // produce a structured result — i.e. no image was uploaded, or local
    // extraction failed, or local_match + catalog_composer both yielded None.
    if fast_path_data.is_some() {
        println!("[ask_ai] Fast path produced a result; skipping fallback AI call to avoid duplicate draft.");
        return Ok(AiResponse {
            text: String::new(),
            detected_action: None,
            action_data: None,
            product_draft: None,
            confidence: None,
            missing_fields: None,
            suggested_actions: None,
            fast_path_data,
        });
    }

    let (provider, api_key, model) = {
        let conn = state.0.lock().await;
        ai::get_ai_config(&conn)?
    };
    if api_key.is_empty() && provider != "local" {
        return Err("AI API key is missing. Please configure it in Settings.".to_string());
    }

    // Fetch web evidence for the FALLBACK path too. Previously web evidence was
    // only fetched for the fast path (catalog_composer). The fallback path
    // (which runs for text-only queries like "internet se photo laao") had no
    // web access, so Gemini truthfully replied "I don't have internet access".
    //
    // Now we fetch web evidence using the user's text prompt and inject it into
    // the system prompt via build_system_prompt_with_web. We also inject a
    // hard disclaimer so the model does not refuse web-related queries.
    //
    // Skip web fetch for empty prompts (defensive — ask_ai already returns
    // early if both prompt and image are empty).
    let fallback_web_evidence: Option<WebEvidence> = if !prompt.trim().is_empty() {
        match duckduckgo::fetch_web_evidence(&prompt).await {
            Ok(evidence) => {
                println!("[Fallback Web Evidence] Found {} results for query '{}'", evidence.result_count, prompt);
                Some(evidence)
            }
            Err(e) => {
                println!("[Fallback Web Evidence] DuckDuckGo error: {}. Continuing without web evidence.", e);
                None
            }
        }
    } else {
        None
    };

    let system_prompt = {
        let conn = state.0.lock().await;
        let mut sp = ai::build_system_prompt_with_web(&conn, &prompt, fallback_web_evidence.as_ref())?;
        sp.push_str("\n\n## Product Intake Mode\n\nWhen the user shares a product image, link, code, or description, you MUST:\n1. Analyze all available information\n2. If product information is detected, return a JSON block at the end of your response:\n\n```json\n{\n  \"draft\": {\n    \"name\": \"...\",\n    \"sku\": \"...\",\n    \"category\": \"...\",\n    \"brand\": \"...\",\n    \"fabric\": \"...\",\n    \"color\": \"...\",\n    \"design\": \"...\",\n    \"season\": \"...\",\n    \"cost_price\": 0.0,\n    \"sale_price\": 0.0,\n    \"retail_price\": 0.0,\n    \"description\": \"...\",\n    \"tags\": [\"...\"],\n    \"keywords\": [\"...\"],\n    \"hashtags\": [\"...\"]\n  },\n  \"confidence\": 0.85,\n  \"missing_fields\": [\"stock_location\", \"purchase_cost\"],\n  \"suggested_actions\": [\"Add To Catalog\", \"Edit Draft\", \"Generate Marketing\"]\n}\n```\n\n3. If no product information is detected, respond normally as a business assistant.\n");
        sp
    };

    let response_text = ai::call_ai_provider(
        &provider, &api_key, &model, &system_prompt, &prompt,
        image_data.as_deref(),
        history.as_deref(),
    ).await?;
    {
        let conn = state.0.lock().await;
        ai::log_request(&conn, &prompt, &response_text, &provider)?;
    }

    let mut resp = AiResponse { text: response_text.clone(), detected_action: None, action_data: None, product_draft: None, confidence: None, missing_fields: None, suggested_actions: None, fast_path_data };

    if let Some(draft_resp) = ai::parse_draft_from_response(&response_text) {
        resp.product_draft = Some(draft_resp.draft);
        resp.confidence = Some(draft_resp.confidence);
        resp.missing_fields = Some(draft_resp.missing_fields);
        resp.suggested_actions = Some(draft_resp.suggested_actions);
        resp.detected_action = Some("product_draft".to_string());
    }

    Ok(resp)
}

#[tauri::command]
pub async fn save_product_draft_to_catalog(state: State<'_, DbState>, draft: ai::ProductDraft) -> Result<i64, String> {
    let now = chrono::Utc::now().to_rfc3339();
    let product = crate::catalog::Product {
        id: None,
        sku: draft.sku.clone().unwrap_or_default(),
        name: draft.name.clone().unwrap_or_else(|| "New Product".to_string()),
        category: draft.category.clone(),
        color: draft.color.clone(),
        design: draft.design.clone(),
        season: draft.season.clone(),
        cost_price: draft.cost_price.unwrap_or(0.0),
        sale_price: draft.sale_price.unwrap_or(0.0),
        purchase_price: draft.retail_price.unwrap_or(0.0),
        description: draft.description.clone(),
        tags: draft.tags.clone().map(|t| t.join(", ")),
        stock_quantity: 0,
        status: "active".to_string(),
        images: draft.images.clone()
            .map(|i| serde_json::to_string(&i).unwrap_or_else(|_| "[]".to_string()))
            .unwrap_or_else(|| "[]".to_string()),
        supplier_id: None,
        created_at: now.clone(),
        updated_at: now,
        // v0.11.0+ profit-mode fields — default to None/empty for manually
        // created drafts. These get populated when a purchase trip item is
        // linked or when stock is sent to an agent.
        product_code: None,
        brand: None,
        fabric: None,
        size_info: None,
        base_unit_cost: None,
        landed_unit_cost: None,
        retail_price: None,
        discount_price: None,
        source_trip_id: None,
        qty_in_head_office: None,
        qty_with_agents: None,
        qty_sold: None,
        qty_reserved: None,
        profit_status: None,
    };
    let conn = state.0.lock().await;
    let id = crate::catalog::add_product(&conn, &product).map_err(|e| e.to_string())?;
    Ok(id)
}

#[tauri::command]
pub async fn save_catalog_draft(state: State<'_, DbState>, draft: crate::ai::catalog_composer::CatalogDraft) -> Result<i64, String> {
    // Download the web image (best_image_url) BEFORE acquiring the DB lock.
    // This avoids holding Mutex<Connection> across a network .await, which
    // would block other Tauri commands for up to 8 seconds (image download
    // timeout). Matches the pattern used in ask_ai where network calls
    // happen outside the lock scope.
    //
    // If best_image_url is None or download fails, fall back to "[]" (no
    // image attached). The save never fails due to image download issues.
    let images_json: String = if let Some(ref url) = draft.best_image_url {
        if !url.is_empty() {
            match download_and_save_image(url).await {
                Ok(filename) => {
                    println!("[save_catalog_draft] Downloaded web image: {}", filename);
                    serde_json::to_string(&[filename]).unwrap_or_else(|_| "[]".to_string())
                }
                Err(e) => {
                    println!("[save_catalog_draft] Web image download failed: {}. Saving without image.", e);
                    "[]".to_string()
                }
            }
        } else {
            "[]".to_string()
        }
    } else {
        "[]".to_string()
    };

    let conn = state.0.lock().await;
    let now = chrono::Utc::now().to_rfc3339();
    let sku = draft.design_code.clone().unwrap_or_default();
    let title = &draft.title;

    // Duplicate check by SKU (design_code)
    if !sku.is_empty() {
        let exists: bool = conn.query_row(
            "SELECT COUNT(*) > 0 FROM products WHERE sku = ?1",
            [&sku],
            |row| row.get(0),
        ).map_err(|e| e.to_string())?;
        if exists {
            return Err(format!("Duplicate item found. SKU: {} already exists in catalog.", sku));
        }
    }

    // Duplicate check by title (case-insensitive)
    if !title.is_empty() {
        let exists: bool = conn.query_row(
            "SELECT COUNT(*) > 0 FROM products WHERE LOWER(name) = LOWER(?1)",
            [title],
            |row| row.get(0),
        ).map_err(|e| e.to_string())?;
        if exists {
            return Err(format!("Duplicate item found. '{}' already exists in catalog.", title));
        }
    }

    let mut tags = Vec::new();
    if let Some(ref brand) = draft.brand {
        if !brand.is_empty() { tags.push(format!("Brand: {}", brand)); }
    }
    if let Some(ref fabric) = draft.fabric {
        if !fabric.is_empty() { tags.push(format!("Fabric: {}", fabric)); }
    }
    let product = crate::catalog::Product {
        id: None,
        sku: draft.design_code.clone().unwrap_or_default(),
        name: draft.title,
        category: None,
        color: None,
        design: draft.brand.clone(),
        season: None,
        // v0.13.8: Save actual prices from draft (was hardcoded to 0.0)
        cost_price: draft.cost_price.unwrap_or(0.0),
        sale_price: draft.sale_price.unwrap_or(0.0),
        purchase_price: draft.cost_price.unwrap_or(0.0),
        description: draft.notes.clone(),
        tags: if tags.is_empty() { None } else { Some(tags.join(", ")) },
        stock_quantity: 0,
        status: "active".to_string(),
        images: images_json,
        supplier_id: None,
        created_at: now.clone(),
        updated_at: now,
        product_code: None,
        brand: draft.brand.clone(),
        fabric: draft.fabric.clone(),
        size_info: None,
        base_unit_cost: draft.cost_price,
        landed_unit_cost: None,
        // v0.13.8: Save retail_price from draft (was hardcoded to None)
        retail_price: draft.retail_price,
        discount_price: None,
        source_trip_id: None,
        qty_in_head_office: None,
        qty_with_agents: None,
        qty_sold: None,
        qty_reserved: None,
        profit_status: None,
    };
    let id = crate::catalog::add_product(&conn, &product).map_err(|e| e.to_string())?;
    Ok(id)
}

/// Download an image from a URL and save it locally using the existing
/// `process_and_save_image_bytes` helper (which normalizes the image to a
/// thumbnail-sized JPEG with aspect-ratio preservation).
///
/// Returns the saved filename (e.g., "1730123456_thumbnail.jpg") on success,
/// or an error string on failure. The caller is expected to handle failures
/// gracefully (fall back to no-image).
///
/// Used by `save_catalog_draft` to persist the `best_image_url` that the
/// AI catalog composer extracted from web search results.
async fn download_and_save_image(url: &str) -> Result<String, String> {
    use std::time::Duration;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(8))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    let res = client
        .get(url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        // Many e-commerce sites hotlink-protect images and 403 any non-self
        // referer. Sending no referer maximizes the chance of success.
        .header("Referer", "")
        .send()
        .await
        .map_err(|e| format!("Image download request failed: {}", e))?;

    if !res.status().is_success() {
        return Err(format!("Image download returned HTTP {}", res.status()));
    }

    let bytes = res.bytes().await
        .map_err(|e| format!("Failed to read image bytes: {}", e))?
        .to_vec();

    if bytes.is_empty() {
        return Err("Image download returned 0 bytes".to_string());
    }

    // Reuse the existing image processing helper. It will:
    // 1. Decode the image (JPEG/PNG/WebP/GIF)
    // 2. Resize preserving aspect ratio to fit within 200x200
    // 3. Save as JPEG to the app's images directory
    // 4. Return the filename
    let images_dir = crate::utils::get_images_dir();
    crate::catalog::process_and_save_image_bytes(&bytes, &images_dir, "thumbnail")
        .map_err(|e| format!("Failed to process/save image: {}", e))
}

#[tauri::command]
pub async fn generate_social_post(
    state: State<'_, DbState>,
    product_id: i64,
    platform: Option<String>,
) -> Result<crate::ai::marketing_engine::MarketingPost, String> {
    // Capture provider along with api_key + model. Previously cfg.0 (provider)
    // was discarded, causing marketing_engine to silently use hardcoded
    // "gemini" — OpenAI/Claude/Ollama users could not use Generate Post.
    let (provider, api_key, model) = {
        let conn = state.0.lock().await;
        let cfg = ai::get_ai_config(&conn)?;
        (cfg.0.clone(), cfg.1.clone(), cfg.2.clone())
    };
    let product = {
        let conn = state.0.lock().await;
        crate::catalog::get_product_by_id(&conn, product_id).map_err(|e| e.to_string())?
    };
    let product_name = &product.name;
    // FIX (Issue #5): Previously `brand = product.design` and `fabric =
    // product.tags` were semantically wrong. Now we pass the actual brand
    // field (or fall back to design if brand is not stored separately) and
    // use category as fabric indicator (since the products table doesn't
    // have a dedicated `fabric` column — `tags` is a JSON array, not a
    // fabric name).
    let brand = product.design.as_deref().unwrap_or("");
    let fabric = product.category.as_deref().unwrap_or("");
    let notes = product.description.as_deref().unwrap_or("");
    crate::ai::marketing_engine::generate_marketing_post(
        product_name, brand, fabric, notes, &provider, &api_key, &model,
        platform.as_deref(),
    ).await
}

#[tauri::command]
pub async fn generate_marketing(state: State<'_, DbState>, product_id: i64) -> Result<Vec<ai::MarketingContent>, String> {
    let (product, provider, api_key, model, has_fb, has_wa) = {
        let conn = state.0.lock().await;
        ai::prepare_marketing_data(&conn, product_id)?
    };
    let prompt = ai::build_marketing_prompt(&product, has_fb, has_wa);
    let posts = ai::generate_marketing_content(&provider, &api_key, &model, &prompt).await?;
    let now = chrono::Utc::now().to_rfc3339();
    {
        let conn = state.0.lock().await;
        for post in &posts {
            // Serialize hashtags array to JSON string for storage.
            let hashtags_json = serde_json::to_string(&post.hashtags).unwrap_or_else(|_| "[]".to_string());
            conn.execute(
                "INSERT INTO social_posts (product_id, platform, content, caption_type, status, created_at, hashtags) VALUES (?1, ?2, ?3, ?4, 'draft', ?5, ?6)",
                rusqlite::params![product_id, post.platform, post.content, post.caption_type, &now, &hashtags_json],
            ).map_err(|e| e.to_string())?;
        }
    }
    Ok(posts)
}

#[tauri::command]
pub async fn get_knowledge(state: State<'_, DbState>) -> Result<Vec<KnowledgeEntry>, String> {
    let conn = state.0.lock().await;
    ai::get_all_knowledge(&conn)
}

#[tauri::command]
pub async fn save_knowledge(state: State<'_, DbState>, topic: String, content: String, source: String) -> Result<(), String> {
    let conn = state.0.lock().await;
    ai::save_knowledge(&conn, &topic, &content, &source)
}

#[tauri::command]
pub async fn delete_knowledge(state: State<'_, DbState>, id: i64) -> Result<(), String> {
    let conn = state.0.lock().await;
    ai::delete_knowledge(&conn, id)
}

// ==================== SETTINGS ====================

#[tauri::command]
pub async fn get_settings(state: State<'_, DbState>) -> Result<std::collections::HashMap<String, String>, String> {
    let conn = state.0.lock().await;
    let mut stmt = conn.prepare("SELECT key, value FROM settings").map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |row| {
        let k: String = row.get(0)?;
        let v: String = row.get(1)?;
        Ok((k, v))
    }).map_err(|e| e.to_string())?;
    let mut map = std::collections::HashMap::new();
    for row in rows { if let Ok((k, v)) = row { map.insert(k, v); } }
    Ok(map)
}

#[tauri::command]
pub async fn update_setting(state: State<'_, DbState>, key: String, value: String) -> Result<(), String> {
    let conn = state.0.lock().await;
    set_setting_val(&conn, &key, &value).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn backup_database_now(state: State<'_, DbState>) -> Result<String, String> {
    let conn = state.0.lock().await;
    let backup_path = get_setting_val(&conn, "backup_path").map_err(|e| e.to_string())?;
    if backup_path.is_empty() { return Err("Backup path is not configured.".to_string()); }
    let backup_dir = Path::new(&backup_path);
    if !backup_dir.exists() { return Err("Backup path does not exist.".to_string()); }
    let db_src = utils::get_db_path();
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let dest = backup_dir.join(format!("manual_backup_{}.db", timestamp));
    std::fs::copy(db_src, &dest).map_err(|e| format!("Failed to copy: {}", e))?;
    Ok(dest.to_string_lossy().to_string())
}

/// Re-run database migrations on demand. This is primarily used by the
/// Locations page's "Sync from Profile" button to trigger the
/// `sync_sales_areas_to_locations` migration step without requiring an
/// app restart. Returns Ok(()) on success.
#[tauri::command]
pub async fn init_database(state: State<'_, DbState>) -> Result<(), String> {
    let mut conn = state.0.lock().await;
    // Re-run migrations by calling run_migrations directly. This is safe
    // because all migration steps are idempotent (CREATE TABLE IF NOT EXISTS,
    // INSERT OR IGNORE, add_col_if_missing).
    crate::database::run_migrations_public(&mut conn).map_err(|e| e.to_string())
}

// ============================================================
// v0.11.0 — Agents (replaces Locations as primary stock-movement entity)
// ============================================================

#[tauri::command]
pub async fn get_agents(state: State<'_, DbState>) -> Result<Vec<AgentSummary>, String> {
    let conn = state.0.lock().await;
    agents::get_all_agent_summaries(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_agent(state: State<'_, DbState>, id: i64) -> Result<AgentSummary, String> {
    let conn = state.0.lock().await;
    agents::get_agent_summary(&conn, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_agent(
    state: State<'_, DbState>,
    name: String,
    phone: Option<String>,
    city: Option<String>,
    area: Option<String>,
    address_notes: Option<String>,
    notes: Option<String>,
) -> Result<i64, String> {
    let conn = state.0.lock().await;
    agents::add_agent(
        &conn, &name,
        phone.as_deref(), city.as_deref(), area.as_deref(),
        address_notes.as_deref(), notes.as_deref(),
    ).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_agent(
    state: State<'_, DbState>,
    id: i64,
    name: String,
    phone: Option<String>,
    city: Option<String>,
    area: Option<String>,
    address_notes: Option<String>,
    notes: Option<String>,
    is_active: bool,
) -> Result<(), String> {
    let conn = state.0.lock().await;
    agents::update_agent(
        &conn, id, &name,
        phone.as_deref(), city.as_deref(), area.as_deref(),
        address_notes.as_deref(), notes.as_deref(), is_active,
    ).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_agent(state: State<'_, DbState>, id: i64) -> Result<(), String> {
    let conn = state.0.lock().await;
    agents::delete_agent(&conn, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_agent_ledger(
    state: State<'_, DbState>,
    agent_id: i64,
    limit: Option<i64>,
) -> Result<Vec<AgentLedgerEntry>, String> {
    let conn = state.0.lock().await;
    let limit = limit.unwrap_or(50);
    agents::get_agent_ledger_entries(&conn, agent_id, limit).map_err(|e| e.to_string())
}

/// Send stock from Head Office to an agent.
/// Validates that the product has enough qty_in_head_office before sending.
#[tauri::command]
pub async fn send_stock_to_agent(
    state: State<'_, DbState>,
    agent_id: i64,
    product_id: i64,
    qty: i64,
    unit_price: f64,
    notes: Option<String>,
) -> Result<i64, String> {
    if qty <= 0 {
        return Err("Quantity must be positive.".to_string());
    }
    let conn = state.0.lock().await;
    // Validate: product must have enough stock in Head Office.
    let ho_qty: i64 = conn.query_row(
        "SELECT COALESCE(qty_in_head_office, stock_quantity, 0) FROM products WHERE id = ?1",
        rusqlite::params![product_id],
        |r| r.get(0),
    ).map_err(|e| format!("Product not found: {}", e))?;
    if ho_qty < qty {
        return Err(format!(
            "Insufficient stock in Head Office. Available: {}, requested: {}.",
            ho_qty, qty
        ));
    }
    agents::send_stock_to_agent(
        &conn, agent_id, product_id, qty, unit_price, notes.as_deref(),
    ).map_err(|e| e.to_string())
}

/// Return stock from an agent back to Head Office.
/// Validates that the agent has enough stock of this product.
#[tauri::command]
pub async fn return_stock_from_agent(
    state: State<'_, DbState>,
    agent_id: i64,
    product_id: i64,
    qty: i64,
    unit_price: f64,
    notes: Option<String>,
) -> Result<i64, String> {
    if qty <= 0 {
        return Err("Quantity must be positive.".to_string());
    }
    let conn = state.0.lock().await;
    // Validate: agent must have enough stock of this product.
    let agent_qty: i64 = conn.query_row(
        "SELECT COALESCE(SUM(CASE WHEN entry_type = 'stock_sent' THEN qty ELSE 0 END) -
                          SUM(CASE WHEN entry_type = 'stock_returned' THEN qty ELSE 0 END) -
                          SUM(CASE WHEN entry_type = 'sale_reported' THEN qty ELSE 0 END), 0)
         FROM agent_ledger_entries WHERE agent_id = ?1 AND product_id = ?2",
        rusqlite::params![agent_id, product_id],
        |r| r.get(0),
    ).unwrap_or(0);
    if agent_qty < qty {
        return Err(format!(
            "Agent does not have enough stock of this product. Agent has: {}, requested: {}.",
            agent_qty, qty
        ));
    }
    agents::return_stock_from_agent(
        &conn, agent_id, product_id, qty, unit_price, notes.as_deref(),
    ).map_err(|e| e.to_string())
}

/// Agent reports a sale (stock sold by agent to end customer).
#[tauri::command]
pub async fn report_agent_sale(
    state: State<'_, DbState>,
    agent_id: i64,
    product_id: i64,
    qty: i64,
    unit_price: f64,
    notes: Option<String>,
) -> Result<i64, String> {
    if qty <= 0 {
        return Err("Quantity must be positive.".to_string());
    }
    let conn = state.0.lock().await;
    agents::report_agent_sale(
        &conn, agent_id, product_id, qty, unit_price, notes.as_deref(),
    ).map_err(|e| e.to_string())
}

/// Record cash received from an agent.
#[tauri::command]
pub async fn receive_agent_cash(
    state: State<'_, DbState>,
    agent_id: i64,
    amount: f64,
    notes: Option<String>,
) -> Result<i64, String> {
    if amount <= 0.0 {
        return Err("Amount must be positive.".to_string());
    }
    let conn = state.0.lock().await;
    agents::receive_agent_cash(&conn, agent_id, amount, notes.as_deref())
        .map_err(|e| e.to_string())
}

/// Manual balance adjustment. Notes are MANDATORY.
#[tauri::command]
pub async fn adjust_agent_balance(
    state: State<'_, DbState>,
    agent_id: i64,
    amount: f64,
    notes: String,
) -> Result<i64, String> {
    if notes.trim().is_empty() {
        return Err("Notes are mandatory for balance adjustments.".to_string());
    }
    let conn = state.0.lock().await;
    agents::adjust_agent_balance(&conn, agent_id, amount, &notes)
        .map_err(|e| e.to_string())
}

// ============================================================
// v0.11.1 — Share Center (share_logs + customer segments)
// ============================================================

/// Log a share action. Called whenever the user shares a product to a
/// social platform. Creates an audit trail entry in share_logs.
#[tauri::command]
pub async fn log_share(
    state: State<'_, DbState>,
    product_id: Option<i64>,
    platform: String,
    share_angle: Option<String>,
    caption_text: Option<String>,
    notes: Option<String>,
) -> Result<i64, String> {
    let conn = state.0.lock().await;
    let now = chrono::Utc::now().to_rfc3339();
    // 'shared_by' is hardcoded to 'Head Office' for now. Future: track
    // which user/device shared (when multi-user support is added).
    conn.execute(
        "INSERT INTO share_logs (product_id, platform, share_angle, caption_text, shared_by, shared_at, notes)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![
            product_id,
            &platform,
            share_angle.as_deref().unwrap_or(""),
            caption_text.as_deref().unwrap_or(""),
            "Head Office",
            &now,
            notes.as_deref().unwrap_or(""),
        ],
    ).map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

/// Get recent share logs. Returns up to `limit` most recent entries.
/// Optionally filter by product_id (if provided).
#[tauri::command]
pub async fn get_share_logs(
    state: State<'_, DbState>,
    product_id: Option<i64>,
    limit: Option<i64>,
) -> Result<Vec<serde_json::Value>, String> {
    let conn = state.0.lock().await;
    let limit = limit.unwrap_or(50);

    if let Some(pid) = product_id {
        let mut s = conn.prepare(
            "SELECT sl.id, sl.product_id, sl.platform, sl.share_angle, sl.caption_text, sl.shared_by, sl.shared_at, sl.notes,
                    COALESCE(p.name, '(deleted)') AS product_name
             FROM share_logs sl
             LEFT JOIN products p ON sl.product_id = p.id
             WHERE sl.product_id = ?1
             ORDER BY sl.shared_at DESC, sl.id DESC
             LIMIT ?2"
        ).map_err(|e| e.to_string())?;
        let rows = s.query_map(rusqlite::params![pid, limit], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "product_id": row.get::<_, Option<i64>>(1)?,
                "platform": row.get::<_, String>(2)?,
                "share_angle": row.get::<_, String>(3)?,
                "caption_text": row.get::<_, String>(4)?,
                "shared_by": row.get::<_, String>(5)?,
                "shared_at": row.get::<_, String>(6)?,
                "notes": row.get::<_, String>(7)?,
                "product_name": row.get::<_, String>(8)?,
            }))
        }).map_err(|e| e.to_string())?;
        let mut result = Vec::new();
        for r in rows { result.push(r.map_err(|e| e.to_string())?); }
        return Ok(result);
    } else {
        let mut s = conn.prepare(
            "SELECT sl.id, sl.product_id, sl.platform, sl.share_angle, sl.caption_text, sl.shared_by, sl.shared_at, sl.notes,
                    COALESCE(p.name, '(deleted)') AS product_name
             FROM share_logs sl
             LEFT JOIN products p ON sl.product_id = p.id
             ORDER BY sl.shared_at DESC, sl.id DESC
             LIMIT ?1"
        ).map_err(|e| e.to_string())?;
        let rows = s.query_map(rusqlite::params![limit], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "product_id": row.get::<_, Option<i64>>(1)?,
                "platform": row.get::<_, String>(2)?,
                "share_angle": row.get::<_, String>(3)?,
                "caption_text": row.get::<_, String>(4)?,
                "shared_by": row.get::<_, String>(5)?,
                "shared_at": row.get::<_, String>(6)?,
                "notes": row.get::<_, String>(7)?,
                "product_name": row.get::<_, String>(8)?,
            }))
        }).map_err(|e| e.to_string())?;
        let mut result = Vec::new();
        for r in rows { result.push(r.map_err(|e| e.to_string())?); }
        return Ok(result);
    }
}

/// Get customers filtered by segment. Used by the Share Center's bulk
/// WhatsApp broadcast feature.
#[tauri::command]
pub async fn get_customers_by_segment(
    state: State<'_, DbState>,
    segment: Option<String>,
) -> Result<Vec<serde_json::Value>, String> {
    let conn = state.0.lock().await;

    if let Some(seg) = segment {
        let mut s = conn.prepare(
            "SELECT id, name, phone, location, notes, segment, is_active
             FROM customers
             WHERE segment = ?1 AND is_active = 1
             ORDER BY name"
        ).map_err(|e| e.to_string())?;
        let rows = s.query_map(rusqlite::params![seg], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "name": row.get::<_, String>(1)?,
                "phone": row.get::<_, Option<String>>(2)?,
                "location": row.get::<_, Option<String>>(3)?,
                "notes": row.get::<_, Option<String>>(4)?,
                "segment": row.get::<_, String>(5)?,
                "is_active": row.get::<_, i64>(6)? != 0,
            }))
        }).map_err(|e| e.to_string())?;
        let mut result = Vec::new();
        for r in rows { result.push(r.map_err(|e| e.to_string())?); }
        Ok(result)
    } else {
        let mut s = conn.prepare(
            "SELECT id, name, phone, location, notes, segment, is_active
             FROM customers
             WHERE is_active = 1
             ORDER BY name"
        ).map_err(|e| e.to_string())?;
        let rows = s.query_map([], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "name": row.get::<_, String>(1)?,
                "phone": row.get::<_, Option<String>>(2)?,
                "location": row.get::<_, Option<String>>(3)?,
                "notes": row.get::<_, Option<String>>(4)?,
                "segment": row.get::<_, String>(5)?,
                "is_active": row.get::<_, i64>(6)? != 0,
            }))
        }).map_err(|e| e.to_string())?;
        let mut result = Vec::new();
        for r in rows { result.push(r.map_err(|e| e.to_string())?); }
        Ok(result)
    }
}

/// Update a customer's segment. Used by the Customers page to assign
/// segments (women, girls, vip, agent, etc.) for bulk broadcasting.
#[tauri::command]
pub async fn update_customer_segment(
    state: State<'_, DbState>,
    customer_id: i64,
    segment: String,
) -> Result<(), String> {
    let conn = state.0.lock().await;
    conn.execute(
        "UPDATE customers SET segment = ?1 WHERE id = ?2",
        rusqlite::params![&segment, customer_id],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

/// Get all distinct customer segments (for populating the segment filter
/// dropdown in the Share Center).
#[tauri::command]
pub async fn get_customer_segments(state: State<'_, DbState>) -> Result<Vec<String>, String> {
    let conn = state.0.lock().await;
    let mut stmt = conn.prepare(
        "SELECT DISTINCT segment FROM customers WHERE segment IS NOT NULL AND segment != '' ORDER BY segment"
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0)).map_err(|e| e.to_string())?;
    let mut result = Vec::new();
    for r in rows { result.push(r.map_err(|e| e.to_string())?); }
    Ok(result)
}

/// Get products that have NOT been shared in the last X days (or never
/// shared). Used by the Share Center's "Stale Stock" detector.
#[tauri::command]
pub async fn get_stale_products(
    state: State<'_, DbState>,
    days: Option<i64>,
) -> Result<Vec<serde_json::Value>, String> {
    let conn = state.0.lock().await;
    let days = days.unwrap_or(7);
    let cutoff = (chrono::Utc::now() - chrono::Duration::days(days)).to_rfc3339();
    // Products where: status = active AND (no share_log exists OR most
    // recent share_log is older than cutoff).
    let mut stmt = conn.prepare(
        "SELECT p.id, p.name, p.sku, p.sale_price, p.stock_quantity,
                COALESCE(p.images, '[]') AS images,
                MAX(sl.shared_at) AS last_shared_at
         FROM products p
         LEFT JOIN share_logs sl ON sl.product_id = p.id
         WHERE p.status = 'active' AND p.stock_quantity > 0
         GROUP BY p.id, p.name, p.sku, p.sale_price, p.stock_quantity, p.images
         HAVING MAX(sl.shared_at) IS NULL OR MAX(sl.shared_at) < ?1
         ORDER BY (MAX(sl.shared_at) IS NULL) DESC, p.name ASC"
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map(rusqlite::params![&cutoff], |row| {
        Ok(serde_json::json!({
            "id": row.get::<_, i64>(0)?,
            "name": row.get::<_, String>(1)?,
            "sku": row.get::<_, String>(2)?,
            "sale_price": row.get::<_, f64>(3)?,
            "stock_quantity": row.get::<_, i64>(4)?,
            "images": row.get::<_, String>(5)?,
            "last_shared_at": row.get::<_, Option<String>>(6)?,
        }))
    }).map_err(|e| e.to_string())?;
    let mut result = Vec::new();
    for r in rows { result.push(r.map_err(|e| e.to_string())?); }
    Ok(result)
}

// ============================================================
// v0.11.2 — Purchase Trips (landed cost tracking)
// ============================================================

#[tauri::command]
pub async fn get_purchase_trips(state: State<'_, DbState>) -> Result<Vec<PurchaseTripSummary>, String> {
    let conn = state.0.lock().await;
    purchase_trips::get_all_purchase_trips(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_purchase_trip(state: State<'_, DbState>, id: i64) -> Result<serde_json::Value, String> {
    let conn = state.0.lock().await;
    let (trip, items) = purchase_trips::get_purchase_trip(&conn, id).map_err(|e| e.to_string())?;
    // Enrich items with product names
    let mut enriched_items: Vec<serde_json::Value> = Vec::new();
    for item in items {
        let product_name: Option<String> = if let Some(pid) = item.product_id {
            conn.query_row(
                "SELECT name FROM products WHERE id = ?1",
                rusqlite::params![pid],
                |r| r.get(0),
            ).ok()
        } else {
            None
        };
        enriched_items.push(serde_json::json!({
            "id": item.id,
            "trip_id": item.trip_id,
            "product_id": item.product_id,
            "product_name": product_name.unwrap_or_else(|| "(deleted)".to_string()),
            "qty_purchased": item.qty_purchased,
            "unit_purchase_cost": item.unit_purchase_cost,
            "total_purchase_cost": item.total_purchase_cost,
            "expense_allocation_amount": item.expense_allocation_amount,
            "landed_unit_cost": item.landed_unit_cost,
        }));
    }
    Ok(serde_json::json!({
        "trip": trip,
        "items": enriched_items,
    }))
}

#[tauri::command]
pub async fn create_purchase_trip(
    state: State<'_, DbState>,
    trip_date: String,
    source_city: Option<String>,
    supplier_notes: Option<String>,
    travel_cost: Option<f64>,
    transport_cost: Option<f64>,
    food_cost: Option<f64>,
    loading_cost: Option<f64>,
    misc_cost: Option<f64>,
) -> Result<i64, String> {
    let conn = state.0.lock().await;
    purchase_trips::create_purchase_trip(
        &conn,
        &trip_date,
        source_city.as_deref().unwrap_or("Faisalabad"),
        supplier_notes.as_deref(),
        travel_cost.unwrap_or(0.0),
        transport_cost.unwrap_or(0.0),
        food_cost.unwrap_or(0.0),
        loading_cost.unwrap_or(0.0),
        misc_cost.unwrap_or(0.0),
    ).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_purchase_trip(
    state: State<'_, DbState>,
    id: i64,
    trip_date: String,
    source_city: String,
    supplier_notes: Option<String>,
    travel_cost: f64,
    transport_cost: f64,
    food_cost: f64,
    loading_cost: f64,
    misc_cost: f64,
) -> Result<(), String> {
    let conn = state.0.lock().await;
    purchase_trips::update_purchase_trip(
        &conn, id, &trip_date, &source_city, supplier_notes.as_deref(),
        travel_cost, transport_cost, food_cost, loading_cost, misc_cost,
    ).map_err(|e| e.to_string())?;
    // Recalculate allocations since expenses may have changed
    purchase_trips::recalculate_trip_allocations(&conn, id).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn delete_purchase_trip(state: State<'_, DbState>, id: i64) -> Result<(), String> {
    let conn = state.0.lock().await;
    purchase_trips::delete_purchase_trip(&conn, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_trip_item(
    state: State<'_, DbState>,
    trip_id: i64,
    product_id: i64,
    qty_purchased: i64,
    unit_purchase_cost: f64,
) -> Result<i64, String> {
    if qty_purchased <= 0 {
        return Err("Quantity must be positive.".to_string());
    }
    let conn = state.0.lock().await;
    let item_id = purchase_trips::add_trip_item(
        &conn, trip_id, product_id, qty_purchased, unit_purchase_cost,
    ).map_err(|e| e.to_string())?;
    // Recalculate allocations for the whole trip (new item changes proportions)
    purchase_trips::recalculate_trip_allocations(&conn, trip_id).map_err(|e| e.to_string())?;
    Ok(item_id)
}

#[tauri::command]
pub async fn remove_trip_item(state: State<'_, DbState>, item_id: i64) -> Result<(), String> {
    let conn = state.0.lock().await;
    // Get trip_id before deleting so we can recalculate after
    let trip_id: Option<i64> = conn.query_row(
        "SELECT trip_id FROM purchase_trip_items WHERE id = ?1",
        rusqlite::params![item_id],
        |r| r.get(0),
    ).ok();
    purchase_trips::remove_trip_item(&conn, item_id).map_err(|e| e.to_string())?;
    if let Some(tid) = trip_id {
        purchase_trips::recalculate_trip_allocations(&conn, tid).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn recalculate_trip(state: State<'_, DbState>, trip_id: i64) -> Result<(), String> {
    let conn = state.0.lock().await;
    purchase_trips::recalculate_trip_allocations(&conn, trip_id).map_err(|e| e.to_string())
}

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
) -> Result<i64, String> {
    if qty <= 0 {
        return Err("Quantity must be positive.".to_string());
    }
    let conn = state.0.lock().await;
    let now = chrono::Utc::now().to_rfc3339();
    let total = qty as f64 * unit_sale_price;

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
            return Err(format!(
                "Agent does not have enough stock. Agent has: {}, requested: {}.",
                agent_qty, qty
            ));
        }
        // Create agent ledger entry for the sale
        let amount = qty as f64 * unit_sale_price;
        conn.execute(
            "INSERT INTO agent_ledger_entries (agent_id, product_id, entry_type, qty, unit_price, amount, reference_code, notes, entry_date, created_at, updated_at)
             VALUES (?1, ?2, 'sale_reported', ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![
                aid, product_id, qty, unit_sale_price, amount,
                format!("SALE-{}", now),
                notes.as_deref().unwrap_or(""),
                &now, &now, &now,
            ],
        ).map_err(|e| e.to_string())?;
        // Reduce agent stock, increase sold
        conn.execute(
            "UPDATE products SET qty_with_agents = MAX(0, qty_with_agents - ?1), qty_sold = qty_sold + ?2, updated_at = ?3 WHERE id = ?4",
            rusqlite::params![qty, qty, &now, product_id],
        ).map_err(|e| e.to_string())?;
    } else {
        // Direct HO sale — validate HO has enough stock
        let ho_qty: i64 = conn.query_row(
            "SELECT COALESCE(qty_in_head_office, stock_quantity, 0) FROM products WHERE id = ?1",
            rusqlite::params![product_id],
            |r| r.get(0),
        ).map_err(|e| format!("Product not found: {}", e))?;
        if ho_qty < qty {
            return Err(format!(
                "Insufficient stock in Head Office. Available: {}, requested: {}.",
                ho_qty, qty
            ));
        }
        // Reduce HO stock, increase sold
        conn.execute(
            "UPDATE products SET qty_in_head_office = qty_in_head_office - ?1, stock_quantity = stock_quantity - ?2, qty_sold = qty_sold + ?3, updated_at = ?4 WHERE id = ?5",
            rusqlite::params![qty, qty, qty, &now, product_id],
        ).map_err(|e| e.to_string())?;
    }

    // Create sales table entry
    conn.execute(
        "INSERT INTO sales (product_id, sale_channel, sale_type, agent_id, qty, unit_sale_price, total_sale_amount, customer_name, customer_phone, notes, sale_date, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        rusqlite::params![
            product_id,
            &sale_channel,
            if agent_id.is_some() { "agent_sale" } else { "direct_sale" },
            agent_id,
            qty,
            unit_sale_price,
            total,
            customer_name.as_deref().unwrap_or(""),
            customer_phone.as_deref().unwrap_or(""),
            notes.as_deref().unwrap_or(""),
            &now,
            &now,
            &now,
        ],
    ).map_err(|e| e.to_string())?;
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
    conn.execute(
        "UPDATE products SET profit_status = ?1, updated_at = ?2 WHERE id = ?3",
        rusqlite::params![new_status, &now, product_id],
    ).map_err(|e| e.to_string())?;

    Ok(sale_id)
}

/// Get recent sales with product names. Optionally filter by channel or agent.
#[tauri::command]
pub async fn get_sales(
    state: State<'_, DbState>,
    limit: Option<i64>,
) -> Result<Vec<serde_json::Value>, String> {
    let conn = state.0.lock().await;
    let limit = limit.unwrap_or(50);
    let mut stmt = conn.prepare(
        "SELECT s.id, s.product_id, s.sale_channel, s.sale_type, s.agent_id,
                s.qty, s.unit_sale_price, s.total_sale_amount,
                s.customer_name, s.customer_phone, s.notes, s.sale_date,
                COALESCE(p.name, '(deleted)') AS product_name,
                COALESCE(a.name, '') AS agent_name
         FROM sales s
         LEFT JOIN products p ON s.product_id = p.id
         LEFT JOIN agents a ON s.agent_id = a.id
         ORDER BY s.sale_date DESC, s.id DESC
         LIMIT ?1"
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map(rusqlite::params![limit], |row| {
        Ok(serde_json::json!({
            "id": row.get::<_, i64>(0)?,
            "product_id": row.get::<_, i64>(1)?,
            "sale_channel": row.get::<_, String>(2)?,
            "sale_type": row.get::<_, String>(3)?,
            "agent_id": row.get::<_, Option<i64>>(4)?,
            "qty": row.get::<_, i64>(5)?,
            "unit_sale_price": row.get::<_, f64>(6)?,
            "total_sale_amount": row.get::<_, f64>(7)?,
            "customer_name": row.get::<_, String>(8)?,
            "customer_phone": row.get::<_, String>(9)?,
            "notes": row.get::<_, String>(10)?,
            "sale_date": row.get::<_, String>(11)?,
            "product_name": row.get::<_, String>(12)?,
            "agent_name": row.get::<_, String>(13)?,
        }))
    }).map_err(|e| e.to_string())?;
    let mut result = Vec::new();
    for r in rows { result.push(r.map_err(|e| e.to_string())?); }
    Ok(result)
}
