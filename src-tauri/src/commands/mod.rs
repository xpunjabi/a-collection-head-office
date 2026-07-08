use crate::catalog::{self, Product};
use crate::inventory::{self, InventorySummary, LowStockItem, DeadStockItem, BestSellerItem};
use crate::customers::{self, Customer, OrderItemInput, OrderHistory};
use crate::reports::{self, SalesReport, InventoryReport, CustomerSummaryReport};
use crate::agents::{self, AgentSummary, AgentLedgerEntry};
use crate::purchase_trips::{self, PurchaseTripSummary};
use crate::adapters::duckduckgo::{self, WebEvidence};
use crate::ai::{self, AiResponse};
use crate::utils;
use tauri::async_runtime::Mutex;
use std::path::Path;
use rusqlite::{Connection, params};
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

/// v0.24.1: Download an image from a URL and save it as a product image.
/// Used by the Catalog form's "Add from URL" button. Many e-commerce sites
/// hotlink-protect their images and return 403 for non-browser Referer
/// headers, so we set a browser-like User-Agent and Referer.
///
/// Reuses the existing `process_and_save_image_bytes` pipeline (resize,
/// convert, save as JPEG) — no new image processing code.
#[tauri::command]
pub async fn save_image_from_url(url: String, format_type: String) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .build()
        .map_err(|e| format!("HTTP client build failed: {}", e))?;

    // Derive a Referer from the URL's origin — many CDNs require this.
    let referer = reqwest::Url::parse(&url)
        .ok()
        .and_then(|parsed| parsed.origin().ascii_serialization().parse().ok())
        .unwrap_or_else(|| String::from("https://www.google.com/"));

    let response = client.get(&url)
        .header("Referer", &referer)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch image: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Image URL returned HTTP {}", response.status()));
    }

    let bytes = response.bytes()
        .await
        .map_err(|e| format!("Failed to read image bytes: {}", e))?;

    let images_dir = utils::get_images_dir();
    catalog::process_and_save_image_bytes(&bytes, &images_dir, &format_type)
        .map_err(|e| e.to_string())
}

/// v0.14.8: Save a product image to a dedicated temp folder
/// (%LOCALAPPDATA%\A-Collection-Share\) and immediately open Windows
/// Explorer with the file highlighted/selected. The user can then drag
/// the highlighted file directly into Facebook/Instagram/WhatsApp post
/// composers — universally supported, 100% reliable on Windows.
///
/// v0.14.7 history: previously used tauri-plugin-shell open(path) which
/// tried to open the image in Windows Photos app. This was unreliable
/// (Photos app sometimes failed to launch, especially on Windows 10 with
/// custom configurations). The new approach uses explorer.exe /select
/// which ALWAYS works — it opens the folder containing the image and
/// selects it, no app association required.
///
/// Returns the full file path so frontend can display it in the alert.
#[tauri::command]
pub async fn save_image_for_share(
    base64_data: String,
    product_name: String,
) -> Result<String, String> {
    use base64::Engine as _;
    // v0.14.9: Strip data URI prefix if present. The get_image_as_base64
    // command returns "data:image/jpeg;base64,/9j/..." — the prefix must
    // be removed before base64 decoding, otherwise decode fails with
    // "InvalidByte" error. This was THE bug causing v0.14.6/7/8 to show
    // "Image save nahi ho payi" alert — the Rust command was throwing
    // a base64 decode error on every call.
    let clean_base64: &str = if base64_data.starts_with("data:") {
        if let Some(idx) = base64_data.find(',') {
            &base64_data[idx + 1..]
        } else {
            &base64_data
        }
    } else {
        &base64_data
    };
    let raw = base64::engine::general_purpose::STANDARD
        .decode(clean_base64)
        .map_err(|e| format!("Base64 decode error: {}", e))?;

    // v0.14.8: Use a dedicated folder in LocalAppData instead of Downloads.
    // Reasons:
    //   1. Keeps user's Downloads folder clean (no share images cluttering)
    //   2. LocalAppData is always writable on Windows
    //   3. Path is predictable: %LOCALAPPDATA%\A-Collection-Share\
    //   4. User doesn't need to find the file — Explorer opens to it
    let target_dir = dirs::data_local_dir()
        .ok_or_else(|| "Could not find LocalAppData directory".to_string())?
        .join("A-Collection-Share");

    // Create the folder if it doesn't exist
    std::fs::create_dir_all(&target_dir)
        .map_err(|e| format!("Could not create share folder: {}", e))?;

    // Sanitize product name for filename: replace spaces/special chars with hyphens
    let safe_name: String = product_name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() { c }
            else if c == '-' || c == '_' { c }
            else { '-' }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string();
    let safe_name = if safe_name.is_empty() { "product".to_string() } else { safe_name };

    // Build filename with timestamp
    let now = chrono::Utc::now();
    let timestamp = now.format("%Y%m%d-%H%M%S").to_string();
    let filename = format!("A-Collection_{}_{}.jpg", safe_name, timestamp);
    let file_path = target_dir.join(&filename);

    // Decode the image and re-save as JPEG (handles PNG/WebP/etc. inputs)
    let img = image::load_from_memory(&raw)
        .map_err(|e| format!("Image decode error: {}", e))?;
    img.save_with_format(&file_path, image::ImageFormat::Jpeg)
        .map_err(|e| format!("Image save error: {}", e))?;

    // v0.14.8: Open Windows Explorer with the file highlighted.
    // `explorer.exe /select,"C:\path\to\file.jpg"` is the canonical Windows
    // way to open a folder and pre-select a file. Always works, no app
    // association needed (unlike shell.open which tries to launch the
    // default image viewer).
    //
    // On non-Windows platforms this is a no-op (the file is still saved,
    // user can find it manually). The Tauri app is Windows-only per the
    // release workflow, so this is acceptable.
    #[cfg(target_os = "windows")]
    {
        let path_str = file_path.to_string_lossy().to_string();
        let select_arg = format!("/select,{}", path_str);
        std::process::Command::new("explorer.exe")
            .arg(&select_arg)
            .spawn()
            .map_err(|e| format!("Could not open Explorer: {}", e))?;
    }

    // Return the full path as a string so frontend can show it to the user
    Ok(file_path.to_string_lossy().to_string())
}

