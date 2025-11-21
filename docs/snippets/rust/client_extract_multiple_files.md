```rust
use reqwest::multipart;
use std::fs::File;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file1 = File::open("doc1.pdf")?;
    let file2 = File::open("doc2.docx")?;

    let form = multipart::Form::new()
        .file("files", "doc1.pdf", file1)?
        .file("files", "doc2.docx", file2)?;

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
