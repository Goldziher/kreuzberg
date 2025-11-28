```python
import asyncio
from kreuzberg import extract_file, ExtractionConfig

async def main() -> None:
    config: ExtractionConfig = ExtractionConfig()
    result = await extract_file("document.pdf", config=config)

    content: str = result.content
    table_count: int = len(result.tables)

    print(f"Content length: {len(content)} characters")
    print(f"Tables: {table_count}")

asyncio.run(main())
```
