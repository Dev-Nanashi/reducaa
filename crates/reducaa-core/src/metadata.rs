use std::io::Cursor;

use exif::{In, Reader as ExifReader, Tag};

use crate::config::RasterImage;
use crate::errors::CompressionError;

/// EXIF orientation values (1–8) as defined in the EXIF specification.
///
/// - 1: Normal (no rotation needed)
/// - 2: Flipped horizontally
/// - 3: Rotated 180°
/// - 4: Flipped vertically
/// - 5: Transposed (flip horizontal + rotate 270° CW)
/// - 6: Rotated 90° CW
/// - 7: Transverse (flip horizontal + rotate 90° CW)
/// - 8: Rotated 270° CW

/// Read the EXIF orientation tag from raw image bytes.
///
/// Returns `1` (Normal) if no EXIF data or no orientation tag is found.
/// Only JPEG files typically carry EXIF orientation; PNG and WebP
/// may also carry it but it's rare.
pub fn read_orientation(data: &[u8]) -> u32 {
    let cursor = Cursor::new(data);
    let exif = match ExifReader::new().read_from_container(&mut std::io::BufReader::new(cursor)) {
        Ok(exif) => exif,
        Err(_) => return 1, // No EXIF data → treat as normal orientation
    };

    match exif.get_field(Tag::Orientation, In::PRIMARY) {
        Some(field) => field.value.get_uint(0).unwrap_or(1),
        None => 1,
    }
}

/// Apply EXIF orientation correction to a `RasterImage`.
///
/// Rotates and/or flips the pixel buffer so the image is visually upright.
/// After this function returns, the image can be treated as orientation=1 (Normal).
pub fn apply_orientation(
    image: &RasterImage,
    orientation: u32,
) -> Result<RasterImage, CompressionError> {
    match orientation {
        1 => {
            // Normal — no transformation needed.
            Ok(image.clone())
        }
        2 => {
            // Flip horizontal
            flip_horizontal(image)
        }
        3 => {
            // Rotate 180°
            rotate_180(image)
        }
        4 => {
            // Flip vertical
            flip_vertical(image)
        }
        5 => {
            // Transpose: flip horizontal, then rotate 270° CW
            let flipped = flip_horizontal(image)?;
            rotate_270(&flipped)
        }
        6 => {
            // Rotate 90° CW
            rotate_90(image)
        }
        7 => {
            // Transverse: flip horizontal, then rotate 90° CW
            let flipped = flip_horizontal(image)?;
            rotate_90(&flipped)
        }
        8 => {
            // Rotate 270° CW
            rotate_270(image)
        }
        _ => {
            // Unknown orientation value — treat as normal.
            Ok(image.clone())
        }
    }
}

/// Flip an image horizontally (mirror around vertical axis).
fn flip_horizontal(image: &RasterImage) -> Result<RasterImage, CompressionError> {
    let (w, h) = (image.width as usize, image.height as usize);
    let mut out = vec![0u8; image.rgba_pixels.len()];

    for y in 0..h {
        for x in 0..w {
            let src = (y * w + x) * 4;
            let dst = (y * w + (w - 1 - x)) * 4;
            out[dst..dst + 4].copy_from_slice(&image.rgba_pixels[src..src + 4]);
        }
    }

    RasterImage::new(image.width, image.height, out)
        .map_err(|e| CompressionError::ExifError(e.to_string()))
}

/// Flip an image vertically (mirror around horizontal axis).
fn flip_vertical(image: &RasterImage) -> Result<RasterImage, CompressionError> {
    let (w, h) = (image.width as usize, image.height as usize);
    let mut out = vec![0u8; image.rgba_pixels.len()];

    for y in 0..h {
        let src_row_start = y * w * 4;
        let dst_row_start = (h - 1 - y) * w * 4;
        out[dst_row_start..dst_row_start + w * 4]
            .copy_from_slice(&image.rgba_pixels[src_row_start..src_row_start + w * 4]);
    }

    RasterImage::new(image.width, image.height, out)
        .map_err(|e| CompressionError::ExifError(e.to_string()))
}

/// Rotate an image 90° clockwise.
/// New dimensions: width becomes height, height becomes width.
fn rotate_90(image: &RasterImage) -> Result<RasterImage, CompressionError> {
    let (w, h) = (image.width as usize, image.height as usize);
    let new_w = h;
    let new_h = w;
    let mut out = vec![0u8; image.rgba_pixels.len()];

    for y in 0..h {
        for x in 0..w {
            let src = (y * w + x) * 4;
            let dst_x = h - 1 - y;
            let dst_y = x;
            let dst = (dst_y * new_w + dst_x) * 4;
            out[dst..dst + 4].copy_from_slice(&image.rgba_pixels[src..src + 4]);
        }
    }

    RasterImage::new(new_w as u32, new_h as u32, out)
        .map_err(|e| CompressionError::ExifError(e.to_string()))
}

