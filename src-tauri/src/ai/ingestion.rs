use image::GenericImageView;
use ocrs::OcrEngine;
use rqrr::PreppedImage;

pub struct LocalExtractionResult {
    pub qr_data: Option<String>,
    pub ocr_text: Option<String>,
}

pub fn extract_local_data(image_bytes: &[u8]) -> Result<LocalExtractionResult, String> {
    let img = image::load_from_memory(image_bytes).map_err(|e| format!("Failed to load image: {}", e))?;

    let qr_data = decode_qr(&img).ok().flatten();

    let ocr_text = run_ocr(&img).ok().flatten();

    Ok(LocalExtractionResult { qr_data, ocr_text })
}

fn decode_qr(img: &image::DynamicImage) -> Result<Option<String>, String> {
    let gray = img.to_luma8();
    let mut prep = PreppedImage::prepare(gray);
    let grids = prep.detect_grids();

    if grids.is_empty() {
        return Ok(None);
    }

    let mut results = Vec::new();
    for grid in &grids {
        if let Ok((_, content)) = grid.decode() {
            results.push(content);
        }
    }

    if results.is_empty() {
        Ok(None)
    } else {
        Ok(Some(results.join("; ")))
    }
}

fn run_ocr(img: &image::DynamicImage) -> Result<Option<String>, String> {
    let rgb = img.to_rgb8();
    let (width, height) = img.dimensions();
    let input = ocrs::OcrInput::from_bytes(&rgb, (width, height))
        .map_err(|e| format!("Failed to create OCR input: {}", e))?;

    let engine = OcrEngine::new(ocrs::OcrEngineParams::default())
        .map_err(|e| format!("Failed to create OCR engine: {}", e))?;

    let output = engine.run(&input)
        .map_err(|e| format!("OCR processing failed: {}", e))?;

    let text = output.to_text();
    let text = text.trim();

    if text.is_empty() {
        Ok(None)
    } else {
        Ok(Some(text.to_string()))
    }
}
