```python
import asyncio
from kreuzberg import extract_bytes, ExtractionConfig

async def main():
    with open("document.pdf", "rb") as f:
        data = f.read()

    result = await extract_bytes(
        data,
        mime_type="application/pdf",
        config=ExtractionConfig()
    )
    print(result.content)

asyncio.run(main())
```
