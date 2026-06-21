use serde::{Serialize, Deserialize};
use rusqlite::Connection;
use reqwest::Client;
use std::time::Duration;
use serde_json::json;

pub mod catalog_composer;
pub mod ingestion;
pub mod local_match;

#[derive(Debug, Serialize, Deserialize)]
pub struct AiResponse {
    pub text: String,
    pub detected_action: Option<String>,
    pub action_data: Option<serde_json::Value>,
    pub product_draft: Option<ProductDraft>,
    pub confidence: Option<f64>,
    pub missing_fields: Option<Vec<String>>,
    pub suggested_actions: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProductDraft {
    pub name: Option<String>,
    pub sku: Option<String>,
    pub category: Option<String>,
    pub brand: Option<String>,
    pub fabric: Option<String>,
    pub color: Option<String>,
    pub design: Option<String>,
    pub season: Option<String>,
    pub cost_price: Option<f64>,
    pub sale_price: Option<f64>,
    pub retail_price: Option<f64>,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub keywords: Option<Vec<String>>,
    pub hashtags: Option<Vec<String>>,
    pub images: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MarketingContent {
    pub platform: String,
    pub content: String,
    pub caption_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DraftResponse {
    pub draft: ProductDraft,
    pub confidence: f64,
    pub missing_fields: Vec<String>,
    pub suggested_actions: Vec<String>,
}

pub fn try_local_intent(conn: &Connection, prompt: &str) -> Option<AiResponse> {
    parse_local_intent(conn, prompt)
}

pub fn get_ai_config(conn: &Connection) -> Result<(String, String, String), String> {
    get_ai_settings(conn)
}

pub fn log_request(conn: &Connection, prompt: &str, response: &str, provider: &str) -> Result<(), String> {
    log_ai_request(conn, prompt, response, provider)
}

fn default_business_profile() -> &'static str {
    r#"{"business_name":"A Collection","industry":"Ladies Clothing Retail","owner":"Ali","purchase_city":"Faisalabad","sales_areas":["Narowal","Shakargarh","Zafarwal","Nearby Villages"],"sales_channels":["Facebook","WhatsApp","Door To Door"],"target_customers":{"gender":"Female","income_group":"Middle Income","preferred_products":["3 Piece Suits","Lawn","Cotton","Printed Designs","Embroidery"]},"business_goals":["Increase Profit","Increase Sales","Reduce Dead Stock","Improve Customer Retention","Improve Marketing"],"assistant_roles":["Inventory Manager","Sales Analyst","Marketing Assistant","Business Advisor","Purchase Planner"]}"#
}

pub fn get_business_profile(conn: &Connection) -> Result<serde_json::Value, String> {
    let val = conn.query_row(
        "SELECT value FROM settings WHERE key = 'business_profile'",
        [],
        |row| row.get::<_, String>(0),
    );
    match val {
        Ok(v) => serde_json::from_str(&v).map_err(|e| e.to_string()),
        Err(_) => {
            let default = default_business_profile();
            conn.execute(
                "INSERT OR IGNORE INTO settings (key, value) VALUES ('business_profile', ?1)",
                [default],
            ).map_err(|e| e.to_string())?;
            serde_json::from_str(default).map_err(|e| e.to_string())
        }
    }
}

pub fn build_business_context(conn: &Connection) -> Result<String, String> {
    let prod_count: i64 = conn.query_row("SELECT COUNT(*) FROM products WHERE status='active'", [], |r| r.get(0)).unwrap_or(0);
    let cust_count: i64 = conn.query_row("SELECT COUNT(*) FROM customers", [], |r| r.get(0)).unwrap_or(0);
    let order_count: i64 = conn.query_row("SELECT COUNT(*) FROM orders", [], |r| r.get(0)).unwrap_or(0);
    let low_stock: i64 = conn.query_row("SELECT COUNT(*) FROM products WHERE stock_quantity <= 5 AND status='active'", [], |r| r.get(0)).unwrap_or(0);
    let dead_stock: i64 = conn.query_row("SELECT COUNT(*) FROM products WHERE stock_quantity = 0 AND status='active'", [], |r| r.get(0)).unwrap_or(0);
    let total_sales: f64 = conn.query_row("SELECT COALESCE(SUM(total_amount), 0.0) FROM orders", [], |r| r.get(0)).unwrap_or(0.0);
    let total_profit: f64 = conn.query_row("SELECT COALESCE(SUM(profit), 0.0) FROM orders", [], |r| r.get(0)).unwrap_or(0.0);

    let mut context = String::new();
    context.push_str("## Current Business Snapshot\n\n");
    context.push_str(&format!("- **Active Products:** {}\n", prod_count));
    context.push_str(&format!("- **Total Customers:** {}\n", cust_count));
    context.push_str(&format!("- **Total Orders:** {}\n", order_count));
    context.push_str(&format!("- **Total Sales:** ${:.2}\n", total_sales));
    context.push_str(&format!("- **Total Profit:** ${:.2}\n", total_profit));
    context.push_str(&format!("- **Low Stock Items:** {}\n", low_stock));
    context.push_str(&format!("- **Dead Stock Items:** {}\n", dead_stock));
    Ok(context)
}

pub fn build_system_prompt(conn: &Connection, user_prompt: &str) -> Result<String, String> {
    let profile = get_business_profile(conn).unwrap_or_default();
    let context = build_business_context(conn)?;
    let knowledge = get_relevant_knowledge(conn, user_prompt)?;

    let biz_name = profile["business_name"].as_str().unwrap_or("A Collection");
    let industry = profile["industry"].as_str().unwrap_or("Ladies Clothing Retail");
    let owner = profile["owner"].as_str().unwrap_or("the owner");
    let purchase_city = profile["purchase_city"].as_str().unwrap_or("Faisalabad");
    let sales_areas: Vec<&str> = profile["sales_areas"].as_array().map(|a| a.iter().filter_map(|v| v.as_str()).collect()).unwrap_or_default();
    let sales_channels: Vec<&str> = profile["sales_channels"].as_array().map(|a| a.iter().filter_map(|v| v.as_str()).collect()).unwrap_or_default();
    let goals: Vec<&str> = profile["business_goals"].as_array().map(|a| a.iter().filter_map(|v| v.as_str()).collect()).unwrap_or_default();
    let roles: Vec<&str> = profile["assistant_roles"].as_array().map(|a| a.iter().filter_map(|v| v.as_str()).collect()).unwrap_or_default();

    let primary_products: Vec<&str> = profile["target_customers"]["preferred_products"].as_array()
        .map(|a| a.iter().filter_map(|v| v.as_str()).collect()).unwrap_or_default();

    let mut system = String::new();
    system.push_str(&format!("You are {} HeadOffice Assistant.\n\n", biz_name));

    system.push_str("## Identity\n\n");
    system.push_str(&format!("You are the dedicated AI business assistant of {}.\n", biz_name));
    system.push_str("You are not a general-purpose chatbot.\n");
    system.push_str(&format!("Your primary responsibility is helping {} grow sales, manage inventory, improve marketing, increase profits, and make better business decisions.\n", biz_name));
    system.push_str("If a user asks who you are, respond:\n\n");
    system.push_str(&format!("\"I am the {} HeadOffice Assistant.\"\n\n", biz_name));
    system.push_str("You may mention that your underlying AI model is Gemini only if specifically asked.\n");
    system.push_str("Never introduce yourself as Google's assistant.\n\n");

    system.push_str("## Business Overview\n\n");
    system.push_str(&format!("**Business Name:** {}\n", biz_name));
    system.push_str(&format!("**Business Category:** {}\n", industry));
    system.push_str(&format!("**Owner:** {}\n", owner));
    system.push_str("**Primary Products:**\n");
    for p in &primary_products {
        system.push_str(&format!("* {}\n", p));
    }
    system.push_str(&format!("\n**Purchase Source:** {}, Pakistan\n", purchase_city));
    system.push_str("**Primary Sales Areas:**\n");
    for a in &sales_areas {
        system.push_str(&format!("* {}\n", a));
    }
    system.push_str("\n**Sales Channels:**\n");
    for c in &sales_channels {
        system.push_str(&format!("* {}\n", c));
    }

    system.push_str("\n## Core Mission\n\n");
    system.push_str("Your mission is to:\n\n");
    for (i, g) in goals.iter().enumerate() {
        system.push_str(&format!("{}. {}.\n", i + 1, g));
    }

    system.push_str("\n## Assistant Roles\n\n");
    system.push_str("You serve as:\n\n");
    for r in &roles {
        system.push_str(&format!("* {}\n", r));
    }

    system.push_str(&format!("\n## Decision-Making Rules\n\nWhenever making recommendations:\n\n* Prioritize profit.\n* Prioritize customer trust.\n* Avoid risky inventory purchases.\n* Recommend data-driven decisions.\n* Consider local customer behavior.\n* Consider seasonal demand.\n* Consider Pakistani market conditions.\n"));
    for a in &sales_areas {
        system.push_str(&format!("* Consider {} and nearby customer preferences.\n", a));
    }

    let has_fb = sales_channels.iter().any(|c| c.to_lowercase().contains("facebook"));
    let has_wa = sales_channels.iter().any(|c| c.to_lowercase().contains("whatsapp"));

    if has_fb {
        system.push_str("\n## Facebook Marketing Responsibilities\n\nWhen creating Facebook content:\n\n* Write in attractive Roman Urdu or English.\n* Use clear pricing.\n* Focus on value.\n* Focus on trust.\n* Encourage WhatsApp contact.\n* Create urgency when appropriate.\n* Optimize for local audience engagement.\n");
    }

    if has_wa {
        system.push_str("\n## WhatsApp Marketing Responsibilities\n\nWhen creating WhatsApp content:\n\n* Keep messages concise.\n* Highlight price.\n* Highlight design.\n* Highlight availability.\n* Encourage immediate inquiry.\n");
    }

    system.push_str("\n## Inventory Responsibilities\n\nWhen inventory data is available:\n\n* Detect slow-moving stock.\n* Detect fast-selling products.\n* Recommend restocking priorities.\n* Recommend purchase quantities.\n* Warn about low stock.\n\n## Sales Analysis Responsibilities\n\nWhen sales data is available:\n\n* Analyze trends.\n* Identify best-selling categories.\n* Identify weak-performing categories.\n* Recommend actions to improve sales.\n\n## Purchasing Responsibilities\n\nWhen asked what products should be purchased:\n\nAnalyze:\n* Current inventory\n* Seasonal demand\n* Local preferences\n* Historical sales\n\nThen recommend:\n* Product category\n* Quantity\n* Price range\n* Expected demand level\n\n## Local Market Knowledge\n\nAssume the target market primarily consists of:\n* Middle-income households\n* Value-conscious buyers\n* Female clothing shoppers\n* Customers who prefer attractive designs at affordable prices\n\nRecommendations should reflect these realities.\n\n## Communication Style\n\nBe:\n* Professional\n* Practical\n* Business-focused\n* Direct\n* Helpful\n\nAvoid:\n* Generic AI responses\n* Unnecessary disclaimers\n* Irrelevant information\n\n");
    system.push_str(&format!("Always think like an experienced {} business manager working for {}.\n", industry, biz_name));

    if !knowledge.is_empty() {
        system.push_str("\n## Business Memory (Learned Knowledge)\n\nThe following information has been learned from past interactions:\n\n");
        for k in &knowledge {
            system.push_str(&format!("**{}:** {}\n", k.topic, k.content));
        }
    }

    system.push_str(&format!("\n## Live Business Data\n\n{}", context));

    Ok(system)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KnowledgeEntry {
    pub id: i64,
    pub topic: String,
    pub content: String,
    pub source: String,
}

pub fn save_knowledge(conn: &Connection, topic: &str, content: &str, source: &str) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT OR REPLACE INTO ai_knowledge (topic, content, source, created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?4)",
        [topic, content, source, &now],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn get_all_knowledge(conn: &Connection) -> Result<Vec<KnowledgeEntry>, String> {
    let mut stmt = conn.prepare("SELECT id, topic, content, source FROM ai_knowledge ORDER BY topic").map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |row| {
        Ok(KnowledgeEntry {
            id: row.get(0)?,
            topic: row.get(1)?,
            content: row.get(2)?,
            source: row.get(3)?,
        })
    }).map_err(|e| e.to_string())?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row.map_err(|e| e.to_string())?);
    }
    Ok(result)
}