/// v0.20.0: Save drafts (caption + HD images) to a user-selected folder.
///
/// Ali bhai's requirement:
/// - "Save Draft" button in Share Center
/// - User can pick a custom folder
/// - Full caption (text) + all HD images of that product saved to folder
///
/// Architecture:
/// - Frontend opens folder picker via @tauri-apps/plugin-dialog open({ directory: true })
/// - Frontend calls this command with chosen folder_path + product_name + captions + image_filenames
/// - This command creates subfolder: "<safe_product_name>_<timestamp>"
/// - Saves caption as "caption_<platform>.txt" (one file per platform)
/// - If multiple platforms, also saves combined "all_captions.txt"
/// - Copies HD images from AppData images dir to subfolder (prefixed with index for ordering)
/// - Writes README.txt summary in subfolder
///
/// Returns the path of the created subfolder.
#[tauri::command]
pub async fn save_drafts_to_folder_with_path(
    folder_path: String,
    product_name: String,
    captions: std::collections::HashMap<String, String>,
    image_filenames: Vec<String>,
) -> Result<String, String> {
    // Sanitize product name for folder name
    let safe_name: String = product_name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() { c }
            else if c == '-' || c == '_' { c }
            else { '-' }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string();
    let safe_name = if safe_name.is_empty() { "product".to_string() } else { safe_name };

    // Timestamp for unique subfolder
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let subfolder_name = format!("{}_{}", safe_name, timestamp);
    let subfolder_path = std::path::Path::new(&folder_path).join(&subfolder_name);

    // Create subfolder
    std::fs::create_dir_all(&subfolder_path)
        .map_err(|e| format!("Failed to create folder: {}", e))?;

    // Save captions as separate .txt files (one per platform)
    for (platform, content) in &captions {
        let filename = format!("caption_{}.txt", platform);
        let filepath = subfolder_path.join(&filename);
        std::fs::write(&filepath, content)
            .map_err(|e| format!("Failed to write {}: {}", filename, e))?;
    }

    // If multiple platforms, also save a combined "all_captions.txt"
    if captions.len() > 1 {
        let mut combined = String::new();
        combined.push_str(&format!("=== {} ===\n\n", product_name));
        for (platform, content) in &captions {
            combined.push_str(&format!("--- {} ---\n{}\n\n", platform.to_uppercase(), content));
        }
        let combined_path = subfolder_path.join("all_captions.txt");
        std::fs::write(&combined_path, combined)
            .map_err(|e| format!("Failed to write all_captions.txt: {}", e))?;
    }

    // Copy HD images from AppData images dir to subfolder
    let images_dir = utils::get_images_dir();
    let mut images_copied = 0;
    let mut image_errors: Vec<String> = Vec::new();

    for (idx, filename) in image_filenames.iter().enumerate() {
        let src_path = images_dir.join(filename);
        if !src_path.exists() {
            image_errors.push(format!("Image not found: {}", filename));
            continue;
        }

        // Prefix with index for ordering, keep original filename
        let dest_filename = format!("{:02}_{}", idx + 1, filename);
        let dest_path = subfolder_path.join(&dest_filename);

        match std::fs::copy(&src_path, &dest_path) {
            Ok(_) => images_copied += 1,
            Err(e) => image_errors.push(format!("Copy {}: {}", filename, e)),
        }
    }

    // Build summary message
    let mut summary = format!(
        "Saved to: {}\nCaptions: {} platforms\nImages: {} copied",
        subfolder_path.display(),
        captions.len(),
        images_copied
    );
    if !image_errors.is_empty() {
        summary.push_str(&format!("\nErrors: {}", image_errors.join("; ")));
    }

    // Write summary as README.txt in subfolder
    let readme_path = subfolder_path.join("README.txt");
    let _ = std::fs::write(&readme_path, &summary);

    Ok(subfolder_path.to_string_lossy().to_string())
}

