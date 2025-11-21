```rust
use reqwest::multipart;
use std::fs::File;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open("scanned.pdf")?;
    let config = json!({
        "ocr": {
            "language": "eng"
        },
        "force_ocr": true
    });

    let form = multipart::Form::new()
        .file("files", "scanned.pdf", file)?
        .text("config", config.to_string());

    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:8000/extract")
        .multipart(form)
        .send()
        .await?;

    let results: serde_json::Value = response.json().await?;
    println!("{:?}", results);

    Ok(())
}
```
