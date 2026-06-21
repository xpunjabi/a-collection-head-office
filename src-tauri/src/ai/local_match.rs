use rusqlite::Connection;

pub struct LocalMatchResult {
    pub item_id: String,
    pub title: String,
    pub design_code: Option<String>,
    pub confidence: f32,
}

pub fn check_local_catalog(
    conn: &Connection,
    qr_data: &Option<String>,
    ocr_text: &Option<String>,
) -> Result<Option<LocalMatchResult>, String> {
    if let Some(ref qr) = qr_data {
        let qr = qr.trim();
        if !qr.is_empty() {
            if let Some(result) = try_match_qr(conn, qr)? {
                return Ok(Some(result));
            }
        }
    }

    if let Some(ref text) = ocr_text {
        let text = text.trim();
        if !text.is_empty() {
            if let Some(result) = try_match_ocr(conn, text)? {
                return Ok(Some(result));
            }
        }
    }

    Ok(None)
}

fn try_match_qr(conn: &Connection, qr: &str) -> Result<Option<LocalMatchResult>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, name, COALESCE(product_code, ''), COALESCE(design, '')
             FROM products
             WHERE status = 'active'
               AND (sku = ?1 OR product_code = ?1 OR name = ?1)
             LIMIT 1",
        )
        .map_err(|e| format!("Failed to prepare QR match query: {}", e))?;

    let mut rows = stmt
        .query(rusqlite::params![qr])
        .map_err(|e| format!("Failed to execute QR match query: {}", e))?;

    if let Some(row) = rows.next().map_err(|e| format!("Failed to read QR match row: {}", e))? {
        let id: i64 = row.get(0).map_err(|e| format!("Failed to get id: {}", e))?;
        let name: String = row.get(1).map_err(|e| format!("Failed to get name: {}", e))?;
        let product_code: String = row.get(2).unwrap_or_default();
        let design: String = row.get(3).unwrap_or_default();

        let design_code = if !product_code.is_empty() {
            Some(product_code)
        } else if !design.is_empty() {
            Some(design)
        } else {
            None
        };

        return Ok(Some(LocalMatchResult {
            item_id: id.to_string(),
            title: name,
            design_code,
            confidence: 1.0,
        }));
    }

    Ok(None)
}

fn try_match_ocr(conn: &Connection, text: &str) -> Result<Option<LocalMatchResult>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, name, COALESCE(product_code, ''), COALESCE(design, '')
             FROM products
             WHERE status = 'active'
               AND (INSTR(?1, sku) > 0
                    OR INSTR(?1, product_code) > 0
                    OR INSTR(?1, name) > 0
                    OR INSTR(?1, design) > 0)
             LIMIT 1",
        )
        .map_err(|e| format!("Failed to prepare OCR match query: {}", e))?;

    let mut rows = stmt
        .query(rusqlite::params![text])
        .map_err(|e| format!("Failed to execute OCR match query: {}", e))?;

    if let Some(row) = rows.next().map_err(|e| format!("Failed to read OCR match row: {}", e))? {
        let id: i64 = row.get(0).map_err(|e| format!("Failed to get id: {}", e))?;
        let name: String = row.get(1).map_err(|e| format!("Failed to get name: {}", e))?;
        let product_code: String = row.get(2).unwrap_or_default();
        let design: String = row.get(3).unwrap_or_default();

        let design_code = if !product_code.is_empty() {
            Some(product_code)
        } else if !design.is_empty() {
            Some(design)
        } else {
            None
        };

        return Ok(Some(LocalMatchResult {
            item_id: id.to_string(),
            title: name,
            design_code,
            confidence: 1.0,
        }));
    }

    Ok(None)
}
