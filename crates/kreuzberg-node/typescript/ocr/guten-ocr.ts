/**
 * Guten OCR backend for document OCR processing.
 *
 * This module provides integration with @gutenye/ocr-node for optical character recognition.
 * Guten OCR uses PaddleOCR models via ONNX Runtime for high-performance text extraction.
 *
 * @module ocr/guten-ocr
 */

import type { OcrBackendProtocol } from "../types.js";

/**
 * Text line detected by Guten OCR.
 */
interface TextLine {
	text: string;
	mean: number;
	box: number[][];
}

/**
 * Guten OCR instance interface.
 */
interface GutenOcr {
	detect(imagePath: string | Buffer, options?: { onnxOptions?: unknown }): Promise<TextLine[]>;
}

/**
 * Guten OCR module interface.
 */
interface GutenOcrModule {
	create(options?: {
		models?: {
			detectionPath: string;
			recognitionPath: string;
			dictionaryPath: string;
		};
		isDebug?: boolean;
		debugOutputDir?: string;
		onnxOptions?: unknown;
	}): Promise<GutenOcr>;
}

/**
 * Guten OCR backend for OCR processing.
 *
 * This backend uses @gutenye/ocr-node for text extraction from images.
 * It uses PaddleOCR models via ONNX Runtime for efficient processing.
 *
 * ## Installation
 *
 * Install the optional dependency:
 * ```bash
 * npm install @gutenye/ocr-node
 * # or
 * pnpm add @gutenye/ocr-node
 * # or
 * bun add @gutenye/ocr-node
 * ```
 *
 * ## Usage
 *
 * ```typescript
 * import { GutenOcrBackend } from '@kreuzberg/node/ocr/guten-ocr';
 * import { registerOcrBackend, extractFile } from '@kreuzberg/node';
 *
 * // Create and register the backend
 * const backend = new GutenOcrBackend();
 * await backend.initialize();
 * registerOcrBackend(backend);
 *
 * // Extract with OCR enabled
 * const result = await extractFile('scanned.pdf', null, {
 *   ocr: { backend: 'guten-ocr', language: 'en' },
 * });
 * console.log(result.content);
 * ```
 *
 * ## Supported Languages
 *
 * Guten OCR supports multiple languages via different model configurations.
 * The default models support English ("en") and Chinese ("ch_sim", "ch_tra").
 *
 * @example
 * ```typescript
 * // Basic usage with default settings
 * const backend = new GutenOcrBackend();
 * await backend.initialize();
 *
 * // Custom model configuration
 * const customBackend = new GutenOcrBackend({
 *   models: {
 *     detectionPath: './models/detection.onnx',
 *     recognitionPath: './models/recognition.onnx',
 *     dictionaryPath: './models/dict.txt'
 *   }
 * });
 * await customBackend.initialize();
 * ```
 */
export class GutenOcrBackend implements OcrBackendProtocol {
	private ocr: GutenOcr | null = null;
	private ocrModule: GutenOcrModule | null = null;
	private options?: {
		models?: {
			detectionPath: string;
			recognitionPath: string;
			dictionaryPath: string;
		};
		isDebug?: boolean;
		debugOutputDir?: string;
		onnxOptions?: unknown;
	};

	/**
	 * Create a new Guten OCR backend.
	 *
	 * @param options - Optional configuration for Guten OCR
	 * @param options.models - Custom model paths (default: uses bundled models)
	 * @param options.isDebug - Enable debug mode (default: false)
	 * @param options.debugOutputDir - Directory for debug output (default: undefined)
	 * @param options.onnxOptions - Custom ONNX Runtime options (default: undefined)
	 *
	 * @example
	 * ```typescript
	 * // Default configuration
	 * const backend = new GutenOcrBackend();
	 *
	 * // With debug enabled
	 * const debugBackend = new GutenOcrBackend({
	 *   isDebug: true,
	 *   debugOutputDir: './ocr_debug'
	 * });
	 * ```
	 */
	constructor(options?: {
		models?: {
			detectionPath: string;
			recognitionPath: string;
			dictionaryPath: string;
		};
		isDebug?: boolean;
		debugOutputDir?: string;
		onnxOptions?: unknown;
	}) {
		if (options !== undefined) {
			this.options = options;
		}
	}

	/**
	 * Get the backend name.
	 *
	 * @returns Backend name ("guten-ocr")
	 */
	name(): string {
		return "guten-ocr";
	}

	/**
	 * Get list of supported language codes.
	 *
	 * Guten OCR supports multiple languages depending on the model configuration.
	 * The default models support English and Chinese.
	 *
	 * @returns Array of ISO 639-1/2 language codes
	 */
	supportedLanguages(): string[] {
		return ["en", "eng", "ch_sim", "ch_tra", "chinese"];
	}

