/**
 * Tesseract WASM OCR Backend
 *
 * Provides OCR capabilities using tesseract-wasm library for browser environments.
 * Loads training data on-demand from jsDelivr CDN and implements the OcrBackendProtocol.
 *
 * ## Browser-Only Requirement
 *
 * This backend requires browser APIs like createImageBitmap and Web Workers.
 * It will NOT work in Node.js environments without additional canvas polyfills.
 *
 * ## Supported Languages
 *
 * Common ISO 639-1 and ISO 639-2 codes:
 * - English: "eng"
 * - German: "deu"
 * - French: "fra"
 * - Spanish: "spa"
 * - Italian: "ita"
 * - Portuguese: "por"
 * - Dutch: "nld"
 * - Russian: "rus"
 * - Chinese (Simplified): "chi_sim"
 * - Chinese (Traditional): "chi_tra"
 * - Japanese: "jpn"
 * - Korean: "kor"
 * - Arabic: "ara"
 * - Hindi: "hin"
 *
 * For complete language list, see: https://github.com/naptha/tesseract.js
 *
 * @example Basic Usage
 * ```typescript
 * import { TesseractWasmBackend } from '@kreuzberg/wasm/ocr/tesseract-wasm-backend';
 * import { registerOcrBackend, extractBytes, initWasm } from '@kreuzberg/wasm';
 *
 * // Initialize
 * await initWasm();
 * const backend = new TesseractWasmBackend();
 * await backend.initialize();
 * registerOcrBackend(backend);
 *
 * // Use in extraction
 * const imageBytes = new Uint8Array(buffer);
 * const result = await extractBytes(imageBytes, 'image/png', {
 *   ocr: { backend: 'tesseract-wasm', language: 'eng' }
 * });
 * console.log(result.content); // Extracted text
 * ```
 *
 * @example With Language Auto-Detection
 * ```typescript
 * const backend = new TesseractWasmBackend();
 * await backend.initialize();
 * registerOcrBackend(backend);
 *
 * // Extract without specifying language - backend will auto-detect
 * const result = await extractBytes(imageBytes, 'image/png', {
 *   ocr: { backend: 'tesseract-wasm' } // language will auto-detect
 * });
 * ```
 */

import type { OcrBackendProtocol } from "@kreuzberg/core";

/**
 * Tesseract WASM Client interface
 * Type definition for tesseract-wasm's OCRClient class
 */
interface TesseractClient {
	loadModel(modelPath: string): Promise<void>;
	loadImage(image: ImageBitmap | Blob): Promise<void>;
	getText(): Promise<string>;
	getConfidence(): Promise<number>;
	getPageMetadata(): Promise<Record<string, unknown>>;
	destroy(): void;
	terminate(): void;
}

/**
 * TesseractWasmBackend - OCR backend using tesseract-wasm library
 *
 * Implements the OcrBackendProtocol for Kreuzberg document extraction pipeline.
 * Provides comprehensive OCR support with model caching, error handling, and progress reporting.
 */
export class TesseractWasmBackend implements OcrBackendProtocol {
	/** Tesseract WASM client instance */
	private client: TesseractClient | null = null;

	/** Track which models are currently loaded to avoid redundant loads */
	private loadedLanguages: Set<string> = new Set();

	/** Cache for language availability validation */
	private supportedLangsCache: string[] | null = null;

	/** Progress callback for UI updates */
	private progressCallback: ((progress: number) => void) | null = null;

	/** Base URL for training data CDN */
	private readonly CDN_BASE_URL = "https://cdn.jsdelivr.net/npm/tesseract-wasm@0.11.0/dist";

	/**
	 * Return the unique name of this OCR backend
	 *
	 * @returns Backend identifier "tesseract-wasm"
	 */
	name(): string {
		return "tesseract-wasm";
	}

