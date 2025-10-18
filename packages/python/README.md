# Kreuzberg

Multi-language document intelligence framework with Rust core and Python bindings.

This package provides Python bindings to the high-performance Rust core library.

For full documentation, visit: <https://kreuzberg.dev>

## Installation

```bash
pip install kreuzberg
```

## Quick Start

```python
from kreuzberg import extract_file, ExtractionConfig

# Extract content from a document
config = ExtractionConfig()
result = extract_file("document.pdf", config=config)
print(result.content)
```

## License

MIT License - see LICENSE file for details.
