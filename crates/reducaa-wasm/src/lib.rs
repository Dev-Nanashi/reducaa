use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use reducaa_core::config::{CompressionJob, CompressionMode, ImageFormat, ResizeOptions};
use reducaa_core::decoder;
use reducaa_core::pipeline;

// ── Panic hook for readable WASM errors in browser console ────────────

/// Initialize the WASM module. Call once on load.
#[wasm_bindgen(js_name = "initReducaa")]
pub fn init() {
    console_error_panic_hook::set_once();
}

// ── Types for JS ↔ Rust serialization ─────────────────────────────────

/// Options passed from JavaScript to configure compression.
#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CompressOptions {
    /// Output quality 1–100. If omitted, uses format defaults.
    pub quality: Option<u8>,
    /// Output format: "jpeg", "png", or "webp". If omitted, keeps input format.
    pub format: Option<String>,
    /// Maximum output width in pixels.
    pub max_width: Option<u32>,
    /// Maximum output height in pixels.
    pub max_height: Option<u32>,
    /// If true, maintains original aspect ratio (defaults to true).
    pub keep_aspect_ratio: Option<bool>,
    /// If true, preserves EXIF metadata (defaults to false / stripped).
    pub preserve_metadata: Option<bool>,
    /// If true, use lossless compression (PNG/WebP only).
    #[serde(default)]
    pub lossless: bool,
}

/// Result returned to JavaScript after compression.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompressResult {
    /// Compressed image bytes.
    #[serde(with = "serde_bytes")]
    pub data: Vec<u8>,
    /// Output format name ("JPEG", "PNG", "WebP").
    pub format: String,
    /// Original file size in bytes.
    pub original_size: usize,
    /// Compressed file size in bytes.
    pub compressed_size: usize,
    /// Output width in pixels.
    pub width: u32,
    /// Output height in pixels.
    pub height: u32,
    /// Percentage reduction (0.0 – 100.0).
    pub reduction_percent: f64,
}

// ── Exported WASM functions ───────────────────────────────────────────

/// Detect the image format from raw bytes.
/// Returns "jpeg", "png", "webp", or throws on unknown format.
#[wasm_bindgen(js_name = "detectFormat")]
pub fn detect_format(data: &[u8]) -> Result<String, JsError> {
    let format = decoder::detect_format(data).map_err(|e| JsError::new(&e.to_string()))?;
    Ok(format.extension().to_string())
}

/// Inspect image headers to read format, orientation, and detect EXIF metadata.
#[wasm_bindgen(js_name = "inspectImage")]
pub fn inspect_image(data: &[u8]) -> Result<JsValue, JsError> {
    let format = decoder::detect_format(data).map_err(|e| JsError::new(&e.to_string()))?;
    let orientation = reducaa_core::metadata::read_orientation(data);
    
    // Check if EXIF is present
    let cursor = std::io::Cursor::new(data);
    let has_exif = exif::Reader::new()
        .read_from_container(&mut std::io::BufReader::new(cursor))
        .is_ok();

    let obj = js_sys::Object::new();
    let _ = js_sys::Reflect::set(&obj, &"format".into(), &format.name().into());
    let _ = js_sys::Reflect::set(&obj, &"orientation".into(), &js_sys::Number::from(orientation).into());
    let _ = js_sys::Reflect::set(&obj, &"hasExif".into(), &js_sys::Boolean::from(has_exif).into());

    Ok(obj.into())
}

/// Compress an image.
///
/// # Arguments
/// * `data` — Raw image file bytes (JPEG, PNG, or WebP)
/// * `options` — JS object with optional fields: `quality`, `format`, `maxWidth`, `maxHeight`, `lossless`
///
/// # Returns
/// A JS object with: `data` (Uint8Array), `format`, `originalSize`, `compressedSize`,
/// `width`, `height`, `reductionPercent`.
#[wasm_bindgen]
pub fn compress(data: &[u8], options: JsValue) -> Result<JsValue, JsError> {
    let opts: CompressOptions = if options.is_undefined() || options.is_null() {
        CompressOptions::default()
    } else {
        serde_wasm_bindgen::from_value(options)
            .map_err(|e| JsError::new(&format!("Invalid options: {}", e)))?
    };

    // Detect input format
    let input_format = decoder::detect_format(data).map_err(|e| JsError::new(&e.to_string()))?;

    // Parse output format
    let output_format = match opts.format.as_deref() {
        Some("jpeg") | Some("jpg") => Some(ImageFormat::Jpeg),
        Some("png") => Some(ImageFormat::Png),
        Some("webp") => Some(ImageFormat::WebP),
        Some(other) => return Err(JsError::new(&format!("Unsupported format: {}", other))),
        None => None,
    };

    // Build compression mode
    let mode = if opts.lossless {
        CompressionMode::Lossless
    } else {
        match opts.quality {
            Some(q) => CompressionMode::Quality(q.clamp(1, 100)),
            None => CompressionMode::Default,
        }
    };

    // Build resize options (only if at least one dimension is specified)
    let keep_aspect_ratio = opts.keep_aspect_ratio.unwrap_or(true);
    let resize = match (opts.max_width, opts.max_height) {
        (None, None) => None,
        (w, h) => Some(ResizeOptions {
            width: w,
            height: h,
            keep_aspect_ratio,
            prevent_upscale: true,
        }),
    };

    // Build the compression job
    let job = CompressionJob {
        image_data: data.to_vec(),
        input_format,
        output_format,
        mode,
        preserve_metadata: opts.preserve_metadata.unwrap_or(false),
        resize,
    };

    // Run the pipeline
    let result = pipeline::process_image(&job).map_err(|e| JsError::new(&e.to_string()))?;

    // Build the JS result
    let reduction_percent = result.reduction_percent();
    let format_name = result.format.name().to_string();
    let original_size = result.original_size;
    let compressed_size = result.compressed_size;
    let width = result.width;
    let height = result.height;

    let js_result = CompressResult {
        data: result.data,
        format: format_name,
        original_size,
        compressed_size,
        width,
        height,
        reduction_percent,
    };

    serde_wasm_bindgen::to_value(&js_result)
        .map_err(|e| JsError::new(&format!("Serialization failed: {}", e)))
}
