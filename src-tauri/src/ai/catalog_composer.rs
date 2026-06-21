use crate::ai::ingestion::LocalExtractionResult;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct CatalogDraft {
    pub title: String,
    pub brand: Option<String>,
    pub fabric: Option<String>,
    pub design_code: Option<String>,
    pub notes: Option<String>,
}

pub async fn generate_catalog_draft(
    extraction: &LocalExtractionResult,
    user_instruction: &Option<String>,
    api_key: &str,
    model: &str,
) -> Result<CatalogDraft, String> {
    let system_prompt = "\
You are a catalog assistant for a clothing/fashion business. \
Your task is to generate a complete product catalog entry from the provided extracted data. \
Return ONLY valid JSON without any markdown formatting, code blocks, or extra text.";

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

    user_prompt.push_str(
        "\nRespond with this exact JSON structure (no markdown, no explanation):\n\
         {\"title\": \"...\", \"brand\": \"... or null\", \"fabric\": \"... or null\", \
         \"design_code\": \"... or null\", \"notes\": \"... or null\"}",
    );

    let response = super::call_ai_provider("gemini", api_key, model, system_prompt, &user_prompt, None).await?;

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

    let draft: CatalogDraft = serde_json::from_str(json_str)
        .map_err(|e| format!("Failed to parse CatalogDraft from AI response: {} — raw: {}", e, json_str))?;

    Ok(draft)
}
