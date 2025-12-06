//! Profile memory usage for extracting documents.
//!
//! This utility can run against a single file or a batch of files (via `--input-list`).
//! For each input it prints or writes a JSON object containing duration, peak RSS,
//! optional flamegraph path, and the top hot functions when CPU profiling is enabled.
//!
//! **Note**: This binary is only available on non-Windows platforms due to pprof dependency.

#[cfg(not(target_os = "windows"))]
use std::env;
#[cfg(not(target_os = "windows"))]
use std::fs::{File, create_dir_all};
#[cfg(not(target_os = "windows"))]
use std::io::{BufRead, BufReader};
#[cfg(not(target_os = "windows"))]
use std::path::{Path, PathBuf};
#[cfg(not(target_os = "windows"))]
use std::time::Instant;

#[cfg(not(target_os = "windows"))]
use kreuzberg::core::config::ExtractionConfig;
#[cfg(not(target_os = "windows"))]
use kreuzberg::core::extractor::extract_file_sync;
#[cfg(not(target_os = "windows"))]
use serde::Serialize;

#[cfg(all(feature = "profiling", not(target_os = "windows")))]
use pprof::{ProfilerGuardBuilder, Report};

#[cfg(all(feature = "profiling", not(target_os = "windows")))]
use std::collections::HashMap;

#[cfg(all(not(target_os = "windows"), target_os = "macos"))]
fn normalize_rss(value: i64) -> i64 {
    value / 1024
}

#[cfg(all(not(target_os = "windows"), unix, not(target_os = "macos")))]
fn normalize_rss(value: i64) -> i64 {
    value
}

#[cfg(not(target_os = "windows"))]
fn max_rss_kb() -> Option<i64> {
    use std::mem::MaybeUninit;

    let mut usage = MaybeUninit::<libc::rusage>::uninit();
    let rc = unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) };
    if rc != 0 {
        return None;
    }
    let usage = unsafe { usage.assume_init() };
    Some(normalize_rss(usage.ru_maxrss))
}

#[cfg(not(target_os = "windows"))]
#[derive(Debug)]
struct Options {
    inputs: Vec<PathBuf>,
    input_list: Option<PathBuf>,
    flamegraph: Option<PathBuf>,
    flamegraph_dir: Option<PathBuf>,
    output_json: Option<PathBuf>,
    output_dir: Option<PathBuf>,
}

#[cfg(not(target_os = "windows"))]
#[derive(Serialize, Clone)]
struct FunctionSample {
    function: String,
    samples: i64,
    percentage: f64,
}

#[cfg(not(target_os = "windows"))]
#[derive(Serialize, Clone)]
struct ProfileOutput {
    input: String,
    duration_secs: f64,
    peak_rss_kb: Option<i64>,
    delta_rss_kb: Option<i64>,
    flamegraph: Option<String>,
    top_functions: Option<Vec<FunctionSample>>,
}

#[cfg(not(target_os = "windows"))]
fn print_usage() {
    eprintln!(
        "Usage: profile_extract [options] <file ...>\n\nOptions:\n  --flamegraph <path>         Write flamegraph SVG (single input)\n  --flamegraph-dir <dir>      Write flamegraph SVGs for each input\n  --output-json <path>        Write JSON output (single input)\n  --output-dir <dir>          Write per-file JSON outputs to directory\n  --input-list <path>         File with newline-separated input paths\n  -h, --help                  Show this help message"
    );
}

#[cfg(not(target_os = "windows"))]
fn parse_options() -> Options {
    let mut args = env::args().skip(1);
    let mut inputs = Vec::new();
    let mut input_list = None;
    let mut flamegraph = None;
    let mut flamegraph_dir = None;
    let mut output_json = None;
    let mut output_dir = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--flamegraph" => {
                let path = args.next().unwrap_or_else(|| {
                    print_usage();
                    std::process::exit(64);
                });
                flamegraph = Some(PathBuf::from(path));
            }
            "--flamegraph-dir" => {
                let path = args.next().unwrap_or_else(|| {
                    print_usage();
                    std::process::exit(64);
                });
                flamegraph_dir = Some(PathBuf::from(path));
            }
            "--output-json" => {
                let path = args.next().unwrap_or_else(|| {
                    print_usage();
                    std::process::exit(64);
                });
                output_json = Some(PathBuf::from(path));
            }
            "--output-dir" => {
                let path = args.next().unwrap_or_else(|| {
                    print_usage();
                    std::process::exit(64);
                });
                output_dir = Some(PathBuf::from(path));
            }
            "--input-list" => {
                let path = args.next().unwrap_or_else(|| {
                    print_usage();
                    std::process::exit(64);
                });
                input_list = Some(PathBuf::from(path));
            }
            "-h" | "--help" => {
                print_usage();
                std::process::exit(0);
            }
            _ if arg.starts_with("--") => {
                eprintln!("Unknown option: {arg}");
                print_usage();
                std::process::exit(64);
            }
            _ => inputs.push(PathBuf::from(arg)),
        }
    }

    Options {
        inputs,
        input_list,
        flamegraph,
        flamegraph_dir,
        output_json,
        output_dir,
    }
}

