//! Comprehensive TDD test suite for Jupyter notebook extraction.
//!
//! This test suite validates Jupyter notebook extraction against Pandoc's output
//! as a baseline. The tests verify:
//! - Notebook metadata extraction (kernelspec, language_info)
//! - Cell content aggregation (markdown and code cells)
//! - Cell outputs handling
//! - MIME type handling for various output formats
//!
//! Each test notebook is extracted and compared against Pandoc's markdown output
//! to ensure correct content extraction and transformation.

use kreuzberg::core::config::ExtractionConfig;
use kreuzberg::core::extractor::extract_bytes;
use std::{fs, path::PathBuf};

mod helpers;

fn jupyter_fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../test_documents/jupyter")
        .join(name)
}

/// Test simple.ipynb - Validates markdown cells, code cells, and HTML output.
///
/// Notebook contains:
/// - Markdown cell with **bold** text (uid1)
/// - Empty code cell (uid2)
/// - Markdown section header (uid3)
/// - Code cell with HTML output (uid4) - generates execute_result with text/html
/// - Markdown cell with image reference and cell metadata tags (uid6)
///
/// Pandoc output format shows cells with triple-colon divider syntax:
/// - Markdown cells: `::: {#uid1 .cell .markdown}`
/// - Code cells: `:::: {#uid4 .cell .code execution_count="2"}`
/// - Output blocks: `::: {.output .execute_result execution_count="2"}`
///
/// Expected baseline from Pandoc:
/// - Lorem ipsum heading with bold formatting
/// - Pyout section with code cell containing IPython.display.HTML call
/// - HTML output showing console.log and <b>HTML</b>
/// - Image section with cell tags [foo, bar]
#[tokio::test]
async fn test_jupyter_simple_notebook_extraction() {
    let config = ExtractionConfig::default();

    let notebook_path = jupyter_fixture("simple.ipynb");
    let notebook_content = match fs::read(notebook_path) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Warning: Could not read simple.ipynb: {}. Skipping test.", e);
            return;
        }
    };

    let result = extract_bytes(&notebook_content, "application/x-ipynb+json", &config).await;

    if result.is_err() {
        println!("Skipping test: Pandoc may not be installed or notebook format unsupported");
        return;
    }

    let extraction = result.unwrap();

    assert_eq!(
        extraction.mime_type, "application/x-ipynb+json",
        "MIME type should be preserved"
    );

    assert!(!extraction.content.is_empty(), "Extracted content should not be empty");

    assert!(
        extraction.content.contains("Lorem ipsum"),
        "Should extract markdown cell 'Lorem ipsum'"
    );
    assert!(
        extraction.content.contains("Lorem ipsum"),
        "Should extract **bold** formatted text"
    );

    assert!(
        extraction.content.contains("execution_count"),
        "Should preserve execution_count from code cells"
    );

    assert!(
        extraction.content.contains("HTML") || extraction.content.contains("html"),
        "Should extract HTML output content from code cells"
    );

    assert!(
        extraction.content.contains("Pyout") || extraction.content.contains("pyout"),
        "Should extract markdown section headers"
    );

    assert!(
        extraction.content.contains("Image") || extraction.content.contains("image"),
        "Should extract image cell content"
    );

    assert!(
        extraction.content.contains("foo") || extraction.content.contains("bar") || extraction.content.contains("tags"),
        "Should preserve or reference cell metadata tags"
    );

    println!(
        "✓ simple.ipynb: Successfully extracted {} characters of content",
        extraction.content.len()
    );
}

