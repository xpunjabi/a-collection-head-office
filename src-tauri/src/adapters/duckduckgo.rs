use scraper::{Html, Selector};
use serde::{Serialize, Deserialize};
use std::time::Duration;
use tokio::task::JoinSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebEvidence {
    pub titles: Vec<String>,
    pub snippets: Vec<String>,
    pub image_urls: Vec<String>,
    pub result_count: usize,
}

/// URL patterns that indicate a trivial / non-product image (icons, tracking pixels, UI chrome).
const BLOCKLIST_PATTERNS: &[&str] = &[
    "favicon",
    "logo",
    "icon",
    "spacer",
    "pixel",
    "tracking",
    "1x1",
    "blank.gif",
    "arrow",
    "chevron",
    "external-content-", // DDG favicon proxy prefix
    "duckduckgo.com/assets/", // DDG UI icons
    "data:image", // inline base64
];

fn is_trivial_image_url(url: &str) -> bool {
    let lower = url.to_lowercase();
    BLOCKLIST_PATTERNS.iter().any(|p| lower.contains(p))
}

/// Normalize a raw `src` attribute into a fully-qualified https URL.
/// Handles: `https://...`, `http://...`, `//host/...` (protocol-relative).
/// Returns `None` for relative URLs (e.g. `/img/foo.png`) or empty strings,
/// because we cannot reliably resolve them without a base URL.
fn normalize_url(src: &str) -> Option<String> {
    let trimmed = src.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.starts_with("https://") || trimmed.starts_with("http://") {
        Some(trimmed.to_string())
    } else if trimmed.starts_with("//") {
        Some(format!("https:{}", trimmed))
    } else {
        None
    }
}

/// DuckDuckGo HTML results wrap real result URLs in a redirector:
///   //duckduckgo.com/l/?uddg=ENCODED_URL&rut=...
/// This fn extracts and URL-decodes the inner target URL.
fn decode_ddg_redirect(href: &str) -> Option<String> {
    if href.is_empty() {
        return None;
    }
    // Find the uddg= parameter (works regardless of leading // or https://)
    let needle = "uddg=";
    let idx = href.find(needle)?;
    let after = &href[idx + needle.len()..];
    let end = after.find('&').unwrap_or(after.len());
    let encoded = &after[..end];
    if encoded.is_empty() {
        return None;
    }
    let decoded = urlencoding::decode(encoded).ok()?;
    let s = decoded.to_string();
    if s.starts_with("http://") || s.starts_with("https://") {
        Some(s)
    } else {
        None
    }
}

/// Extract up to `limit` real result page URLs from the DDG HTML document
/// by parsing each `.result__title a` href through `decode_ddg_redirect`.
fn extract_result_page_urls(doc: &Html, limit: usize) -> Vec<String> {
    let mut urls = Vec::new();
    let sel = match Selector::parse(".result__title a") {
        Ok(s) => s,
        Err(_) => return urls,
    };
    for a in doc.select(&sel).take(limit) {
        if let Some(href) = a.value().attr("href") {
            if let Some(decoded) = decode_ddg_redirect(href) {
                if !urls.contains(&decoded) {
                    urls.push(decoded);
                }
            }
        }
    }
    urls
}

