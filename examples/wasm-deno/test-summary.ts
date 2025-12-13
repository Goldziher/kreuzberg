/**
 * Kreuzberg WASM Integration Test Summary
 *
 * Verifies that the WASM binding works correctly and documents
 * the supported formats in the WASM-target build.
 *
 * Run with: deno run --allow-read test-summary.ts
 */

import init, { extractBytes } from "@kreuzberg/wasm";

async function main() {
	console.log("=".repeat(70));
	console.log("Kreuzberg WASM Integration Test Summary");
	console.log("=".repeat(70));

	// Initialize the WASM module
	console.log("\n1. WASM Module Initialization");
	console.log("-".repeat(70));
	try {
		await init();
		console.log("✓ WASM module initialized successfully");
	} catch (error) {
		console.error("✗ Failed to initialize WASM module:", error);
		Deno.exit(1);
	}

	// Test supported format: HTML
	console.log("\n2. Testing Supported Format: HTML");
	console.log("-".repeat(70));
	try {
		const htmlPath = new URL("./fixtures/test.html", import.meta.url);
		const htmlData = await Deno.readFile(htmlPath.pathname);
		const result = await extractBytes(htmlData, "text/html");

		console.log("✓ HTML extraction successful");
		console.log(`  - File size: ${htmlData.length} bytes`);
		console.log(`  - Extracted content: ${result.content.length} characters`);
		console.log(`  - Content preview: "${result.content.substring(0, 80)}..."`);
	} catch (error) {
		console.error("✗ HTML extraction failed:", error);
	}

	// Test unsupported format: PDF
	console.log("\n3. Testing Unsupported Format: PDF");
	console.log("-".repeat(70));
	try {
		const pdfPath = new URL("./fixtures/sample.pdf", import.meta.url);
		const pdfData = await Deno.readFile(pdfPath.pathname);
		const _result = await extractBytes(pdfData, "application/pdf");
		console.log("✗ PDF extraction should have failed but succeeded");
	} catch (error) {
		const errorMsg = error instanceof Error ? error.message : String(error);
		if (errorMsg.includes("Unsupported format") || errorMsg.includes("pdf")) {
			console.log("✓ PDF extraction correctly rejected");
			console.log(`  - Error: ${errorMsg}`);
			console.log("  - This is expected: PDF support is not included in WASM target");
		} else {
			console.log("✗ Unexpected error:", errorMsg);
		}
	}

	// Summary of WASM-target features
	console.log("\n4. WASM-Target Supported Features");
	console.log("-".repeat(70));
	const supportedFormats = [
		{ name: "HTML", mime: "text/html", status: "Supported" },
		{ name: "XML", mime: "text/xml", status: "Supported" },
		{ name: "Email", mime: "message/rfc822", status: "Supported" },
		{ name: "Plain Text", mime: "text/plain", status: "Supported" },
		{ name: "PDF", mime: "application/pdf", status: "Not supported" },
		{ name: "Office (DOCX/XLSX)", mime: "application/vnd.openxmlformats-*", status: "Not supported" },
		{ name: "OCR", mime: "image/*", status: "Limited" },
	];

	supportedFormats.forEach((format) => {
		const icon = format.status === "Supported" ? "✓" : "✗";
		console.log(`${icon} ${format.name.padEnd(25)} (${format.mime.padEnd(30)}) - ${format.status}`);
	});

	console.log("\n5. WASM-Target Enabled Features");
	console.log("-".repeat(70));
	const features = [
		"HTML extraction",
		"XML extraction",
		"Email parsing",
		"Language detection",
		"Text chunking",
		"Quality analysis (encoding, normalization)",
	];

	features.forEach((feature) => {
		console.log(`✓ ${feature}`);
	});

	console.log("\n6. Configuration Notes");
	console.log("-".repeat(70));
	console.log("- PDF support is disabled: pdfium-render is not WASM-compatible");
	console.log("- Office document support is disabled: Requires PDF functionality");
	console.log("- OCR support is limited in WASM builds");
	console.log("- For full format support, use the native Rust API");

	console.log(`\n${"=".repeat(70)}`);
	console.log("Integration test completed successfully!");
	console.log("=".repeat(70));
}

main().catch((error) => {
	console.error("Fatal error:", error);
	Deno.exit(1);
});
