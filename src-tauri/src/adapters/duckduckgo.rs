use scraper::{Html, Selector};
use serde::{Serialize, Deserialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebEvidence {
    pub titles: Vec<String>,
    pub snippets: Vec<String>,
    pub image_urls: Vec<String>,
    pub result_count: usize,
}

pub async fn fetch_web_evidence(query: &str) -> Result<WebEvidence, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(8))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    let url = "https://html.duckduckgo.com/html/";
    let res = client
        .get(url)
        .query(&[("q", query)])
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .send()
        .await
        .map_err(|e| format!("DuckDuckGo request failed: {}", e))?;

    if !res.status().is_success() {
        let err_body = res.text().await.unwrap_or_default();
        return Err(format!("DuckDuckGo returned error: {}", err_body));
    }

    let html = res.text().await.map_err(|e| format!("Failed to read response: {}", e))?;
    let doc = Html::parse_document(&html);

    let result_sel = Selector::parse(".result").map_err(|e| format!("Selector error: {}", e))?;
    let title_sel = Selector::parse(".result__title a").map_err(|e| format!("Selector error: {}", e))?;
    let snippet_sel = Selector::parse(".result__snippet").map_err(|e| format!("Selector error: {}", e))?;

    let mut titles = Vec::new();
    let mut snippets = Vec::new();

    for result in doc.select(&result_sel).take(5) {
        if let Some(title_el) = result.select(&title_sel).next() {
            let title = title_el.text().collect::<String>().trim().to_string();
            if !title.is_empty() {
                titles.push(title);
            }
        }
        if let Some(snippet_el) = result.select(&snippet_sel).next() {
            let snippet = snippet_el.text().collect::<String>().trim().to_string();
            if !snippet.is_empty() {
                snippets.push(snippet);
            }
        }
    }

    // Scrape image URLs from all <img> tags in the HTML
    let img_sel = Selector::parse("img").map_err(|e| format!("Selector error: {}", e))?;
    let mut image_urls = Vec::new();
    
    for img in doc.select(&img_sel) {
        if let Some(src) = img.value().attr("src") {
            let url = src.trim().to_string();
            
            // Filter out trivial icons/logos
            if !url.is_empty() 
                && url.starts_with("http") 
                && !url.contains("favicon")
                && !url.contains("logo")
                && !url.contains("icon")
                && !url.contains("spacer")
                && !url.contains("pixel")
                && !url.contains("tracking")
                && image_urls.len() < 5 {
                
                image_urls.push(url);
            }
        }
    }

    let result_count = titles.len();
    Ok(WebEvidence { titles, snippets, image_urls, result_count })
}
