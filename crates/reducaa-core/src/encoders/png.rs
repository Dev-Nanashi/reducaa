use std::io::Cursor;

use image::{ImageEncoder, codecs::png::PngEncoder};

use crate::config::RasterImage;
use crate::errors::CompressionError;

/// Encode a `RasterImage` as PNG.
///
/// When `lossless` is true, uses maximum compression effort.
/// Otherwise, uses the default compression level for a good speed/size trade-off.
///
/// PNG always produces lossless output, so the `lossless` flag here
/// controls the compression effort (more effort = smaller file, slower).
///
/// # Arguments
/// * `image` - The source RGBA image to encode
/// * `lossless` - If true, use best compression (slower). If false, use default.
pub fn encode(image: &RasterImage, lossless: bool) -> Result<Vec<u8>, CompressionError> {
    let mut output = Vec::new();
    let cursor = Cursor::new(&mut output);

    let compression = if lossless {
        image::codecs::png::CompressionType::Best
    } else {
        image::codecs::png::CompressionType::Default
    };

    let filter = image::codecs::png::FilterType::Adaptive;

    let encoder = PngEncoder::new_with_quality(cursor, compression, filter);

    encoder
        .write_image(
            &image.rgba_pixels,
            image.width,
            image.height,
            image::ExtendedColorType::Rgba8,
        )
        .map_err(|e| CompressionError::EncodeError(format!("PNG encode failed: {}", e)))?;

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_tiny_png() {
        // 1×1 red pixel
        let image = RasterImage::new(1, 1, vec![255, 0, 0, 255]).unwrap();
        let result = encode(&image, false);
        assert!(result.is_ok());
        let data = result.unwrap();
        // PNG starts with 89 50 4E 47
        assert!(data.starts_with(&[0x89, 0x50, 0x4E, 0x47]));
    }

    #[test]
    fn test_lossless_produces_valid_png() {
        let image = RasterImage::new(2, 2, vec![0u8; 16]).unwrap();
        let result = encode(&image, true);
        assert!(result.is_ok());
        assert!(result.unwrap().starts_with(&[0x89, 0x50, 0x4E, 0x47]));
    }
}
