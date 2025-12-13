```rust title="Rust"
use reqwest::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let response = client
        .post("http://localhost:8000/extract")
        .send()
        .await?;

    let status = response.status();
    if status.is_client_error() || status.is_server_error() {
        let error: serde_json::Value = response.json().await?;
        eprintln!(
            "Error: {}: {}",
            error["error_type"], error["message"]
        );
    } else {
        let results: serde_json::Value = response.json().await?;
        println!("{:?}", results);
    }

    Ok(())
}
```
