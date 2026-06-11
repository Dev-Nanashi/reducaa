use thiserror::Error;

/// Errors that can occur during image compression operations.
#[derive(Debug, Error)]
pub enum CompressionError {
    /// The input buffer is empty (0 bytes).
    #[error("Input buffer is empty")]
    EmptyBuffer,

    /// The file's magic bytes do not match any supported format.
    #[error("Unrecognized image format: could not detect JPEG, PNG, or WebP from file header")]
    InvalidMagicBytes,

    /// The detected or requested format is not supported.
    #[error("Unsupported image format: {0}")]
    UnsupportedFormat(String),

    /// The image dimensions exceed the maximum allowed size.
    #[error("Image dimensions {width}×{height} exceed maximum {max}×{max}")]
    ImageTooLarge {
        width: u32,
        height: u32,
        max: u32,
    },

    /// An error occurred during image decoding.
    #[error("Failed to decode image: {0}")]
    DecodeError(String),

    /// An error occurred reading or processing EXIF metadata.
    #[error("EXIF metadata error: {0}")]
    ExifError(String),

    /// An error occurred during image resizing.
    #[error("Failed to resize image: {0}")]
    ResizeError(String),

    /// An error occurred during image encoding.
    #[error("Failed to encode image: {0}")]
    EncodeError(String),

    /// Resize options are invalid (e.g., both width and height are None).
    #[error("Invalid resize options: {0}")]
    InvalidResizeOptions(String),
}
