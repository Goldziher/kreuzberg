```rust
use serde_json::json;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut child = Command::new("kreuzberg")
        .arg("mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;

    {
        let stdin = child.stdin.as_mut().ok_or("Failed to open stdin")?;
        let request = json!({
            "method": "tools/call",
            "params": {
                "name": "extract_file",
                "arguments": {
                    "path": "document.pdf",
                    "async": true
                }
            }
        });
        stdin.write_all(request.to_string().as_bytes())?;
        stdin.write_all(b"\n")?;
    }

    let stdout = child.stdout.take().ok_or("Failed to open stdout")?;
    let reader = BufReader::new(stdout);
    for line in reader.lines() {
        if let Ok(line) = line {
            println!("{}", line);
            break;
        }
    }

    child.wait()?;
    Ok(())
}
```
