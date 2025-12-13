import { registerOcrBackend, unregisterOcrBackend, listOcrBackends, extractBytes, initWasm } from "@kreuzberg/wasm";

class CustomOcrBackend {
  private name: string = "custom-ocr";
  private enabled: boolean = true;

  async initialize(): Promise<void> {
    console.log("Initializing custom OCR backend");
  }

  async recognize(imageData: Uint8Array, language: string): Promise<string> {
    console.log(`Recognizing text in ${language} from ${imageData.byteLength} bytes`);
    return "Placeholder OCR result";
  }

  getName(): string {
    return this.name;
  }

  isEnabled(): boolean {
    return this.enabled;
  }

  setEnabled(enabled: boolean) {
    this.enabled = enabled;
  }

  async cleanup(): Promise<void> {
    console.log("Cleaning up custom OCR backend");
  }
}

async function demonstrateCustomBackend() {
  await initWasm();

  const backend = new CustomOcrBackend();
  await backend.initialize();

  registerOcrBackend(backend);

  const backends = listOcrBackends();
  console.log("Registered backends:", backends);

  unregisterOcrBackend("custom-ocr");

  const afterUnregister = listOcrBackends();
  console.log("Backends after unregister:", afterUnregister);

  await backend.cleanup();
}

demonstrateCustomBackend().catch(console.error);