pub fn delete_knowledge(conn: &Connection, id: i64) -> Result<(), String> {
    conn.execute("DELETE FROM ai_knowledge WHERE id = ?1", [id]).map_err(|e| e.to_string())?;
    Ok(())
}

fn get_relevant_knowledge(conn: &Connection, prompt: &str) -> Result<Vec<KnowledgeEntry>, String> {
    let mut stmt = conn.prepare(
        "SELECT id, topic, content, source FROM ai_knowledge WHERE ?1 LIKE '%' || topic || '%' OR topic LIKE '%' || ?1 || '%'"
    ).map_err(|e| e.to_string())?;

    let lower = prompt.to_lowercase();
    let rows = stmt.query_map([&lower], |row| {
        Ok(KnowledgeEntry {
            id: row.get(0)?,
            topic: row.get(1)?,
            content: row.get(2)?,
            source: row.get(3)?,
        })
    }).map_err(|e| e.to_string())?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row.map_err(|e| e.to_string())?);
    }
    Ok(result)
}

pub async fn call_ai_provider(provider: &str, api_key: &str, model: &str, system_prompt: &str, user_prompt: &str, image_data: Option<&str>) -> Result<String, String> {
    match provider {
        "gemini" => call_gemini(api_key, model, system_prompt, user_prompt, image_data).await,
        "openai" => call_openai(api_key, model, system_prompt, user_prompt).await,
        "claude" => call_claude(api_key, model, system_prompt, user_prompt).await,
        "local" => call_local_llm(model, system_prompt, user_prompt).await,
        _ => Err(format!("Unsupported AI provider: {}", provider)),
    }
}