	/**
	 * Return list of supported language codes
	 *
	 * Returns a curated list of commonly available Tesseract language models.
	 * Tesseract supports many more languages through custom models.
	 *
	 * @returns Array of ISO 639-1/2/3 language codes
	 */
	supportedLanguages(): string[] {
		// Return cached list if already computed
		if (this.supportedLangsCache) {
			return this.supportedLangsCache;
		}

		// Comprehensive list of languages supported by tesseract-wasm
		// Includes both 3-letter (ISO 639-2) and 2-letter (ISO 639-1) codes where applicable
		this.supportedLangsCache = [
			// Major languages
			"eng", // English
			"deu", // German
			"fra", // French
			"spa", // Spanish
			"ita", // Italian
			"por", // Portuguese
			"nld", // Dutch
			"rus", // Russian
			"jpn", // Japanese
			"kor", // Korean
			"chi_sim", // Chinese (Simplified)
			"chi_tra", // Chinese (Traditional)

			// Additional European languages
			"pol", // Polish
			"tur", // Turkish
			"swe", // Swedish
			"dan", // Danish
			"fin", // Finnish
			"nor", // Norwegian
			"ces", // Czech
			"slk", // Slovak
			"ron", // Romanian
			"hun", // Hungarian
			"hrv", // Croatian
			"srp", // Serbian
			"bul", // Bulgarian
			"ukr", // Ukrainian
			"ell", // Greek

			// Asian languages
			"ara", // Arabic
			"heb", // Hebrew
			"hin", // Hindi
			"tha", // Thai
			"vie", // Vietnamese
			"mkd", // Macedonian
			"ben", // Bengali
			"tam", // Tamil
			"tel", // Telugu
			"kan", // Kannada
			"mal", // Malayalam
			"mya", // Burmese
			"khm", // Khmer
			"lao", // Lao
			"sin", // Sinhala
		];

		return this.supportedLangsCache;
	}

	/**
	 * Initialize the OCR backend
	 *
	 * Creates the Tesseract WASM client instance. This is called once when
	 * the backend is registered with the extraction pipeline.
	 *
	 * The actual model loading happens in processImage() on-demand to avoid
	 * loading all models upfront.
	 *
	 * @throws {Error} If tesseract-wasm is not available or initialization fails
	 *
	 * @example
	 * ```typescript
	 * const backend = new TesseractWasmBackend();
	 * try {
	 *   await backend.initialize();
	 * } catch (error) {
	 *   console.error('Failed to initialize OCR:', error);
	 * }
	 * ```
	 */
	async initialize(): Promise<void> {
		if (this.client) {
			return; // Already initialized
		}

		try {
			// Dynamically import tesseract-wasm
			const tesseractModule = await this.loadTesseractWasm();

			// @ts-expect-error - tesseract-wasm types are not fully typed
			if (!tesseractModule || typeof tesseractModule.OCRClient !== "function") {
				throw new Error("tesseract-wasm OCRClient not found. Ensure tesseract-wasm is installed and available.");
			}

			// Create client instance
			// @ts-expect-error - tesseract-wasm types are not fully typed
			this.client = new tesseractModule.OCRClient();

			// Initialize tracking
			this.loadedLanguages.clear();
		} catch (error) {
			const message = error instanceof Error ? error.message : String(error);
			throw new Error(`Failed to initialize TesseractWasmBackend: ${message}`);
		}
	}