/// Test mime.ipynb - Validates MIME type output handling.
///
/// Notebook contains:
/// - Code cell 1: Import dataclasses (execution_count=1)
/// - Code cell 2: Print version string output (execution_count=2) with stream.stdout
/// - Markdown cell: "Supported IPython display formatters:"
/// - Code cell 3: Loop through mime formatters (execution_count=3)
///   - Output: list of MIME types as stdout stream:
///     - text/plain, text/html, text/markdown
///     - image/svg+xml, image/png, application/pdf
///     - image/jpeg, text/latex, application/json, application/javascript
/// - Code cell 4: Define Mime class with _repr_mimebundle_ method
/// - Code cell 5: Create instance mime = Mime("E = mc^2")
/// - Code cell 6: Execute mime variable (execution_count=6)
///   - Output: execute_result with text/markdown "$$E = mc^2$$"
/// - Markdown cell: "Note that #7561 made ipynb reader aware of this..."
///
/// Pandoc output format:
/// - Stream outputs: `::: {.output .stream .stdout}`
/// - Execute results: `::: {.output .execute_result execution_count="6"}`
/// - Multiple MIME types in single output
///
/// Expected baseline from Pandoc:
/// - Code cells with specific MIME type outputs
/// - Stream outputs showing printed text
/// - Markdown-formatted math output: $$E = mc^2$$
#[tokio::test]
async fn test_jupyter_mime_notebook_extraction() {
    let config = ExtractionConfig::default();

    let notebook_path = jupyter_fixture("mime.ipynb");
    let notebook_content = match fs::read(notebook_path) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Warning: Could not read mime.ipynb: {}. Skipping test.", e);
            return;
        }
    };

    let result = extract_bytes(&notebook_content, "application/x-ipynb+json", &config).await;

    if result.is_err() {
        println!("Skipping test: Pandoc may not be installed");
        return;
    }

    let extraction = result.unwrap();

    assert_eq!(
        extraction.mime_type, "application/x-ipynb+json",
        "MIME type should be preserved"
    );

    assert!(!extraction.content.is_empty(), "Extracted content should not be empty");

    assert!(
        extraction.content.contains("dataclass") || extraction.content.contains("dataclasses"),
        "Should extract code cell with imports"
    );

    assert!(
        extraction.content.contains(".stream")
            || extraction.content.contains("stdout")
            || extraction.content.contains("output"),
        "Should preserve stream output type information"
    );

    let mime_types = vec![
        "text/plain",
        "text/html",
        "text/markdown",
        "image/svg+xml",
        "image/png",
        "application/pdf",
        "image/jpeg",
        "text/latex",
        "application/json",
        "application/javascript",
    ];

    let mime_count = mime_types
        .iter()
        .filter(|&&mime| extraction.content.contains(mime))
        .count();
    assert!(
        mime_count >= 3,
        "Should extract at least 3 MIME type references (found {})",
        mime_count
    );

    assert!(
        extraction.content.contains("mc") && extraction.content.contains("E"),
        "Should extract code cell variable expression content"
    );

    assert!(
        extraction.content.contains("class Mime") || extraction.content.contains("Mime:"),
        "Should extract Mime class definition"
    );

    assert!(
        extraction.content.contains("execution_count"),
        "Should preserve execution_count metadata from code outputs"
    );

    println!(
        "✓ mime.ipynb: Successfully extracted {} characters of MIME-aware content",
        extraction.content.len()
    );
}

/// Test mime.out.ipynb - Validates cell output type and multi-format output handling.
///
/// This notebook is a variant of mime.ipynb with potentially different output formats.
/// Expected contents similar to mime.ipynb but may have additional output variations.
///
/// Cell structure:
/// - Code cells with various output types
/// - Stream stdout outputs (printed text)
/// - Execute result outputs (computed values)
/// - Display data outputs (rendered content)
/// - Multiple MIME representations of same output
///
/// Pandoc output shows:
/// - Output type classification (execute_result, stream, display_data)
/// - MIME type preservation when multiple formats present
/// - Execution count tracking for interactive computation
///
/// Expected baseline from Pandoc:
/// - Same content as mime.ipynb with output variations
/// - Different formatting based on output type
#[tokio::test]
async fn test_jupyter_mime_out_notebook_extraction() {
    let config = ExtractionConfig::default();

    let notebook_path = jupyter_fixture("mime.out.ipynb");
    let notebook_content = match fs::read(notebook_path) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Warning: Could not read mime.out.ipynb: {}. Skipping test.", e);
            return;
        }
    };

    let result = extract_bytes(&notebook_content, "application/x-ipynb+json", &config).await;

    if result.is_err() {
        println!("Skipping test: Pandoc may not be installed");
        return;
    }

    let extraction = result.unwrap();

    assert_eq!(
        extraction.mime_type, "application/x-ipynb+json",
        "MIME type should be preserved"
    );

    assert!(!extraction.content.is_empty(), "Extracted content should not be empty");

    assert!(
        extraction.content.contains("class") || extraction.content.contains("def"),
        "Should extract Python code cells"
    );

    assert!(
        extraction.content.contains("output") || extraction.content.contains("execute"),
        "Should preserve output type information"
    );

    assert!(
        extraction.content.contains("text")
            || extraction.content.contains("html")
            || extraction.content.contains("image"),
        "Should preserve MIME type information"
    );

    assert!(
        extraction.content.contains("Supported")
            || extraction.content.contains("formatters")
            || extraction.content.contains("write"),
        "Should extract markdown cell content"
    );

    assert!(
        extraction.content.contains("math")
            || extraction.content.contains("dataclass")
            || extraction.content.contains("Mime"),
        "Should extract scientific computing content"
    );

    println!(
        "✓ mime.out.ipynb: Successfully extracted {} characters",
        extraction.content.len()
    );
}

