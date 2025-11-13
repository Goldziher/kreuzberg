/**
 * Error types for Kreuzberg document intelligence framework.
 *
 * These error classes mirror the Rust core error types and provide
 * type-safe error handling for TypeScript consumers.
 *
 * ## Error Hierarchy
 *
 * ```
 * Error (JavaScript built-in)
 *   └── KreuzbergError (base class)
 *       ├── ValidationError
 *       ├── ParsingError
 *       ├── OcrError
 *       ├── CacheError
 *       ├── ImageProcessingError
 *       ├── PluginError
 *       ├── MissingDependencyError
 *       └── ... (other error types)
 * ```
 *
 * @module errors
 */

/**
 * Base error class for all Kreuzberg errors.
 *
 * All error types thrown by Kreuzberg extend this class, allowing
 * consumers to catch all Kreuzberg-specific errors with a single catch block.
 *
 * @example
 * ```typescript
 * import { extractFile, KreuzbergError } from 'kreuzberg';
 *
 * try {
 *   const result = await extractFile('document.pdf');
 * } catch (error) {
 *   if (error instanceof KreuzbergError) {
 *     console.error('Kreuzberg error:', error.message);
 *   } else {
 *     throw error; // Re-throw non-Kreuzberg errors
 *   }
 * }
 * ```
 */
export class KreuzbergError extends Error {
	constructor(message: string) {
		super(message);
		this.name = "KreuzbergError";
		// Ensure proper prototype chain for instanceof checks
		Object.setPrototypeOf(this, KreuzbergError.prototype);
	}

	toJSON() {
		return {
			name: this.name,
			message: this.message,
			stack: this.stack,
		};
	}
}

/**
 * Error thrown when document validation fails.
 *
 * Validation errors occur when a document doesn't meet specified criteria,
 * such as minimum content length, required metadata fields, or quality thresholds.
 *
 * @example
 * ```typescript
 * import { extractFile, ValidationError } from 'kreuzberg';
 *
 * try {
 *   const result = await extractFile('document.pdf');
 * } catch (error) {
 *   if (error instanceof ValidationError) {
 *     console.error('Document validation failed:', error.message);
 *   }
 * }
 * ```
 */
export class ValidationError extends KreuzbergError {
	constructor(message: string) {
		super(message);
		this.name = "ValidationError";
		Object.setPrototypeOf(this, ValidationError.prototype);
	}
}

/**
 * Error thrown when document parsing fails.
 *
 * Parsing errors occur when a document is corrupted, malformed, or cannot
 * be processed by the extraction engine. This includes issues like:
 * - Corrupted PDF files
 * - Invalid XML/JSON syntax
 * - Unsupported file format versions
 * - Encrypted documents without valid passwords
 *
 * @example
 * ```typescript
 * import { extractFile, ParsingError } from 'kreuzberg';
 *
 * try {
 *   const result = await extractFile('corrupted.pdf');
 * } catch (error) {
 *   if (error instanceof ParsingError) {
 *     console.error('Failed to parse document:', error.message);
 *   }
 * }
 * ```
 */
export class ParsingError extends KreuzbergError {
	constructor(message: string) {
		super(message);
		this.name = "ParsingError";
		Object.setPrototypeOf(this, ParsingError.prototype);
	}
}

/**
 * Error thrown when OCR processing fails.
 *
 * OCR errors occur during optical character recognition, such as:
 * - OCR backend initialization failures
 * - Image preprocessing errors
 * - Language model loading issues
 * - OCR engine crashes
 *
 * @example
 * ```typescript
 * import { extractFile, OcrError } from 'kreuzberg';
 *
 * try {
 *   const result = await extractFile('scanned.pdf', null, {
 *     ocr: { backend: 'tesseract', language: 'eng' }
 *   });
 * } catch (error) {
 *   if (error instanceof OcrError) {
 *     console.error('OCR processing failed:', error.message);
 *   }
 * }
 * ```
 */
export class OcrError extends KreuzbergError {
	constructor(message: string) {
		super(message);
		this.name = "OcrError";
		Object.setPrototypeOf(this, OcrError.prototype);
	}
}