// ==================== LOCATIONS ====================

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

    let extraction = if let Some(ref b64) = image_data {
        use base64::Engine as _;
        if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(b64) {
            match crate::ai::ingestion::extract_local_data(&bytes) {
                Ok(result) => {
                    Some(result)
                }
                Err(e) => {
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
                fast_path_data = Some(ai::AssistantResult::LocalMatchFound(mr));
            }
            Ok(None) => {
                // Capture provider along with api_key + model so we can pass
                // it to catalog_composer. Previously cfg.0 (provider) was
                // discarded, causing catalog_composer to silently use
                // hardcoded "gemini" — meaning OpenAI/Claude/Ollama users
                // would get a Gemini API call (which fails without a Gemini
                // API key).
                let (provider, api_key, model, base_url) = {
                    let conn = state.0.lock().await;
                    let cfg = ai::get_ai_config(&conn)?;
                    (cfg.0.clone(), cfg.1.clone(), cfg.2.clone(), cfg.3.clone())
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
                            Some(evidence)
                        }
                        Err(e) => {
                            None
                        }
                    }
                } else {
                    None
                };

                match crate::ai::catalog_composer::generate_catalog_draft(
                    extraction, &Some(prompt.clone()), &provider, &api_key, &model, &base_url, &web_evidence, image_data.as_deref()
                ).await {
                    Ok(draft) => {
                        fast_path_data = Some(ai::AssistantResult::NewCatalogDraft(draft));
                    }
                    Err(_e) => {
                    }
                }
            }
            Err(_e) => {
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

    let (provider, api_key, model, base_url) = {
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
                Some(evidence)
            }
            Err(e) => {
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
        &provider, &api_key, &model, &base_url, &system_prompt, &prompt,
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
    // v0.14.4: Auto-generate SKU when empty. The products.sku column has a
    // UNIQUE constraint, and an empty string ("") is itself a value — so
    // two drafts saved without SKU would collide on the second one.
    // Generating `AC-<timestamp_ms>` avoids the collision and gives the
    // user a sensible placeholder they can edit later in the Catalog form.
    let sku = match draft.sku.as_deref() {
        Some(s) if !s.trim().is_empty() => s.trim().to_string(),
        _ => format!("AC-{}", chrono::Utc::now().timestamp_millis()),
    };
    let product = crate::catalog::Product {
        id: None,
        sku,
        name: draft.name.clone().unwrap_or_else(|| "New Product".to_string()),
        category: draft.category.clone(),
        color: draft.color.clone(),
        design: draft.design.clone(),
        season: draft.season.clone(),
        cost_price: draft.cost_price.unwrap_or(0.0),
        sale_price: draft.sale_price.unwrap_or(0.0),
        // v0.14.3: purchase_price is being phased out (removed from Catalog
        // form UI). Default to cost_price for backward DB compat.
        purchase_price: draft.cost_price.unwrap_or(0.0),
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
        // v0.14.3: Persist retail_price from draft (was hardcoded to None —
        // never made it to the products table even when add_product supported it).
        retail_price: draft.retail_price,
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
    // v0.13.9: Check saved_image_filename FIRST (user-uploaded image).
    // If not present, fall back to best_image_url (web image download).
    // This was THE critical bug — user's uploaded image was never saved
    // because only best_image_url was checked.
    let images_json: String = if let Some(ref filename) = draft.saved_image_filename {
        if !filename.is_empty() {
            serde_json::to_string(&[filename]).unwrap_or_else(|_| "[]".to_string())
        } else {
            "[]".to_string()
        }
    } else if let Some(ref url) = draft.best_image_url {
        if !url.is_empty() {
            match download_and_save_image(url).await {
                Ok(filename) => {
                    serde_json::to_string(&[filename]).unwrap_or_else(|_| "[]".to_string())
                }
                Err(e) => {
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
    // v0.14.5: Store gender in tags (no dedicated products.gender column —
    // matches the Catalog.tsx handleSave pattern that also puts designType +
    // gender in tags to avoid a schema migration).
    if let Some(ref gender) = draft.gender {
        if !gender.is_empty() { tags.push(gender.clone()); }
    }
    let product = crate::catalog::Product {
        id: None,
        sku: draft.design_code.clone().unwrap_or_default(),
        name: draft.title,
        // v0.14.5: Pass through AI-generated catalog metadata fields.
        // Previously these were hardcoded to None — when user clicked
        // "Add to Catalog", the resulting product row had empty category,
        // color, season, gender. Now the AI's draft values flow through
        // directly and the user can edit them in the Catalog form if
        // needed.
        category: draft.category.clone(),
        color: draft.color.clone(),
        design: draft.brand.clone(),
        season: draft.season.clone(),
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
    let (provider, api_key, model, base_url) = {
        let conn = state.0.lock().await;
        let cfg = ai::get_ai_config(&conn)?;
        (cfg.0.clone(), cfg.1.clone(), cfg.2.clone(), cfg.3.clone())
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
    let (product, provider, api_key, model, base_url, has_fb, has_wa) = {
        let conn = state.0.lock().await;
        ai::prepare_marketing_data(&conn, product_id)?
    };
    let prompt = ai::build_marketing_prompt(&product, has_fb, has_wa);
    let posts = ai::generate_marketing_content(&provider, &api_key, &model, &base_url, &prompt).await?;
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
pub async fn save_knowledge(state: State<'_, DbState>, topic: String, content: String, source: String) -> Result<(), String> {
    let conn = state.0.lock().await;
    ai::save_knowledge(&conn, &topic, &content, &source)
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
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
    let dest = backup_dir.join(format!("manual_backup_{}.db", timestamp));
    std::fs::copy(db_src, &dest).map_err(|e| format!("Failed to copy: {}", e))?;
    Ok(dest.to_string_lossy().to_string())
}

/// v0.22.0: List available backup files (DB + ZIP) from backup_path.
///
/// Returns Vec of (filename, size_bytes, modified_date_iso) sorted newest first.
/// Used by Settings UI to populate "Restore Backup" dropdown.
#[tauri::command]
pub async fn list_backups(state: State<'_, DbState>) -> Result<Vec<serde_json::Value>, String> {
    let conn = state.0.lock().await;
    let backup_path = get_setting_val(&conn, "backup_path").map_err(|e| e.to_string())?;
    if backup_path.is_empty() {
        return Ok(Vec::new());  // No backup path configured
    }
    let backup_dir = Path::new(&backup_path);
    if !backup_dir.exists() {
        return Ok(Vec::new());
    }

    let mut backups: Vec<serde_json::Value> = Vec::new();
    let entries = std::fs::read_dir(backup_dir).map_err(|e| format!("Read dir: {}", e))?;
    for entry in entries.flatten() {
        let path = entry.path();
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };
        // Only include .db and .zip files
        if !name.ends_with(".db") && !name.ends_with(".zip") { continue; }
        // Skip weekly_report text files
        if name.contains("weekly_report") { continue; }

        let metadata = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        let size = metadata.len();
        let modified = metadata.modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| {
                chrono::DateTime::<chrono::Utc>::from_timestamp(d.as_secs() as i64, 0)
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_default()
            })
            .unwrap_or_default();

        // Skip empty/corrupt DB files (size < 10 KB = likely empty)
        let is_valid = if name.ends_with(".db") { size > 10240 } else { true };

        backups.push(serde_json::json!({
            "name": name,
            "size": size,
            "modified": modified,
            "is_zip": name.ends_with(".zip"),
            "is_valid": is_valid,
        }));
    }

    // Sort by modified date descending (newest first)
    backups.sort_by(|a, b| {
        b["modified"].as_str().unwrap_or("").cmp(a["modified"].as_str().unwrap_or(""))
    });

    Ok(backups)
}

/// v0.22.0: Restore a backup file (DB-only or full ZIP).
///
/// Ali bhai's requirement: "Restore Backup" button in Settings that auto-picks
/// latest valid backup and restores it.
///
/// Behavior:
/// - If filename ends with .db: copy to AppData/database.db (overwrite existing)
/// - If filename ends with .zip: extract database.db + images/ + settings.json
///   from ZIP into AppData
/// - Before overwrite: create a safety backup of current DB (if exists)
///   named `pre_restore_YYYYMMDD_HHMMSS.db`
/// - Returns success message with what was restored
#[tauri::command]
pub async fn restore_backup(
    state: State<'_, DbState>,
    filename: String,
) -> Result<String, String> {
    use std::io::Write;
    let conn = state.0.lock().await;
    let backup_path = get_setting_val(&conn, "backup_path").map_err(|e| e.to_string())?;
    if backup_path.is_empty() {
        return Err("Backup path is not configured.".to_string());
    }
    drop(conn);  // Release DB lock before file operations

    let backup_dir = Path::new(&backup_path);
    let source_path = backup_dir.join(&filename);
    if !source_path.exists() {
        return Err(format!("Backup file not found: {}", filename));
    }

    let db_path = utils::get_db_path();
    let images_dir = utils::get_images_dir();
    let app_dir = utils::get_app_dir();

    // Safety: backup current DB before overwrite (if it exists)
    let safety_timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
    if db_path.exists() {
        let safety_path = backup_dir.join(format!("pre_restore_{}.db", safety_timestamp));
        let _ = std::fs::copy(&db_path, &safety_path);
    }

    if filename.ends_with(".zip") {
        // Full ZIP restore: extract DB + images + settings
        // Phase 1: Read all entries from ZIP (synchronous I/O)
        let zip_file = std::fs::File::open(&source_path)
            .map_err(|e| format!("Failed to open zip: {}", e))?;
        let mut archive = zip::ZipArchive::new(zip_file)
            .map_err(|e| format!("Failed to read zip: {}", e))?;

        std::fs::create_dir_all(&app_dir)
            .map_err(|e| format!("Failed to create app dir: {}", e))?;
        std::fs::create_dir_all(&images_dir)
            .map_err(|e| format!("Failed to create images dir: {}", e))?;

        let mut db_restored = false;
        let mut images_restored = 0;
        let mut settings_data: Option<std::collections::HashMap<String, String>> = None;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)
                .map_err(|e| format!("Failed to read zip entry {}: {}", i, e))?;
            let name = file.name().to_string();

            if name == "database.db" {
                let mut bytes = Vec::new();
                std::io::Read::read_to_end(&mut file, &mut bytes)
                    .map_err(|e| format!("Read DB from zip: {}", e))?;
                std::fs::write(&db_path, &bytes)
                    .map_err(|e| format!("Write DB: {}", e))?;
                db_restored = true;
            } else if name.starts_with("images/") {
                let img_name = name.trim_start_matches("images/");
                if !img_name.is_empty() && !img_name.contains('/') {
                    let dest = images_dir.join(img_name);
                    let mut bytes = Vec::new();
                    std::io::Read::read_to_end(&mut file, &mut bytes)
                        .map_err(|e| format!("Read image: {}", e))?;
                    std::fs::write(&dest, &bytes)
                        .map_err(|e| format!("Write image: {}", e))?;
                    images_restored += 1;
                }
            } else if name == "settings.json" {
                let mut bytes = Vec::new();
                std::io::Read::read_to_end(&mut file, &mut bytes)
                    .map_err(|e| format!("Read settings: {}", e))?;
                let settings_str = String::from_utf8_lossy(&bytes).to_string();
                let settings: std::collections::HashMap<String, String> =
                    serde_json::from_str(&settings_str)
                        .map_err(|e| format!("Parse settings JSON: {}", e))?;
                settings_data = Some(settings);
            }
        }

        if !db_restored {
            return Err("ZIP did not contain database.db".to_string());
        }

        // Phase 2: Apply settings to DB (separate from ZIP I/O to keep future Send)
        let mut settings_restored = false;
        if let Some(settings) = settings_data {
            let conn = state.0.lock().await;
            for (key, value) in &settings {
                let _ = (&*conn).execute(
                    "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
                    rusqlite::params![key, value],
                );
            }
            settings_restored = true;
        }

        Ok(format!(
            "Full backup restored successfully!\n\nDatabase: ✓\nImages: {} restored\nSettings: {}",
            images_restored,
            if settings_restored { "✓" } else { "✗" }
        ))
    } else if filename.ends_with(".db") {
        // DB-only restore
        std::fs::copy(&source_path, &db_path)
            .map_err(|e| format!("Failed to copy DB: {}", e))?;
        Ok("Database restored successfully! Restart the app for changes to take effect.".to_string())
    } else {
        Err("Unsupported backup file type. Only .db and .zip supported.".to_string())
    }
}

/// v0.22.0: Import products from a catalog.json file (URL or local path).
///
/// Ali bhai's requirement after data loss: recover product data from published
/// catalog.json (frontend) if local DB is lost/corrupt.
///
/// Behavior:
/// - Fetches catalog.json from given URL (default: live catalog)
/// - Parses products array
/// - For each product: INSERT into products table (OR IGNORE to skip existing SKUs)
/// - Returns summary (total imported, skipped, failed)
///
/// This is a RECOVERY feature. It will restore: name, SKU, price, retail_price,
/// category, color, fabric, season, description, images.
/// It will NOT restore: cost_price, supplier, sales history, customers, agents,
/// ledger entries — those are private and not in catalog.json.
#[tauri::command]
pub async fn import_from_catalog_json(
    state: State<'_, DbState>,
    catalog_url: Option<String>,
) -> Result<serde_json::Value, String> {
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    struct CatalogProduct {
        id: i64,
        name: String,
        sku: Option<String>,
        sale_price: f64,
        retail_price: Option<f64>,
        category: Option<String>,
        color: Option<String>,
        fabric: Option<String>,
        season: Option<String>,
        description: Option<String>,
        images: Vec<String>,
        availability: String,
    }

    #[derive(Debug, Deserialize)]
    struct CatalogJsonImport {
        brand: Option<String>,
        whatsapp_number: Option<String>,
        products: Vec<CatalogProduct>,
    }

    // Default URL: live catalog
    let url = catalog_url.unwrap_or_else(|| {
        "https://xpunjabi.github.io/a-collection-catalog/data/catalog.json".to_string()
    });

    // Derive the catalog's image base URL from the catalog.json URL.
    // catalog.json lives at <base>/data/catalog.json — images live at <base>/data/images/<filename>.
    // We swap the trailing "catalog.json" with "images/".
    let image_base_url = url
        .rsplitn(2, "catalog.json")
        .last()
        .unwrap_or("")
        .to_string() + "images/";

    // Fetch JSON
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("a-collection-head-office/0.22.5")
        .build()
        .map_err(|e| format!("HTTP client build failed: {}", e))?;

    let response = client.get(&url).send().await
        .map_err(|e| format!("Failed to fetch catalog.json: {}", e))?;
    let body = response.text().await
        .map_err(|e| format!("Failed to read response: {}", e))?;
    let catalog: CatalogJsonImport = serde_json::from_str(&body)
        .map_err(|e| format!("Failed to parse catalog.json: {}", e))?;

    // ====================================================================
    // Phase 1a: Acquire DB lock briefly to filter out SKUs that already exist.
    // Lock is dropped BEFORE any HTTP `.await` for image downloads (Rule #12:
    // NEVER hold DB Mutex across `.await`).
    // ====================================================================
    let existing_skus: std::collections::HashSet<String> = {
        let conn = state.0.lock().await;
        let mut set = std::collections::HashSet::new();
        for product in &catalog.products {
            if let Some(sku) = product.sku.as_ref() {
                if !sku.is_empty() {
                    let exists: Option<i64> = (&*conn).query_row(
                        "SELECT id FROM products WHERE sku = ?1",
                        rusqlite::params![sku],
                        |r| r.get(0),
                    ).ok();
                    if exists.is_some() {
                        set.insert(sku.clone());
                    }
                }
            }
        }
        set
    }; // ← DB lock dropped here

    // Build the list of products to import (skip duplicates).
    // Pre-compute the per-product import data so Phase 1b/2 don't re-read
    // catalog.products with filter logic.
    struct ProductToImport<'a> {
        product: &'a CatalogProduct,
        images_json: String,
        qty_in_ho: i64,
        profit_status: Option<String>,
    }

    let mut to_import: Vec<ProductToImport> = Vec::new();
    let mut skipped = 0;
    for product in &catalog.products {
        let sku = product.sku.as_deref().unwrap_or("");
        if !sku.is_empty() && existing_skus.contains(sku) {
            skipped += 1;
            continue;
        }

        let profit_status = if product.availability == "sold_out" {
            Some("sold_out".to_string())
        } else if product.availability == "low_stock" {
            Some("in_head_office".to_string())
        } else {
            None
        };

        let qty_in_ho: i64 = if product.availability == "sold_out" { 0 } else { 3 };

        let images_json = serde_json::to_string(&product.images)
            .unwrap_or_else(|_| "[]".to_string());

        to_import.push(ProductToImport {
            product,
            images_json,
            qty_in_ho,
            profit_status,
        });
    }

    // ====================================================================
    // Phase 1b: Download all product images to AppData/images/.
    // No DB lock held — HTTP I/O only. Skips files that already exist on disk
    // (so re-running import after a partial failure doesn't re-download).
    // Failures are logged but don't abort the import — product text data
    // still lands in the DB. Ali bhai can manually add missing images later
    // via the Edit Product form.
    // ====================================================================
    let images_dir = crate::utils::get_images_dir();
    let mut images_downloaded = 0u32;
    let mut images_skipped = 0u32;
    let mut image_errors: Vec<String> = Vec::new();

    for item in &to_import {
        for filename in &item.product.images {
            if filename.is_empty() {
                continue;
            }
            let dest_path = images_dir.join(filename);
            if dest_path.exists() {
                images_skipped += 1;
                continue;
            }

            let img_url = format!("{}{}", image_base_url, filename);
            match client.get(&img_url).send().await {
                Ok(resp) => {
                    if resp.status().is_success() {
                        match resp.bytes().await {
                            Ok(bytes) => {
                                if let Err(e) = std::fs::write(&dest_path, &bytes) {
                                    image_errors.push(
                                        format!("{}: write failed: {}", filename, e)
                                    );
                                } else {
                                    images_downloaded += 1;
                                }
                            }
                            Err(e) => image_errors.push(
                                format!("{}: read body failed: {}", filename, e)
                            ),
                        }
                    } else {
                        image_errors.push(
                            format!("{}: HTTP {}", filename, resp.status())
                        );
                    }
                }
                Err(e) => image_errors.push(
                    format!("{}: fetch failed: {}", filename, e)
                ),
            }
        }
    }

    // ====================================================================
    // Phase 2: Acquire DB lock and INSERT all products.
    // No `.await` inside this block — pure sync rusqlite calls.
    // ====================================================================
    let now = chrono::Utc::now().to_rfc3339();
    let mut imported = 0u32;
    let mut failed = 0u32;
    let mut errors: Vec<String> = Vec::new();

    {
        let conn = state.0.lock().await;

        for item in &to_import {
            let product = item.product;
            // 22 columns, 22 placeholders (?1..?22), 22 params — all in same order.
            // No `design` column here because catalog.json does not carry it.
            // created_at + updated_at use distinct slots (?21, ?22) but share
            // the same `&now` value.
            match (&*conn).execute(
                "INSERT INTO products (sku, name, category, color, season, description, images,
                    cost_price, sale_price, purchase_price, stock_quantity, status,
                    qty_in_head_office, qty_with_agents, qty_sold, qty_reserved, profit_status,
                    retail_price, brand, fabric, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22)",
                rusqlite::params![
                    product.sku.as_deref().unwrap_or(""),                  // ?1
                    &product.name,                                          // ?2
                    product.category.as_deref().unwrap_or(""),              // ?3
                    product.color.as_deref().unwrap_or(""),                 // ?4
                    product.season.as_deref().unwrap_or(""),                // ?5
                    product.description.as_deref().unwrap_or(&product.name),// ?6
                    &item.images_json,                                      // ?7
                    0.0,                                                    // ?8  cost_price (private)
                    product.sale_price,                                     // ?9
                    product.sale_price,                                     // ?10 purchase_price fallback
                    item.qty_in_ho,                                         // ?11 stock_quantity
                    "active",                                               // ?12 status
                    item.qty_in_ho,                                         // ?13 qty_in_head_office
                    0,                                                      // ?14 qty_with_agents
                    0,                                                      // ?15 qty_sold
                    0,                                                      // ?16 qty_reserved
                    &item.profit_status,                                    // ?17 profit_status
                    product.retail_price,                                   // ?18 retail_price
                    catalog.brand.as_deref().unwrap_or(""),                 // ?19 brand
                    product.fabric.as_deref().unwrap_or(""),                // ?20 fabric
                    &now,                                                   // ?21 created_at
                    &now,                                                   // ?22 updated_at (same value, distinct slot)
                ],
            ) {
                Ok(_) => imported += 1,
                Err(e) => {
                    failed += 1;
                    errors.push(format!("{}: {}", product.name, e));
                    if errors.len() > 5 { break; }  // Limit error list
                }
            }
        }
    } // ← DB lock dropped here

    Ok(serde_json::json!({
        "total_in_catalog": catalog.products.len(),
        "imported": imported,
        "skipped": skipped,
        "failed": failed,
        "errors": errors,
        "brand": catalog.brand.unwrap_or_default(),
        "whatsapp_number": catalog.whatsapp_number.unwrap_or_default(),
        "images_downloaded": images_downloaded,
        "images_skipped_existing": images_skipped,
        "image_errors": image_errors,
    }))
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

/// v0.14.10: Get current stock held by each agent for a specific product.
/// Used by the Catalog form's "Agent Stock (initial allocation)" section
/// so that editing a product shows the ACTUAL current allocation per
/// agent (not zeroed-out). Returns Vec of {agent_id, agent_name, quantity}.
#[tauri::command]
pub async fn get_product_agent_stock(
    state: State<'_, DbState>,
    product_id: i64,
) -> Result<Vec<serde_json::Value>, String> {
    let conn = state.0.lock().await;
    let rows = agents::get_product_stock_by_agents(&conn, product_id).map_err(|e| e.to_string())?;
    Ok(rows.into_iter().map(|(id, name, qty)| serde_json::json!({
        "agent_id": id,
        "agent_name": name,
        "quantity": qty,
    })).collect())
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
    // v0.14.4: Wrap the read-check-write in BEGIN IMMEDIATE / COMMIT.
    // The Mutex already serializes Tauri commands, but rusqlite's autocommit
    // mode means the read (ho_qty check) and the write (ledger insert +
    // product update) are separate transactions. If the app crashes between
    // them, the ledger entry is committed but the product's
    // qty_in_head_office is not decremented — leading to phantom stock.
    // BEGIN IMMEDIATE acquires a RESERVED lock at the start, ensuring the
    // read+check+write trio is atomic.
    conn.execute("BEGIN IMMEDIATE", []).map_err(|e| e.to_string())?;

    // Validate: product must have enough stock in Head Office.
    let ho_qty: i64 = conn.query_row(
        "SELECT COALESCE(qty_in_head_office, stock_quantity, 0) FROM products WHERE id = ?1",
        rusqlite::params![product_id],
        |r| r.get(0),
    ).map_err(|e| {
        let _ = conn.execute("ROLLBACK", []);
        format!("Product not found: {}", e)
    })?;
    if ho_qty < qty {
        let _ = conn.execute("ROLLBACK", []);
        return Err(format!(
            "Insufficient stock in Head Office. Available: {}, requested: {}.",
            ho_qty, qty
        ));
    }
    let result = agents::send_stock_to_agent(
        &conn, agent_id, product_id, qty, unit_price, notes.as_deref(),
    ).map_err(|e| {
        let _ = conn.execute("ROLLBACK", []);
        e.to_string()
    });
    conn.execute("COMMIT", []).map_err(|e| e.to_string())?;
    result
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
    // v0.14.4: Wrap in BEGIN IMMEDIATE / COMMIT (see send_stock_to_agent
    // comment for full rationale).
    conn.execute("BEGIN IMMEDIATE", []).map_err(|e| e.to_string())?;

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
        let _ = conn.execute("ROLLBACK", []);
        return Err(format!(
            "Agent does not have enough stock of this product. Agent has: {}, requested: {}.",
            agent_qty, qty
        ));
    }
    let result = agents::return_stock_from_agent(
        &conn, agent_id, product_id, qty, unit_price, notes.as_deref(),
    ).map_err(|e| {
        let _ = conn.execute("ROLLBACK", []);
        e.to_string()
    });
    conn.execute("COMMIT", []).map_err(|e| e.to_string())?;
    result
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

    // v0.14.4: Wrap the entire sale-recording flow in BEGIN IMMEDIATE / COMMIT.
    // record_sale writes to 2-3 tables (agent_ledger_entries, products,
    // sales) and does a stock-availability check before mutating. Without
    // a transaction, a crash between any two of these writes leaves the DB
    // inconsistent — e.g., the sales row exists but the product's
    // qty_sold was never incremented, or the ledger entry exists but the
    // sales row doesn't. BEGIN IMMEDIATE acquires a RESERVED lock so the
    // whole "check stock → insert ledger → update product → insert sale →
    // update profit_status" sequence is atomic.
    conn.execute("BEGIN IMMEDIATE", []).map_err(|e| e.to_string())?;

    // Helper closure to rollback on error and convert rusqlite::Error to String.
    // Used for the early-return paths below.
    macro_rules! try_or_rollback {
        ($expr:expr) => {
            match $expr {
                Ok(v) => v,
                Err(e) => {
                    let _ = conn.execute("ROLLBACK", []);
                    return Err(e);
                }
            }
        };
    }

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
            let _ = conn.execute("ROLLBACK", []);
            return Err(format!(
                "Agent does not have enough stock. Agent has: {}, requested: {}.",
                agent_qty, qty
            ));
        }
        // Create agent ledger entry for the sale
        let amount = qty as f64 * unit_sale_price;
        try_or_rollback!(conn.execute(
            "INSERT INTO agent_ledger_entries (agent_id, product_id, entry_type, qty, unit_price, amount, reference_code, notes, entry_date, created_at, updated_at)
             VALUES (?1, ?2, 'sale_reported', ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![
                aid, product_id, qty, unit_sale_price, amount,
                format!("SALE-{}", now),
                notes.as_deref().unwrap_or(""),
                &now, &now, &now,
            ],
        ).map_err(|e| e.to_string()));
        // Reduce agent stock, increase sold
        try_or_rollback!(conn.execute(
            "UPDATE products SET qty_with_agents = MAX(0, qty_with_agents - ?1), qty_sold = qty_sold + ?2, updated_at = ?3 WHERE id = ?4",
            rusqlite::params![qty, qty, &now, product_id],
        ).map_err(|e| e.to_string()));
    } else {
        // Direct HO sale — validate HO has enough stock
        let ho_qty: i64 = match conn.query_row(
            "SELECT COALESCE(qty_in_head_office, stock_quantity, 0) FROM products WHERE id = ?1",
            rusqlite::params![product_id],
            |r| r.get(0),
        ) {
            Ok(v) => v,
            Err(e) => {
                let _ = conn.execute("ROLLBACK", []);
                return Err(format!("Product not found: {}", e));
            }
        };
        if ho_qty < qty {
            let _ = conn.execute("ROLLBACK", []);
            return Err(format!(
                "Insufficient stock in Head Office. Available: {}, requested: {}.",
                ho_qty, qty
            ));
        }
        // Reduce HO stock, increase sold
        try_or_rollback!(conn.execute(
            "UPDATE products SET qty_in_head_office = qty_in_head_office - ?1, stock_quantity = stock_quantity - ?2, qty_sold = qty_sold + ?3, updated_at = ?4 WHERE id = ?5",
            rusqlite::params![qty, qty, qty, &now, product_id],
        ).map_err(|e| e.to_string()));
    }

    // Create sales table entry
    try_or_rollback!(conn.execute(
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
    ).map_err(|e| e.to_string()));
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
    try_or_rollback!(conn.execute(
        "UPDATE products SET profit_status = ?1, updated_at = ?2 WHERE id = ?3",
        rusqlite::params![new_status, &now, product_id],
    ).map_err(|e| e.to_string()));

    conn.execute("COMMIT", []).map_err(|e| {
        let _ = conn.execute("ROLLBACK", []);
        e.to_string()
    })?;

    Ok(sale_id)
}

// ============================================================
// v0.15.0: PUBLIC CATALOG PUBLISHING
// ============================================================

/// Preview what will be published to the public catalog.
/// Returns stats (product count, image count, total size, catalog URL).
/// Called by the UI to show a confirmation modal before publishing.
///
/// v0.15.2: Critical — must acquire DB lock, read settings + products, then
/// RELEASE the lock before doing any async work. rusqlite::Connection is not
/// Send, so holding the Mutex across .await causes "future cannot be sent
/// between threads safely" compile error. This pattern: lock → read → drop →
/// process is the same one used by ask_ai (search ask_ai in commands/mod.rs).
#[tauri::command]
pub async fn preview_catalog_publish(
    state: State<'_, DbState>,
) -> Result<crate::catalog_publish::CatalogPreview, String> {
    // Scope the lock — read all needed data, then release before any .await
    let (brand, whatsapp, repo, preview) = {
        let conn = state.0.lock().await;

        let get_setting = |key: &str| -> String {
            conn.query_row(
                "SELECT value FROM settings WHERE key = ?1",
                params![key],
                |r| r.get::<_, String>(0),
            ).unwrap_or_default()
        };

        let brand = {
            let v = get_setting("catalog_brand");
            if v.is_empty() { "A Collection Narowal".to_string() } else { v }
        };
        let whatsapp = {
            let v = get_setting("catalog_whatsapp");
            if v.is_empty() { "923420830995".to_string() } else { v }
        };
        let repo = {
            let v = get_setting("catalog_repo");
            if v.is_empty() { "xpunjabi/a-collection-catalog".to_string() } else { v }
        };

        // build_preview does file I/O (reading image sizes) but no async work,
        // so it's safe to call while holding the lock.
        let preview = crate::catalog_publish::build_preview(&conn, &brand, &whatsapp, &repo)?;
        (brand, whatsapp, repo, preview)
    };
    // Lock released here

    Ok(preview)
}

/// Publish catalog to GitHub via Contents API.
/// v0.15.2: Same pattern as preview_catalog_publish — lock, read, release,
/// then do async GitHub API calls. Connection is not Send so can't be held
/// across .await.
/// v0.16.0: Now logs every publish attempt to catalog_publish_history table.
#[tauri::command]
pub async fn publish_catalog_to_github(
    state: State<'_, DbState>,
) -> Result<crate::catalog_publish::PublishResult, String> {
    let start_time = std::time::Instant::now();

    // Phase 1: Acquire lock, read all settings + build catalog data, release lock
    let (catalog, image_mapping, repo, github_token, warnings_count, errors_count) = {
        let conn = state.0.lock().await;

        let get_setting = |key: &str| -> String {
            conn.query_row(
                "SELECT value FROM settings WHERE key = ?1",
                params![key],
                |r| r.get::<_, String>(0),
            ).unwrap_or_default()
        };

        let brand = {
            let v = get_setting("catalog_brand");
            if v.is_empty() { "A Collection Narowal".to_string() } else { v }
        };
        let whatsapp = {
            let v = get_setting("catalog_whatsapp");
            if v.is_empty() { "923420830995".to_string() } else { v }
        };
        let repo = {
            let v = get_setting("catalog_repo");
            if v.is_empty() { "xpunjabi/a-collection-catalog".to_string() } else { v }
        };
        let github_token = get_setting("catalog_github_token");
        if github_token.is_empty() {
            return Err("GitHub token not configured. Go to Settings → Catalog and paste your GitHub Personal Access Token.".to_string());
        }

        // Build catalog data (sync — safe to call while holding lock)
        let mut catalog = crate::catalog_publish::build_catalog_json(&conn, &brand, &whatsapp)
            .map_err(|e| format!("Failed to build catalog: {}", e))?;
        let image_mapping = crate::catalog_publish::generate_webp_images(&conn)
            .map_err(|e| format!("Failed to generate images: {}", e))?;

        // v0.16.2: CRITICAL FIX — catalog.json stores original image filenames
        // (e.g. "123456_thumbnail.jpg") but uploaded files have catalog names
        // (e.g. "123456_catalog.jpg"). Without this rewrite, the frontend tries
        // to load data/images/123456_thumbnail.jpg but the file on GitHub is
        // data/images/123456_catalog.jpg → broken images!
        for product in &mut catalog.products {
            let mut catalog_images: Vec<String> = Vec::new();
            for orig_img in &product.images {
                if let Some(catalog_img) = image_mapping.get(orig_img) {
                    catalog_images.push(catalog_img.clone());
                } else {
                    catalog_images.push(orig_img.clone());
                }
            }
            product.images = catalog_images;
        }

        // v0.16.0: Get validation counts for history logging
        let (warnings_count, errors_count) = match crate::catalog_publish::build_preview(&conn, &brand, &whatsapp, &repo) {
            Ok(p) => (p.warnings.len() as i64, p.errors.len() as i64),
            Err(_) => (0, 0),
        };

        (catalog, image_mapping, repo, github_token, warnings_count, errors_count)
    };
    // Lock released here — now safe to do async GitHub API work

    // Phase 2: Upload to GitHub (no DB access, pure async HTTP)
    let result = crate::catalog_publish::upload_to_github(
        &catalog, &image_mapping, &repo, &github_token,
    ).await;

    let duration_ms = start_time.elapsed().as_millis() as i64;

    // Phase 3: Log to history (acquire lock briefly)
    {
        let conn = state.0.lock().await;
        let (success, error_msg) = match &result {
            Ok(r) => (r.success, if r.errors.is_empty() { None } else { Some(r.errors.join("; ")) }),
            Err(e) => (false, Some(e.clone())),
        };
        let products_count = catalog.products.len() as i64;
        let images_uploaded = result.as_ref().map(|r| r.images_uploaded as i64).unwrap_or(0);
        let images_deleted = result.as_ref().map(|r| r.images_deleted as i64).unwrap_or(0);
        let catalog_version = catalog.version.clone();

        let _ = crate::catalog_publish::log_publish_history(
            &conn,
            duration_ms,
            products_count,
            images_uploaded,
            images_deleted,
            success,
            Some(&catalog_version),
            error_msg.as_deref(),
            warnings_count,
            errors_count,
        );
    }

    result
}

/// v0.16.0: Get the last N publish history entries (most recent first).
/// Used by the UI to show publish status + history in Settings.
#[tauri::command]
pub async fn get_catalog_publish_history(
    state: State<'_, DbState>,
    limit: Option<i64>,
) -> Result<Vec<crate::catalog_publish::PublishHistoryEntry>, String> {
    let conn = state.0.lock().await;
    let limit = limit.unwrap_or(10);
    crate::catalog_publish::get_publish_history(&conn, limit).map_err(|e| e.to_string())
}
