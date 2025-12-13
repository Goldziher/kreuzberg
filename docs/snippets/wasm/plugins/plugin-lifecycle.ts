import {
  TesseractWasmBackend,
  registerOcrBackend,
  unregisterOcrBackend,
  listOcrBackends,
  initWasm,
} from "@kreuzberg/wasm";

class PluginManager {
  private plugins: Map<string, any> = new Map();

  async registerPlugin(name: string, plugin: any): Promise<void> {
    console.log(`Registering plugin: ${name}`);

    if (plugin.initialize) {
      await plugin.initialize();
    }

    this.plugins.set(name, plugin);
    console.log(`Plugin ${name} registered successfully`);
  }

  async unregisterPlugin(name: string): Promise<void> {
    const plugin = this.plugins.get(name);
    if (!plugin) {
      console.warn(`Plugin ${name} not found`);
      return;
    }

    if (plugin.cleanup) {
      await plugin.cleanup();
    }

    this.plugins.delete(name);
    console.log(`Plugin ${name} unregistered`);
  }

  listPlugins(): string[] {
    return Array.from(this.plugins.keys());
  }

  async reloadPlugin(name: string): Promise<void> {
    const plugin = this.plugins.get(name);
    if (!plugin) {
      console.warn(`Plugin ${name} not found`);
      return;
    }

    console.log(`Reloading plugin: ${name}`);
    await this.unregisterPlugin(name);
    await this.registerPlugin(name, plugin);
  }
}

async function demonstratePluginLifecycle() {
  await initWasm();

  const manager = new PluginManager();

  const backend = new TesseractWasmBackend();
  await manager.registerPlugin("tesseract", backend);

  console.log("Active plugins:", manager.listPlugins());

  await manager.reloadPlugin("tesseract");

  await manager.unregisterPlugin("tesseract");

  console.log("Active plugins:", manager.listPlugins());
}

demonstratePluginLifecycle().catch(console.error);
