```python
import asyncio
from kreuzberg import ExtractionConfig, PdfConfig, extract_file

async def main() -> None:
    config: ExtractionConfig = ExtractionConfig(
        pdf_options=PdfConfig(
            extract_images=True,
            extract_metadata=True,
            passwords=["password1", "password2"],
        )
    )
    result = await extract_file("document.pdf", config=config)
    print(f"Content: {result.content[:100]}")

asyncio.run(main())
```