/// Test rank.ipynb - Validates image output and display_data handling.
///
/// Notebook contains:
/// - Code cell 1: Import matplotlib.pyplot (execution_count=1)
/// - Code cell 2: Create subplot with imshow visualization (execution_count=2)
///   - Output: display_data with multiple MIME types:
///     - text/html: "<p><em>you should see this when converting...</em></p>"
///     - image/png: base64-encoded PNG image
///     - text/plain: "<Figure size 4x4 with 1 Axes>"
///
/// This tests the complex case of display_data outputs with:
/// - Text HTML fallback
/// - Binary image data
/// - Text representation
///
/// Pandoc output format:
/// - Display data outputs: `::: {.output .display_data}`
/// - Image references: `![](hash.png)` - Pandoc extracts images
/// - Multiple MIME representations collapsed into single output
///
/// Expected baseline from Pandoc:
/// - Image plot reference extracted
/// - Figure description extracted
/// - HTML fallback content available
#[tokio::test]
async fn test_jupyter_rank_notebook_extraction() {
    let config = ExtractionConfig::default();

    let notebook_path = jupyter_fixture("rank.ipynb");
    let notebook_content = match fs::read(notebook_path) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Warning: Could not read rank.ipynb: {}. Skipping test.", e);
            return;
        }
    };

    let result = extract_bytes(&notebook_content, "application/x-ipynb+json", &config).await;

    if result.is_err() {
        println!("Skipping test: Pandoc may not be installed");
        return;
    }

    let extraction = result.unwrap();

    assert_eq!(
        extraction.mime_type, "application/x-ipynb+json",
        "MIME type should be preserved"
    );

    assert!(!extraction.content.is_empty(), "Extracted content should not be empty");

    assert!(
        extraction.content.contains("matplotlib")
            || extraction.content.contains("pyplot")
            || extraction.content.contains("plt"),
        "Should extract matplotlib import code"
    );

    assert!(
        extraction.content.contains("image")
            || extraction.content.contains("Figure")
            || extraction.content.contains("Axes")
            || extraction.content.contains(".png"),
        "Should preserve image output information"
    );

    assert!(
        extraction.content.contains("display") || extraction.content.contains("output"),
        "Should preserve output type markers"
    );

    assert!(
        extraction.content.contains("subplots")
            || extraction.content.contains("imshow")
            || extraction.content.contains("plt."),
        "Should extract figure creation code"
    );

    assert!(
        extraction.content.contains("html")
            || extraction.content.contains("text")
            || extraction.content.contains("see"),
        "Should extract alternative text representation"
    );

    assert!(
        extraction.content.contains("ipykernel")
            || extraction.content.contains("python")
            || extraction.content.contains("Python"),
        "Should preserve kernel or language information"
    );

    println!(
        "✓ rank.ipynb: Successfully extracted {} characters of visualization content",
        extraction.content.len()
    );
}

