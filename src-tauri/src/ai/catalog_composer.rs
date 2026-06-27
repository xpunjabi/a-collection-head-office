use crate::ai::ingestion::LocalExtractionResult;
use crate::adapters::duckduckgo::WebEvidence;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct CatalogDraft {
    pub title: String,
    pub brand: Option<String>,
    pub fabric: Option<String>,
    pub design_code: Option<String>,
    pub notes: Option<String>,
    pub web_evidence_count: Option<usize>,
    pub web_evidence_snippets: Option<Vec<String>>,
    pub best_image_url: Option<String>,
    // v0.13.8: Price fields added so user can edit them in the draft
    // before saving to catalog. Previously these were hardcoded to 0.
    #[serde(default)]
    pub cost_price: Option<f64>,
    #[serde(default)]
    pub retail_price: Option<f64>,
    #[serde(default)]
    pub sale_price: Option<f64>,
    // v0.13.9: Locally saved image filename (from user upload).
    // When this is set, save_catalog_draft uses it directly instead of
    // trying to download from best_image_url.
    #[serde(default)]
    pub saved_image_filename: Option<String>,
    // v0.14.5: Catalog metadata fields. Previously the AI draft only
    // carried title/brand/fabric/notes — when the user clicked "Add to
    // Catalog", the resulting product row had empty category, season,
    // gender, color, forcing them to manually fill these in the Catalog
    // form. Now the AI generates these directly from the image + OCR.
    // Values must match the dropdown options in Catalog.tsx:
    //   category: '3 Piece' | '2 Piece' | 'Cut Piece' | 'Gents'
    //   season:   'Summer' | 'Winter' | 'Eid Special' | 'Festive' | 'Spring' | 'Autumn'
    //   gender:   'Ladies' | 'Gents' | 'Kids'
    //   color:    free-form text (e.g. 'Maroon', 'Bottle Green')
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub season: Option<String>,
    #[serde(default)]
    pub gender: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
}

pub async fn generate_catalog_draft(
    extraction: &LocalExtractionResult,
    user_instruction: &Option<String>,
    provider: &str,
    api_key: &str,
    model: &str,
    web_evidence: &Option<WebEvidence>,
    image_base64: Option<&str>,
) -> Result<CatalogDraft, String> {
    let mut system_prompt = "\
You are an autonomous fashion catalog AI operating inside a desktop app. \
You ALWAYS have access to local database matching and web search tools. \
Never say 'I don't have access to the internet' or 'I cannot browse'. \
Always provide the best possible answer based on the provided evidence.
You are also provided with an image of the product. \
Analyze the visual details (color, pattern, fabric type, embroidery). \
Combine your visual analysis with the provided OCR text and Web Evidence to generate the most accurate catalog draft.
Your task is to generate a complete product catalog entry from the provided extracted data. \
Return ONLY valid JSON without any markdown formatting, code blocks, or extra text.".to_string();

    if let Some(ref we) = web_evidence {
        system_prompt.push_str("\n\n## Web Evidence (from internet search)\n\nThe following information was found by searching the web for the extracted text. Use it to generate a more accurate catalog entry.\n");
        for (i, title) in we.titles.iter().enumerate() {
            system_prompt.push_str(&format!("\n- **Result {}:** {}\n", i + 1, title));
            if let Some(snippet) = we.snippets.get(i) {
                system_prompt.push_str(&format!("  *Snippet:* {}\n", snippet));
            }
        }
        
        // Add image URLs to the prompt
        if !we.image_urls.is_empty() {
            system_prompt.push_str("\n\n## Image URLs (from internet search)\n\nYou are also provided with image URLs found from the web search. Select the single best image URL that matches the product. Return this URL in the JSON as 'best_image_url'.\n");
            for (i, url) in we.image_urls.iter().enumerate() {
                system_prompt.push_str(&format!("\n- Image {}: {}\n", i + 1, url));
            }
        }
    }

    system_prompt.push_str("\n\nRespond with this exact JSON structure (no markdown, no explanation):\n\
{\"title\": \"...\", \"brand\": \"... or null\", \"fabric\": \"... or null\", \
\"design_code\": \"... or null\", \"notes\": \"... or null\", \"best_image_url\": \"... or null\", \
\"category\": \"... or null\", \"season\": \"... or null\", \"gender\": \"... or null\", \"color\": \"... or null\", \
\"cost_price\": 0.0, \"retail_price\": 0.0, \"sale_price\": 0.0}\n\n\
FIELD RULES:\n\
- category: MUST be one of: \"3 Piece\", \"2 Piece\", \"Cut Piece\", \"Gents\". Pick the closest match for the product.\n\
- season: One of: \"Summer\", \"Winter\", \"Eid Special\", \"Festive\", \"Spring\", \"Autumn\". If unclear, pick the most likely.\n\
- gender: One of: \"Ladies\", \"Gents\", \"Kids\". Infer from the product image + OCR.\n\
- color: A short color name like \"Maroon\", \"Bottle Green\", \"Royal Blue\". Infer from the image.\n\
- prices: cost_price is what the shopkeeper paid; retail_price is the suggested MRP; sale_price is the actual selling price. If unknown, set to 0.\n\
- title: A descriptive product name (brand + fabric + design + piece-count, e.g. \"Nishat 3-Piece Lawn Suit - Maroon Floral\").");

    let mut user_prompt = String::from("Generate a product catalog entry from the following extracted data:\n");
    if let Some(ref qr) = extraction.qr_data {
        user_prompt.push_str(&format!("QR Code Data: {}\n", qr));
    }
    if let Some(ref ocr) = extraction.ocr_text {
        user_prompt.push_str(&format!("OCR Text: {}\n", ocr));
    }
    if let Some(ref instruction) = user_instruction {
        if !instruction.is_empty() {
            user_prompt.push_str(&format!("User Instruction: {}\n", instruction));
        }
    }

    let response = super::call_ai_provider(provider, api_key, model, &system_prompt, &user_prompt, image_base64, None).await?;

    let body = response.trim();

    let json_str = if let Some(start) = body.find('{') {
        let candidate = &body[start..];
        if let Some(end) = candidate.rfind('}') {
            &candidate[..=end]
        } else {
            candidate
        }
    } else {
        body
    };

    let mut draft: CatalogDraft = serde_json::from_str(json_str)
        .map_err(|e| format!("Failed to parse CatalogDraft from AI response: {} — raw: {}", e, json_str))?;

    if let Some(ref we) = web_evidence {
        draft.web_evidence_count = Some(we.result_count);
        draft.web_evidence_snippets = Some(we.snippets.clone());
    }

    Ok(draft)
}
