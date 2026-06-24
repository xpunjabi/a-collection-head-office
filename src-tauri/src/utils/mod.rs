use std::path::PathBuf;
use std::fs;

pub fn get_app_dir() -> PathBuf {
    let mut base = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    base.push("com.airdropia.collectionheadoffice");
    // Ensure app directory exists
    let _ = fs::create_dir_all(&base);
    base
}

pub fn get_db_path() -> PathBuf {
    let mut dir = get_app_dir();
    dir.push("database.db");
    dir
}

pub fn get_images_dir() -> PathBuf {
    let mut dir = get_app_dir();
    dir.push("images");
    let _ = fs::create_dir_all(&dir);
    dir
}

/// Format a money amount using the configured currency code.
///
/// This centralizes currency formatting so all AI prompts, weekly reports,
/// and business context strings render money consistently. Previously the
/// codebase hardcoded `${:.2}` everywhere, which was wrong for a Pakistani
/// business whose business_profile declares `"currency": "PKR"`.
///
/// Conventions:
/// - "PKR" -> "Rs. 2500.00" (Pakistani Rupee, common rendering; uses "Rs."
///   prefix to avoid the ₨ font gap on some systems)
/// - "USD" -> "$ 2500.00"
/// - "EUR" -> "€ 2500.00"
/// - other -> "<CODE> 2500.00"
///
/// Decimals are kept (2 places) for accounting precision. UI rendering
/// can strip them where appropriate.
pub fn format_money(amount: f64, currency: &str) -> String {
    let prefix = match currency.to_uppercase().as_str() {
        "PKR" => "Rs.",
        "USD" => "$",
        "EUR" => "€",
        "GBP" => "£",
        "INR" => "₹",
        other => other,
    };
    format!("{} {:.2}", prefix, amount)
}

