//! Integration tests for server commands (serve and mcp).

#[cfg(not(coverage))]
use std::process::{Command, Stdio};
#[cfg(not(coverage))]
use std::thread;
#[cfg(not(coverage))]
use std::time::Duration;

#[cfg(not(coverage))]
#[test]
#[ignore]
fn test_serve_command_starts() {
    let status = Command::new("cargo")
        .args(["build", "--bin", "kreuzberg", "--features", "all"])
        .status()
        .expect("Failed to build binary");

    assert!(status.success(), "Failed to build kreuzberg binary");

    let mut child = Command::new("./target/debug/kreuzberg")
        .args(["serve", "-H", "127.0.0.1", "-p", "18000"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start server");

    thread::sleep(Duration::from_secs(3));

    let mut health_response = ureq::get("http://127.0.0.1:18000/health")
        .call()
        .expect("Failed to call health endpoint");

    assert_eq!(health_response.status(), 200);

    let health_json: serde_json::Value = health_response
        .body_mut()
        .read_json()
        .expect("Failed to parse health response");

    assert_eq!(health_json["status"], "healthy");
    assert!(health_json["version"].is_string());

    let mut info_response = ureq::get("http://127.0.0.1:18000/info")
        .call()
        .expect("Failed to call info endpoint");

    assert_eq!(info_response.status(), 200);

    let info_json: serde_json::Value = info_response
        .body_mut()
        .read_json()
        .expect("Failed to parse info response");

    assert!(info_json["rust_backend"].as_bool().unwrap_or(false));

    child.kill().expect("Failed to kill server");
    child.wait().expect("Failed to wait for server");
}

#[cfg(not(coverage))]
#[test]
#[ignore]
fn test_serve_command_with_config() {
    use std::fs;

    let config_content = r#"
use_cache = true
enable_quality_processing = true

[ocr]
backend = "tesseract"
language = "eng"
"#;

    fs::write("test_config.toml", config_content).expect("Failed to write test config");

    let mut child = Command::new("./target/debug/kreuzberg")
        .args(["serve", "-H", "127.0.0.1", "-p", "18001", "-c", "test_config.toml"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start server");

    thread::sleep(Duration::from_secs(3));

    let health_response = ureq::get("http://127.0.0.1:18001/health").call();

    assert!(health_response.is_ok(), "Server should be running with custom config");

    child.kill().expect("Failed to kill server");
    child.wait().expect("Failed to wait for server");

    fs::remove_file("test_config.toml").ok();
}

#[cfg(not(coverage))]
#[test]
fn test_serve_command_help() {
    let build_status = Command::new("cargo")
        .args(["build", "--bin", "kreuzberg", "--features", "all"])
        .status()
        .expect("Failed to build binary");

    assert!(build_status.success(), "Failed to build kreuzberg binary");

    let binary_path = env!("CARGO_TARGET_TMPDIR")
        .split("target")
        .next()
        .map(|s| format!("{}target/debug/kreuzberg", s))
        .unwrap_or_else(|| "../target/debug/kreuzberg".to_string());

    let output = Command::new(&binary_path)
        .args(["serve", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Start the API server"));
    assert!(stdout.contains("--host"));
    assert!(stdout.contains("--port"));
    assert!(stdout.contains("--config"));
}

#[cfg(not(coverage))]
#[test]
fn test_mcp_command_help() {
    let build_status = Command::new("cargo")
        .args(["build", "--bin", "kreuzberg", "--features", "all"])
        .status()
        .expect("Failed to build binary");

    assert!(build_status.success(), "Failed to build kreuzberg binary");

    let binary_path = env!("CARGO_TARGET_TMPDIR")
        .split("target")
        .next()
        .map(|s| format!("{}target/debug/kreuzberg", s))
        .unwrap_or_else(|| "../target/debug/kreuzberg".to_string());

    let output = Command::new(&binary_path)
        .args(["mcp", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Start the MCP (Model Context Protocol) server"));
    assert!(stdout.contains("--config"));
}
