```python
import asyncio
from kreuzberg import extract_file, ExtractionConfig

async def main():
    result = await extract_file("document.pdf", config=ExtractionConfig())
    print(result.content)

asyncio.run(main())
```
