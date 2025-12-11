```python
import subprocess
import json
import sys
from pathlib import Path

def extract_with_config(file_path: str, config_path: str) -> dict:
    result = subprocess.run(
        ["kreuzberg", "extract", file_path, "--config", config_path, "--format", "json"],
        capture_output=True,
        text=True,
    )

    if result.returncode != 0:
        print(f"Error: {result.stderr}", file=sys.stderr)
        sys.exit(1)

    return json.loads(result.stdout)

config_file = Path("kreuzberg.toml")
document = "document.pdf"

print(f"Extracting {document} with config {config_file}")
result = extract_with_config(str(document), str(config_file))

print(f"Content length: {len(result.get('content', ''))}")
print(f"Format: {result.get('format')}")
print(f"Languages: {result.get('languages')}")
```
