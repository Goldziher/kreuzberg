/**
 * Basic WASM Extraction Example
 *
 * Demonstrates basic document extraction with @kreuzberg/wasm package.
 * Shows both sync and async extraction, configuration options, and error handling.
 *
 * Run with: deno run --allow-read basic.ts
 */

import type { ExtractionConfig } from "@kreuzberg/wasm";
import init, { extractBytes } from "@kreuzberg/wasm";

/**
 * Load a fixture file from the fixtures directory
 */
async function loadFixture(filename: string): Promise<Uint8Array> {
	try {
		const path = new URL(`./fixtures/${filename}`, import.meta.url);
		return await Deno.readFile(path.pathname);
	} catch {
		console.warn(`Could not load fixture: fixtures/${filename}`);
		return new Uint8Array(0);
	}
}

/**
 * Format bytes to human-readable size
 */
function formatSize(bytes: number): string {
	if (bytes === 0) return "0 B";
	const k = 1024;
	const sizes = ["B", "KB", "MB"];
	const i = Math.floor(Math.log(bytes) / Math.log(k));
	return `${(bytes / k ** i).toFixed(2)} ${sizes[i]}`;
}

/**
 * Main example function
 */
async function main() {
	// Initialize the WASM module
	await init();

	console.log("=".repeat(60));
	console.log("Kreuzberg WASM Extraction Examples");
	console.log("=".repeat(60));

	// Example 1: Basic extraction with async
	console.log("\n--- Example 1: Basic Async Extraction ---");
	try {
		const sampleData = await loadFixture("sample.pdf");

		if (sampleData.length === 0) {
			console.log("Sample PDF not found in fixtures/. Creating demo with text content...");
			// Create a simple demo by simulating extraction
			const demoResult = {
				content: "Sample extracted content\nThis is a demonstration of the extraction API.",
				mimeType: "application/pdf",
				metadata: { pages: 1 },
			};
			console.log(`Content length: ${demoResult.content.length} characters`);
			console.log(`MIME type: ${demoResult.mimeType}`);
			console.log(`Preview: ${demoResult.content.substring(0, 80)}...`);
		} else {
			const result = await extractBytes(sampleData, "application/pdf");
			console.log(`Content length: ${result.content.length} characters`);
			console.log(`MIME type: ${result.mimeType}`);
			console.log(`Preview: ${result.content.substring(0, 80)}...`);
		}
	} catch (error) {
		console.error(`Error: ${error instanceof Error ? error.message : String(error)}`);
	}

	// Example 2: Extraction with configuration
	console.log("\n--- Example 2: Extraction with Configuration ---");
	try {
		const sampleData = await loadFixture("sample.pdf");

		if (sampleData.length > 0) {
			const config: ExtractionConfig = {
				chunking: {
					maxChars: 1000,
					maxOverlap: 100,
				},
			};

			const result = await extractBytes(sampleData, "application/pdf", config);
			console.log(`Configured extraction completed`);
			console.log(`Content length: ${result.content.length} characters`);
			console.log(`Chunking configured: max 1000 chars per chunk`);
		} else {
			console.log("Skipping - sample PDF not available in fixtures directory");
		}
	} catch (error) {
		console.error(`Error: ${error instanceof Error ? error.message : String(error)}`);
	}

	// Example 3: MIME type detection
	console.log("\n--- Example 3: MIME Type Detection ---");
	const mimeExamples = [
		{ type: "application/pdf", description: "PDF Document" },
		{
			type: "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
			description: "Microsoft Word (.docx)",
		},
		{
			type: "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
			description: "Microsoft Excel (.xlsx)",
		},
		{ type: "text/html", description: "HTML Document" },
		{ type: "text/plain", description: "Plain Text" },
		{ type: "image/png", description: "PNG Image" },
		{ type: "image/jpeg", description: "JPEG Image" },
	];

	console.log("Supported document types:");
	mimeExamples.forEach((example) => {
		console.log(`  - ${example.description}: ${example.type}`);
	});

	// Example 4: Error handling
	console.log("\n--- Example 4: Error Handling ---");
	try {
		const invalidData = new Uint8Array([0xff, 0xd8, 0xff, 0xe0]); // Partial JPEG header
		console.log("Attempting to extract from invalid/incomplete data...");
		const result = await extractBytes(invalidData, "image/jpeg");
		console.log(`Extraction completed: ${result.content.length} characters`);
	} catch (error) {
		if (error instanceof Error) {
			console.log(`Expected error caught: ${error.message}`);
		}
	}

	// Example 5: Metadata access
	console.log("\n--- Example 5: Metadata Access ---");
	console.log("ExtractionResult includes metadata such as:");
	console.log("  - mimeType: Detected document type");
	console.log("  - metadata.pdf?: PDF-specific metadata (pages, author, etc.)");
	console.log("  - metadata.ocr?: OCR metadata (language, confidence)");
	console.log("  - metadata.format_type?: Document format type");

	// Example 6: Working with results
	console.log("\n--- Example 6: Processing Results ---");
	try {
		const sampleData = await loadFixture("sample.pdf");

		if (sampleData.length > 0) {
			const result = await extractBytes(sampleData, "application/pdf");

			// Print statistics
			const lines = result.content.split("\n").length;
			const words = result.content.split(/\s+/).length;
			const chars = result.content.length;

			console.log("Content statistics:");
			console.log(`  - Lines: ${lines}`);
			console.log(`  - Words: ${words}`);
			console.log(`  - Characters: ${chars}`);
			console.log(`  - Size: ${formatSize(new TextEncoder().encode(result.content).length)}`);
		}
	} catch (error) {
		console.error(`Error: ${error instanceof Error ? error.message : String(error)}`);
	}

	// Example 7: Configuration best practices
	console.log("\n--- Example 7: Configuration Best Practices ---");
	const configs: Record<string, ExtractionConfig> = {
		default: {},

		performance: {
			chunking: {
				maxChars: 2000,
				maxOverlap: 200,
			},
		},

		precise: {
			chunking: {
				maxChars: 500,
				maxOverlap: 50,
			},
		},

		quality: {
			chunking: {
				maxChars: 1000,
				maxOverlap: 100,
			},
		},
	};

	console.log("Available configurations:");
	Object.entries(configs).forEach(([name, config]) => {
		const maxChars = config.chunking?.maxChars || "auto";
		console.log(`  - ${name}: maxChars=${maxChars}`);
	});

	console.log(`\n${"=".repeat(60)}`);
	console.log("Examples completed");
	console.log("=".repeat(60));
}

// Run main function
main().catch((error) => {
	console.error("Fatal error:", error);
	Deno.exit(1);
});