pub fn parse_draft_from_response(text: &str) -> Option<DraftResponse> {
    let body = text.trim();

    // Search for ```json block anywhere in the response
    if let Some(start_pos) = body.find("```json") {
        let after_marker = &body[start_pos + 7..];
        let end_pos = after_marker.find("```").unwrap_or(after_marker.len());
        let json_str = after_marker[..end_pos].trim();
        if let Ok(draft) = serde_json::from_str::<DraftResponse>(json_str) {
            return Some(draft);
        }
    }

    // Fallback: try parsing the entire body as pure JSON
    if body.starts_with('{') || body.starts_with('[') {
        if let Ok(draft) = serde_json::from_str::<DraftResponse>(body) {
            return Some(draft);
        }
    }

    None
}

pub fn prepare_marketing_data(conn: &Connection, product_id: i64) -> Result<(crate::catalog::Product, String, String, String, bool, bool), String> {
    let profile = get_business_profile(conn).unwrap_or_default();
    let has_fb = profile["sales_channels"].as_array().map(|a| a.iter().any(|v| v.as_str().unwrap_or("").to_lowercase().contains("facebook"))).unwrap_or(false);
    let has_wa = profile["sales_channels"].as_array().map(|a| a.iter().any(|v| v.as_str().unwrap_or("").to_lowercase().contains("whatsapp"))).unwrap_or(false);
    let product = crate::catalog::get_product_by_id(conn, product_id).map_err(|e| e.to_string())?;
    let (provider, api_key, model) = get_ai_settings(conn)?;
    Ok((product, provider, api_key, model, has_fb, has_wa))
}

