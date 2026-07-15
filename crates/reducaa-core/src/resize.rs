use fast_image_resize::images::Image;
use fast_image_resize::{PixelType, ResizeAlg, ResizeOptions as FirResizeOptions, Resizer};

use crate::config::{RasterImage, ResizeOptions};
use crate::errors::CompressionError;

/// Resize a `RasterImage` according to the given `ResizeOptions`.
///
/// Supports:
/// - Width-only resize (height calculated from aspect ratio)
/// - Height-only resize (width calculated from aspect ratio)
/// - Width + height resize (aspect ratio may be ignored if `keep_aspect_ratio` is false)
/// - Upscale prevention (if `prevent_upscale` is true, dimensions are clamped)
pub fn resize(
    image: &RasterImage,
    options: &ResizeOptions,
) -> Result<RasterImage, CompressionError> {
    let (target_w, target_h) =
        calculate_dimensions(image.width, image.height, options)?;

    // If dimensions haven't changed, skip resize.
    if target_w == image.width && target_h == image.height {
        return Ok(image.clone());
    }

    if target_w == 0 || target_h == 0 {
        return Err(CompressionError::ResizeError("Target dimensions must be non-zero".to_string()));
    }

    if image.width == 0 || image.height == 0 {
        return Err(CompressionError::ResizeError("Source dimensions must be non-zero".to_string()));
    }

    // Create source image view for fast_image_resize (takes u32 directly)
    let src_image = Image::from_vec_u8(image.width, image.height, image.rgba_pixels.clone(), PixelType::U8x4)
        .map_err(|e| CompressionError::ResizeError(e.to_string()))?;

    // Create destination image buffer
    let mut dst_image = Image::new(target_w, target_h, PixelType::U8x4);

    // Perform the resize using Lanczos3 filter (high quality)
    let mut resizer = Resizer::new();
    let resize_options = FirResizeOptions::new().resize_alg(ResizeAlg::Convolution(
        fast_image_resize::FilterType::Lanczos3,
    ));

    resizer
        .resize(&src_image, &mut dst_image, Some(&resize_options))
        .map_err(|e| CompressionError::ResizeError(e.to_string()))?;

    RasterImage::new(target_w, target_h, dst_image.into_vec())
        .map_err(|e| CompressionError::ResizeError(e.to_string()))
}

/// Calculate target dimensions based on resize options, source dimensions,
/// aspect ratio preservation, and upscale prevention.
fn calculate_dimensions(
    src_w: u32,
    src_h: u32,
    options: &ResizeOptions,
) -> Result<(u32, u32), CompressionError> {
    let (mut target_w, mut target_h) = match (options.width, options.height) {
        (Some(w), Some(h)) => {
            if options.keep_aspect_ratio {
                // Fit within the given box while preserving aspect ratio.
                fit_within(src_w, src_h, w, h)
            } else {
                (w, h)
            }
        }
        (Some(w), None) => {
            // Calculate height from width, preserving aspect ratio.
            let ratio = src_h as f64 / src_w as f64;
            let h = (w as f64 * ratio).round() as u32;
            (w, h.max(1))
        }
        (None, Some(h)) => {
            // Calculate width from height, preserving aspect ratio.
            let ratio = src_w as f64 / src_h as f64;
            let w = (h as f64 * ratio).round() as u32;
            (w.max(1), h)
        }
        (None, None) => {
            return Err(CompressionError::InvalidResizeOptions(
                "At least one of width or height must be specified".to_string(),
            ));
        }
    };

    // Prevent upscaling if requested
    if options.prevent_upscale {
        target_w = target_w.min(src_w);
        target_h = target_h.min(src_h);
    }

    // Ensure dimensions are at least 1×1
    target_w = target_w.max(1);
    target_h = target_h.max(1);

    Ok((target_w, target_h))
}

/// Fit source dimensions within a target bounding box, preserving aspect ratio.
/// Returns the largest dimensions that fit within `max_w × max_h`.
fn fit_within(src_w: u32, src_h: u32, max_w: u32, max_h: u32) -> (u32, u32) {
    let ratio_w = max_w as f64 / src_w as f64;
    let ratio_h = max_h as f64 / src_h as f64;
    let scale = ratio_w.min(ratio_h);

    let w = (src_w as f64 * scale).round() as u32;
    let h = (src_h as f64 * scale).round() as u32;

    (w.max(1), h.max(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_width_only() {
        let options = ResizeOptions {
            width: Some(500),
            height: None,
            keep_aspect_ratio: true,
            prevent_upscale: false,
        };
        let (w, h) = calculate_dimensions(1000, 800, &options).unwrap();
        assert_eq!(w, 500);
        assert_eq!(h, 400);
    }

    #[test]
    fn test_calculate_height_only() {
        let options = ResizeOptions {
            width: None,
            height: Some(400),
            keep_aspect_ratio: true,
            prevent_upscale: false,
        };
        let (w, h) = calculate_dimensions(1000, 800, &options).unwrap();
        assert_eq!(w, 500);
        assert_eq!(h, 400);
    }

    #[test]
    fn test_prevent_upscale() {
        let options = ResizeOptions {
            width: Some(2000),
            height: None,
            keep_aspect_ratio: true,
            prevent_upscale: true,
        };
        let (w, h) = calculate_dimensions(1000, 800, &options).unwrap();
        assert_eq!(w, 1000); // Clamped to source width
        assert_eq!(h, 800); // Clamped to source height
    }

    #[test]
    fn test_fit_within_landscape() {
        let (w, h) = fit_within(2000, 1000, 800, 600);
        assert_eq!(w, 800);
        assert_eq!(h, 400);
    }

    #[test]
    fn test_fit_within_portrait() {
        let (w, h) = fit_within(1000, 2000, 800, 600);
        assert_eq!(w, 300);
        assert_eq!(h, 600);
    }

    #[test]
    fn test_no_dimensions_is_error() {
        let options = ResizeOptions {
            width: None,
            height: None,
            keep_aspect_ratio: true,
            prevent_upscale: false,
        };
        assert!(calculate_dimensions(1000, 800, &options).is_err());
    }
}

/* _GIT_HISTORY_DUMMY_ */ /* Revision 17 - 9rj0i8 */
