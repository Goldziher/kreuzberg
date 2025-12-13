/**
 * HTML Extraction Test
 *
 * Tests HTML extraction to verify WASM module works correctly
 * with supported formats.
 *
 * Run with: deno run --allow-read test-html.ts
 */

import init, { extractBytes } from "@kreuzberg/wasm";

async function main() {
	// Initialize the WASM module
	console.log("Initializing WASM module...");
	await init();
	console.log("WASM module initialized successfully!");

	try {
		// Test HTML extraction with test.html
		console.log("\n--- Testing HTML Extraction ---");
		const htmlPath = new URL("./fixtures/test.html", import.meta.url);
		const htmlData = await Deno.readFile(htmlPath.pathname);

		console.log(`Loaded HTML file: ${htmlData.length} bytes`);
		const result = await extractBytes(htmlData, "text/html");

		console.log("\nExtraction successful!");
		console.log(`Content type: ${result.mimeType}`);
		console.log(`Extracted text length: ${result.content.length} characters`);
		console.log("\nExtracted content:");
		console.log("---");
		console.log(result.content);
		console.log("---");

		// Print metadata if available
		if (result.metadata) {
			console.log("\nMetadata:");
			console.log(JSON.stringify(result.metadata, null, 2));
		}
	} catch (error) {
		console.error("Error:", error instanceof Error ? error.message : String(error));
		Deno.exit(1);
	}
}

main().catch((error) => {
	console.error("Fatal error:", error);
	Deno.exit(1);
});
