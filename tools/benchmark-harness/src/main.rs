//! Benchmark harness CLI

use benchmark_harness::{BenchmarkConfig, BenchmarkMode, FixtureManager, Result};
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

/// CLI enum for benchmark mode
#[derive(Debug, Clone, Copy, ValueEnum)]
enum CliMode {
    /// Single-file mode: Sequential execution for fair latency comparison
    SingleFile,
    /// Batch mode: Concurrent execution for throughput measurement
    Batch,
}

impl From<CliMode> for BenchmarkMode {
    fn from(mode: CliMode) -> Self {
        match mode {
            CliMode::SingleFile => BenchmarkMode::SingleFile,
            CliMode::Batch => BenchmarkMode::Batch,
        }
    }
}

#[derive(Parser)]
#[command(name = "benchmark-harness")]
#[command(about = "Benchmark harness for document extraction frameworks", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List all fixtures from a directory
    ListFixtures {
        /// Directory or file pattern to search for fixtures
        #[arg(short, long)]
        fixtures: PathBuf,
    },

    /// Validate fixtures without running benchmarks
    Validate {
        /// Directory or file pattern to search for fixtures
        #[arg(short, long)]
        fixtures: PathBuf,
    },

    /// Run benchmarks
    Run {
        /// Directory or file pattern to search for fixtures
        #[arg(short, long)]
        fixtures: PathBuf,

        /// Frameworks to benchmark (comma-separated)
        #[arg(short = 'F', long, value_delimiter = ',')]
        frameworks: Vec<String>,

        /// Output directory for results
        #[arg(short, long, default_value = "results")]
        output: PathBuf,

        /// Maximum concurrent extractions
        #[arg(short = 'c', long)]
        max_concurrent: Option<usize>,

        /// Timeout in seconds
        #[arg(short = 't', long)]
        timeout: Option<u64>,

        /// Benchmark mode: single-file (sequential) or batch (concurrent)
        #[arg(short = 'm', long, value_enum, default_value = "batch")]
        mode: CliMode,

        /// Number of warmup iterations (discarded from statistics)
        #[arg(short = 'w', long, default_value = "1")]
        warmup: usize,

        /// Number of benchmark iterations for statistical analysis
        #[arg(short = 'i', long, default_value = "3")]
        iterations: usize,

        /// Enable OCR for image extraction
        #[arg(long, default_value = "true")]
        ocr: bool,

        /// Enable quality assessment
        #[arg(long, default_value = "true")]
        measure_quality: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::ListFixtures { fixtures } => {
            let mut manager = FixtureManager::new();

            if fixtures.is_dir() {
                manager.load_fixtures_from_dir(&fixtures)?;
            } else {
                manager.load_fixture(&fixtures)?;
            }

            println!("Loaded {} fixture(s)", manager.len());
            for (path, fixture) in manager.fixtures() {
                println!(
                    "  {} - {} ({} bytes)",
                    path.display(),
                    fixture.document.display(),
                    fixture.file_size
                );
            }

            Ok(())
        }

        Commands::Validate { fixtures } => {
            let mut manager = FixtureManager::new();

            if fixtures.is_dir() {
                manager.load_fixtures_from_dir(&fixtures)?;
            } else {
                manager.load_fixture(&fixtures)?;
            }

            println!("✓ All {} fixture(s) are valid", manager.len());
            Ok(())
        }

        Commands::Run {
            fixtures,
            frameworks,
            output,
            max_concurrent,
            timeout,
            mode,
            warmup,
            iterations,
            ocr,
            measure_quality,
        } => {
            use benchmark_harness::{AdapterRegistry, BenchmarkRunner, NativeAdapter};
            use kreuzberg::{ExtractionConfig, OcrConfig};
            use std::sync::Arc;

            let config = BenchmarkConfig {
                output_dir: output.clone(),
                max_concurrent: max_concurrent.unwrap_or_else(num_cpus::get),
                timeout: std::time::Duration::from_secs(timeout.unwrap_or(1800)),
                benchmark_mode: mode.into(),
                warmup_iterations: warmup,
                benchmark_iterations: iterations,
                measure_quality,
                ..Default::default()
            };

            config.validate()?;

            let extraction_config = if ocr {
                ExtractionConfig {
                    ocr: Some(OcrConfig {
                        backend: "tesseract".to_string(),
                        language: "eng".to_string(),
                        tesseract_config: None,
                    }),
                    ..Default::default()
                }
            } else {
                ExtractionConfig::default()
            };

            let mut registry = AdapterRegistry::new();

            registry.register(Arc::new(NativeAdapter::with_config(extraction_config)))?;
            eprintln!("[adapter] ✓ kreuzberg-native (registered)");

            use benchmark_harness::adapters::{
                create_csharp_sync_adapter, create_go_batch_adapter, create_go_sync_adapter, create_java_sync_adapter,
                create_node_async_adapter, create_node_batch_adapter, create_python_async_adapter,
                create_python_batch_adapter, create_python_sync_adapter, create_ruby_batch_adapter,
                create_ruby_sync_adapter,
            };

            let mut kreuzberg_count = 1;

            if let Ok(adapter) = create_python_sync_adapter() {
                if let Ok(()) = registry.register(Arc::new(adapter)) {
                    eprintln!("[adapter] ✓ kreuzberg-python-sync (registered)");
                    kreuzberg_count += 1;
                } else {
                    eprintln!("[adapter] ✗ kreuzberg-python-sync (registration failed)");
                }
            } else {
                eprintln!("[adapter] ✗ kreuzberg-python-sync (initialization failed)");
            }

            if let Ok(adapter) = create_python_async_adapter() {
                if let Ok(()) = registry.register(Arc::new(adapter)) {
                    eprintln!("[adapter] ✓ kreuzberg-python-async (registered)");
                    kreuzberg_count += 1;
                } else {
                    eprintln!("[adapter] ✗ kreuzberg-python-async (registration failed)");
                }
            } else {
                eprintln!("[adapter] ✗ kreuzberg-python-async (initialization failed)");
            }

            if let Ok(adapter) = create_python_batch_adapter() {
                if let Ok(()) = registry.register(Arc::new(adapter)) {
                    eprintln!("[adapter] ✓ kreuzberg-python-batch (registered)");
                    kreuzberg_count += 1;
                } else {
                    eprintln!("[adapter] ✗ kreuzberg-python-batch (registration failed)");
                }
            } else {
                eprintln!("[adapter] ✗ kreuzberg-python-batch (initialization failed)");
            }

            match create_go_sync_adapter() {
                Ok(adapter) => {
                    if let Err(err) = registry.register(Arc::new(adapter)) {
                        eprintln!("[adapter] ✗ kreuzberg-go-sync (registration failed: {err})");
                    } else {
                        eprintln!("[adapter] ✓ kreuzberg-go-sync (registered)");
                        kreuzberg_count += 1;
                    }
                }
                Err(err) => eprintln!("[adapter] ✗ kreuzberg-go-sync (initialization failed: {err})"),
            }

            match create_go_batch_adapter() {
                Ok(adapter) => {
                    if let Err(err) = registry.register(Arc::new(adapter)) {
                        eprintln!("[adapter] ✗ kreuzberg-go-batch (registration failed: {err})");
                    } else {
                        eprintln!("[adapter] ✓ kreuzberg-go-batch (registered)");
                        kreuzberg_count += 1;
                    }
                }
                Err(err) => eprintln!("[adapter] ✗ kreuzberg-go-batch (initialization failed: {err})"),
            }

            match create_node_async_adapter() {
                Ok(adapter) => {
                    if let Err(err) = registry.register(Arc::new(adapter)) {
                        eprintln!("[adapter] ✗ kreuzberg-node-async (registration failed: {err})");
                    } else {
                        eprintln!("[adapter] ✓ kreuzberg-node-async (registered)");
                        kreuzberg_count += 1;
                    }
                }
                Err(err) => eprintln!("[adapter] ✗ kreuzberg-node-async (initialization failed: {err})"),
            }

            match create_node_batch_adapter() {
                Ok(adapter) => {
                    if let Err(err) = registry.register(Arc::new(adapter)) {
                        eprintln!("[adapter] ✗ kreuzberg-node-batch (registration failed: {err})");
                    } else {
                        eprintln!("[adapter] ✓ kreuzberg-node-batch (registered)");
                        kreuzberg_count += 1;
                    }
                }
                Err(err) => eprintln!("[adapter] ✗ kreuzberg-node-batch (initialization failed: {err})"),
            }

            match create_ruby_sync_adapter() {
                Ok(adapter) => {
                    if let Err(err) = registry.register(Arc::new(adapter)) {
                        eprintln!("[adapter] ✗ kreuzberg-ruby-sync (registration failed: {err})");
                    } else {
                        eprintln!("[adapter] ✓ kreuzberg-ruby-sync (registered)");
                        kreuzberg_count += 1;
                    }
                }
                Err(err) => eprintln!("[adapter] ✗ kreuzberg-ruby-sync (initialization failed: {err})"),
            }

            match create_ruby_batch_adapter() {
                Ok(adapter) => {
                    if let Err(err) = registry.register(Arc::new(adapter)) {
                        eprintln!("[adapter] ✗ kreuzberg-ruby-batch (registration failed: {err})");
                    } else {
                        eprintln!("[adapter] ✓ kreuzberg-ruby-batch (registered)");
                        kreuzberg_count += 1;
                    }
                }
                Err(err) => eprintln!("[adapter] ✗ kreuzberg-ruby-batch (initialization failed: {err})"),
            }

            match create_java_sync_adapter() {
                Ok(adapter) => {
                    if let Err(err) = registry.register(Arc::new(adapter)) {
                        eprintln!("[adapter] ✗ kreuzberg-java-sync (registration failed: {err})");
                    } else {
                        eprintln!("[adapter] ✓ kreuzberg-java-sync (registered)");
                        kreuzberg_count += 1;
                    }
                }
                Err(err) => eprintln!("[adapter] ✗ kreuzberg-java-sync (initialization failed: {err})"),
            }

            match create_csharp_sync_adapter() {
                Ok(adapter) => {
                    if let Err(err) = registry.register(Arc::new(adapter)) {
                        eprintln!("[adapter] ✗ kreuzberg-csharp-sync (registration failed: {err})");
                    } else {
                        eprintln!("[adapter] ✓ kreuzberg-csharp-sync (registered)");
                        kreuzberg_count += 1;
                    }
                }
                Err(err) => eprintln!("[adapter] ✗ kreuzberg-csharp-sync (initialization failed: {err})"),
            }

            eprintln!("[adapter] Kreuzberg bindings: {}/11 available", kreuzberg_count);

            use benchmark_harness::adapters::external::{
                create_docling_adapter, create_docling_batch_adapter, create_markitdown_adapter, create_pandoc_adapter,
                create_tika_batch_adapter, create_tika_sync_adapter, create_unstructured_adapter,
            };

            let mut external_count = 0;

            if let Ok(adapter) = create_docling_adapter() {
                if let Ok(()) = registry.register(Arc::new(adapter)) {
                    eprintln!("[adapter] ✓ docling (registered)");
                    external_count += 1;
                } else {
                    eprintln!("[adapter] ✗ docling (registration failed)");
                }
            } else {
                eprintln!("[adapter] ✗ docling (initialization failed)");
            }

            if let Ok(adapter) = create_docling_batch_adapter() {
                if let Ok(()) = registry.register(Arc::new(adapter)) {
                    eprintln!("[adapter] ✓ docling-batch (registered)");
                    external_count += 1;
                } else {
                    eprintln!("[adapter] ✗ docling-batch (registration failed)");
                }
            } else {
                eprintln!("[adapter] ✗ docling-batch (initialization failed)");
            }

            if let Ok(adapter) = create_markitdown_adapter() {
                if let Ok(()) = registry.register(Arc::new(adapter)) {
                    eprintln!("[adapter] ✓ markitdown (registered)");
                    external_count += 1;
                } else {
                    eprintln!("[adapter] ✗ markitdown (registration failed)");
                }
            } else {
                eprintln!("[adapter] ✗ markitdown (initialization failed)");
            }

            if let Ok(adapter) = create_pandoc_adapter() {
                if let Ok(()) = registry.register(Arc::new(adapter)) {
                    eprintln!("[adapter] ✓ pandoc (registered)");
                    external_count += 1;
                } else {
                    eprintln!("[adapter] ✗ pandoc (registration failed)");
                }
            } else {
                eprintln!("[adapter] ✗ pandoc (initialization failed)");
            }

            if let Ok(adapter) = create_unstructured_adapter() {
                if let Ok(()) = registry.register(Arc::new(adapter)) {
                    eprintln!("[adapter] ✓ unstructured (registered)");
                    external_count += 1;
                } else {
                    eprintln!("[adapter] ✗ unstructured (registration failed)");
                }
            } else {
                eprintln!("[adapter] ✗ unstructured (initialization failed)");
            }

            if let Ok(adapter) = create_tika_sync_adapter() {
                if let Ok(()) = registry.register(Arc::new(adapter)) {
                    eprintln!("[adapter] ✓ tika-sync (registered)");
                    external_count += 1;
                } else {
                    eprintln!("[adapter] ✗ tika-sync (registration failed)");
                }
            } else {
                eprintln!("[adapter] ✗ tika-sync (initialization failed)");
            }

            if let Ok(adapter) = create_tika_batch_adapter() {
                if let Ok(()) = registry.register(Arc::new(adapter)) {
                    eprintln!("[adapter] ✓ tika-batch (registered)");
                    external_count += 1;
                } else {
                    eprintln!("[adapter] ✗ tika-batch (registration failed)");
                }
            } else {
                eprintln!("[adapter] ✗ tika-batch (initialization failed)");
            }

            eprintln!(
                "[adapter] Open source extraction frameworks: {}/7 available",
                external_count
            );
            eprintln!(
                "[adapter] Total adapters: {} available",
                kreuzberg_count + external_count
            );

            let mut runner = BenchmarkRunner::new(config, registry);
            runner.load_fixtures(&fixtures)?;

            println!("Loaded {} fixture(s)", runner.fixture_count());
            println!("Frameworks: {:?}", frameworks);
            println!("Configuration: {:?}", runner.config());

            if runner.fixture_count() == 0 {
                println!("No fixtures to benchmark");
                return Ok(());
            }

            println!("\nRunning benchmarks...");
            let results = runner.run(&frameworks).await?;

            println!("\nCompleted {} benchmark(s)", results.len());

            let mut success_count = 0;
            let mut failure_count = 0;

            for result in &results {
                if result.success {
                    success_count += 1;
                } else {
                    failure_count += 1;
                }
            }

            println!("\nSummary:");
            println!("  Successful: {}", success_count);
            println!("  Failed: {}", failure_count);
            println!("  Total: {}", results.len());

            use benchmark_harness::{write_by_extension_analysis, write_json};
            let output_file = output.join("results.json");
            write_json(&results, &output_file)?;
            println!("\nResults written to: {}", output_file.display());

            let by_ext_file = output.join("by-extension.json");
            write_by_extension_analysis(&results, &by_ext_file)?;
            println!("Per-extension analysis written to: {}", by_ext_file.display());

            Ok(())
        }
    }
}
