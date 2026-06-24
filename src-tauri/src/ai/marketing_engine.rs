use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MarketingPost {
    pub short_caption: String,
    pub long_caption: String,
    pub hashtags: Vec<String>,
}

/// Supported social platforms with their hashtag conventions.
///
/// Each platform has different best practices for hashtag count and style.
/// Reference:
/// - Instagram: up to 30 allowed, 5-10 recommended. Mix trending + niche.
///   Trending tags include #reelvsfeed, #instafashion, #ootd.
/// - Facebook: 0-2 hashtags. Too many hurts reach. Brand + 1 topical tag.
/// - WhatsApp: 0 hashtags. WhatsApp is a messaging app, hashtags are not
///   clickable and look spammy. Keep messages clean and personal.
/// - Twitter/X: 1-2 hashtags max due to 280 char limit. Use trending tags.
/// - TikTok: 3-5 hashtags. MUST include #fyp and #foryou for reach.
///
/// The `platform` parameter controls:
///   1. The prompt's platform-specific instruction (caption style + length)
///   2. The requested hashtag count
///   3. The platform-specific trending tags to include
pub fn platform_hashtag_count(platform: &str) -> usize {
    match platform.to_lowercase().as_str() {
        "instagram" => 10,
        "tiktok" => 5,
        "facebook" => 2,
        "twitter" | "twitter/x" | "x" => 2,
        "whatsapp" | "whatsapp_status" | "whatsapp_channel" => 0,
        _ => 5,
    }
}

pub fn platform_trending_tags(platform: &str) -> Vec<&'static str> {
    match platform.to_lowercase().as_str() {
        "instagram" => vec!["#instafashion", "#ootd", "#reelvsfeed", "#pakistaniweddingwear", "#lawncollection"],
        "tiktok" => vec!["#fyp", "#foryou", "#pakistanifashion", "#lawn", "#tiktokfashion"],
        "facebook" => vec!["#ACollection", "#PakistaniFashion"],
        "twitter" | "twitter/x" | "x" => vec!["#PakistaniFashion", "#LawnCollection"],
        "whatsapp" | "whatsapp_status" | "whatsapp_channel" => vec![],
        _ => vec![],
    }
}

/// Generate platform-aware marketing content.
///
/// The `platform` parameter must be one of: "whatsapp", "facebook",
/// "instagram", "twitter" (or "twitter/x"), "tiktok". The function builds a
/// platform-specific prompt that:
///   - Specifies the caption style (long-form for IG/FB, short for WhatsApp,
///     punchy for Twitter, hook-driven for TikTok)
///   - Specifies the hashtag count per platform convention
///   - Lists platform-specific trending tags to mix in
///
/// Returns a MarketingPost with `short_caption` (for WhatsApp/preview),
/// `long_caption` (full caption), and `hashtags` (already prefixed with #).
pub async fn generate_marketing_post(
    product_name: &str,
    brand: &str,
    fabric: &str,
    notes: &str,
    provider: &str,
    api_key: &str,
    model: &str,
    platform: Option<&str>,
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

    let (platform_instruction, hashtag_count, _trending_tags) = match platform {
        Some(p) => {
            let count = platform_hashtag_count(p);
            let trending = platform_trending_tags(p);
            let instr = match p.to_lowercase().as_str() {
                "whatsapp" | "whatsapp_status" | "whatsapp_channel" => format!(
                    "Platform: WhatsApp\n\
                     Caption style: Personal, friendly broadcast message. 2-3 short lines max.\n\
                     No emojis overload — keep it clean and easy to read on mobile.\n\
                     Include price and inquiry prompt (e.g., 'For order WhatsApp us').\n\
                     Hashtag count: 0 (WhatsApp does not use hashtags; do not include any # tags)."
                ),
                "facebook" => format!(
                    "Platform: Facebook\n\
                     Caption style: Engaging 3-5 line post with a hook line, product details, and call-to-action.\n\
                     Tone: Trustworthy, community-oriented. Mention value and contact method.\n\
                     Hashtag count: {} — use these trending tags where relevant: {}",
                    count, trending.join(", ")
                ),
                "instagram" => format!(
                    "Platform: Instagram\n\
                     Caption style: Trendy, emoji-rich caption with line breaks for readability.\n\
                     Start with a strong hook. Include product story + price + CTA in bio.\n\
                     Hashtag count: {} — mix niche + trending tags from this list: {}",
                    count, trending.join(", ")
                ),
                "twitter" | "twitter/x" | "x" => format!(
                    "Platform: Twitter/X\n\
                     Caption style: Punchy, under 200 characters. One strong line + price.\n\
                     Hashtag count: {} — use these trending tags: {}",
                    count, trending.join(", ")
                ),
                "tiktok" => format!(
                    "Platform: TikTok\n\
                     Caption style: Hook-driven, casual, trending-sound-friendly.\n\
                     2-3 lines max. Use Gen-Z friendly tone.\n\
                     Hashtag count: {} — MUST include #fyp and #foryou + these: {}",
                    count, trending.join(", ")
                ),
                _ => format!(
                    "Platform: General social media\n\
                     Hashtag count: {}",
                    count
                ),
            };
            (instr, count, trending)
        }
        None => (
            "Platform: Generic (generate content suitable for all platforms)".to_string(),
            8,
            Vec::new(),
        ),
    };

    let user_prompt = format!(
        "Generate social media content for this product:\n\
         Product Name: {}\n\
         Brand: {}\n\
         Fabric: {}\n\
         Notes: {}\n\n\
         {}\n\n\
         Respond with this exact JSON structure (no markdown, no explanation):\n\
         {{\"short_caption\": \"... (1-2 lines for WhatsApp/preview)\", \
         \"long_caption\": \"... (detailed caption for the platform)\", \
         \"hashtags\": [\"#hashtag1\", \"#hashtag2\", \"...\"]}}\n\n\
         IMPORTANT:\n\
         - The hashtags array MUST contain exactly {} hashtags (or 0 if platform is WhatsApp).\n\
         - Each hashtag MUST start with #.\n\
         - If trending tags were specified, include 2-3 of them in the hashtags array.",
        product_name, brand, fabric, notes,
        platform_instruction,
        hashtag_count
    );

    let response = super::call_ai_provider(provider, api_key, model, system_prompt, &user_prompt, None).await?;

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

    let mut post: MarketingPost = serde_json::from_str(json_str)
        .map_err(|e| format!("Failed to parse MarketingPost from AI response: {} — raw: {}", e, json_str))?;

    // Defensive: ensure every hashtag starts with #. AI sometimes forgets.
    post.hashtags = post.hashtags.into_iter().map(|h| {
        let trimmed = h.trim();
        if trimmed.starts_with('#') { trimmed.to_string() } else { format!("#{}", trimmed) }
    }).collect();

    // Defensive: enforce platform hashtag count. If AI gave more than requested,
    // truncate. If less, do not pad (better to have relevant few than padded junk).
    if hashtag_count > 0 && post.hashtags.len() > hashtag_count {
        post.hashtags.truncate(hashtag_count);
    }

    Ok(post)
}
