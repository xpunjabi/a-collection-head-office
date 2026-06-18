use serde::{Serialize, Deserialize};
use rusqlite::Connection;
use reqwest::Client;
use std::time::Duration;
use serde_json::json;

#[derive(Debug, Serialize, Deserialize)]
pub struct AiResponse {
    pub text: String,
    pub detected_action: Option<String>, // "low_stock", "product_detail", "eid_campaign", etc.
    pub action_data: Option<serde_json::Value>,
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

pub async fn call_ai_provider(provider: &str, api_key: &str, model: &str, prompt: &str) -> Result<String, String> {
    match provider {
        "gemini" => call_gemini(api_key, model, prompt).await,
        "openai" => call_openai(api_key, model, prompt).await,
        "claude" => call_claude(api_key, model, prompt).await,
        "local" => call_local_llm(model, prompt).await,
        _ => Err(format!("Unsupported AI provider: {}", provider)),
    }
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

    // Command: Low Stock
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
                });
            }
        }
    }

    // Command: Search for specific SKU or product details
    if lower_prompt.contains("sku ") || lower_prompt.contains("product ") {
        // Try to extract SKU word
        let words: Vec<&str> = lower_prompt.split_whitespace().collect();
        for (i, word) in words.iter().enumerate() {
            if (*word == "sku" || *word == "product") && i + 1 < words.len() {
                let target = words[i + 1].trim_matches(|c| c == ',' || c == '.' || c == '"' || c == '\'');
                // Search in products by sku or name
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
                            });
                        }
                    }
                }
            }
        }
    }

    None
}

async fn call_gemini(api_key: &str, model: &str, prompt: &str) -> Result<String, String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        model, api_key
    );

    let payload = json!({
        "contents": [{
            "parts": [{"text": prompt}]
        }]
    });

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

async fn call_openai(api_key: &str, model: &str, prompt: &str) -> Result<String, String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;

    let url = "https://api.openai.com/v1/chat/completions";

    let payload = json!({
        "model": model,
        "messages": [{"role": "user", "content": prompt}]
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

async fn call_claude(api_key: &str, model: &str, prompt: &str) -> Result<String, String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;

    let url = "https://api.anthropic.com/v1/messages";

    let payload = json!({
        "model": model,
        "max_tokens": 1024,
        "messages": [{"role": "user", "content": prompt}]
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

async fn call_local_llm(model: &str, prompt: &str) -> Result<String, String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(45))
        .build()
        .map_err(|e| e.to_string())?;

    let url = "http://localhost:11434/api/generate";

    let payload = json!({
        "model": model,
        "prompt": prompt,
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
