use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;


use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use rayon::prelude::*;
use walkdir::WalkDir;

use reducaa_core::{CompressionJob, CompressionResult, ImageFormat};

use crate::args::CliArgs;

/// Supported file extensions for image discovery.
const SUPPORTED_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp"];

/// Result of processing a single file (success or failure).
#[derive(Debug)]
pub struct FileResult {
    pub input_path: PathBuf,
    pub output_path: PathBuf,
    pub outcome: FileOutcome,
}

/// Outcome of processing a single file.
#[derive(Debug)]
pub enum FileOutcome {
    Success(CompressionResult),
    Skipped(String),
    Failed(String),
}

/// Collect all valid image files from the input paths.
///
/// Handles both individual files and directories (scanned recursively).
pub fn collect_files(inputs: &[PathBuf]) -> Vec<PathBuf> {
    let mut files = Vec::new();

    for path in inputs {
        if path.is_file() {
            if is_supported_image(path) {
                files.push(path.clone());
            }
        } else if path.is_dir() {
            for entry in WalkDir::new(path)
                .follow_links(true)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if entry.file_type().is_file() && is_supported_image(entry.path()) {
                    files.push(entry.path().to_path_buf());
                }
            }
        }
    }

    files
}

/// Check if a file has a supported image extension.
fn is_supported_image(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| SUPPORTED_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

/// Resolve the output path for a given input file.
fn resolve_output_path(
    input_path: &Path,
    output_dir: Option<&Path>,
    output_format: Option<ImageFormat>,
    overwrite: bool,
) -> PathBuf {
    let stem = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");

    let extension = if let Some(fmt) = output_format {
        fmt.extension().to_string()
    } else {
        input_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("jpg")
            .to_string()
    };

    let base_dir = output_dir
        .map(|d| d.to_path_buf())
        .unwrap_or_else(|| {
            input_path
                .parent()
                .unwrap_or(Path::new("."))
                .to_path_buf()
        });

    if overwrite {
        base_dir.join(format!("{}.{}", stem, extension))
    } else {
        // Add _compressed suffix to avoid overwriting originals
        base_dir.join(format!("{}_compressed.{}", stem, extension))
    }
}

/// Process all collected files in parallel using Rayon.
///
/// Returns a vector of `FileResult` (one per input file).
pub fn process_batch(args: &CliArgs, files: Vec<PathBuf>) -> Vec<FileResult> {
    // Configure rayon thread pool
    if let Some(threads) = args.threads {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build_global()
            .ok(); // Ignore if already initialized
    }

    // Create output directory if specified
    if let Some(ref output_dir) = args.output_dir {
        let _ = fs::create_dir_all(output_dir);
    }

    let mode = args.compression_mode();
    let resize_opts = args.resize_options();

    // Progress tracking
    let multi_progress = MultiProgress::new();
    let overall_bar = multi_progress.add(ProgressBar::new(files.len() as u64));
    overall_bar.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})",
        )
        .unwrap()
        .progress_chars("█▉▊▋▌▍▎▏  "),
    );
    overall_bar.set_message("Compressing...");

    let success_count = AtomicUsize::new(0);
    let fail_count = AtomicUsize::new(0);
    let total_original = AtomicUsize::new(0);
    let total_compressed = AtomicUsize::new(0);
    let results = Mutex::new(Vec::with_capacity(files.len()));

    // Process in parallel
    files.par_iter().for_each(|input_path| {
        let output_path = resolve_output_path(
            input_path,
            args.output_dir.as_deref(),
            args.format,
            args.overwrite,
        );

        let result = process_single_file(input_path, &output_path, mode, args.format, args.preserve_metadata, resize_opts.clone());

        match &result.outcome {
            FileOutcome::Success(cr) => {
                success_count.fetch_add(1, Ordering::Relaxed);
                total_original.fetch_add(cr.original_size, Ordering::Relaxed);
                total_compressed.fetch_add(cr.compressed_size, Ordering::Relaxed);
            }
            FileOutcome::Skipped(_) => {}
            FileOutcome::Failed(_) => {
                fail_count.fetch_add(1, Ordering::Relaxed);
            }
        }

        results.lock().unwrap().push(result);
        overall_bar.inc(1);
    });

    overall_bar.finish_with_message("Done!");

    let results = results.into_inner().unwrap();
    results
}

/// Process a single image file: read → compress → write.
fn process_single_file(
    input_path: &Path,
    output_path: &Path,
    mode: reducaa_core::CompressionMode,
    output_format: Option<ImageFormat>,
    preserve_metadata: bool,
    resize: Option<reducaa_core::ResizeOptions>,
) -> FileResult {
    let make_result = |outcome| FileResult {
        input_path: input_path.to_path_buf(),
        output_path: output_path.to_path_buf(),
        outcome,
    };

    // Read input file
    let image_data = match fs::read(input_path) {
        Ok(data) => data,
        Err(e) => {
            return make_result(FileOutcome::Failed(format!("Read error: {}", e)));
        }
    };

    // Detect input format
    let input_format = match reducaa_core::decoder::detect_format(&image_data) {
        Ok(fmt) => fmt,
        Err(e) => {
            return make_result(FileOutcome::Failed(format!("{}", e)));
        }
    };

    // Build compression job
    let job = CompressionJob {
        image_data,
        input_format,
        output_format,
        mode,
        preserve_metadata,
        resize,
    };

    // Run pipeline
    let result = match reducaa_core::process_image(&job) {
        Ok(r) => r,
        Err(e) => {
            return make_result(FileOutcome::Failed(format!("{}", e)));
        }
    };

    // Create output directory if needed
    if let Some(parent) = output_path.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            return make_result(FileOutcome::Failed(format!(
                "Could not create output dir: {}",
                e
            )));
        }
    }

    // Write compressed output
    if let Err(e) = fs::write(output_path, &result.data) {
        return make_result(FileOutcome::Failed(format!("Write error: {}", e)));
    }

    make_result(FileOutcome::Success(result))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_supported_image() {
        assert!(is_supported_image(Path::new("photo.jpg")));
        assert!(is_supported_image(Path::new("photo.JPEG")));
        assert!(is_supported_image(Path::new("image.png")));
        assert!(is_supported_image(Path::new("pic.webp")));
        assert!(!is_supported_image(Path::new("document.pdf")));
        assert!(!is_supported_image(Path::new("file.txt")));
        assert!(!is_supported_image(Path::new("noext")));
    }

    #[test]
    fn test_resolve_output_path_no_overwrite() {
        let path = resolve_output_path(
            Path::new("/images/photo.jpg"),
            None,
            None,
            false,
        );
        assert!(path.to_str().unwrap().contains("_compressed"));
    }

    #[test]
    fn test_resolve_output_path_with_overwrite() {
        let path = resolve_output_path(
            Path::new("/images/photo.jpg"),
            None,
            None,
            true,
        );
        assert!(!path.to_str().unwrap().contains("_compressed"));
    }

    #[test]
    fn test_resolve_output_path_format_conversion() {
        let path = resolve_output_path(
            Path::new("/images/photo.jpg"),
            None,
            Some(ImageFormat::WebP),
            false,
        );
        assert!(path.to_str().unwrap().ends_with(".webp"));
    }
}

/* _GIT_HISTORY_DUMMY_ */ /* Revision 9 - d09nyq */
