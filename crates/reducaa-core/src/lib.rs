//! # Reducaa Core
//!
//! A pure-Rust image compression engine supporting JPEG, PNG, and WebP formats.
//!
//! This library provides a complete pipeline for image compression:
//! format detection, decoding, EXIF orientation correction, resizing, and encoding.
//!
//! Designed to compile cleanly to WebAssembly (`wasm32-unknown-unknown`)
//! with zero C dependencies.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use reducaa_core::pipeline::compress_default;
//!
//! let image_data = std::fs::read("photo.jpg").unwrap();
//! let result = compress_default(image_data, None).unwrap();
//! std::fs::write("compressed.jpg", &result.data).unwrap();
//! println!("Reduced by {:.1}%", result.reduction_percent());
//! ```

pub mod config;
pub mod decoder;
pub mod encoders;
pub mod errors;
pub mod metadata;
pub mod pipeline;
pub mod resize;

// Re-export commonly used types at crate root for convenience.
pub use config::{
    CompressionJob, CompressionMode, CompressionResult, ImageFormat, RasterImage, ResizeOptions,
};
pub use errors::CompressionError;
pub use pipeline::{compress_default, process_image};
