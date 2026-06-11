mod args;
mod batch;
mod ui;

use anyhow::Result;
use clap::Parser;

use args::CliArgs;

fn main() -> Result<()> {
    let args = CliArgs::parse();

    ui::print_banner();

    // Collect all input files
    let files = batch::collect_files(&args.input);

    if files.is_empty() {
        eprintln!(
            "No supported image files found in the provided paths.\n\
             Supported formats: JPEG (.jpg, .jpeg), PNG (.png), WebP (.webp)"
        );
        std::process::exit(1);
    }

    println!(
        "  Found {} image{} to process.\n",
        files.len(),
        if files.len() == 1 { "" } else { "s" }
    );

    // Process all files
    let results = batch::process_batch(&args, files);

    // Print results summary
    ui::print_results(&results);

    // Exit with error code if any files failed
    let has_failures = results
        .iter()
        .any(|r| matches!(r.outcome, batch::FileOutcome::Failed(_)));

    if has_failures {
        std::process::exit(1);
    }

    Ok(())
}
