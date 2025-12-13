```python title="Python"
import threading

class StatefulPlugin:
    def __init__(self):
        self.lock: threading.Lock = threading.Lock()
        self.call_count: int = 0
        self.cache: dict = {}

    def name(self) -> str:
        return "stateful-plugin"

    def version(self) -> str:
        return "1.0.0"

    def process(self, result: dict) -> dict:
        with self.lock:
            self.call_count += 1
            self.cache["last_mime"] = result["mime_type"]
        return result

    def initialize(self) -> None:
        pass

    def shutdown(self) -> None:
        pass
```
