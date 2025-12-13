/**
 * Result type definitions for Kreuzberg document extraction.
 *
 * These types represent the output of extraction operations,
 * including extracted content, metadata, tables, chunks, and images.
 */

import type { Metadata } from "./metadata.js";

// ============================================================================
// Table Results
// ============================================================================

export interface Table {
	cells: string[][];
	markdown: string;
	pageNumber: number;
}

// ============================================================================
// Chunk Results
// ============================================================================

export interface ChunkMetadata {
	charStart: number;
	charEnd: number;
	tokenCount?: number | null;
	chunkIndex: number;
	totalChunks: number;
}

export interface Chunk {
	content: string;
	embedding?: number[] | null;
	metadata: ChunkMetadata;
}

// ============================================================================
// Image Results
// ============================================================================

export interface ExtractedImage {
	data: Uint8Array;
	format: string;
	imageIndex: number;
	pageNumber?: number | null;
	width?: number | null;
	height?: number | null;
	colorspace?: string | null;
	bitsPerComponent?: number | null;
	isMask: boolean;
	description?: string | null;
	ocrResult?: ExtractionResult | null;
}

// ============================================================================
// Main Extraction Result
// ============================================================================

export interface ExtractionResult {
	content: string;
	mimeType: string;
	metadata: Metadata;
	tables: Table[];
	detectedLanguages: string[] | null;
	chunks: Chunk[] | null;
	images: ExtractedImage[] | null;
}
