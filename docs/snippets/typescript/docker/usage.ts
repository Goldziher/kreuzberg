```typescript
import { exec } from "child_process";
import { promisify } from "util";
import axios from "axios";
import { readFileSync } from "fs";
import { basename } from "path";
import FormData from "form-data";

const execAsync = promisify(exec);

class DockerKreuzbergClient {
  private containerName: string;
  private containerImage: string;
  private apiPort: number;

  constructor(
    containerName: string = "kreuzberg-api",
    containerImage: string = "kreuzberg:latest",
    apiPort: number = 8000
  ) {
    this.containerName = containerName;
    this.containerImage = containerImage;
    this.apiPort = apiPort;
  }

  async startContainer(): Promise<void> {
    console.log("Starting Kreuzberg Docker container...");
    const cmd = `docker run -d --name ${this.containerName} -p ${this.apiPort}:8000 ${this.containerImage}`;
    await execAsync(cmd);
    console.log(`Container started on http:
  }

  async extractFile(filePath: string): Promise<string> {
    const fileContent = readFileSync(filePath);
    const form = new FormData();
    form.append("file", fileContent, basename(filePath));

    const response = await axios.post(
      `http://localhost:${this.apiPort}/api/extract`,
      form,
      { headers: form.getHeaders() }
    );

    return response.data.content;
  }

  async stopContainer(): Promise<void> {
    console.log("Stopping Kreuzberg Docker container...");
    await execAsync(`docker stop ${this.containerName}`);
    await execAsync(`docker rm ${this.containerName}`);
    console.log("Container stopped and removed");
  }
}

(async () => {
  const dockerClient = new DockerKreuzbergClient();

  try {
    await dockerClient.startContainer();
    await new Promise((resolve) => setTimeout(resolve, 2000));

    const content = await dockerClient.extractFile("document.pdf");
    console.log(`Extracted content:\n${content}`);
  } finally {
    await dockerClient.stopContainer();
  }
})();
```
