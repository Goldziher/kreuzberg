```rust
use std::process::Command;
use anyhow::Result;

fn extract_with_cli(file_path: &str, output_format: &str) -> Result<String> {
    let output = Command::new("kreuzberg")
        .args(&["extract", file_path, "--format", output_format])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("CLI error: {}", stderr);
    }

    Ok(String::from_utf8(output.stdout)?)
}

fn main() -> Result<()> {
    let document = "document.pdf";

    let text_output = extract_with_cli(document, "text")?;
    println!("Extracted: {} characters", text_output.len());

    let json_output = extract_with_cli(document, "json")?;
    let parsed: serde_json::Value = serde_json::from_str(&json_output)?;
    println!("Format: {}", parsed.get("format").unwrap_or(&"unknown".into()));

    Ok(())
}
```
