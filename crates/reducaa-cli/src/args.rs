use std::path::PathBuf;

use clap::Parser;

/// Reducaa — Fast, offline image compression.
///
/// Compress JPEG, PNG, and WebP images locally without sending
/// data to any server. Supports batch processing, format conversion,
/// resizing, and configurable quality.
#[derive(Parser, Debug)]
#[command(
    name = "reducaa",
    version,
    about = "Fast, offline image compression powered by Rust",
    long_about = "Compress JPEG, PNG, and WebP images entirely offline.\n\
                  Supports batch processing, format conversion, resizing,\n\
                  and configurable quality — all without uploading your files."
)]
pub struct CliArgs {
    /// Input file(s) or directory path(s) to compress.
    ///
    /// Accepts one or more files, or directories (scanned recursively).
    /// Supported formats: JPEG (.jpg, .jpeg), PNG (.png), WebP (.webp)
    #[arg(short, long, required = true, num_args = 1..)]
    pub input: Vec<PathBuf>,

    /// Output directory for compressed files.
    ///
    /// If not specified, compressed files are saved alongside originals
    /// with a `_compressed` suffix.
    #[arg(short, long)]
    pub output_dir: Option<PathBuf>,

    /// Output format for conversion (jpeg, png, webp).
    ///
    /// If not specified, files keep their original format.
    #[arg(short, long, value_parser = parse_format)]
    pub format: Option<reducaa_core::ImageFormat>,

    /// Compression quality (1–100).
    ///
    /// Higher values produce better quality but larger files.
    /// Defaults: JPEG=85, WebP=80. PNG is always lossless.
    #[arg(short, long, value_parser = clap::value_parser!(u8).range(1..=100))]
    pub quality: Option<u8>,

    /// Target width in pixels for resizing.
    ///
    /// Aspect ratio is preserved by default. Use `--no-aspect-ratio`
    /// to allow stretching.
    #[arg(long)]
    pub width: Option<u32>,

    /// Target height in pixels for resizing.
    ///
    /// Aspect ratio is preserved by default.
    #[arg(long)]
    pub height: Option<u32>,

    /// Disable aspect ratio preservation when resizing.
    #[arg(long, default_value_t = false)]
    pub no_aspect_ratio: bool,

    /// Allow upscaling images beyond their original dimensions.
    #[arg(long, default_value_t = false)]
    pub allow_upscale: bool,

    /// Preserve image metadata (minimal EXIF).
    ///
    /// By default, all EXIF metadata is stripped for privacy and size savings.
    #[arg(long, default_value_t = false)]
    pub preserve_metadata: bool,

    /// Number of threads for parallel processing.
    ///
    /// Defaults to the number of CPU cores.
    #[arg(short, long)]
    pub threads: Option<usize>,

    /// Overwrite existing output files without prompting.
    #[arg(long, default_value_t = false)]
    pub overwrite: bool,

    /// Enable lossless compression mode.
    ///
    /// For PNG, this increases compression effort.
    /// For WebP, this uses lossless encoding.
    /// For JPEG, this sets quality to 100.
    #[arg(long, default_value_t = false)]
    pub lossless: bool,
}

/// Parse a format string into an `ImageFormat`.
fn parse_format(s: &str) -> Result<reducaa_core::ImageFormat, String> {
    reducaa_core::ImageFormat::from_extension(s)
        .ok_or_else(|| format!("Unsupported format '{}'. Use: jpeg, png, or webp", s))
}

impl CliArgs {
    /// Build a `CompressionMode` from the CLI arguments.
    pub fn compression_mode(&self) -> reducaa_core::CompressionMode {
        if self.lossless {
            reducaa_core::CompressionMode::Lossless
        } else if let Some(q) = self.quality {
            reducaa_core::CompressionMode::Quality(q)
        } else {
            reducaa_core::CompressionMode::Default
        }
    }

    /// Build `ResizeOptions` from the CLI arguments, if any resize flags are set.
    pub fn resize_options(&self) -> Option<reducaa_core::ResizeOptions> {
        if self.width.is_none() && self.height.is_none() {
            return None;
        }

        Some(reducaa_core::ResizeOptions {
            width: self.width,
            height: self.height,
            keep_aspect_ratio: !self.no_aspect_ratio,
            prevent_upscale: !self.allow_upscale,
        })
    }
}
