/// Supported image formats for input and output processing.
///
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImageFormat {
    Jpeg,
    Png,
    WebP,
}

impl ImageFormat {
    /// Returns the canonical file extension for this format.
    pub fn extension(&self) -> &'static str {
        match self {
            ImageFormat::Jpeg => "jpg",
            ImageFormat::Png => "png",
            ImageFormat::WebP => "webp",
        }
    }

    /// Returns the human-readable format name.
    pub fn name(&self) -> &'static str {
        match self {
            ImageFormat::Jpeg => "JPEG",
            ImageFormat::Png => "PNG",
            ImageFormat::WebP => "WebP",
        }
    }

    /// Attempt to determine the format from a file extension string (case-insensitive).
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_ascii_lowercase().as_str() {
            "jpg" | "jpeg" => Some(ImageFormat::Jpeg),
            "png" => Some(ImageFormat::Png),
            "webp" => Some(ImageFormat::WebP),
            _ => None,
        }
    }
}

impl std::fmt::Display for ImageFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Decoded image stored as raw RGBA pixels.
///
/// After decoding and orientation correction, all pipeline stages
/// operate on this upright, normalized representation.
#[derive(Debug, Clone)]
pub struct RasterImage {
    pub width: u32,
    pub height: u32,
    /// Raw pixel data in RGBA8 format (4 bytes per pixel).
    pub rgba_pixels: Vec<u8>,
}

impl RasterImage {
    /// Create a new RasterImage, validating buffer length.
    pub fn new(width: u32, height: u32, rgba_pixels: Vec<u8>) -> Result<Self, &'static str> {
        let expected = (width as usize) * (height as usize) * BYTES_PER_PIXEL;
        if rgba_pixels.len() != expected {
            return Err("Pixel buffer length does not match width × height × 4");
        }
        Ok(Self {
            width,
            height,
            rgba_pixels,
        })
    }

    /// Total number of pixels in this image.
    pub fn pixel_count(&self) -> usize {
        (self.width as usize) * (self.height as usize)
    }
}

/// Compression modes supported by the image pipeline.
#[derive(Debug, Clone, Copy)]
pub enum CompressionMode {
    /// Automatic defaults: JPEG/PNG quality 85, WebP quality 80.
    Default,
    /// User-specified quality level (1–100).
    Quality(u8),
    /// Lossless compression (applicable to PNG and WebP).
    Lossless,
}

/// Resize options applied to an image during processing.
#[derive(Debug, Clone, Copy)]
pub struct ResizeOptions {
    /// Target width in pixels. If `None`, calculated from height + aspect ratio.
    pub width: Option<u32>,
    /// Target height in pixels. If `None`, calculated from width + aspect ratio.
    pub height: Option<u32>,
    /// When true, maintains the original aspect ratio. Defaults to true.
    pub keep_aspect_ratio: bool,
    /// When true, prevents enlarging images beyond original dimensions.
    pub prevent_upscale: bool,
}

impl Default for ResizeOptions {
    fn default() -> Self {
        Self {
            width: None,
            height: None,
            keep_aspect_ratio: true,
            prevent_upscale: true,
        }
    }
}

/// A compression job describing an image transformation request.
#[derive(Debug, Clone)]
pub struct CompressionJob {
    /// Raw bytes of the input image file.
    pub image_data: Vec<u8>,
    /// Detected or user-specified input format.
    pub input_format: ImageFormat,
    /// Desired output format. If `None`, output matches input format.
    pub output_format: Option<ImageFormat>,
    /// Compression mode (default auto, quality slider, or lossless).
    pub mode: CompressionMode,
    /// When true, preserves minimal EXIF metadata (orientation = Normal).
    pub preserve_metadata: bool,
    /// Optional resize parameters.
    pub resize: Option<ResizeOptions>,
}

/// Result of a successful compression operation.
#[derive(Debug, Clone)]
pub struct CompressionResult {
    /// Compressed image bytes ready for writing to disk.
    pub data: Vec<u8>,
    /// The output format used.
    pub format: ImageFormat,
    /// Size of the original input in bytes.
    pub original_size: usize,
    /// Size of the compressed output in bytes.
    pub compressed_size: usize,
    /// Output image width in pixels.
    pub width: u32,
    /// Output image height in pixels.
    pub height: u32,
}

impl CompressionResult {
    /// Calculates the compression ratio as a percentage reduction.
    /// Returns a value between 0.0 and 100.0.
    pub fn reduction_percent(&self) -> f64 {
        if self.original_size == 0 {
            return 0.0;
        }
        let saved = self.original_size.saturating_sub(self.compressed_size);
        (saved as f64 / self.original_size as f64) * 100.0
    }

    /// Returns the number of bytes saved.
    pub fn bytes_saved(&self) -> usize {
        self.original_size.saturating_sub(self.compressed_size)
    }
}

// ── Constants ──────────────────────────────────────────────────────────

/// Default JPEG compression quality (1–100).
pub const DEFAULT_JPEG_QUALITY: u8 = 85;

/// Default PNG compression quality (oxipng optimization level, 1–6).
pub const DEFAULT_PNG_OPT_LEVEL: u8 = 3;

/// Default WebP compression quality (1–100).
pub const DEFAULT_WEBP_QUALITY: u8 = 80;

/// Maximum supported image dimension (width or height) in pixels.
pub const MAX_IMAGE_DIMENSION: u32 = 8_192;

/// Number of bytes used per pixel in the RGBA buffer.
pub const BYTES_PER_PIXEL: usize = 4;

/// Number of bytes in one kilobyte.
pub const KB: usize = 1024;

/// Number of bytes in one megabyte.
pub const MB: usize = 1024 * KB;
