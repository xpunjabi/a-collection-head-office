use crate::catalog::{self, Product, ProductLocationStock};
use crate::inventory::{self, InventorySummary, LowStockItem, DeadStockItem, BestSellerItem};
use crate::customers::{self, Customer, OrderItemInput, OrderHistory};
use crate::reports::{self, SalesReport, InventoryReport, CustomerSummaryReport};
use crate::locations::{self, Location};
use crate::adapters::duckduckgo::{self, WebEvidence};
use crate::ai::{self, AiResponse, KnowledgeEntry};
use crate::utils;
use std::sync::Mutex;
use std::path::Path;
use rusqlite::Connection;
use tauri::State;

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
    let conn = state.0.lock().unwrap();
    catalog::get_all_products(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_product(state: State<'_, DbState>, id: i64) -> Result<Product, String> {
    let conn = state.0.lock().unwrap();
    catalog::get_product_by_id(&conn, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_product(state: State<'_, DbState>, product: Product) -> Result<i64, String> {
    let conn = state.0.lock().unwrap();
    catalog::add_product(&conn, &product).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_product(state: State<'_, DbState>, product: Product) -> Result<(), String> {
    let conn = state.0.lock().unwrap();
    catalog::update_product(&conn, &product).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_product(state: State<'_, DbState>, id: i64) -> Result<(), String> {
    let conn = state.0.lock().unwrap();
    catalog::delete_product(&conn, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_product_locations(state: State<'_, DbState>, product_id: i64) -> Result<Vec<ProductLocationStock>, String> {
    let conn = state.0.lock().unwrap();
    catalog::get_product_locations(&conn, product_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn upsert_product_location(state: State<'_, DbState>, product_id: i64, location_id: i64, quantity: i64) -> Result<(), String> {
    let conn = state.0.lock().unwrap();
    catalog::upsert_product_location(&conn, product_id, location_id, quantity).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn search_products_by_color(state: State<'_, DbState>, color: String) -> Result<Vec<Product>, String> {
    let conn = state.0.lock().unwrap();
    catalog::search_by_color(&conn, &color).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn export_products_csv(state: State<'_, DbState>) -> Result<String, String> {
    let conn = state.0.lock().unwrap();
    catalog::export_to_csv(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn import_products_csv(state: State<'_, DbState>, csv_content: String) -> Result<(), String> {
    let conn = state.0.lock().unwrap();
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
    let conn = state.0.lock().unwrap();
    locations::get_all_locations(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_location(state: State<'_, DbState>, name: String, address: String) -> Result<i64, String> {
    let conn = state.0.lock().unwrap();
    locations::add_location(&conn, &name, &address).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_location(state: State<'_, DbState>, id: i64, name: String, address: String, is_active: bool) -> Result<(), String> {
    let conn = state.0.lock().unwrap();
    locations::update_location(&conn, id, &name, &address, is_active).map_err(|e| e.to_string())
}

// ==================== INVENTORY ====================

#[tauri::command]
pub async fn get_inventory_summary(state: State<'_, DbState>) -> Result<InventorySummary, String> {
    let conn = state.0.lock().unwrap();
    inventory::get_inventory_summary(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_low_stock(state: State<'_, DbState>, threshold: i64) -> Result<Vec<LowStockItem>, String> {
    let conn = state.0.lock().unwrap();
    inventory::get_low_stock_items(&conn, threshold).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_dead_stock(state: State<'_, DbState>, days_limit: i64) -> Result<Vec<DeadStockItem>, String> {
    let conn = state.0.lock().unwrap();
    inventory::get_dead_stock_items(&conn, days_limit).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_best_sellers(state: State<'_, DbState>, limit: i64) -> Result<Vec<BestSellerItem>, String> {
    let conn = state.0.lock().unwrap();
    inventory::get_best_sellers(&conn, limit).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn adjust_stock(state: State<'_, DbState>, product_id: i64, adjustment: i64) -> Result<(), String> {
    let conn = state.0.lock().unwrap();
    inventory::adjust_stock(&conn, product_id, adjustment).map_err(|e| e.to_string())
}

// ==================== CUSTOMERS ====================

#[tauri::command]
pub async fn get_customers(state: State<'_, DbState>) -> Result<Vec<Customer>, String> {
    let conn = state.0.lock().unwrap();
    customers::get_all_customers(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_customer(state: State<'_, DbState>, customer: Customer) -> Result<i64, String> {
    let conn = state.0.lock().unwrap();
    customers::add_customer(&conn, &customer).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_customer(state: State<'_, DbState>, customer: Customer) -> Result<(), String> {
    let conn = state.0.lock().unwrap();
    customers::update_customer(&conn, &customer).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_customer(state: State<'_, DbState>, id: i64) -> Result<(), String> {
    let conn = state.0.lock().unwrap();
    customers::delete_customer(&conn, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_order(state: State<'_, DbState>, customer_id: i64, items: Vec<OrderItemInput>) -> Result<i64, String> {
    let mut conn = state.0.lock().unwrap();
    customers::create_order(&mut conn, customer_id, items).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_customer_history(state: State<'_, DbState>, customer_id: i64) -> Result<Vec<OrderHistory>, String> {
    let conn = state.0.lock().unwrap();
    customers::get_customer_purchase_history(&conn, customer_id).map_err(|e| e.to_string())
}

// ==================== REPORTS ====================

#[tauri::command]
pub async fn get_sales_report(state: State<'_, DbState>, start_date: String, end_date: String) -> Result<SalesReport, String> {
    let conn = state.0.lock().unwrap();
    reports::generate_sales_report(&conn, &start_date, &end_date).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_inventory_report(state: State<'_, DbState>) -> Result<InventoryReport, String> {
    let conn = state.0.lock().unwrap();
    reports::generate_inventory_report(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_customer_report(state: State<'_, DbState>) -> Result<CustomerSummaryReport, String> {
    let conn = state.0.lock().unwrap();
    reports::generate_customer_report(&conn).map_err(|e| e.to_string())
}

// ==================== AI ====================

#[tauri::command]
pub async fn ask_ai(state: State<'_, DbState>, prompt: String, image_data: Option<String>) -> Result<AiResponse, String> {
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
            let conn = state.0.lock().unwrap();
            crate::ai::local_match::check_local_catalog(&conn, &extraction.qr_data, &extraction.ocr_text)
        };
        match match_result {
            Ok(Some(mr)) => {
                println!("[Local Match] Found: id={} title={} confidence={}", mr.item_id, mr.title, mr.confidence);
                fast_path_data = Some(ai::AssistantResult::LocalMatchFound(mr));
            }
            Ok(None) => {
                println!("[Local Match] No match found. Proceeding to web evidence + AI draft.");
                let (api_key, model) = {
                    let conn = state.0.lock().unwrap();
                    let cfg = ai::get_ai_config(&conn)?;
                    (cfg.1.clone(), cfg.2.clone())
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
                    extraction, &Some(prompt.clone()), &api_key, &model, &web_evidence, image_data.as_deref()
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
        let conn = state.0.lock().unwrap();
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
        let conn = state.0.lock().unwrap();
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
        let conn = state.0.lock().unwrap();
        let mut sp = ai::build_system_prompt_with_web(&conn, &prompt, fallback_web_evidence.as_ref())?;
        sp.push_str("\n\n## Product Intake Mode\n\nWhen the user shares a product image, link, code, or description, you MUST:\n1. Analyze all available information\n2. If product information is detected, return a JSON block at the end of your response:\n\n```json\n{\n  \"draft\": {\n    \"name\": \"...\",\n    \"sku\": \"...\",\n    \"category\": \"...\",\n    \"brand\": \"...\",\n    \"fabric\": \"...\",\n    \"color\": \"...\",\n    \"design\": \"...\",\n    \"season\": \"...\",\n    \"cost_price\": 0.0,\n    \"sale_price\": 0.0,\n    \"retail_price\": 0.0,\n    \"description\": \"...\",\n    \"tags\": [\"...\"],\n    \"keywords\": [\"...\"],\n    \"hashtags\": [\"...\"]\n  },\n  \"confidence\": 0.85,\n  \"missing_fields\": [\"stock_location\", \"purchase_cost\"],\n  \"suggested_actions\": [\"Add To Catalog\", \"Edit Draft\", \"Generate Marketing\"]\n}\n```\n\n3. If no product information is detected, respond normally as a business assistant.\n");
        sp
    };

    let response_text = ai::call_ai_provider(&provider, &api_key, &model, &system_prompt, &prompt, image_data.as_deref()).await?;
    {
        let conn = state.0.lock().unwrap();
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
    };
    let conn = state.0.lock().unwrap();
    let id = crate::catalog::add_product(&conn, &product).map_err(|e| e.to_string())?;
    Ok(id)
}

#[tauri::command]
pub async fn save_catalog_draft(state: State<'_, DbState>, draft: crate::ai::catalog_composer::CatalogDraft) -> Result<i64, String> {
    let conn = state.0.lock().unwrap();
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
        cost_price: 0.0,
        sale_price: 0.0,
        purchase_price: 0.0,
        description: draft.notes.clone(),
        tags: if tags.is_empty() { None } else { Some(tags.join(", ")) },
        stock_quantity: 0,
        status: "active".to_string(),
        images: "[]".to_string(),
        supplier_id: None,
        created_at: now.clone(),
        updated_at: now,
    };
    let id = crate::catalog::add_product(&conn, &product).map_err(|e| e.to_string())?;
    Ok(id)
}

#[tauri::command]
pub async fn generate_social_post(state: State<'_, DbState>, product_id: i64) -> Result<crate::ai::marketing_engine::MarketingPost, String> {
    let (api_key, model) = {
        let conn = state.0.lock().unwrap();
        let cfg = ai::get_ai_config(&conn)?;
        (cfg.1.clone(), cfg.2.clone())
    };
    let product = {
        let conn = state.0.lock().unwrap();
        crate::catalog::get_product_by_id(&conn, product_id).map_err(|e| e.to_string())?
    };
    let product_name = &product.name;
    let brand = product.design.as_deref().unwrap_or("");
    let fabric = product.tags.as_deref().unwrap_or("");
    let notes = product.description.as_deref().unwrap_or("");
    crate::ai::marketing_engine::generate_marketing_post(product_name, brand, fabric, notes, &api_key, &model).await
}

#[tauri::command]
pub async fn generate_marketing(state: State<'_, DbState>, product_id: i64) -> Result<Vec<ai::MarketingContent>, String> {
    let (product, provider, api_key, model, has_fb, has_wa) = {
        let conn = state.0.lock().unwrap();
        ai::prepare_marketing_data(&conn, product_id)?
    };
    let prompt = ai::build_marketing_prompt(&product, has_fb, has_wa);
    let posts = ai::generate_marketing_content(&provider, &api_key, &model, &prompt).await?;
    let now = chrono::Utc::now().to_rfc3339();
    {
        let conn = state.0.lock().unwrap();
        for post in &posts {
            conn.execute(
                "INSERT INTO social_posts (product_id, platform, content, caption_type, status, created_at) VALUES (?1, ?2, ?3, ?4, 'draft', ?5)",
                rusqlite::params![product_id, post.platform, post.content, post.caption_type, &now],
            ).map_err(|e| e.to_string())?;
        }
    }
    Ok(posts)
}

#[tauri::command]
pub async fn get_knowledge(state: State<'_, DbState>) -> Result<Vec<KnowledgeEntry>, String> {
    let conn = state.0.lock().unwrap();
    ai::get_all_knowledge(&conn)
}

#[tauri::command]
pub async fn save_knowledge(state: State<'_, DbState>, topic: String, content: String, source: String) -> Result<(), String> {
    let conn = state.0.lock().unwrap();
    ai::save_knowledge(&conn, &topic, &content, &source)
}

#[tauri::command]
pub async fn delete_knowledge(state: State<'_, DbState>, id: i64) -> Result<(), String> {
    let conn = state.0.lock().unwrap();
    ai::delete_knowledge(&conn, id)
}

// ==================== SETTINGS ====================

#[tauri::command]
pub async fn get_settings(state: State<'_, DbState>) -> Result<std::collections::HashMap<String, String>, String> {
    let conn = state.0.lock().unwrap();
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
    let conn = state.0.lock().unwrap();
    set_setting_val(&conn, &key, &value).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn backup_database_now(state: State<'_, DbState>) -> Result<String, String> {
    let conn = state.0.lock().unwrap();
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
