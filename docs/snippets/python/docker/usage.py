```python
import subprocess
import httpx
import json
from pathlib import Path

class DockerKreuzbergClient:
    def __init__(self, container_name: str = "kreuzberg-api", port: int = 8000):
        self.container_name = container_name
        self.port = port
        self.api_url = f"http://localhost:{port}/api/extract"

    def start_container(self, image: str = "kreuzberg:latest"):
        print("Starting Kreuzberg Docker container...")
        subprocess.run(
            [
                "docker", "run", "-d",
                "--name", self.container_name,
                "-p", f"{self.port}:8000",
                image,
            ],
            check=True,
        )
        print(f"Container started on http://localhost:{self.port}")

    async def extract_file(self, file_path: str) -> str:
        file_bytes = Path(file_path).read_bytes()
        files = {"file": (Path(file_path).name, file_bytes)}

        async with httpx.AsyncClient() as client:
            response = await client.post(self.api_url, files=files)
            response.raise_for_status()
            result = response.json()
            return result.get("content", "")

    def stop_container(self):
        print("Stopping Kreuzberg Docker container...")
        subprocess.run(["docker", "stop", self.container_name], check=True)
        subprocess.run(["docker", "rm", self.container_name], check=True)
        print("Container stopped and removed")

async def main():
    docker_client = DockerKreuzbergClient()

    try:
        docker_client.start_container()
        import asyncio
        await asyncio.sleep(2)

        content = await docker_client.extract_file("document.pdf")
        print(f"Extracted content:\n{content}")
    finally:
        docker_client.stop_container()

if __name__ == "__main__":
    import asyncio
    asyncio.run(main())
```
