use crate::ai::ingestion::LocalExtractionResult;
use crate::adapters::serpapi::WebEvidence;
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
}

pub async fn generate_catalog_draft(
    extraction: &LocalExtractionResult,
    user_instruction: &Option<String>,
    api_key: &str,
    model: &str,
    web_evidence: &Option<WebEvidence>,
) -> Result<CatalogDraft, String> {
    let mut system_prompt = "\
You are a catalog assistant for a clothing/fashion business. \
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
    }

    system_prompt.push_str("\n\nRespond with this exact JSON structure (no markdown, no explanation):\n\
{\"title\": \"...\", \"brand\": \"... or null\", \"fabric\": \"... or null\", \
\"design_code\": \"... or null\", \"notes\": \"... or null\"}");

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

    let response = super::call_ai_provider("gemini", api_key, model, &system_prompt, &user_prompt, None).await?;

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