	/**
	 * Process image bytes and extract text via OCR
	 *
	 * Handles image loading, model loading, OCR processing, and result formatting.
	 * Automatically loads the language model on first use and caches it for subsequent calls.
	 *
	 * @param imageBytes - Raw image data (Uint8Array) or Base64-encoded string
	 * @param language - ISO 639-2/3 language code (e.g., "eng", "deu")
	 * @returns Promise resolving to OCR result with content and metadata
	 * @throws {Error} If image processing fails, model loading fails, or language is unsupported
	 *
	 * @example
	 * ```typescript
	 * const backend = new TesseractWasmBackend();
	 * await backend.initialize();
	 *
	 * const imageBuffer = fs.readFileSync('scanned.png');
	 * const result = await backend.processImage(
	 *   new Uint8Array(imageBuffer),
	 *   'eng'
	 * );
	 *
	 * console.log(result.content); // Extracted text
	 * console.log(result.metadata.confidence); // OCR confidence score
	 * ```
	 */
	async processImage(
		imageBytes: Uint8Array | string,
		language: string,
	): Promise<{
		content: string;
		mime_type: string;
		metadata: Record<string, unknown>;
		tables: unknown[];
	}> {
		if (!this.client) {
			throw new Error("TesseractWasmBackend not initialized. Call initialize() first.");
		}

		// Validate language support
		const supported = this.supportedLanguages();
		// Normalize language code for comparison
		const normalizedLang = language.toLowerCase();
		const isSupported = supported.some((lang) => lang.toLowerCase() === normalizedLang);

		if (!isSupported) {
			throw new Error(`Language "${language}" is not supported. Supported languages: ${supported.join(", ")}`);
		}

		try {
			// Load language model if not already loaded
			if (!this.loadedLanguages.has(normalizedLang)) {
				this.reportProgress(10); // Progress: loading model
				await this.loadLanguageModel(normalizedLang);
				this.loadedLanguages.add(normalizedLang);
				this.reportProgress(30); // Progress: model loaded
			}

			// Convert image bytes to ImageBitmap
			this.reportProgress(40); // Progress: processing image
			const imageBitmap = await this.convertToImageBitmap(imageBytes);

			// Load image into Tesseract
			this.reportProgress(50); // Progress: loading image
			await this.client.loadImage(imageBitmap);

			// Perform OCR
			this.reportProgress(70); // Progress: performing OCR
			const text = await this.client.getText();

			// Get confidence and metadata
			const confidence = await this.getConfidenceScore();
			const pageMetadata = await this.getPageMetadata();

			this.reportProgress(90); // Progress: nearly complete

			// Return result in Kreuzberg format
			return {
				content: text,
				mime_type: "text/plain",
				metadata: {
					language: normalizedLang,
					confidence,
					...pageMetadata,
				},
				tables: [], // Tesseract-wasm doesn't provide structured table detection
			};
		} catch (error) {
			const message = error instanceof Error ? error.message : String(error);
			throw new Error(`OCR processing failed for language "${language}": ${message}`);
		} finally {
			this.reportProgress(100); // Progress: complete
		}
	}

	/**
	 * Shutdown the OCR backend and release resources
	 *
	 * Properly cleans up the Tesseract WASM client, freeing memory and Web Workers.
	 * Called when the backend is unregistered or the application shuts down.
	 *
	 * @throws {Error} If cleanup fails (errors are logged but not critical)
	 *
	 * @example
	 * ```typescript
	 * const backend = new TesseractWasmBackend();
	 * await backend.initialize();
	 * // ... use backend ...
	 * await backend.shutdown(); // Clean up resources
	 * ```
	 */
	async shutdown(): Promise<void> {
		try {
			if (this.client) {
				// Try both destroy and terminate for compatibility
				if (typeof this.client.destroy === "function") {
					this.client.destroy();
				}
				if (typeof this.client.terminate === "function") {
					this.client.terminate();
				}
				this.client = null;
			}

			// Clear cached state
			this.loadedLanguages.clear();
			this.supportedLangsCache = null;
			this.progressCallback = null;
		} catch (error) {
			// Log error but don't throw - shutdown is best-effort
			console.warn(
				`Warning during TesseractWasmBackend shutdown: ${error instanceof Error ? error.message : String(error)}`,
			);
		}
	}

	/**
	 * Set a progress callback for UI updates
	 *
	 * Allows the UI to display progress during OCR processing.
	 * The callback will be called with values from 0 to 100.
	 *
	 * @param callback - Function to call with progress percentage
	 *
	 * @example
	 * ```typescript
	 * const backend = new TesseractWasmBackend();
	 * backend.setProgressCallback((progress) => {
	 *   console.log(`OCR Progress: ${progress}%`);
	 *   document.getElementById('progress-bar').style.width = `${progress}%`;
	 * });
	 * ```
	 */
	setProgressCallback(callback: (progress: number) => void): void {
		this.progressCallback = callback;
	}

	/**
	 * Load language model from CDN
	 *
	 * Fetches the training data for a specific language from jsDelivr CDN.
	 * This is an MVP approach - models are cached by the browser.
	 *
	 * @param language - ISO 639-2/3 language code
	 * @throws {Error} If model download fails or language is not available
	 *
	 * @internal
	 */
	private async loadLanguageModel(language: string): Promise<void> {
		if (!this.client) {
			throw new Error("Client not initialized");
		}

		// Construct model URL - models are named with their language code
		const modelFilename = `${language}.traineddata`;
		const modelUrl = `${this.CDN_BASE_URL}/${modelFilename}`;

		try {
			await this.client.loadModel(modelUrl);
		} catch (error) {
			const message = error instanceof Error ? error.message : String(error);
			throw new Error(`Failed to load model for language "${language}" from ${modelUrl}: ${message}`);
		}
	}

