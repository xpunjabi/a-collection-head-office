use image::GenericImageView;
use ocrs::OcrEngine;
use rqrr::PreparedImage;

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
    let (w, h) = (gray.width() as usize, gray.height() as usize);
    let pixels = gray.into_raw();

    let mut prep = PreparedImage::prepare_from_greyscale(w, h, move |x, y| pixels[y * w + x]);
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
    let (h, w) = (rgb.height() as usize, rgb.width() as usize);
    let raw = rgb.into_raw();

    let mut chw = vec![0.0f32; 3 * h * w];
    for y in 0..h {
        for x in 0..w {
            let src = (y * w + x) * 3;
            let dst = y * w + x;
            chw[dst] = raw[src] as f32 / 255.0;
            chw[1 * h * w + dst] = raw[src + 1] as f32 / 255.0;
            chw[2 * h * w + dst] = raw[src + 2] as f32 / 255.0;
        }
    }

    let tensor = rten_tensor::NdTensor::from_data([3, h, w], chw);

    let engine = OcrEngine::new(ocrs::OcrEngineParams::default())
        .map_err(|e| format!("Failed to create OCR engine: {}", e))?;

    let input = engine.prepare_input(tensor.nd_view())
        .map_err(|e| format!("Failed to prepare OCR input: {}", e))?;

    let text = engine.get_text(&input)
        .map_err(|e| format!("OCR processing failed: {}", e))?;

    let text = text.trim().to_string();
    if text.is_empty() {
        Ok(None)
    } else {
        Ok(Some(text))
    }
}
