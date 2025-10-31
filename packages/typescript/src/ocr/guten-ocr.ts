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
	score: number;
	frame: {
		top: number;
		left: number;
		width: number;
		height: number;
	};
}

/**
 * Result from Guten OCR detection.
 */
interface GutenOcrResult {
	texts: TextLine[];
	resizedImageWidth: number;
	resizedImageHeight: number;
}

/**
 * Guten OCR instance interface.
 */
interface GutenOcr {
	detect(
		imagePath:
			| string
			| {
					data: Uint8Array | Uint8ClampedArray | Buffer;
					width: number;
					height: number;
			  },
		options?: { onnxOptions?: unknown },
	): Promise<GutenOcrResult>;
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
 * import { GutenOcrBackend } from '@goldziher/kreuzberg/ocr/guten-ocr';
 * import { registerOcrBackend, extractFile } from '@goldziher/kreuzberg';
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
		this.options = options;
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
		// Guten OCR uses PaddleOCR models which support multiple languages
		// The exact list depends on the model configuration
		// Default models support English and Chinese
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
			// Dynamic import to handle optional dependency
			this.ocrModule = await import("@gutenye/ocr-node").then((m) => m.default || m);
		} catch (e) {
			const error = e as Error;
			throw new Error(
				`Guten OCR support requires the '@gutenye/ocr-node' package. ` +
					`Install with: npm install @gutenye/ocr-node. ` +
					`Error: ${error.message}`,
			);
		}

		try {
			this.ocr = await this.ocrModule!.create(this.options);
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
		imageBytes: Uint8Array,
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
			// Import sharp for image decoding
			const sharp = await import("sharp").then((m: any) => m.default || m);

			// Decode image to get pixel data and dimensions
			const image = sharp(Buffer.from(imageBytes));
			const metadata = await image.metadata();
			const { data, info } = await image.raw().toBuffer({ resolveWithObject: true });

			// Create image input for Guten OCR
			const imageInput = {
				data: new Uint8Array(data),
				width: info.width,
				height: info.height,
			};

			// Run OCR detection
			const result = await this.ocr.detect(imageInput);

			// Process detected text lines
			const textLines = result.texts.map((line) => line.text);
			const content = textLines.join("\n");

			// Calculate average confidence
			const avgConfidence =
				result.texts.length > 0 ? result.texts.reduce((sum, line) => sum + line.score, 0) / result.texts.length : 0;

			return {
				content,
				mime_type: "text/plain",
				metadata: {
					width: info.width,
					height: info.height,
					confidence: avgConfidence,
					text_regions: result.texts.length,
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
