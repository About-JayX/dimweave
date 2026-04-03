//! Image compression pipeline for Claude API.
//!
//! Replicates the Claude Code v2.1.89 resize strategy:
//! 1. If buffer ≤ TARGET_BYTES and dimensions ≤ MAX_DIM → use as-is
//! 2. Resize to MAX_DIM, try JPEG quality steps
//! 3. Fallback: resize to FALLBACK_WIDTH + JPEG quality 20

use base64::Engine;
use image::imageops::FilterType;
use image::DynamicImage;
use std::io::Cursor;

const TARGET_BYTES: usize = 3_932_160; // 3.75 MB (Claude Code `mL`)
const MAX_DIM: u32 = 2000;
const FALLBACK_WIDTH: u32 = 1000;
const QUALITY_STEPS: &[u8] = &[80, 60, 40, 20];

pub struct CompressedImage {
    pub base64: String,
    pub media_type: String,
}

/// Read an image file, compress if needed, return base64 + media_type.
pub async fn compress_for_claude(path: &str) -> Result<CompressedImage, String> {
    let raw = tokio::fs::read(path).await.map_err(|e| format!("read {path}: {e}"))?;
    let img = image::load_from_memory(&raw).map_err(|e| format!("decode {path}: {e}"))?;
    let (w, h) = (img.width(), img.height());

    // Fast path: small enough already
    if raw.len() <= TARGET_BYTES && w <= MAX_DIM && h <= MAX_DIM {
        let media_type = guess_media_type(path);
        let b64 = base64::engine::general_purpose::STANDARD.encode(&raw);
        return Ok(CompressedImage { base64: b64, media_type });
    }

    // Resize if dimensions exceed limit
    let resized = if w > MAX_DIM || h > MAX_DIM {
        img.resize(MAX_DIM, MAX_DIM, FilterType::Lanczos3)
    } else {
        img.clone()
    };

    // Try JPEG at decreasing quality
    for &q in QUALITY_STEPS {
        let buf = encode_jpeg(&resized, q)?;
        if buf.len() <= TARGET_BYTES {
            let b64 = base64::engine::general_purpose::STANDARD.encode(&buf);
            return Ok(CompressedImage { base64: b64, media_type: "image/jpeg".into() });
        }
    }

    // Fallback: aggressive resize + lowest quality
    let small = resized.resize(FALLBACK_WIDTH, FALLBACK_WIDTH, FilterType::Lanczos3);
    let buf = encode_jpeg(&small, 20)?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&buf);
    Ok(CompressedImage { base64: b64, media_type: "image/jpeg".into() })
}

fn encode_jpeg(img: &DynamicImage, quality: u8) -> Result<Vec<u8>, String> {
    let mut buf = Cursor::new(Vec::new());
    let rgb = img.to_rgb8();
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, quality);
    image::ImageEncoder::write_image(
        encoder,
        rgb.as_raw(),
        rgb.width(),
        rgb.height(),
        image::ExtendedColorType::Rgb8,
    )
    .map_err(|e| format!("jpeg encode: {e}"))?;
    Ok(buf.into_inner())
}

fn guess_media_type(path: &str) -> String {
    let ext = path.rsplit('.').next().unwrap_or("").to_lowercase();
    match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        _ => "image/jpeg",
    }
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guess_media_type_from_extension() {
        assert_eq!(guess_media_type("/tmp/foo.png"), "image/png");
        assert_eq!(guess_media_type("/tmp/bar.JPG"), "image/jpeg");
        assert_eq!(guess_media_type("/tmp/baz.webp"), "image/webp");
        assert_eq!(guess_media_type("/tmp/unknown"), "image/jpeg");
    }
}
