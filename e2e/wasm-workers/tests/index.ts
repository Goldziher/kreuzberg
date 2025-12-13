// Cloudflare Workers entry point for Miniflare testing
// This file is referenced in vitest.config.ts

// Re-export all test files
export * from "./smoke.spec.js";
export * from "./pdf.spec.js";
export * from "./html.spec.js";
export * from "./image.spec.js";
export * from "./email.spec.js";
export * from "./ocr.spec.js";
export * from "./structured.spec.js";
export * from "./xml.spec.js";
export * from "./plugin-apis.spec.js";

// The actual test execution is handled by Vitest with Miniflare
