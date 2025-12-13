//! Test program for HTML generation functionality
//!
//! Loads all results.json files from /tmp/benchmark-results/ and generates
//! an interactive HTML visualization with Chart.js.

use benchmark_harness::{BenchmarkResult, write_html};
use std::fs;
use std::path::{Path, PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let results_dir = Path::new("/tmp/benchmark-results");
    let output_dir = Path::new("/tmp/test-benchmark-viz");
    let output_file = output_dir.join("index.html");

    println!("Kreuzberg Benchmark HTML Generation Test");
    println!("=========================================\n");

    // Create output directory
    fs::create_dir_all(output_dir)?;
    println!("Output directory: {}", output_dir.display());

    // Load all results.json files
    let mut all_results = Vec::new();
    let mut files_loaded = 0;
    let mut load_errors = 0;

    if !results_dir.exists() {
        return Err(format!("Results directory not found: {}", results_dir.display()).into());
    }

    println!("Scanning for benchmark results...\n");

    for entry in fs::read_dir(results_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            // Look for results.json in subdirectories
            let results_file = find_results_json(&path);

            if let Some(results_file) = results_file {
                match load_results(&results_file) {
                    Ok(mut results) => {
                        let count = results.len();
                        all_results.append(&mut results);
                        files_loaded += 1;
                        println!("  Loaded: {} ({} results)", results_file.display(), count);
                    }
                    Err(e) => {
                        load_errors += 1;
                        eprintln!("  Error loading {}: {}", results_file.display(), e);
                    }
                }
            }
        }
    }

    println!("\n{} files loaded, {} errors\n", files_loaded, load_errors);

    if all_results.is_empty() {
        return Err("No benchmark results found!".into());
    }

    // Generate statistics
    let total_results = all_results.len();
    let successful = all_results.iter().filter(|r| r.success).count();
    let failed = total_results - successful;
    let success_rate = (successful as f64 / total_results as f64) * 100.0;

    // Count unique frameworks and extensions
    let mut frameworks = std::collections::HashSet::new();
    let mut extensions = std::collections::HashSet::new();

    for result in &all_results {
        frameworks.insert(result.framework.clone());
        extensions.insert(result.file_extension.clone());
    }

    println!("Benchmark Summary:");
    println!("  Total results:     {}", total_results);
    println!("  Successful:        {} ({:.1}%)", successful, success_rate);
    println!("  Failed:            {}", failed);
    println!("  Unique frameworks: {}", frameworks.len());
    println!("  File types:        {}", extensions.len());
    println!("\nFrameworks:");
    for fw in sorted_vec(frameworks) {
        println!("  - {}", fw);
    }
    println!("\nFile types:");
    for ext in sorted_vec(extensions) {
        println!("  - .{}", ext);
    }

    // Generate HTML
    println!("\n\nGenerating HTML...");
    write_html(&all_results, &output_file, None)?;
    println!("HTML generated successfully!");

    // Validate output
    let metadata = fs::metadata(&output_file)?;
    let file_size = metadata.len();

    println!("\n\nValidation Results:");
    println!("  Output file: {}", output_file.display());
    println!(
        "  File size:   {} bytes ({:.2} KB)",
        file_size,
        file_size as f64 / 1024.0
    );

    if file_size < 100_000 {
        eprintln!("  Warning: File size < 100 KB (expected larger for embedded data)");
    } else {
        println!("  File size check: PASS");
    }

    // Check for valid HTML
    let content = fs::read_to_string(&output_file)?;
    if content.contains("<!DOCTYPE html>") && content.contains("</html>") {
        println!("  HTML structure: PASS");
    } else {
        eprintln!("  HTML structure: FAIL");
    }

    if content.contains("benchmarkData") && content.contains("new Chart") {
        println!("  Chart.js integration: PASS");
    } else {
        eprintln!("  Chart.js integration: FAIL");
    }

    if content.contains("duration-chart") && content.contains("throughput-chart") {
        println!("  Chart elements: PASS");
    } else {
        eprintln!("  Chart elements: FAIL");
    }

    println!("\n\nTest completed successfully!");
    println!("Open the HTML file in a browser to view the interactive visualization:");
    println!("  file://{}", output_file.display());

    Ok(())
}

/// Find results.json in directory or subdirectories
fn find_results_json(dir: &Path) -> Option<PathBuf> {
    // First check if results.json exists in this directory
    let direct_path = dir.join("results.json");
    if direct_path.exists() {
        return Some(direct_path);
    }

    // Then check first level subdirectories
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.is_dir() {
                    let results_file = path.join("results.json");
                    if results_file.exists() {
                        return Some(results_file);
                    }
                }
            }
        }
    }

    None
}

/// Load results from a JSON file
fn load_results(path: &Path) -> Result<Vec<BenchmarkResult>, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    let results: Vec<BenchmarkResult> = serde_json::from_str(&content)?;
    Ok(results)
}

/// Convert HashSet to sorted Vec
fn sorted_vec<T: Ord>(set: std::collections::HashSet<T>) -> Vec<T> {
    let mut vec: Vec<T> = set.into_iter().collect();
    vec.sort();
    vec
}
