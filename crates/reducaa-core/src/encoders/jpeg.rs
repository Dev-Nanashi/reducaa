use zenjpeg::encoder::{ChromaSubsampling, EncoderConfig, PixelLayout, Unstoppable};

use crate::config::RasterImage;
use crate::errors::CompressionError;

/// Encode a `RasterImage` as JPEG using `zenjpeg`.
///
/// zenjpeg is a pure-Rust JPEG encoder with perceptual optimizations
/// including adaptive quantization, trellis quantization, and progressive
/// encoding. It produces smaller files at the same visual quality compared
/// to MozJPEG, while being fully WASM-compatible with zero C dependencies.
///
/// # Arguments
/// * `image` - The source RGBA image to encode
/// * `quality` - JPEG quality level (1 = worst, 100 = best)
pub fn encode(image: &RasterImage, quality: u8) -> Result<Vec<u8>, CompressionError> {
    // Configure: YCbCr color mode with 4:2:0 chroma subsampling (standard web JPEG)
    // Progressive encoding gives ~3% smaller files with no quality loss
    let config = EncoderConfig::ycbcr(quality, ChromaSubsampling::Quarter)
        .progressive(true);

    // Encode from raw RGBA bytes using Rgbx8Srgb layout:
    // 4 bytes/pixel where the 4th byte (alpha) is ignored — perfect for RGBA→JPEG
    let mut encoder = config
        .encode_from_bytes(
            image.width,
            image.height,
            PixelLayout::Rgbx8Srgb,
        )
        .map_err(|e| CompressionError::EncodeError(format!("JPEG encoder init failed: {}", e)))?;

    // Push all pixel rows at once (Unstoppable = no cancellation token)
    encoder
        .push_packed(&image.rgba_pixels, Unstoppable)
        .map_err(|e| CompressionError::EncodeError(format!("JPEG encode failed: {}", e)))?;

    // Finalize and get JPEG bytes
    let jpeg_data = encoder
        .finish()
        .map_err(|e| CompressionError::EncodeError(format!("JPEG finalize failed: {}", e)))?;

    Ok(jpeg_data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_tiny_image() {
        // 2×2 solid red image
        let image = RasterImage::new(
            2,
            2,
            vec![
                255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255,
            ],
        )
        .unwrap();
        let result = encode(&image, 85);
        assert!(result.is_ok(), "encode failed: {:?}", result.err());
        let data = result.unwrap();
        // JPEG starts with FF D8 FF
        assert!(data.starts_with(&[0xFF, 0xD8, 0xFF]));
        assert!(data.len() > 100); // Sanity check: not empty
    }

    #[test]
    fn test_encode_quality_range() {
        let image = RasterImage::new(4, 4, vec![128u8; 4 * 4 * 4]).unwrap();
        // Low quality should produce smaller output than high quality
        let low = encode(&image, 10).unwrap();
        let high = encode(&image, 95).unwrap();
        assert!(low.len() < high.len(), "low quality should be smaller");
    }
}
