```typescript
import { spawn } from "child_process";

interface ExtractionResult {
  content: string;
  format?: string;
  languages?: string[];
}

async function extractWithCli(
  filePath: string,
  outputFormat: string = "text"
): Promise<string | ExtractionResult> {
  return new Promise((resolve, reject) => {
    const child = spawn("kreuzberg", ["extract", filePath, "--format", outputFormat]);

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

      if (outputFormat === "json") {
        resolve(JSON.parse(stdout));
      } else {
        resolve(stdout);
      }
    });
  });
}

const document = "document.pdf";
const textOutput = await extractWithCli(document, "text");
console.log(`Extracted: ${(textOutput as string).length} characters`);

const jsonOutput = (await extractWithCli(document, "json")) as ExtractionResult;
console.log(`Format: ${jsonOutput.format}`);
```
