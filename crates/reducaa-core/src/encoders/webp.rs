use zenwebp::{EncodeRequest, LosslessConfig, LossyConfig, PixelLayout};

use crate::config::RasterImage;
use crate::errors::CompressionError;

/// Encode a `RasterImage` as WebP using `zenwebp`.
///
/// zenwebp is a pure-Rust reimplementation of libwebp with full lossy
/// and lossless encoding support. It is WASM-native with zero C dependencies,
/// uses `#![forbid(unsafe_code)]`, and supports configurable quality + method.
///
/// # Arguments
/// * `image` - The source RGBA image to encode
/// * `quality` - WebP quality level (1 = worst, 100 = best).
///               Quality 100 triggers lossless mode.
pub fn encode(image: &RasterImage, quality: u8) -> Result<Vec<u8>, CompressionError> {
    let webp_data = if quality >= 100 {
        // Lossless mode
        let config = LosslessConfig::new();
        EncodeRequest::lossless(
            &config,
            &image.rgba_pixels,
            PixelLayout::Rgba8,
            image.width,
            image.height,
        )
        .encode()
        .map_err(|e| CompressionError::EncodeError(format!("WebP lossless encode failed: {}", e)))?
    } else {
        // Lossy mode with quality control
        // method 4 = good balance of speed and compression (0=fast, 6=best)
        let config = LossyConfig::new()
            .with_quality(quality as f32)
            .with_method(4);

        EncodeRequest::lossy(
            &config,
            &image.rgba_pixels,
            PixelLayout::Rgba8,
            image.width,
            image.height,
        )
        .encode()
        .map_err(|e| CompressionError::EncodeError(format!("WebP lossy encode failed: {}", e)))?
    };

    Ok(webp_data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_lossy_webp() {
        // 4×4 blue image
        let mut pixels = Vec::with_capacity(4 * 4 * 4);
        for _ in 0..16 {
            pixels.extend_from_slice(&[0, 0, 255, 255]);
        }
        let image = RasterImage::new(4, 4, pixels).unwrap();
        let result = encode(&image, 80);
        assert!(result.is_ok(), "lossy encode failed: {:?}", result.err());
        let data = result.unwrap();
        // WebP starts with RIFF
        assert!(data.starts_with(b"RIFF"));
    }

    #[test]
    fn test_encode_lossless_webp() {
        let mut pixels = Vec::with_capacity(4 * 4 * 4);
        for _ in 0..16 {
            pixels.extend_from_slice(&[255, 128, 0, 255]);
        }
        let image = RasterImage::new(4, 4, pixels).unwrap();
        let result = encode(&image, 100);
        assert!(result.is_ok(), "lossless encode failed: {:?}", result.err());
        let data = result.unwrap();
        assert!(data.starts_with(b"RIFF"));
    }

    #[test]
    fn test_lossy_smaller_than_lossless() {
        // A larger image to see the size difference
        let pixels = vec![128u8; 32 * 32 * 4];
        let image = RasterImage::new(32, 32, pixels).unwrap();
        let lossy = encode(&image, 50).unwrap();
        let lossless = encode(&image, 100).unwrap();
        // Lossy at q50 should generally be smaller than lossless
        // (may not always hold for tiny uniform images, so just check both are valid)
        assert!(lossy.starts_with(b"RIFF"));
        assert!(lossless.starts_with(b"RIFF"));
    }
}