	/**
	 * Initialize the OCR backend.
	 *
	 * This method loads the Guten OCR module and creates an OCR instance.
	 * Call this before using processImage().
	 *
	 * @throws {Error} If @gutenye/ocr-node is not installed
	 * @throws {Error} If OCR initialization fails
	 *
	 * @example
	 * ```typescript
	 * const backend = new GutenOcrBackend();
	 * await backend.initialize();
	 * ```
	 */
	async initialize(): Promise<void> {
		if (this.ocr !== null) {
			return;
		}

		try {
			this.ocrModule = await import("@gutenye/ocr-node").then((m) => (m.default || m) as GutenOcrModule);
		} catch (e) {
			const error = e as Error;
			throw new Error(
				`Guten OCR support requires the '@gutenye/ocr-node' package. ` +
					`Install with: npm install @gutenye/ocr-node. ` +
					`Error: ${error.message}`,
			);
		}

		try {
			this.ocr = (await this.ocrModule?.create(this.options)) ?? null;
		} catch (e) {
			const error = e as Error;
			throw new Error(`Failed to initialize Guten OCR: ${error.message}`);
		}
	}

	/**
	 * Shutdown the backend and release resources.
	 *
	 * @example
	 * ```typescript
	 * const backend = new GutenOcrBackend();
	 * await backend.initialize();
	 * // ... use backend ...
	 * await backend.shutdown();
	 * ```
	 */
	async shutdown(): Promise<void> {
		this.ocr = null;
		this.ocrModule = null;
	}

	/**
	 * Process image bytes and extract text using Guten OCR.
	 *
	 * This method:
	 * 1. Decodes the image using sharp (if pixel data is needed) or passes bytes directly
	 * 2. Runs OCR detection to find text regions
	 * 3. Runs OCR recognition on each text region
	 * 4. Returns extracted text with metadata
	 *
	 * @param imageBytes - Raw image data (PNG, JPEG, TIFF, etc.)
	 * @param language - Language code (must be in supportedLanguages())
	 * @returns Promise resolving to OCR result with content and metadata
	 *
	 * @throws {Error} If backend is not initialized
	 * @throws {Error} If OCR processing fails
	 *
	 * @example
	 * ```typescript
	 * import { readFile } from 'fs/promises';
	 *
	 * const backend = new GutenOcrBackend();
	 * await backend.initialize();
	 *
	 * const imageBytes = await readFile('scanned.png');
	 * const result = await backend.processImage(imageBytes, 'en');
	 * console.log(result.content);
	 * console.log(result.metadata.confidence);
	 * ```
	 */
	async processImage(
		imageBytes: Uint8Array | string,
		language: string,
	): Promise<{
		content: string;
		mime_type: string;
		metadata: {
			width: number;
			height: number;
			confidence: number;
			text_regions: number;
			language: string;
		};
		tables: never[];
	}> {
		if (this.ocr === null) {
			await this.initialize();
		}

		if (this.ocr === null) {
			throw new Error("Guten OCR backend failed to initialize");
		}

		try {
			const buffer = typeof imageBytes === "string" ? Buffer.from(imageBytes, "base64") : Buffer.from(imageBytes);

			// biome-ignore lint/complexity/useLiteralKeys: required for strict TypeScript noUncheckedIndexedAccess
			const debugEnv = process.env["KREUZBERG_DEBUG_GUTEN"];
			if (debugEnv === "1") {
				const header = Array.from(buffer.subarray(0, 8));
				console.log("[Guten OCR] Debug input header:", header);
				console.log(
					"[Guten OCR] Buffer?",
					Buffer.isBuffer(buffer),
					"constructor",
					imageBytes?.constructor?.name,
					"length",
					buffer.length,
					"type",
					typeof imageBytes,
				);
			}

			let width = 0;
			let height = 0;

			try {
				const sharpModule = await import("sharp");
				const sharp = (sharpModule as unknown as { default?: unknown }).default || sharpModule;
				const image = (sharp as (buffer: Buffer) => { metadata: () => Promise<Record<string, unknown>> })(buffer);
				const metadata = await image.metadata();
				const metadataRecord = metadata as Record<string, unknown>;
				// biome-ignore lint/complexity/useLiteralKeys: TypeScript TS4111 requires bracket notation for index signature properties
				width = (metadataRecord["width"] as number | undefined) ?? 0;
				// biome-ignore lint/complexity/useLiteralKeys: TypeScript TS4111 requires bracket notation for index signature properties
				height = (metadataRecord["height"] as number | undefined) ?? 0;
			} catch (metadataError) {
				const error = metadataError as Error;
				console.warn(`[Guten OCR] Unable to read image metadata via sharp: ${error.message}`);
			}

			const result = await this.ocr.detect(buffer);

			const textLines = result.map((line) => line.text);
			const content = textLines.join("\n");

			const avgConfidence = result.length > 0 ? result.reduce((sum, line) => sum + line.mean, 0) / result.length : 0;

			return {
				content,
				mime_type: "text/plain",
				metadata: {
					width,
					height,
					confidence: avgConfidence,
					text_regions: result.length,
					language,
				},
				tables: [],
			};
		} catch (e) {
			const error = e as Error;
			throw new Error(`Guten OCR processing failed: ${error.message}`);
		}
	}
}
