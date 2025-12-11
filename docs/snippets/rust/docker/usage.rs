```rust
use std::process::Command;
use anyhow::Result;
use reqwest::multipart;
use std::fs;

struct DockerKreuzbergClient {
    container_name: String,
    api_port: u16,
}

impl DockerKreuzbergClient {
    fn new(container_name: &str, api_port: u16) -> Self {
        Self {
            container_name: container_name.to_string(),
            api_port,
        }
    }

    fn start_container(&self, image: &str) -> Result<()> {
        println!("Starting Kreuzberg Docker container...");
        let status = Command::new("docker")
            .args(&[
                "run",
                "-d",
                "--name",
                &self.container_name,
                "-p",
                &format!("{}:8000", self.api_port),
                image,
            ])
            .status()?;

        if !status.success() {
            anyhow::bail!("Failed to start Docker container");
        }

        println!("Container started on http://localhost:{}", self.api_port);
        Ok(())
    }

    async fn extract_file(&self, file_path: &str) -> Result<String> {
        let file_bytes = fs::read(file_path)?;
        let file_part = multipart::Part::bytes(file_bytes)
            .file_name(file_path.to_string());
        let form = multipart::Form::new()
            .part("file", file_part);

        let client = reqwest::Client::new();
        let response = client
            .post(&format!("http://localhost:{}/api/extract", self.api_port))
            .multipart(form)
            .send()
            .await?;

        let json: serde_json::Value = response.json().await?;
        Ok(json["content"].as_str().unwrap_or("").to_string())
    }

    fn stop_container(&self) -> Result<()> {
        println!("Stopping Kreuzberg Docker container...");
        Command::new("docker")
            .args(&["stop", &self.container_name])
            .status()?;
        Command::new("docker")
            .args(&["rm", &self.container_name])
            .status()?;
        println!("Container stopped and removed");
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let docker_client = DockerKreuzbergClient::new("kreuzberg-api", 8000);

    docker_client.start_container("kreuzberg:latest")?;
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    let content = docker_client.extract_file("document.pdf").await?;
    println!("Extracted content:\n{}", content);

    docker_client.stop_container()?;
    Ok(())
}
```
