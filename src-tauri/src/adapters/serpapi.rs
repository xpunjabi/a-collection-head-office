use serde::{Serialize, Deserialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebEvidence {
    pub titles: Vec<String>,
    pub snippets: Vec<String>,
    pub result_count: usize,
}

pub async fn fetch_web_evidence(query: &str, api_key: &str) -> Result<WebEvidence, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(8))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    let url = "https://serpapi.com/search.json";
    let res = client
        .get(url)
        .query(&[("q", query), ("api_key", api_key), ("num", "5")])
        .send()
        .await
        .map_err(|e| format!("SerpApi request failed: {}", e))?;

    if !res.status().is_success() {
        let err_body = res.text().await.unwrap_or_default();
        return Err(format!("SerpApi returned error: {}", err_body));
    }

    let body: serde_json::Value = res
        .json()
        .await
        .map_err(|e| format!("Failed to parse SerpApi response: {}", e))?;

    let organic = body["organic_results"]
        .as_array()
        .ok_or_else(|| "No organic_results in SerpApi response".to_string())?;

    let mut titles = Vec::new();
    let mut snippets = Vec::new();

    for result in organic.iter().take(5) {
        if let Some(title) = result["title"].as_str() {
            titles.push(title.to_string());
        }
        if let Some(snippet) = result["snippet"].as_str() {
            snippets.push(snippet.to_string());
        }
    }

    let result_count = titles.len();
    Ok(WebEvidence { titles, snippets, result_count })
}
