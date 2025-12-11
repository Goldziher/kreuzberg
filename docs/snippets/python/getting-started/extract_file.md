```python
import asyncio
from pathlib import Path
from kreuzberg import extract_file

async def main() -> None:
    file_path: Path = Path("document.pdf")

    result = await extract_file(file_path)

    print(f"Content: {result.content}")
    print(f"MIME Type: {result.metadata.format_type}")
    print(f"Tables: {len(result.tables)}")

asyncio.run(main())
```
