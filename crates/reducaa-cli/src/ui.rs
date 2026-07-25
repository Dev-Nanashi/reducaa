use colored::Colorize;

use crate::batch::{FileOutcome, FileResult};

/// Format a byte count into a human-readable string (e.g., "2.4 MB", "340 KB").
pub fn format_bytes(bytes: usize) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;

    if bytes as f64 >= GB {
        format!("{:.2} GB", bytes as f64 / GB)
    } else if bytes as f64 >= MB {
        format!("{:.2} MB", bytes as f64 / MB)
    } else if bytes as f64 >= KB {
        format!("{:.2} KB", bytes as f64 / KB)
    } else {
        format!("{} B", bytes)
    }
}

/// Print the results of a batch compression run as a formatted summary table.
pub fn print_results(results: &[FileResult]) {
    if results.is_empty() {
        println!("{}", "No files processed.".yellow());
        return;
    }

    println!();
    println!("{}", "─".repeat(80).dimmed());
    println!("{}", " COMPRESSION RESULTS".bold());
    println!("{}", "─".repeat(80).dimmed());

    let mut total_original: usize = 0;
    let mut total_compressed: usize = 0;
    let mut success_count: usize = 0;
    let mut fail_count: usize = 0;
    let mut skip_count: usize = 0;

    for result in results {
        let input_name = result
            .input_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("<unknown>");

        match &result.outcome {
            FileOutcome::Success(cr) => {
                success_count += 1;
                total_original += cr.original_size;
                total_compressed += cr.compressed_size;

                let reduction = cr.reduction_percent();
                let reduction_str = if reduction > 0.0 {
                    format!("-{:.1}%", reduction).green().to_string()
                } else {
                    format!("+{:.1}%", -reduction).red().to_string()
                };

                println!(
                    "  {} {} → {} ({})",
                    "✓".green(),
                    input_name.bold(),
                    format!(
                        "{} → {}",
                        format_bytes(cr.original_size),
                        format_bytes(cr.compressed_size)
                    )
                    .dimmed(),
                    reduction_str,
                );
            }
            FileOutcome::Skipped(reason) => {
                skip_count += 1;
                println!(
                    "  {} {} — {}",
                    "⊘".yellow(),
                    input_name.bold(),
                    reason.yellow(),
                );
            }
            FileOutcome::Failed(error) => {
                fail_count += 1;
                println!(
                    "  {} {} — {}",
                    "✗".red(),
                    input_name.bold(),
                    error.red(),
                );
            }
        }
    }

    println!("{}", "─".repeat(80).dimmed());

    // Summary line
    let total = results.len();
    let overall_reduction = if total_original > 0 {
        let saved = total_original.saturating_sub(total_compressed);
        (saved as f64 / total_original as f64) * 100.0
    } else {
        0.0
    };

    println!();
    println!(
        "  {} {} of {} files compressed successfully",
        "▸".cyan(),
        success_count.to_string().green().bold(),
        total,
    );

    if fail_count > 0 {
        println!(
            "  {} {} files failed",
            "▸".cyan(),
            fail_count.to_string().red().bold(),
        );
    }

    if skip_count > 0 {
        println!(
            "  {} {} files skipped",
            "▸".cyan(),
            skip_count.to_string().yellow().bold(),
        );
    }

    if success_count > 0 {
        println!(
            "  {} Total: {} → {} ({} saved, {:.1}% reduction)",
            "▸".cyan(),
            format_bytes(total_original).bold(),
            format_bytes(total_compressed).bold(),
            format_bytes(total_original - total_compressed).green().bold(),
            overall_reduction,
        );
    }

    println!();
}

/// Print a banner at CLI startup.
pub fn print_banner() {
    println!();
    println!(
        "  {} {}",
        "reducaa".bold().cyan(),
        "— Fast, offline image compression".dimmed(),
    );
    println!();
}

/* _GIT_HISTORY_DUMMY_ */ /* Revision 22 - c6f82s */
