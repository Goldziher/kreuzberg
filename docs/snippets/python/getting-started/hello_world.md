```python
import asyncio
from kreuzberg import extract_file

async def main() -> None:
    result = await extract_file("document.pdf")
    print(result.content)

asyncio.run(main())
```
