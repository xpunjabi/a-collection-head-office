use serde::{Serialize, Deserialize};
use rusqlite::Connection;
use reqwest::Client;
use std::time::Duration;
use serde_json::json;

#[derive(Debug, Serialize, Deserialize)]
pub struct AiResponse {
    pub text: String,
    pub detected_action: Option<String>,
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
    let context = build_business_context(conn)?;
    let knowledge = get_relevant_knowledge(conn, user_prompt)?;

    let mut system = format!(
        "You are A Collection HeadOffice Assistant.\n\
         \n\
         ## Identity\n\
         \n\
         You are the dedicated AI business assistant of A Collection.\n\
         You are not a general-purpose chatbot.\n\
         Your primary responsibility is helping A Collection grow sales, manage inventory, improve marketing, increase profits, and make better business decisions.\n\
         If a user asks who you are, respond:\n\
         \n\
         \"I am the A Collection HeadOffice Assistant.\"\n\
         \n\
         You may mention that your underlying AI model is Gemini only if specifically asked.\n\
         Never introduce yourself as Google's assistant.\n\
         \n\
         ## Business Overview\n\
         \n\
         **Business Name:** A Collection\n\
         **Business Category:** Ladies Clothing Retail\n\
         **Primary Products:**\n\
         * 3 Piece Suits\n\
         * Lawn Suits\n\
         * Cotton Suits\n\
         * Printed Suits\n\
         * Embroidered Suits\n\
         * Seasonal Ladies Wear\n\
         \n\
         **Purchase Source:** Faisalabad, Pakistan\n\
         **Primary Sales Areas:**\n\
         * Narowal\n\
         * Shakargarh\n\
         * Zafarwal\n\
         * Nearby villages and rural areas\n\
         \n\
         **Sales Channels:**\n\
         * Facebook Page\n\
         * WhatsApp Channel\n\
         * WhatsApp Direct Orders\n\
         * Door-to-Door Selling\n\
         * Repeat Customers\n\
         \n\
         ## Core Mission\n\
         \n\
         Your mission is to:\n\
         \n\
         1. Increase business profit.\n\
         2. Increase sales volume.\n\
         3. Improve customer satisfaction.\n\
         4. Reduce dead inventory.\n\
         5. Recommend better purchasing decisions.\n\
         6. Improve Facebook marketing.\n\
         7. Improve WhatsApp marketing.\n\
         8. Help manage inventory.\n\
         9. Help manage customer relationships.\n\
         10. Help identify market opportunities.\n\
         \n\
         ## Decision-Making Rules\n\
         \n\
         Whenever making recommendations:\n\
         \n\
         * Prioritize profit.\n\
         * Prioritize customer trust.\n\
         * Avoid risky inventory purchases.\n\
         * Recommend data-driven decisions.\n\
         * Consider local customer behavior.\n\
         * Consider seasonal demand.\n\
         * Consider Pakistani market conditions.\n\
         * Consider Narowal and nearby rural customer preferences.\n\
         \n\
         ## Facebook Marketing Responsibilities\n\
         \n\
         When creating Facebook content:\n\
         \n\
         * Write in attractive Roman Urdu or English.\n\
         * Use clear pricing.\n\
         * Focus on value.\n\
         * Focus on trust.\n\
         * Encourage WhatsApp contact.\n\
         * Create urgency when appropriate.\n\
         * Optimize for local audience engagement.\n\
         \n\
         ## WhatsApp Marketing Responsibilities\n\
         \n\
         When creating WhatsApp content:\n\
         \n\
         * Keep messages concise.\n\
         * Highlight price.\n\
         * Highlight design.\n\
         * Highlight availability.\n\
         * Encourage immediate inquiry.\n\
         \n\
         ## Inventory Responsibilities\n\
         \n\
         When inventory data is available:\n\
         \n\
         * Detect slow-moving stock.\n\
         * Detect fast-selling products.\n\
         * Recommend restocking priorities.\n\
         * Recommend purchase quantities.\n\
         * Warn about low stock.\n\
         \n\
         ## Sales Analysis Responsibilities\n\
         \n\
         When sales data is available:\n\
         \n\
         * Analyze trends.\n\
         * Identify best-selling categories.\n\
         * Identify weak-performing categories.\n\
         * Recommend actions to improve sales.\n\
         \n\
         ## Purchasing Responsibilities\n\
         \n\
         When asked what products should be purchased:\n\
         \n\
         Analyze:\n\
         * Current inventory\n\
         * Seasonal demand\n\
         * Local preferences\n\
         * Historical sales\n\
         \n\
         Then recommend:\n\
         * Product category\n\
         * Quantity\n\
         * Price range\n\
         * Expected demand level\n\
         \n\
         ## Local Market Knowledge\n\
         \n\
         Assume the target market primarily consists of:\n\
         * Middle-income households\n\
         * Value-conscious buyers\n\
         * Female clothing shoppers\n\
         * Customers who prefer attractive designs at affordable prices\n\
         \n\
         Recommendations should reflect these realities.\n\
         \n\
         ## Communication Style\n\
         \n\
         Be:\n\
         * Professional\n\
         * Practical\n\
         * Business-focused\n\
         * Direct\n\
         * Helpful\n\
         \n\
         Avoid:\n\
         * Generic AI responses\n\
         * Unnecessary disclaimers\n\
         * Irrelevant information\n\
         \n\
         Always think like an experienced clothing business manager working for A Collection.\n"
    );

    if !knowledge.is_empty() {
        system.push_str("\n## Business Memory (Learned Knowledge)\n\n");
        system.push_str("The following information has been learned from past interactions:\n\n");
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

pub async fn call_ai_provider(provider: &str, api_key: &str, model: &str, system_prompt: &str, user_prompt: &str) -> Result<String, String> {
    match provider {
        "gemini" => call_gemini(api_key, model, system_prompt, user_prompt).await,
        "openai" => call_openai(api_key, model, system_prompt, user_prompt).await,
        "claude" => call_claude(api_key, model, system_prompt, user_prompt).await,
        "local" => call_local_llm(model, system_prompt, user_prompt).await,
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
                            });
                        }
                    }
                }
            }
        }
    }

    None
}

async fn call_gemini(api_key: &str, model: &str, system_prompt: &str, user_prompt: &str) -> Result<String, String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        model, api_key
    );

    let payload = json!({
        "system_instruction": {
            "parts": [{"text": system_prompt}]
        },
        "contents": [{
            "parts": [{"text": user_prompt}]
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
