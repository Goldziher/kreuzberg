/**
 * Type definitions for the Kreuzberg WASM API
 */

export interface ExtractionConfig {
	chunking?: {
		maxChars?: number;
		maxOverlap?: number;
	};
}

export interface ExtractionResult {
	content: string;
	mimeType: string;
	metadata?: {
		pages?: number;
		pdf?: Record<string, unknown>;
		ocr?: Record<string, unknown>;
		format_type?: string;
	};
}

export interface ExtractionError {
	message: string;
	code?: string;
}

export interface UIState {
	isProcessing: boolean;
	currentFile: {
		name: string;
		size: number;
		mimeType: string;
	} | null;
	results: ExtractionResult | null;
	error: ExtractionError | null;
}
