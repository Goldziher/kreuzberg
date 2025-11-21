```rust
use reqwest::multipart;
use std::fs::File;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open("document.pdf")?;
    let form = multipart::Form::new()
        .file("files", "document.pdf", file)?;

    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:8000/extract")
        .multipart(form)
        .send()
        .await?;

    let results: serde_json::Value = response.json().await?;
    println!("{:?}", results[0]["content"]);

    Ok(())
}
```
