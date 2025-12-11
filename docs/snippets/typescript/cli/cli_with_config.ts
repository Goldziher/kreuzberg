```typescript
import { spawn } from "child_process";

interface ExtractionResult {
  content: string;
  format?: string;
  languages?: string[];
}

async function extractWithConfig(
  filePath: string,
  configPath: string
): Promise<ExtractionResult> {
  return new Promise((resolve, reject) => {
    const child = spawn("kreuzberg", [
      "extract",
      filePath,
      "--config",
      configPath,
      "--format",
      "json",
    ]);

    let stdout = "";
    let stderr = "";

    child.stdout.on("data", (data) => {
      stdout += data.toString();
    });

    child.stderr.on("data", (data) => {
      stderr += data.toString();
    });

    child.on("close", (code) => {
      if (code !== 0) {
        reject(new Error(`CLI exited with code ${code}: ${stderr}`));
        return;
      }

      resolve(JSON.parse(stdout));
    });
  });
}

const configFile = "kreuzberg.toml";
const document = "document.pdf";

console.log(`Extracting ${document} with config ${configFile}`);
const result = await extractWithConfig(document, configFile);

console.log(`Content length: ${result.content.length}`);
console.log(`Format: ${result.format}`);
console.log(`Languages: ${result.languages?.join(", ")}`);
```
