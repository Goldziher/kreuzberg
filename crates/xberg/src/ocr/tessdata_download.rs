//! Tessdata language pack download utilities for runtime language pack materialization.
//!
//! Provides functions to download `.traineddata` files from the official Tesseract
//! tessdata_fast repository on GitHub, with fallback URLs for redundancy.

use crate::ocr::error::OcrError;
use std::path::Path;

/// Download a language pack from the tessdata_fast repository.
///
/// Attempts to download `{lang}.traineddata` from GitHub with fallback URLs
/// for redundancy. Skips download if the file already exists. Uses `ureq`
/// with rustls + platform-verifier for HTTPS (OS trust store; respects SSL_CERT_FILE).
///
/// # Arguments
///
/// * `lang` - Language code (e.g., "eng", "fra", "deu")
/// * `output_dir` - Directory to write the traineddata file
///
/// # Returns
///
/// `Ok(())` if download succeeds or file already exists, `Err(OcrError)` on failure.
///
/// # Panics
///
/// Does not panic. All errors are returned as `OcrError`.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn download_language_pack(lang: &str, output_dir: &Path) -> Result<(), OcrError> {
    let traineddata_path = output_dir.join(format!("{}.traineddata", lang));

    if traineddata_path.exists() {
        tracing::debug!(
            "Language pack '{}' already exists at {}",
            lang,
            traineddata_path.display()
        );
        return Ok(());
    }

    let urls = [
        format!(
            "https://github.com/tesseract-ocr/tessdata_fast/raw/main/{}.traineddata",
            lang
        ),
        format!(
            "https://raw.githubusercontent.com/tesseract-ocr/tessdata_fast/main/{}.traineddata",
            lang
        ),
    ];

    tracing::info!(
        "Downloading Tesseract language pack '{}' to {} (source: tessdata_fast)",
        lang,
        traineddata_path.display()
    );

    for url in &urls {
        tracing::info!("Fetching language pack '{}' from {}", lang, url);
        match download_file(url, &traineddata_path) {
            Ok(_) => {
                tracing::info!(
                    "Successfully downloaded language pack '{}' to {}",
                    lang,
                    traineddata_path.display()
                );
                return Ok(());
            }
            Err(e) => {
                tracing::warn!("Failed to download from {}: {}", url, e);
                continue;
            }
        }
    }

    Err(OcrError::TesseractInitializationFailed(format!(
        "Failed to download language pack '{}' from tessdata_fast repository. \
         Tried URLs: {}. Check your network connection and verify the language code is valid.",
        lang,
        urls.join(", ")
    )))
}

/// Maximum traineddata download size (64 MB). The largest `tessdata_fast`
/// language packs are well under this; it guards against pathological responses.
#[cfg(not(target_arch = "wasm32"))]
const MAX_TRAINEDDATA_BYTES: u64 = 64 * 1024 * 1024;