	/**
	 * Convert image bytes or Base64 string to ImageBitmap
	 *
	 * Handles both Uint8Array and Base64-encoded image data, converting to
	 * ImageBitmap format required by Tesseract WASM.
	 *
	 * @param imageBytes - Image data as Uint8Array or Base64 string
	 * @returns Promise resolving to ImageBitmap
	 * @throws {Error} If conversion fails (browser API not available or invalid image data)
	 *
	 * @internal
	 */
	private async convertToImageBitmap(imageBytes: Uint8Array | string): Promise<ImageBitmap> {
		// Check if createImageBitmap is available (browser only)
		if (typeof createImageBitmap === "undefined") {
			throw new Error("createImageBitmap is not available. TesseractWasmBackend requires a browser environment.");
		}

		try {
			// Convert to Uint8Array if string (Base64)
			let bytes = imageBytes;
			if (typeof imageBytes === "string") {
				// Decode Base64 to binary
				const binaryString = atob(imageBytes);
				bytes = new Uint8Array(binaryString.length);
				for (let i = 0; i < binaryString.length; i++) {
					(bytes as Uint8Array)[i] = binaryString.charCodeAt(i);
				}
			}

			// Create Blob from bytes
			const blob = new Blob([bytes as Uint8Array] as BlobPart[]);

			// Convert Blob to ImageBitmap
			const imageBitmap = await createImageBitmap(blob);
			return imageBitmap;
		} catch (error) {
			const message = error instanceof Error ? error.message : String(error);
			throw new Error(`Failed to convert image bytes to ImageBitmap: ${message}`);
		}
	}

	/**
	 * Get confidence score from OCR result
	 *
	 * Attempts to retrieve confidence score from Tesseract.
	 * Returns a safe default if unavailable.
	 *
	 * @returns Confidence score between 0 and 1
	 *
	 * @internal
	 */
	private async getConfidenceScore(): Promise<number> {
		try {
			if (this.client && typeof this.client.getConfidence === "function") {
				const confidence = await this.client.getConfidence();
				// Normalize to 0-1 range if needed (some versions return 0-100)
				return confidence > 1 ? confidence / 100 : confidence;
			}
		} catch {
			// Silently fail - confidence is optional
		}
		return 0.9; // Default reasonable confidence
	}

	/**
	 * Get page metadata from OCR result
	 *
	 * Retrieves additional metadata like image dimensions and processing info.
	 *
	 * @returns Metadata object (may be empty if unavailable)
	 *
	 * @internal
	 */
	private async getPageMetadata(): Promise<Record<string, unknown>> {
		try {
			if (this.client && typeof this.client.getPageMetadata === "function") {
				return await this.client.getPageMetadata();
			}
		} catch {
			// Silently fail - metadata is optional
		}
		return {};
	}

	/**
	 * Dynamically load tesseract-wasm module
	 *
	 * Uses dynamic import to load tesseract-wasm only when needed,
	 * avoiding hard dependency in browser environments where it may not be bundled.
	 *
	 * @returns tesseract-wasm module object
	 * @throws {Error} If module cannot be imported
	 *
	 * @internal
	 */
	private async loadTesseractWasm(): Promise<unknown> {
		try {
			// Use dynamic import to handle both ESM and CJS
			// @ts-expect-error - tesseract-wasm has package.json exports issues with TypeScript
			const module = await import("tesseract-wasm");
			return module;
		} catch (error) {
			const message = error instanceof Error ? error.message : String(error);
			throw new Error(
				`Failed to import tesseract-wasm. Ensure it is installed via: npm install tesseract-wasm. Error: ${message}`,
			);
		}
	}

	/**
	 * Report progress to progress callback
	 *
	 * Internal helper for notifying progress updates during OCR processing.
	 *
	 * @param progress - Progress percentage (0-100)
	 *
	 * @internal
	 */
	private reportProgress(progress: number): void {
		if (this.progressCallback) {
			try {
				this.progressCallback(Math.min(100, Math.max(0, progress)));
			} catch {
				// Ignore callback errors to prevent blocking OCR processing
			}
		}
	}
}
