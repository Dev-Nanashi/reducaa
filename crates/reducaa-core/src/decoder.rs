use std::io::Cursor;

use image::DynamicImage;

use crate::config::{ImageFormat, RasterImage, MAX_IMAGE_DIMENSION};
use crate::errors::CompressionError;

/// Magic byte signatures for supported image formats.
const JPEG_MAGIC: &[u8] = &[0xFF, 0xD8, 0xFF];
const PNG_MAGIC: &[u8] = &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
const WEBP_MAGIC_RIFF: &[u8] = b"RIFF";
const WEBP_MAGIC_WEBP: &[u8] = b"WEBP";

/// Detect the image format by inspecting file header magic bytes.
///
/// This is more reliable than relying on file extensions, since users
/// may rename files or extensions may be missing.
pub fn detect_format(data: &[u8]) -> Result<ImageFormat, CompressionError> {
    if data.is_empty() {
        return Err(CompressionError::EmptyBuffer);
    }

    if data.len() < 12 {
        return Err(CompressionError::InvalidMagicBytes);
    }

    // JPEG: starts with FF D8 FF
    if data.starts_with(JPEG_MAGIC) {
        return Ok(ImageFormat::Jpeg);
    }

    // PNG: starts with 89 50 4E 47 0D 0A 1A 0A
    if data.starts_with(PNG_MAGIC) {
        return Ok(ImageFormat::Png);
    }

    // WebP: starts with RIFF....WEBP
    if data.starts_with(WEBP_MAGIC_RIFF) && &data[8..12] == WEBP_MAGIC_WEBP {
        return Ok(ImageFormat::WebP);
    }

    Err(CompressionError::InvalidMagicBytes)
}

/// Decode raw image bytes into an RGBA `RasterImage`.
///
/// The image is decoded using the `image` crate and converted to RGBA8.
/// Dimensions are validated against `MAX_IMAGE_DIMENSION`.
pub fn decode(data: &[u8]) -> Result<(RasterImage, ImageFormat), CompressionError> {
    let format = detect_format(data)?;

    let cursor = Cursor::new(data);
    let dynamic_image = image::ImageReader::new(cursor)
        .with_guessed_format()
        .map_err(|e| CompressionError::DecodeError(e.to_string()))?
        .decode()
        .map_err(|e| CompressionError::DecodeError(e.to_string()))?;

    let (width, height) = (dynamic_image.width(), dynamic_image.height());

    // Validate dimensions
    if width > MAX_IMAGE_DIMENSION || height > MAX_IMAGE_DIMENSION {
        return Err(CompressionError::ImageTooLarge {
            width,
            height,
            max: MAX_IMAGE_DIMENSION,
        });
    }

    let rgba = dynamic_image_to_rgba(dynamic_image);

    let raster = RasterImage::new(width, height, rgba)
        .map_err(|e| CompressionError::DecodeError(e.to_string()))?;

    Ok((raster, format))
}

/// Convert a `DynamicImage` into a raw RGBA8 byte vector.
fn dynamic_image_to_rgba(img: DynamicImage) -> Vec<u8> {
    img.into_rgba8().into_raw()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_jpeg() {
        // Minimal JPEG header
        let mut data = vec![0xFF, 0xD8, 0xFF, 0xE0];
        data.extend_from_slice(&[0u8; 8]); // pad to 12 bytes
        assert_eq!(detect_format(&data).unwrap(), ImageFormat::Jpeg);
    }

    #[test]
    fn test_detect_png() {
        let mut data = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        data.extend_from_slice(&[0u8; 4]); // pad to 12 bytes
        assert_eq!(detect_format(&data).unwrap(), ImageFormat::Png);
    }

    #[test]
    fn test_detect_webp() {
        let mut data = b"RIFF".to_vec();
        data.extend_from_slice(&[0u8; 4]); // file size placeholder
        data.extend_from_slice(b"WEBP");
        assert_eq!(detect_format(&data).unwrap(), ImageFormat::WebP);
    }

    #[test]
    fn test_detect_empty() {
        assert!(matches!(
            detect_format(&[]),
            Err(CompressionError::EmptyBuffer)
        ));
    }

    #[test]
    fn test_detect_too_short() {
        assert!(matches!(
            detect_format(&[0xFF, 0xD8]),
            Err(CompressionError::InvalidMagicBytes)
        ));
    }

    #[test]
    fn test_detect_unknown() {
        let data = [0u8; 12];
        assert!(matches!(
            detect_format(&data),
            Err(CompressionError::InvalidMagicBytes)
        ));
    }
}
