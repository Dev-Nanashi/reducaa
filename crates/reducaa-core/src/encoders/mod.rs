pub mod jpeg;
pub mod png;
pub mod webp;

use crate::config::{CompressionMode, ImageFormat, RasterImage, DEFAULT_JPEG_QUALITY, DEFAULT_WEBP_QUALITY};
use crate::errors::CompressionError;

/// Encode a `RasterImage` to the specified output format and compression mode.
///
/// Dispatches to the appropriate format-specific encoder.
pub fn encode(
    image: &RasterImage,
    format: ImageFormat,
    mode: CompressionMode,
) -> Result<Vec<u8>, CompressionError> {
    match format {
        ImageFormat::Jpeg => {
            let quality = match mode {
                CompressionMode::Default => DEFAULT_JPEG_QUALITY,
                CompressionMode::Quality(q) => q.clamp(1, 100),
                CompressionMode::Lossless => 100,
            };
            jpeg::encode(image, quality)
        }
        ImageFormat::Png => {
            let lossless = matches!(mode, CompressionMode::Lossless);
            png::encode(image, lossless)
        }
        ImageFormat::WebP => {
            let quality = match mode {
                CompressionMode::Default => DEFAULT_WEBP_QUALITY,
                CompressionMode::Quality(q) => q.clamp(1, 100),
                CompressionMode::Lossless => 100,
            };
            webp::encode(image, quality)
        }
    }
}