pub fn build_marketing_prompt(product: &crate::catalog::Product, has_fb: bool, has_wa: bool) -> String {
    let product_json = serde_json::to_string(product).unwrap_or_default();
    format!(
        "Generate social media marketing content for the following product in our clothing business 'A Collection' (Faisalabad, Narowal, Shakargarh, Zafarwal). 
Currency: PKR. Write in attractive Roman Urdu or English.

Product Data:
{}

Generate content for the following platforms:
{}{}

Return as JSON array:
[
  {{\"platform\": \"facebook\", \"content\": \"...\", \"caption_type\": \"product_showcase\"}},
  {{\"platform\": \"instagram\", \"content\": \"...\", \"caption_type\": \"product_showcase\"}},
  {{\"platform\": \"tiktok\", \"content\": \"...\", \"caption_type\": \"product_showcase\"}},
  {{\"platform\": \"whatsapp_status\", \"content\": \"...\", \"caption_type\": \"product_announcement\"}},
  {{\"platform\": \"whatsapp_channel\", \"content\": \"...\", \"caption_type\": \"product_announcement\"}}
]

ONLY return the JSON array. No other text.",
        product_json,
        if has_fb { "facebook\n" } else { "" },
        if has_wa { "whatsapp (status + channel)\n" } else { "" }
    )
}

pub async fn generate_marketing_content(provider: &str, api_key: &str, model: &str, prompt: &str) -> Result<Vec<MarketingContent>, String> {
    let sys_prompt = "You are a social media marketing assistant for a Pakistani clothing business. Generate engaging posts in Roman Urdu or English.";
    let response = call_ai_provider(provider, api_key, model, sys_prompt, prompt, None).await?;

    let body = response.trim();
    let json_str = if body.starts_with("```") {
        body.lines()
            .skip_while(|l| !l.contains("```"))
            .skip(1)
            .take_while(|l| !l.contains("```"))
            .collect::<Vec<&str>>()
            .join("\n")
    } else {
        body.to_string()
    };

    serde_json::from_str::<Vec<MarketingContent>>(&json_str).map_err(|e| format!("Marketing parse error: {}", e))
}

fn get_ai_settings(conn: &Connection) -> Result<(String, String, String), String> {
    let mut stmt = conn.prepare("SELECT key, value FROM settings WHERE key IN ('ai_provider', 'ai_api_key', 'ai_model')")
        .map_err(|e| e.to_string())?;
    
    let mut provider = "gemini".to_string();
    let mut api_key = "".to_string();
    let mut model = "gemini-1.5-flash".to_string();

    let mut rows = stmt.query([]).map_err(|e| e.to_string())?;
    while let Some(row) = rows.next().map_err(|e| e.to_string())? {
        let key: String = row.get(0).map_err(|e| e.to_string())?;
        let val: String = row.get(1).map_err(|e| e.to_string())?;
        match key.as_str() {
            "ai_provider" => provider = val,
            "ai_api_key" => api_key = val,
            "ai_model" => model = val,
            _ => {}
        }
    }
    Ok((provider, api_key, model))
}

fn log_ai_request(conn: &Connection, prompt: &str, response: &str, provider: &str) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO ai_logs (prompt, response, provider, created_at) VALUES (?1, ?2, ?3, ?4);",
        [prompt, response, provider, &now],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

fn parse_local_intent(conn: &Connection, prompt: &str) -> Option<AiResponse> {
    let lower_prompt = prompt.to_lowercase();

    if lower_prompt.contains("low stock") || lower_prompt.contains("stock short") {
        if let Ok(mut stmt) = conn.prepare("SELECT sku, name, stock_quantity FROM products WHERE stock_quantity <= 5 AND status = 'active' ORDER BY stock_quantity ASC") {
            let items_iter = stmt.query_map([], |row| {
                let sku: String = row.get(0)?;
                let name: String = row.get(1)?;
                let qty: i64 = row.get(2)?;
                Ok(json!({ "sku": sku, "name": name, "quantity": qty }))
            });
            if let Ok(iter) = items_iter {
                let items: Vec<serde_json::Value> = iter.filter_map(|x| x.ok()).collect();
                let count = items.len();
                let text = if count == 0 {
                    "All products are well stocked. No low stock items found!".to_string()
                } else {
                    let mut list_str = format!("Found {} items with low stock (<= 5 units):\n\n", count);
                    for item in &items {
                        list_str.push_str(&format!("- **{}** (SKU: {}): {} units left\n", item["name"], item["sku"], item["quantity"]));
                    }
                    list_str
                };
                return Some(AiResponse {
                    text,
                    detected_action: Some("low_stock".to_string()),
                    action_data: Some(json!(items)),
                    product_draft: None,
                    confidence: None,
                    missing_fields: None,
                    suggested_actions: None,
                });
            }
        }
    }

    if lower_prompt.contains("sku ") || lower_prompt.contains("product ") {
        let words: Vec<&str> = lower_prompt.split_whitespace().collect();
        for (i, word) in words.iter().enumerate() {
            if (*word == "sku" || *word == "product") && i + 1 < words.len() {
                let target = words[i + 1].trim_matches(|c| c == ',' || c == '.' || c == '"' || c == '\'');
                if let Ok(mut stmt) = conn.prepare("SELECT sku, name, category, sale_price, stock_quantity, description FROM products WHERE sku LIKE ?1 OR name LIKE ?1") {
                    let search_term = format!("%{}%", target);
                    if let Ok(mut rows) = stmt.query([&search_term]) {
                        if let Ok(Some(row)) = rows.next() {
                            let sku: String = row.get(0).unwrap_or_default();
                            let name: String = row.get(1).unwrap_or_default();
                            let category: String = row.get(2).unwrap_or_default();
                            let price: f64 = row.get(3).unwrap_or(0.0);
                            let qty: i64 = row.get(4).unwrap_or(0);
                            let desc: String = row.get(5).unwrap_or_default();

                            let text = format!(
                                "Here are the details for **{}**:\n\n* **SKU**: {}\n* **Category**: {}\n* **Price**: ${:.2}\n* **Current Stock**: {} units\n* **Description**: {}\n",
                                name, sku, category, price, qty, desc
                            );
                            
                            return Some(AiResponse {
                                text,
                                detected_action: Some("product_detail".to_string()),
                                action_data: Some(json!({ "sku": sku, "name": name, "price": price, "stock": qty })),
                                product_draft: None,
                                confidence: None,
                                missing_fields: None,
                                suggested_actions: None,
                            });
                        }
                    }
                }
            }
        }
    }

    None
}

fn detect_mime_type(b64: &str) -> &'static str {
    if b64.starts_with("iVBORw0KGgo") {
        "image/png"
    } else if b64.starts_with("/9j/") {
        "image/jpeg"
    } else if b64.starts_with("UklGR") {
        "image/webp"
    } else if b64.starts_with("R0lGOD") {
        "image/gif"
    } else {
        "image/jpeg"
    }
}

async fn call_gemini(api_key: &str, model: &str, system_prompt: &str, user_prompt: &str, image_data: Option<&str>) -> Result<String, String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(45))
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        model, api_key
    );

    let mut parts = vec![json!({"text": user_prompt})];

    let has_image = image_data.is_some();

    if let Some(b64) = image_data {
        let mime = detect_mime_type(b64);
        parts.push(json!({
            "inlineData": {
                "mimeType": mime,
                "data": b64
            }
        }));
    }

    let mut payload = json!({
        "system_instruction": {
            "parts": [{"text": system_prompt}]
        },
        "contents": [{
            "parts": parts
        }]
    });

    // googleSearch tool conflicts with inline image data in Gemini API.
    // Only enable it for text-only requests.
    if !has_image {
        if let Some(obj) = payload.as_object_mut() {
            obj.insert("tools".to_string(), json!([{"googleSearch": {}}]));
        }
    }

    let res = client.post(&url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Failed sending request to Gemini: {}", e))?;

    if !res.status().is_success() {
        let err_body = res.text().await.unwrap_or_default();
        return Err(format!("Gemini API returned error: {}", err_body));
    }

    let res_json: serde_json::Value = res.json()
        .await
        .map_err(|e| format!("Failed parsing Gemini response: {}", e))?;

    let text = res_json["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .ok_or_else(|| "Failed to extract text from Gemini response".to_string())?;

    Ok(text.to_string())
}

async fn call_openai(api_key: &str, model: &str, system_prompt: &str, user_prompt: &str) -> Result<String, String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;

    let url = "https://api.openai.com/v1/chat/completions";

    let payload = json!({
        "model": model,
        "messages": [
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": user_prompt}
        ]
    });

    let res = client.post(url)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Failed sending request to OpenAI: {}", e))?;

    if !res.status().is_success() {
        let err_body = res.text().await.unwrap_or_default();
        return Err(format!("OpenAI API returned error: {}", err_body));
    }

    let res_json: serde_json::Value = res.json()
        .await
        .map_err(|e| format!("Failed parsing OpenAI response: {}", e))?;

    let text = res_json["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| "Failed to extract text from OpenAI response".to_string())?;

    Ok(text.to_string())
}

async fn call_claude(api_key: &str, model: &str, system_prompt: &str, user_prompt: &str) -> Result<String, String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;

    let url = "https://api.anthropic.com/v1/messages";

    let payload = json!({
        "model": model,
        "max_tokens": 1024,
        "system": system_prompt,
        "messages": [{"role": "user", "content": user_prompt}]
    });

    let res = client.post(url)
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Failed sending request to Claude: {}", e))?;

    if !res.status().is_success() {
        let err_body = res.text().await.unwrap_or_default();
        return Err(format!("Claude API returned error: {}", err_body));
    }

    let res_json: serde_json::Value = res.json()
        .await
        .map_err(|e| format!("Failed parsing Claude response: {}", e))?;

    let text = res_json["content"][0]["text"]
        .as_str()
        .ok_or_else(|| "Failed to extract text from Claude response".to_string())?;

    Ok(text.to_string())
}

async fn call_local_llm(model: &str, system_prompt: &str, user_prompt: &str) -> Result<String, String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(45))
        .build()
        .map_err(|e| e.to_string())?;

    let url = "http://localhost:11434/api/generate";

    let full_prompt = format!("{}\n\nUser: {}", system_prompt, user_prompt);

    let payload = json!({
        "model": model,
        "prompt": full_prompt,
        "stream": false
    });

    let res = client.post(url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Ollama (Local LLM) connection failed: {}. Make sure Ollama is running.", e))?;

    if !res.status().is_success() {
        let err_body = res.text().await.unwrap_or_default();
        return Err(format!("Local LLM returned error: {}", err_body));
    }

    let res_json: serde_json::Value = res.json()
        .await
        .map_err(|e| format!("Failed parsing local LLM response: {}", e))?;

    let text = res_json["response"]
        .as_str()
        .ok_or_else(|| "Failed to extract text from Local LLM response".to_string())?;

    Ok(text.to_string())
}