/// Rotate an image 180°.
fn rotate_180(image: &RasterImage) -> Result<RasterImage, CompressionError> {
    let total_pixels = image.rgba_pixels.len() / 4;
    let mut out = vec![0u8; image.rgba_pixels.len()];

    for i in 0..total_pixels {
        let src = i * 4;
        let dst = (total_pixels - 1 - i) * 4;
        out[dst..dst + 4].copy_from_slice(&image.rgba_pixels[src..src + 4]);
    }

    RasterImage::new(image.width, image.height, out)
        .map_err(|e| CompressionError::ExifError(e.to_string()))
}

/// Rotate an image 270° clockwise (= 90° counter-clockwise).
fn rotate_270(image: &RasterImage) -> Result<RasterImage, CompressionError> {
    let (w, h) = (image.width as usize, image.height as usize);
    let new_w = h;
    let new_h = w;
    let mut out = vec![0u8; image.rgba_pixels.len()];

    for y in 0..h {
        for x in 0..w {
            let src = (y * w + x) * 4;
            let dst_x = y;
            let dst_y = w - 1 - x;
            let dst = (dst_y * new_w + dst_x) * 4;
            out[dst..dst + 4].copy_from_slice(&image.rgba_pixels[src..src + 4]);
        }
    }

    RasterImage::new(new_w as u32, new_h as u32, out)
        .map_err(|e| CompressionError::ExifError(e.to_string()))
}

/// Extract the APP1 (EXIF) segment from a JPEG file, if present.
pub fn extract_jpeg_exif(data: &[u8]) -> Option<Vec<u8>> {
    if data.len() < 4 || data[0] != 0xFF || data[1] != 0xD8 {
        return None;
    }
    let mut offset = 2;
    while offset + 4 <= data.len() {
        if data[offset] != 0xFF {
            break;
        }
        let marker = data[offset + 1];
        if marker == 0xD9 || marker == 0xDA {
            // EOI (End of Image) or SOS (Start of Scan)
            break;
        }
        let length = u16::from_be_bytes([data[offset + 2], data[offset + 3]]) as usize;
        let segment_end = offset + 2 + length;
        if segment_end > data.len() {
            break;
        }
        // Check for APP1 with "Exif\0\0" header
        if marker == 0xE1 && length >= 8 && offset + 10 <= data.len() && &data[offset + 4..offset + 10] == b"Exif\0\0" {
            return Some(data[offset..segment_end].to_vec());
        }
        offset = segment_end;
    }
    None
}

/// Inject an extracted APP1 EXIF segment into a target JPEG right after SOI (FF D8).
pub fn inject_jpeg_exif(jpeg_data: &[u8], exif_segment: &[u8]) -> Vec<u8> {
    if jpeg_data.len() < 2 || jpeg_data[0] != 0xFF || jpeg_data[1] != 0xD8 {
        return jpeg_data.to_vec();
    }
    let mut out = Vec::with_capacity(jpeg_data.len() + exif_segment.len());
    out.extend_from_slice(&jpeg_data[..2]); // FF D8
    out.extend_from_slice(exif_segment);
    out.extend_from_slice(&jpeg_data[2..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create a 2×2 test image with distinct pixel colors.
    fn make_2x2_image() -> RasterImage {
        let pixels = vec![
            255, 0, 0, 255, // Red
            0, 255, 0, 255, // Green
            0, 0, 255, 255, // Blue
            255, 255, 255, 255, // White
        ];
        RasterImage::new(2, 2, pixels).unwrap()
    }

    fn pixel_at(img: &RasterImage, x: usize, y: usize) -> [u8; 4] {
        let idx = (y * img.width as usize + x) * 4;
        [
            img.rgba_pixels[idx],
            img.rgba_pixels[idx + 1],
            img.rgba_pixels[idx + 2],
            img.rgba_pixels[idx + 3],
        ]
    }

    #[test]
    fn test_orientation_1_is_identity() {
        let img = make_2x2_image();
        let result = apply_orientation(&img, 1).unwrap();
        assert_eq!(result.rgba_pixels, img.rgba_pixels);
    }

    #[test]
    fn test_flip_horizontal() {
        let img = make_2x2_image();
        let result = flip_horizontal(&img).unwrap();
        assert_eq!(pixel_at(&result, 0, 0), [0, 255, 0, 255]);
        assert_eq!(pixel_at(&result, 1, 0), [255, 0, 0, 255]);
    }

    #[test]
    fn test_no_exif_returns_1() {
        let data = vec![0u8; 64];
        assert_eq!(read_orientation(&data), 1);
    }
}

/* _GIT_HISTORY_DUMMY_ */ /* Revision 9 - 3469t */