#[cfg(not(target_os = "windows"))]
fn sanitize_file_name(path: &Path) -> String {
    let name_owned;
    let name = match path.file_name().and_then(|n| n.to_str()) {
        Some(value) => value,
        None => {
            name_owned = path.to_string_lossy().into_owned();
            &name_owned
        }
    };
    let sanitized: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    if sanitized.is_empty() {
        "output".to_string()
    } else {
        sanitized
    }
}

#[cfg(not(target_os = "windows"))]
fn read_inputs_from_file(list_path: &Path) -> Result<Vec<PathBuf>, String> {
    let file = File::open(list_path).map_err(|e| format!("Failed to open input list {}: {e}", list_path.display()))?;
    let reader = BufReader::new(file);
    let mut inputs = Vec::new();
    for line in reader.lines() {
        let line = line.map_err(|e| format!("Failed to read line from {}: {e}", list_path.display()))?;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        inputs.push(PathBuf::from(trimmed));
    }
    Ok(inputs)
}

#[cfg(not(target_os = "windows"))]
fn main() {
    let options = parse_options();

    let mut targets = options.inputs.clone();
    if let Some(list_path) = &options.input_list {
        match read_inputs_from_file(list_path) {
            Ok(mut list_inputs) => targets.append(&mut list_inputs),
            Err(err) => {
                eprintln!("{err}");
                std::process::exit(66);
            }
        }
    }

    if targets.is_empty() {
        eprintln!("No input files specified");
        print_usage();
        std::process::exit(64);
    }

    let multiple = targets.len() > 1;

    if multiple && options.flamegraph.is_some() && options.flamegraph_dir.is_none() {
        eprintln!("Note: --flamegraph applies to a single input. Use --flamegraph-dir for batch runs.");
    }

    if multiple && options.output_json.is_some() && options.output_dir.is_none() {
        eprintln!("Note: --output-json applies to a single input. Use --output-dir for batch runs.");
    }

    if let Some(dir) = &options.output_dir
        && let Err(err) = create_dir_all(dir)
    {
        eprintln!("Failed to create output directory {}: {err}", dir.display());
    }

    if let Some(dir) = &options.flamegraph_dir
        && let Err(err) = create_dir_all(dir)
    {
        eprintln!("Failed to create flamegraph directory {}: {err}", dir.display());
    }

    let mut aggregated_results: Vec<ProfileOutput> = Vec::new();

    for target in targets {
        let flamegraph_path = if let Some(dir) = &options.flamegraph_dir {
            Some(dir.join(format!("{}.svg", sanitize_file_name(&target))))
        } else if !multiple {
            options.flamegraph.clone()
        } else {
            None
        };

        let output_json_path = if let Some(dir) = &options.output_dir {
            Some(dir.join(format!("{}.json", sanitize_file_name(&target))))
        } else if !multiple {
            options.output_json.clone()
        } else {
            None
        };

        match run_profile(&target, flamegraph_path.clone()) {
            Ok(profile) => {
                if let Some(json_path) = output_json_path {
                    if let Some(parent) = json_path.parent()
                        && let Err(err) = create_dir_all(parent)
                    {
                        eprintln!("Failed to create output directory {}: {err}", parent.display());
                    }

                    match File::create(&json_path) {
                        Ok(file) => {
                            if let Err(err) = serde_json::to_writer_pretty(file, &profile) {
                                eprintln!("Failed to write JSON output {}: {err}", json_path.display());
                            } else {
                                eprintln!("Profile summary written to {}", json_path.display());
                            }
                        }
                        Err(err) => eprintln!("Failed to create JSON output file {}: {err}", json_path.display()),
                    }
                } else {
                    aggregated_results.push(profile);
                }
            }
            Err(err) => {
                eprintln!("{}: {err}", target.display());
            }
        }
    }

    if options.output_json.is_none() && options.output_dir.is_none() {
        if aggregated_results.len() == 1 {
            if let Ok(json) = serde_json::to_string_pretty(&aggregated_results[0]) {
                println!("{json}");
            }
        } else if !aggregated_results.is_empty()
            && let Ok(json) = serde_json::to_string_pretty(&aggregated_results)
        {
            println!("{json}");
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn run_profile(path: &Path, flamegraph_path: Option<PathBuf>) -> Result<ProfileOutput, String> {
    if !path.exists() {
        return Err("Input file does not exist".to_string());
    }

    #[cfg(feature = "profiling")]
    let guard = if flamegraph_path.is_some() {
        #[cfg_attr(not(target_os = "macos"), allow(unused_mut))]
        let mut builder = ProfilerGuardBuilder::default().frequency(100);

        #[cfg(target_os = "macos")]
        {
            builder = builder.blocklist(&[
                "libsystem_kernel.dylib",
                "libsystem_pthread.dylib",
                "libsystem_platform.dylib",
                "libdyld.dylib",
            ]);
        }

        match builder.build() {
            Ok(guard) => Some(guard),
            Err(err) => {
                eprintln!("Failed to start profiler: {err}");
                None
            }
        }
    } else {
        None
    };

    #[cfg(not(feature = "profiling"))]
    if flamegraph_path.is_some() {
        eprintln!(
            "--flamegraph requested but build missing 'profiling' feature; recompile with `--features profiling`."
        );
    }

    let start_rss = max_rss_kb();
    let start = Instant::now();

    let config = ExtractionConfig::default();
    let result = extract_file_sync(path, None, &config).map_err(|e| format!("Extraction failed: {e:?}"))?;
    let _ = result;
    let duration = start.elapsed();
    let end_rss = max_rss_kb();

    #[cfg(feature = "profiling")]
    let (flamegraph_path_str, top_functions) = match (flamegraph_path.clone(), guard) {
        (Some(path), Some(guard)) => match guard.report().build() {
            Ok(report) => {
                if let Some(parent) = path.parent()
                    && let Err(err) = create_dir_all(parent)
                {
                    eprintln!("Failed to create flamegraph directory: {err}");
                }

                match File::create(&path) {
                    Ok(mut file) => {
                        if let Err(err) = report.flamegraph(&mut file) {
                            eprintln!("Failed to write flamegraph: {err}");
                        }
                    }
                    Err(err) => {
                        eprintln!("Failed to create flamegraph file {}: {err}", path.display());
                    }
                }

                let tops = summarize_top_functions(&report, 10);
                let filtered = if tops.is_empty() { None } else { Some(tops) };
                (Some(path.display().to_string()), filtered)
            }
            Err(err) => {
                eprintln!("Failed to build profiling report: {err}");
                (None, None)
            }
        },
        (Some(path), None) => {
            eprintln!("Profiler guard was not initialised; skipping flamegraph generation");
            if let Some(parent) = path.parent() {
                let _ = create_dir_all(parent);
            }
            (None, None)
        }
        _ => (None, None),
    };

    #[cfg(all(not(feature = "profiling"), not(target_os = "windows")))]
    let (flamegraph_path_str, top_functions): (Option<String>, Option<Vec<FunctionSample>>) = (None, None);

    let peak_kb = end_rss.or(start_rss);
    let delta_kb = match (start_rss, end_rss) {
        (Some(before), Some(after)) => Some(after.saturating_sub(before)),
        _ => None,
    };

    Ok(ProfileOutput {
        input: path.display().to_string(),
        duration_secs: duration.as_secs_f64(),
        peak_rss_kb: peak_kb,
        delta_rss_kb: delta_kb,
        flamegraph: flamegraph_path_str,
        top_functions,
    })
}

#[cfg(all(feature = "profiling", not(target_os = "windows")))]
fn summarize_top_functions(report: &Report, limit: usize) -> Vec<FunctionSample> {
    let mut totals: HashMap<String, i64> = HashMap::new();

    for (frames, count) in &report.data {
        let count = *count as i64;
        if count <= 0 {
            continue;
        }

        for frame_symbols in &frames.frames {
            for symbol in frame_symbols {
                let name = symbol.name();
                *totals.entry(name).or_insert(0) += count;
            }
        }
    }

    let total_counts: i64 = totals.values().copied().sum();

    let mut summary: Vec<FunctionSample> = totals
        .into_iter()
        .map(|(function, samples)| {
            let percentage = if total_counts > 0 {
                (samples as f64 / total_counts as f64) * 100.0
            } else {
                0.0
            };
            FunctionSample {
                function,
                samples,
                percentage,
            }
        })
        .collect();

    summary.sort_by(|a, b| b.samples.cmp(&a.samples));

    let filtered: Vec<FunctionSample> = summary
        .iter()
        .filter(|entry| {
            let name = entry.function.as_str();
            !name.starts_with("__") && !name.contains("libsystem") && !name.contains("dyld")
        })
        .take(limit)
        .cloned()
        .collect();

    if filtered.is_empty() {
        summary.into_iter().take(limit).collect()
    } else {
        filtered
    }
}

// Windows stub: profiling features are not available on Windows due to pprof dependency
#[cfg(target_os = "windows")]
fn main() {
    eprintln!("Error: profile_extract is not available on Windows.");
    eprintln!("The pprof profiling library requires Unix-specific APIs.");
    eprintln!("Please use this tool on Linux or macOS.");
    std::process::exit(1);
}