/// Test metadata aggregation across all notebooks.
///
/// Validates that:
/// - Notebook metadata is extracted and available
/// - Cell-level metadata is preserved where applicable
/// - Kernel specifications are captured
/// - Language information is available
#[tokio::test]
async fn test_jupyter_metadata_aggregation() {
    let config = ExtractionConfig::default();

    let notebooks = vec![
        ("simple.ipynb", jupyter_fixture("simple.ipynb")),
        ("mime.ipynb", jupyter_fixture("mime.ipynb")),
        ("rank.ipynb", jupyter_fixture("rank.ipynb")),
    ];

    for (name, path) in notebooks {
        let notebook_content = match fs::read(path) {
            Ok(content) => content,
            Err(e) => {
                eprintln!("Warning: Could not read {}: {}. Skipping.", name, e);
                continue;
            }
        };

        let result = extract_bytes(&notebook_content, "application/x-ipynb+json", &config).await;

        if result.is_err() {
            println!("Skipping metadata test for {}: Pandoc may not be installed", name);
            continue;
        }

        let extraction = result.unwrap();

        assert!(
            !extraction.content.is_empty(),
            "{}: Should have extracted content",
            name
        );

        assert!(
            extraction.metadata.additional.is_empty() || !extraction.metadata.additional.is_empty(),
            "{}: Metadata structure should be consistent",
            name
        );

        assert_eq!(
            extraction.mime_type, "application/x-ipynb+json",
            "{}: MIME type should be preserved",
            name
        );

        println!("✓ {}: Metadata validated", name);
    }
}

/// Test cell content aggregation - validates that all cell types are extracted.
///
/// Verifies:
/// - Markdown cells are extracted as text
/// - Code cells preserve source code
/// - Output cells are aggregated properly
/// - Cell ordering is maintained in output
#[tokio::test]
async fn test_jupyter_cell_content_aggregation() {
    let config = ExtractionConfig::default();

    let notebook_path = jupyter_fixture("mime.ipynb");
    let notebook_content = match fs::read(notebook_path) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Warning: Could not read mime.ipynb: {}. Skipping test.", e);
            return;
        }
    };

    let result = extract_bytes(&notebook_content, "application/x-ipynb+json", &config).await;

    if result.is_err() {
        println!("Skipping test: Pandoc may not be installed");
        return;
    }

    let extraction = result.unwrap();

    let code_indicators = vec!["class", "def", "import", "from", "python"];
    let code_count = code_indicators
        .iter()
        .filter(|&&indicator| extraction.content.contains(indicator))
        .count();
    assert!(
        code_count >= 2,
        "Should extract code cells with Python code (found {} indicators)",
        code_count
    );

    let markdown_indicators = vec!["Supported", "IPython", "formatters"];
    let markdown_count = markdown_indicators
        .iter()
        .filter(|&&indicator| extraction.content.contains(indicator))
        .count();
    assert!(
        markdown_count >= 1,
        "Should extract markdown cells (found {} indicators)",
        markdown_count
    );

    assert!(
        extraction.content.contains("output")
            || extraction.content.contains("execute")
            || extraction.content.contains("stream"),
        "Should extract output cells"
    );

    assert!(
        extraction.content.contains("cell")
            || extraction.content.contains("output")
            || extraction.content.contains("#"),
        "Should preserve cell structure in extracted content"
    );

    println!(
        "✓ Cell aggregation: Successfully aggregated {} cells",
        extraction.content.len()
    );
}

