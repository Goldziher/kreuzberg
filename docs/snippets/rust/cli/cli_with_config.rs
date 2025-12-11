```rust
use std::process::Command;
use anyhow::Result;

fn extract_with_config(file_path: &str, config_path: &str) -> Result<serde_json::Value> {
    let output = Command::new("kreuzberg")
        .args(&[
            "extract",
            file_path,
            "--config",
            config_path,
            "--format",
            "json",
        ])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("CLI error: {}", stderr);
    }

    let result: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    Ok(result)
}

fn main() -> Result<()> {
    let config_file = "kreuzberg.toml";
    let document = "document.pdf";

    println!("Extracting {} with config {}", document, config_file);
    let result = extract_with_config(document, config_file)?;

    println!("Content length: {}", result["content"].as_str().unwrap_or("").len());
    println!("Format: {}", result["format"].as_str().unwrap_or("unknown"));
    println!("Languages: {}", result["languages"].to_string());

    Ok(())
}
```
