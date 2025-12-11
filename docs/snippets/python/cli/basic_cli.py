```python
import subprocess
import json
import sys
from pathlib import Path

def extract_with_cli(file_path: str, output_format: str = "text") -> str:
    result = subprocess.run(
        ["kreuzberg", "extract", file_path, "--format", output_format],
        capture_output=True,
        text=True,
    )

    if result.returncode != 0:
        print(f"Error: {result.stderr}", file=sys.stderr)
        sys.exit(1)

    if output_format == "json":
        return json.loads(result.stdout)
    return result.stdout

document = "document.pdf"
text_output = extract_with_cli(document, "text")
print(f"Extracted: {len(text_output)} characters")

json_output = extract_with_cli(document, "json")
print(f"Format: {json_output.get('format', 'unknown')}")
```