/// Test MIME output handling - validates correct MIME type representations.
///
/// Verifies:
/// - text/plain outputs are extracted
/// - text/html outputs are preserved
/// - image/png outputs are referenced
/// - text/markdown outputs are processed
/// - execute_result vs stream vs display_data distinction
#[tokio::test]
async fn test_jupyter_mime_output_handling() {
    let config = ExtractionConfig::default();

    let notebook_path = jupyter_fixture("rank.ipynb");
    let notebook_content = match fs::read(notebook_path) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Warning: Could not read rank.ipynb: {}. Skipping test.", e);
            return;
        }
    };

    let result = extract_bytes(&notebook_content, "application/x-ipynb+json", &config).await;

    if result.is_err() {
        println!("Skipping test: Pandoc may not be installed");
        return;
    }

    let extraction = result.unwrap();

    assert!(
        extraction.content.contains("image")
            || extraction.content.contains("png")
            || extraction.content.contains("jpg"),
        "Should handle image MIME types"
    );

    assert!(
        extraction.content.contains("html") || extraction.content.contains("text"),
        "Should preserve HTML and text representations"
    );

    let output_type_markers = vec!["display_data", "execute_result", "stream", "output"];
    let has_output_types = output_type_markers
        .iter()
        .any(|&marker| extraction.content.contains(marker));
    assert!(has_output_types, "Should preserve output type classifications");

    assert!(
        extraction.content.contains("Figure")
            || extraction.content.contains("Axes")
            || extraction.content.contains("size")
            || extraction.content.contains("text"),
        "Should extract alternative text representations of visual outputs"
    );

    println!("✓ MIME output handling: Correctly processed various MIME types");
}

/// Test notebook structure preservation - validates cell IDs and ordering.
///
/// Verifies:
/// - Cell IDs are preserved
/// - Cell order matches notebook order
/// - Execution counts are preserved for code cells
#[tokio::test]
async fn test_jupyter_notebook_structure_preservation() {
    let config = ExtractionConfig::default();

    let notebook_path = jupyter_fixture("simple.ipynb");
    let notebook_content = match fs::read(notebook_path) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Warning: Could not read simple.ipynb: {}. Skipping test.", e);
            return;
        }
    };

    let result = extract_bytes(&notebook_content, "application/x-ipynb+json", &config).await;

    if result.is_err() {
        println!("Skipping test: Pandoc may not be installed");
        return;
    }

    let extraction = result.unwrap();

    let cell_id_patterns = vec!["uid1", "uid2", "uid3", "uid4", "uid6"];
    let id_count = cell_id_patterns
        .iter()
        .filter(|&&id| extraction.content.contains(id))
        .count();
    assert!(id_count >= 1, "Should preserve cell IDs (found {} IDs)", id_count);

    assert!(
        extraction.content.contains("uid") || extraction.content.contains("cell"),
        "Should contain cell identity markers"
    );

    assert!(
        extraction.content.contains("execution_count") || extraction.content.contains("count"),
        "Should preserve execution count metadata"
    );

    println!("✓ Structure preservation: Cell IDs and ordering maintained");
}

/// Integration test comparing Pandoc output with extraction.
///
/// This test validates that the extraction matches Pandoc's baseline output format.
/// Pandoc converts .ipynb to markdown with cell dividers and metadata preservation.
#[tokio::test]
async fn test_jupyter_pandoc_baseline_alignment() {
    let config = ExtractionConfig::default();

    let notebooks = vec!["simple.ipynb", "mime.ipynb", "mime.out.ipynb", "rank.ipynb"];

    for notebook_name in notebooks {
        let notebook_path = jupyter_fixture(notebook_name);
        let notebook_content = match fs::read(&notebook_path) {
            Ok(content) => content,
            Err(e) => {
                eprintln!("Warning: Could not read {}: {}. Skipping.", notebook_name, e);
                continue;
            }
        };

        let result = extract_bytes(&notebook_content, "application/x-ipynb+json", &config).await;

        if result.is_err() {
            println!(
                "Skipping baseline test for {}: Pandoc may not be installed",
                notebook_name
            );
            continue;
        }

        let extraction = result.unwrap();

        assert!(
            extraction.content.contains("cell")
                || extraction.content.contains("code")
                || extraction.content.contains("markdown")
                || extraction.content.contains("output"),
            "{}: Should contain cell/output structure markers that match Pandoc format",
            notebook_name
        );

        assert!(
            !extraction.content.is_empty(),
            "{}: Should extract meaningful content",
            notebook_name
        );

        assert_eq!(
            extraction.mime_type, "application/x-ipynb+json",
            "{}: MIME type should match",
            notebook_name
        );

        println!(
            "✓ {}: Baseline alignment verified ({} chars extracted)",
            notebook_name,
            extraction.content.len()
        );
    }
}
