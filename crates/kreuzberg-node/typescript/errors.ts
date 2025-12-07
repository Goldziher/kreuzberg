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
 * FFI error codes matching kreuzberg-ffi C library error types.
 *
 * @example
 * ```typescript
 * import { ErrorCode, getLastErrorCode } from '@kreuzberg/node';
 *
 * try {
 *   const result = await extractFile('document.pdf');
 * } catch (error) {
 *   const code = getLastErrorCode();
 *   if (code === ErrorCode.Panic) {
 *     console.error('A panic occurred in the native library');
 *   }
 * }
 * ```
 */
export enum ErrorCode {
	/**
	 * No error (success)
	 */
	Success = 0,
	/**
	 * Generic error
	 */
	GenericError = 1,
	/**
	 * Panic occurred in native code
	 */
	Panic = 2,
	/**
	 * Invalid argument provided
	 */
	InvalidArgument = 3,
	/**
	 * I/O error (file system, network, etc.)
	 */
	IoError = 4,
	/**
	 * Error parsing document content
	 */
	ParsingError = 5,
	/**
	 * Error in OCR processing
	 */
	OcrError = 6,
	/**
	 * Required system dependency is missing
	 */
	MissingDependency = 7,
}

/**
 * Context information for panics in native code.
 *
 * Contains file location, line number, function name, panic message,
 * and timestamp for debugging native library issues.
 *
 * @example
 * ```typescript
 * import { KreuzbergError } from '@kreuzberg/node';
 *
 * try {
 *   const result = await extractFile('document.pdf');
 * } catch (error) {
 *   if (error instanceof KreuzbergError && error.panicContext) {
 *     console.error('Panic occurred:');
 *     console.error(`File: ${error.panicContext.file}`);
 *     console.error(`Line: ${error.panicContext.line}`);
 *     console.error(`Function: ${error.panicContext.function}`);
 *     console.error(`Message: ${error.panicContext.message}`);
 *   }
 * }
 * ```
 */
export interface PanicContext {
	/**
	 * Source file where panic occurred
	 */
	file: string;
	/**
	 * Line number in source file
	 */
	line: number;
	/**
	 * Function name where panic occurred
	 */
	function: string;
	/**
	 * Panic message
	 */
	message: string;
	/**
	 * Unix timestamp (seconds since epoch)
	 */
	timestamp_secs: number;
}

/**
 * Base error class for all Kreuzberg errors.
 *
 * All error types thrown by Kreuzberg extend this class, allowing
 * consumers to catch all Kreuzberg-specific errors with a single catch block.
 *
 * @example
 * ```typescript
 * import { extractFile, KreuzbergError } from '@kreuzberg/node';
 *
 * try {
 *   const result = await extractFile('document.pdf');
 * } catch (error) {
 *   if (error instanceof KreuzbergError) {
 *     console.error('Kreuzberg error:', error.message);
 *     if (error.panicContext) {
 *       console.error('Panic at:', error.panicContext.file + ':' + error.panicContext.line);
 *     }
 *   } else {
 *     throw error; // Re-throw non-Kreuzberg errors
 *   }
 * }
 * ```
 */
export class KreuzbergError extends Error {
	/**
	 * Panic context if error was caused by a panic in native code.
	 * Will be null for non-panic errors.
	 */
	public readonly panicContext: PanicContext | null;

	constructor(message: string, panicContext?: PanicContext | null) {
		super(message);
		this.name = "KreuzbergError";
		this.panicContext = panicContext ?? null;
		Object.setPrototypeOf(this, KreuzbergError.prototype);
	}

	toJSON() {
		return {
			name: this.name,
			message: this.message,
			panicContext: this.panicContext,
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
 * import { extractFile, ValidationError } from '@kreuzberg/node';
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
	constructor(message: string, panicContext?: PanicContext | null) {
		super(message, panicContext);
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
 * import { extractFile, ParsingError } from '@kreuzberg/node';
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
	constructor(message: string, panicContext?: PanicContext | null) {
		super(message, panicContext);
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
 * import { extractFile, OcrError } from '@kreuzberg/node';
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
	constructor(message: string, panicContext?: PanicContext | null) {
		super(message, panicContext);
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
 * import { extractFile, CacheError } from '@kreuzberg/node';
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
	constructor(message: string, panicContext?: PanicContext | null) {
		super(message, panicContext);
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
 * import { extractFile, ImageProcessingError } from '@kreuzberg/node';
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
	constructor(message: string, panicContext?: PanicContext | null) {
		super(message, panicContext);
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
 * import { extractFile, PluginError } from '@kreuzberg/node';
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

	constructor(message: string, pluginName: string, panicContext?: PanicContext | null) {
		super(`Plugin error in '${pluginName}': ${message}`, panicContext);
		this.name = "PluginError";
		this.pluginName = pluginName;
		Object.setPrototypeOf(this, PluginError.prototype);
	}

	override toJSON() {
		return {
			name: this.name,
			message: this.message,
			pluginName: this.pluginName,
			panicContext: this.panicContext,
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
 * import { extractFile, MissingDependencyError } from '@kreuzberg/node';
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
	constructor(message: string, panicContext?: PanicContext | null) {
		super(message, panicContext);
		this.name = "MissingDependencyError";
		Object.setPrototypeOf(this, MissingDependencyError.prototype);
	}
}
