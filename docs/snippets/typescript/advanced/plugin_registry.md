```typescript
import {
  registerPostProcessor,
  registerValidator,
  registerOcrBackend,
  listPostProcessors,
  listValidators,
  listOcrBackends,
  unregisterPostProcessor,
  unregisterValidator,
  clearPostProcessors,
  clearValidators,
  clearOcrBackends,
  type PostProcessorProtocol,
  type ValidatorProtocol,
  type OcrBackendProtocol,
} from "@goldziher/kreuzberg";

/**
 * Plugin registry and lifecycle management
 * Demonstrates how to register, list, and unregister plugins
 * @example
 * const registry = new PluginRegistry();
 * registry.registerAll();
 * registry.listAll();
 */
class PluginRegistry {
  private postProcessors: PostProcessorProtocol[] = [];
  private validators: ValidatorProtocol[] = [];
  private ocrBackends: OcrBackendProtocol[] = [];

  /**
   * Register all available plugins
   */
  registerAll(): void {
    console.log("Registering all plugins...");

    // Register post-processors
    this.postProcessors.forEach((processor) => {
      registerPostProcessor(processor);
      console.log(`Registered post-processor: ${processor.name()}`);
    });

    // Register validators
    this.validators.forEach((validator) => {
      registerValidator(validator);
      console.log(`Registered validator: ${validator.name()}`);
    });

    // Register OCR backends
    this.ocrBackends.forEach((backend) => {
      registerOcrBackend(backend);
      console.log(`Registered OCR backend: ${backend.name()}`);
    });
  }

  /**
   * List all registered plugins
   */
  listAll(): void {
    const processors = listPostProcessors();
    const validators = listValidators();
    const backends = listOcrBackends();

    console.log("Registered plugins:");
    console.log(`  Post-processors: ${processors.join(", ")}`);
    console.log(`  Validators: ${validators.join(", ")}`);
    console.log(`  OCR backends: ${backends.join(", ")}`);
  }

  /**
   * Unregister specific plugin by name
   */
  unregisterPlugin(name: string, type: "processor" | "validator"): void {
    if (type === "processor") {
      unregisterPostProcessor(name);
      console.log(`Unregistered post-processor: ${name}`);
    } else if (type === "validator") {
      unregisterValidator(name);
      console.log(`Unregistered validator: ${name}`);
    }
  }

  /**
   * Clear all registered plugins
   */
  clearAll(): void {
    clearPostProcessors();
    clearValidators();
    clearOcrBackends();
    console.log("Cleared all plugins");
  }

  /**
   * Add plugin to registry
   */
  addPostProcessor(processor: PostProcessorProtocol): void {
    this.postProcessors.push(processor);
  }

  addValidator(validator: ValidatorProtocol): void {
    this.validators.push(validator);
  }

  addOcrBackend(backend: OcrBackendProtocol): void {
    this.ocrBackends.push(backend);
  }
}

// Usage
const registry = new PluginRegistry();

// Add plugins to registry
// registry.addPostProcessor(customProcessor);
// registry.addValidator(customValidator);

// Register all plugins
registry.registerAll();

// List registered plugins
registry.listAll();

// Unregister specific plugin
// registry.unregisterPlugin("custom-processor", "processor");

// Clear all plugins
// registry.clearAll();
```