/**
 * Error thrown when cache operations fail.
 *
 * Cache errors are typically non-fatal and occur during caching operations, such as:
 * - Cache directory creation failures
 * - Disk write errors
 * - Cache entry corruption
 * - Insufficient disk space
 *
 * These errors are usually logged but don't prevent extraction from completing.
 *
 * @example
 * ```typescript
 * import { extractFile, CacheError } from 'kreuzberg';
 *
 * try {
 *   const result = await extractFile('document.pdf', null, {
 *     useCache: true
 *   });
 * } catch (error) {
 *   if (error instanceof CacheError) {
 *     console.warn('Cache operation failed, continuing without cache:', error.message);
 *   }
 * }
 * ```
 */
export class CacheError extends KreuzbergError {
	constructor(message: string) {
		super(message);
		this.name = "CacheError";
		Object.setPrototypeOf(this, CacheError.prototype);
	}
}

/**
 * Error thrown when image processing operations fail.
 *
 * Image processing errors occur during image manipulation, such as:
 * - Image decoding failures
 * - Unsupported image formats
 * - Image resizing/scaling errors
 * - DPI adjustment failures
 * - Color space conversion issues
 *
 * @example
 * ```typescript
 * import { extractFile, ImageProcessingError } from 'kreuzberg';
 *
 * try {
 *   const result = await extractFile('document.pdf', null, {
 *     images: {
 *       extractImages: true,
 *       targetDpi: 300
 *     }
 *   });
 * } catch (error) {
 *   if (error instanceof ImageProcessingError) {
 *     console.error('Image processing failed:', error.message);
 *   }
 * }
 * ```
 */
export class ImageProcessingError extends KreuzbergError {
	constructor(message: string) {
		super(message);
		this.name = "ImageProcessingError";
		Object.setPrototypeOf(this, ImageProcessingError.prototype);
	}
}

/**
 * Error thrown when a plugin operation fails.
 *
 * Plugin errors occur in custom plugins (postprocessors, validators, OCR backends), such as:
 * - Plugin initialization failures
 * - Plugin processing errors
 * - Plugin crashes or timeouts
 * - Invalid plugin configuration
 *
 * The error message includes the plugin name to help identify which plugin failed.
 *
 * @example
 * ```typescript
 * import { extractFile, PluginError } from 'kreuzberg';
 *
 * try {
 *   const result = await extractFile('document.pdf');
 * } catch (error) {
 *   if (error instanceof PluginError) {
 *     console.error(`Plugin '${error.pluginName}' failed:`, error.message);
 *   }
 * }
 * ```
 */
export class PluginError extends KreuzbergError {
	/**
	 * Name of the plugin that threw the error.
	 */
	public readonly pluginName: string;

	constructor(message: string, pluginName: string) {
		super(`Plugin error in '${pluginName}': ${message}`);
		this.name = "PluginError";
		this.pluginName = pluginName;
		Object.setPrototypeOf(this, PluginError.prototype);
	}

	toJSON() {
		return {
			name: this.name,
			message: this.message,
			pluginName: this.pluginName,
			stack: this.stack,
		};
	}
}

/**
 * Error thrown when a required system dependency is missing.
 *
 * Missing dependency errors occur when external tools or libraries are not available, such as:
 * - LibreOffice (for DOC/PPT/XLS files)
 * - Tesseract OCR (for OCR processing)
 * - ImageMagick (for image processing)
 * - Poppler (for PDF rendering)
 *
 * @example
 * ```typescript
 * import { extractFile, MissingDependencyError } from 'kreuzberg';
 *
 * try {
 *   const result = await extractFile('document.doc');
 * } catch (error) {
 *   if (error instanceof MissingDependencyError) {
 *     console.error('Missing dependency:', error.message);
 *     console.log('Please install LibreOffice to process DOC files');
 *   }
 * }
 * ```
 */
export class MissingDependencyError extends KreuzbergError {
	constructor(message: string) {
		super(message);
		this.name = "MissingDependencyError";
		Object.setPrototypeOf(this, MissingDependencyError.prototype);
	}
}