/// Download a file from a URL and write it atomically to the specified path.
///
/// Writes to a sibling temp file first, then renames into place so an
/// interrupted download never leaves a partial `.traineddata` behind.
#[cfg(not(target_arch = "wasm32"))]
fn download_file(url: &str, output_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let response = ureq::get(url).call()?;

    if response.status() != 200 {
        return Err(format!("HTTP {}", response.status()).into());
    }

    let bytes = response
        .into_body()
        .with_config()
        .limit(MAX_TRAINEDDATA_BYTES)
        .read_to_vec()?;

    let tmp_path = output_path.with_extension("traineddata.partial");
    std::fs::write(&tmp_path, bytes)?;
    std::fs::rename(&tmp_path, output_path)?;

    Ok(())
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::download_file;
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;
    use std::process::{Child, Command, ExitStatus};
    use std::time::{Duration, Instant};

    const CHILD_OUTPUT: &str = "XBERG_TESSDATA_PROXY_TEST_OUTPUT";
    const TEST_NAME: &str =
        "ocr::tessdata_download::tests::should_decode_percent_encoded_proxy_credentials_when_downloading";
    const DOWNLOAD_URL: &str = "http://tessdata.invalid/eng.traineddata";
    const PAYLOAD: &[u8] = b"synthetic traineddata\0\xff";
    const TIMEOUT: Duration = Duration::from_secs(15);
    const POLL_INTERVAL: Duration = Duration::from_millis(10);

    #[test]
    fn should_decode_percent_encoded_proxy_credentials_when_downloading() {
        if let Some(output_path) = std::env::var_os(CHILD_OUTPUT) {
            let output_path = std::path::PathBuf::from(output_path);
            download_file(DOWNLOAD_URL, &output_path).expect("download through authenticated proxy");
            assert_eq!(std::fs::read(&output_path).unwrap(), PAYLOAD);
            assert!(!output_path.with_extension("traineddata.partial").exists());
            return;
        }

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let directory = tempfile::tempdir().unwrap();
        let output_path = directory.path().join("eng.traineddata");
        let proxy = format!("http://user%25name:p%25ass%3Aword@{}", listener.local_addr().unwrap());
        let mut child = proxy_child(&proxy, &output_path);
        let request = std::panic::catch_unwind(|| serve_proxy(&listener));
        if request.is_err() {
            let _ = child.kill();
        }
        let status = wait_for_child(&mut child);
        let (request, tunneled_request) = request.unwrap_or_else(|panic| std::panic::resume_unwind(panic));
        assert!(status.success(), "isolated downloader failed: {status}");
        assert_eq!(std::fs::read(output_path).unwrap(), PAYLOAD);
        assert_eq!(request[0], "CONNECT tessdata.invalid:80 HTTP/1.1");
        assert_eq!(tunneled_request[0], "GET /eng.traineddata HTTP/1.1");
        let authorization = request.iter().find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("proxy-authorization").then_some(value.trim())
        });
        assert_eq!(authorization, Some("Basic dXNlciVuYW1lOnAlYXNzOndvcmQ="));
    }

    fn proxy_child(proxy: &str, output_path: &std::path::Path) -> Child {
        // ~keep A child isolates proxy environment changes from concurrently running tests.
        let mut command = Command::new(std::env::current_exe().unwrap());
        command.args(["--exact", TEST_NAME, "--nocapture"]);
        for name in ["HTTP_PROXY", "http_proxy"] {
            command.env(name, proxy);
        }
        for name in ["HTTPS_PROXY", "https_proxy", "ALL_PROXY", "all_proxy"] {
            command.env_remove(name);
        }
        command.env("NO_PROXY", "").env("no_proxy", "");
        command.env(CHILD_OUTPUT, output_path).spawn().unwrap()
    }

    fn serve_proxy(listener: &TcpListener) -> (Vec<String>, Vec<String>) {
        let deadline = Instant::now() + TIMEOUT;
        let mut stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(Instant::now() < deadline, "downloader did not reach proxy");
                    std::thread::sleep(POLL_INTERVAL);
                }
                Err(error) => panic!("accept proxy connection: {error}"),
            }
        };
        stream.set_nonblocking(false).unwrap();
        stream.set_read_timeout(Some(TIMEOUT)).unwrap();
        stream.set_write_timeout(Some(TIMEOUT)).unwrap();
        let request = read_request(&stream);
        stream
            .write_all(b"HTTP/1.1 200 Connection established\r\n\r\n")
            .unwrap();
        let tunneled_request = read_request(&stream);
        let mut response = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            PAYLOAD.len()
        )
        .into_bytes();
        response.extend_from_slice(PAYLOAD);
        stream.write_all(&response).unwrap();
        (request, tunneled_request)
    }

    fn read_request(stream: &std::net::TcpStream) -> Vec<String> {
        let mut reader = BufReader::new(stream);
        let mut request = Vec::new();
        loop {
            let mut line = String::new();
            assert_ne!(reader.read_line(&mut line).unwrap(), 0, "incomplete proxy request");
            if line == "\r\n" {
                return request;
            }
            request.push(line.trim_end().to_owned());
        }
    }

    fn wait_for_child(child: &mut Child) -> ExitStatus {
        let deadline = Instant::now() + TIMEOUT;
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                return status;
            }
            if Instant::now() >= deadline {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("isolated downloader exceeded timeout");
            }
            std::thread::sleep(POLL_INTERVAL);
        }
    }
}
