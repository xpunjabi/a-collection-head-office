use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MarketingPost {
    pub short_caption: String,
    pub long_caption: String,
    pub hashtags: Vec<String>,
}

pub async fn generate_marketing_post(
    product_name: &str,
    brand: &str,
    fabric: &str,
    notes: &str,
    api_key: &str,
    model: &str,
) -> Result<MarketingPost, String> {
    let system_prompt = "\
You are an autonomous fashion catalog AI operating inside a desktop app. \
You ALWAYS have access to local database matching and web search tools. \
Never say 'I don't have access to the internet' or 'I cannot browse'. \
Always provide the best possible answer based on the provided evidence. \
You are a premium fashion marketing expert for a Pakistani clothing business called 'A Collection'. \
Your task is to generate engaging social media content for a product. \
Write in attractive Roman Urdu or English. \
Return ONLY valid JSON without any markdown formatting, code blocks, or extra text.";

    let user_prompt = format!(
        "Generate social media content for this product:\n\
         Product Name: {}\n\
         Brand: {}\n\
         Fabric: {}\n\
         Notes: {}\n\n\
         Respond with this exact JSON structure (no markdown, no explanation):\n\
         {{\"short_caption\": \"... (1-2 lines for WhatsApp)\", \
         \"long_caption\": \"... (detailed caption for Instagram/Facebook)\", \
         \"hashtags\": [\"#hashtag1\", \"#hashtag2\", \"...\"]}}",
        product_name, brand, fabric, notes
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

    let post: MarketingPost = serde_json::from_str(json_str)
        .map_err(|e| format!("Failed to parse MarketingPost from AI response: {} — raw: {}", e, json_str))?;

    Ok(post)
}
