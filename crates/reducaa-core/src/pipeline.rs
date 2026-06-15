use crate::config::{CompressionJob, CompressionResult, ImageFormat};
use crate::decoder;
use crate::encoders;
use crate::errors::CompressionError;
use crate::metadata;
use crate::resize;

/// Execute the full image compression pipeline.
///
/// Pipeline stages:
/// 1. **Detect format** — Inspect magic bytes to determine input format
/// 2. **Decode** — Convert to RGBA pixels
/// 3. **Orientation** — Read EXIF orientation, rotate/flip pixels to upright
/// 4. **Resize** — Optionally resize to target dimensions
/// 5. **Encode** — Compress to output format with specified quality
///
/// Returns a `CompressionResult` containing the compressed bytes and statistics.
pub fn process_image(job: &CompressionJob) -> Result<CompressionResult, CompressionError> {
    let original_size = job.image_data.len();

    // Stage 1 & 2: Detect format and decode to RGBA
    let (mut raster, detected_format) = decoder::decode(&job.image_data)?;

    // Verify detected format matches the declared input format
    // (use detected format as ground truth, since magic bytes are more reliable)
    let _input_format = detected_format;

    // Stage 3: Apply EXIF orientation correction
    if !job.preserve_metadata {
        let orientation = metadata::read_orientation(&job.image_data);
        if orientation != 1 {
            raster = metadata::apply_orientation(&raster, orientation)?;
        }
    }

    // Stage 4: Optional resize
    if let Some(ref resize_opts) = job.resize {
        raster = resize::resize(&raster, resize_opts)?;
    }

    // Stage 5: Encode to output format
    let output_format = job.output_format.unwrap_or(detected_format);
    let mut compressed_data = encoders::encode(&raster, output_format, job.mode)?;

    // Preserve original EXIF if requested and format is JPEG
    if job.preserve_metadata && detected_format == ImageFormat::Jpeg && output_format == ImageFormat::Jpeg {
        if let Some(exif_seg) = metadata::extract_jpeg_exif(&job.image_data) {
            compressed_data = metadata::inject_jpeg_exif(&compressed_data, &exif_seg);
        }
    }

    Ok(CompressionResult {
        data: compressed_data,
        format: output_format,
        original_size,
        compressed_size: 0, // Will be set below
        width: raster.width,
        height: raster.height,
    })
    .map(|mut result| {
        result.compressed_size = result.data.len();
        result
    })
}

/// Convenience function: compress with default settings (auto quality, no resize).
///
/// Suitable for simple "make this smaller" use cases without advanced options.
pub fn compress_default(
    image_data: Vec<u8>,
    output_format: Option<ImageFormat>,
) -> Result<CompressionResult, CompressionError> {
    let input_format = decoder::detect_format(&image_data)?;

    let job = CompressionJob {
        image_data,
        input_format,
        output_format,
        mode: crate::config::CompressionMode::Default,
        preserve_metadata: false,
        resize: None,
    };

    process_image(&job)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::CompressionMode;

    /// Helper: create a minimal valid JPEG in memory for testing.
    fn create_test_jpeg() -> Vec<u8> {
        use zenjpeg::encoder::{ChromaSubsampling, EncoderConfig, PixelLayout, Unstoppable};

        // 4×4 solid red RGBA image
        let rgba: Vec<u8> = vec![255, 0, 0, 255].repeat(4 * 4);

        let config = EncoderConfig::ycbcr(85, ChromaSubsampling::Quarter);
        let mut enc = config
            .encode_from_bytes(4, 4, PixelLayout::Rgbx8Srgb)
            .unwrap();
        enc.push_packed(&rgba, Unstoppable).unwrap();
        enc.finish().unwrap()
    }

    /// Helper: create a minimal valid PNG in memory for testing.
    fn create_test_png() -> Vec<u8> {
        use image::{ImageEncoder, RgbaImage, codecs::png::PngEncoder};
        use std::io::Cursor;

        let img = RgbaImage::from_pixel(4, 4, image::Rgba([0, 128, 255, 255]));
        let mut output = Vec::new();
        let cursor = Cursor::new(&mut output);
        let encoder = PngEncoder::new(cursor);
        encoder
            .write_image(img.as_raw(), 4, 4, image::ExtendedColorType::Rgba8)
            .unwrap();
        output
    }

    #[test]
    fn test_pipeline_jpeg_to_jpeg() {
        let jpeg_data = create_test_jpeg();
        let job = CompressionJob {
            image_data: jpeg_data,
            input_format: ImageFormat::Jpeg,
            output_format: Some(ImageFormat::Jpeg),
            mode: CompressionMode::Quality(75),
            preserve_metadata: false,
            resize: None,
        };
        let result = process_image(&job).unwrap();
        assert_eq!(result.format, ImageFormat::Jpeg);
        assert!(result.data.starts_with(&[0xFF, 0xD8, 0xFF]));
        assert!(result.compressed_size > 0);
        assert_eq!(result.width, 4);
        assert_eq!(result.height, 4);
    }

    #[test]
    fn test_pipeline_png_to_webp() {
        let png_data = create_test_png();
        let job = CompressionJob {
            image_data: png_data,
            input_format: ImageFormat::Png,
            output_format: Some(ImageFormat::WebP),
            mode: CompressionMode::Default,
            preserve_metadata: false,
            resize: None,
        };
        let result = process_image(&job).unwrap();
        assert_eq!(result.format, ImageFormat::WebP);
        assert!(result.data.starts_with(b"RIFF"));
    }

    #[test]
    fn test_compress_default() {
        let jpeg_data = create_test_jpeg();
        let result = compress_default(jpeg_data, None).unwrap();
        assert_eq!(result.format, ImageFormat::Jpeg);
        assert!(result.compressed_size > 0);
    }

    #[test]
    fn test_pipeline_with_resize() {
        let png_data = create_test_png();
        let job = CompressionJob {
            image_data: png_data,
            input_format: ImageFormat::Png,
            output_format: Some(ImageFormat::Png),
            mode: CompressionMode::Default,
            preserve_metadata: false,
            resize: Some(crate::config::ResizeOptions {
                width: Some(2),
                height: Some(2),
                keep_aspect_ratio: true,
                prevent_upscale: false,
            }),
        };
        let result = process_image(&job).unwrap();
        assert_eq!(result.width, 2);
        assert_eq!(result.height, 2);
    }

    #[test]
    fn test_corrupted_input() {
        let garbage = vec![0u8; 100];
        let result = compress_default(garbage, None);
        assert!(result.is_err());
    }
}

/* _GIT_HISTORY_DUMMY_ */ /* Revision 2 - uxvhvr */