/// Fetch each page URL in parallel and extract `og:image` (preferred) or
/// the first `<img width >= 200>` as a fallback. Network errors are silently
/// skipped (one bad page should not break the whole feature).
async fn fetch_og_images(client: &reqwest::Client, page_urls: Vec<String>) -> Vec<String> {
    let mut join_set: JoinSet<Vec<String>> = JoinSet::new();

    for url in page_urls {
        let client = client.clone();
        join_set.spawn(async move {
            let res = client.get(&url).send().await;
            match res {
                Ok(r) if r.status().is_success() => {
                    let html = r.text().await.unwrap_or_default();
                    if html.is_empty() {
                        return Vec::new();
                    }
                    let doc = Html::parse_document(&html);
                    let mut found: Vec<String> = Vec::new();

                    // 1. og:image meta tags
                    if let Ok(sel) = Selector::parse(r#"meta[property="og:image"]"#) {
                        for meta in doc.select(&sel) {
                            if let Some(content) = meta.value().attr("content") {
                                if let Some(normalized) = normalize_url(content) {
                                    if !is_trivial_image_url(&normalized) && !found.contains(&normalized) {
                                        found.push(normalized);
                                    }
                                }
                            }
                        }
                    }
                    // Some sites use `name="og:image"` instead of `property="og:image"`
                    if found.is_empty() {
                        if let Ok(sel) = Selector::parse(r#"meta[name="og:image"]"#) {
                            for meta in doc.select(&sel) {
                                if let Some(content) = meta.value().attr("content") {
                                    if let Some(normalized) = normalize_url(content) {
                                        if !is_trivial_image_url(&normalized) && !found.contains(&normalized) {
                                            found.push(normalized);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    // Twitter card image as another fallback
                    if found.is_empty() {
                        if let Ok(sel) = Selector::parse(r#"meta[name="twitter:image"]"#) {
                            for meta in doc.select(&sel) {
                                if let Some(content) = meta.value().attr("content") {
                                    if let Some(normalized) = normalize_url(content) {
                                        if !is_trivial_image_url(&normalized) && !found.contains(&normalized) {
                                            found.push(normalized);
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // 2. Last-resort: first <img> with width >= 200 attribute
                    if found.is_empty() {
                        if let Ok(sel) = Selector::parse("img") {
                            for img in doc.select(&sel) {
                                let width_ok = img
                                    .value()
                                    .attr("width")
                                    .and_then(|w| w.trim_end_matches("px").parse::<u32>().ok())
                                    .map(|w| w >= 200)
                                    .unwrap_or(false);
                                if !width_ok {
                                    continue;
                                }
                                if let Some(src) = img.value().attr("src") {
                                    if let Some(normalized) = normalize_url(src) {
                                        if !is_trivial_image_url(&normalized) {
                                            found.push(normalized);
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }

                    found
                }
                _ => Vec::new(),
            }
        });
    }

    let mut all_images: Vec<String> = Vec::new();
    while let Some(res) = join_set.join_next().await {
        match res {
            Ok(images) => {
                for img in images {
                    if !all_images.contains(&img) {
                        all_images.push(img);
                    }
                }
            }
            Err(_) => continue,
        }
    }
    all_images
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

    // NOTE: `scraper::Html` is NOT `Send` (it internally uses tendril::Tendril which
    // contains a `Cell<usize>` refcount). Tauri commands require `Send` futures, so we
    // must scope all `Html` usage to a block that ends BEFORE any subsequent `.await`.
    // We extract everything we need into `Send`-safe Vec<String> values here.
    let (titles, snippets, mut image_urls, result_page_urls) = {
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

        // --- Image URL collection ---
        // Step 1: scrape <img> tags scoped to .result blocks (not global), with stricter filters.
        let mut image_urls: Vec<String> = Vec::new();
        if let Ok(img_sel) = Selector::parse(".result img") {
            for img in doc.select(&img_sel) {
                if image_urls.len() >= 5 {
                    break;
                }
                if let Some(src) = img.value().attr("src") {
                    if let Some(normalized) = normalize_url(src) {
                        if !is_trivial_image_url(&normalized) && !image_urls.contains(&normalized) {
                            image_urls.push(normalized);
                        }
                    }
                }
            }
        }

        // Step 2 (prepare): extract top 3 result page URLs for og:image fetching.
        // The actual HTTP fetch happens OUTSIDE this block (after `doc` is dropped).
        let result_page_urls = extract_result_page_urls(&doc, 3);

        (titles, snippets, image_urls, result_page_urls)
        // `doc` is dropped here — safe to `.await` after this point.
    };

    // Step 2 (execute): fetch top 3 result pages and extract og:image / twitter:image meta tags.
    // This is the PRIMARY source of real product photos — DDG HTML page itself only has favicons.
    if !result_page_urls.is_empty() {
        let og_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .map_err(|e| format!("Failed to build OG client: {}", e))?;

        let og_images = fetch_og_images(&og_client, result_page_urls).await;
        for img in og_images {
            if image_urls.len() >= 5 {
                break;
            }
            if !image_urls.contains(&img) {
                image_urls.push(img);
            }
        }
    }

    let result_count = titles.len();
    Ok(WebEvidence { titles, snippets, image_urls, result_count })
}
