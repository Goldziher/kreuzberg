/**
 * Kreuzberg WASM Browser Example
 *
 * A complete browser application demonstrating document extraction
 * using the Kreuzberg WASM library with Vite.
 *
 * Features:
 * - File upload with drag-and-drop support
 * - Real-time progress indication
 * - Extracted content display with syntax highlighting
 * - Metadata visualization
 * - Copy to clipboard and download functionality
 * - Comprehensive error handling
 * - Mobile-responsive design
 */

import { extractBytes } from "@kreuzberg/wasm";
import type { ExtractionResult } from "./types";

// ============================================================================
// DOM Elements
// ============================================================================

const dropZone = document.getElementById("dropZone") as HTMLElement;
const fileInput = document.getElementById("fileInput") as HTMLInputElement;
const sampleButton = document.getElementById("sampleButton") as HTMLButtonElement;

const uploadSection = document.getElementById("uploadSection") as HTMLElement;
const progressSection = document.getElementById("progressSection") as HTMLElement;
const progressFill = document.getElementById("progressFill") as HTMLElement;
const progressText = document.getElementById("progressText") as HTMLElement;

const resultsSection = document.getElementById("resultsSection") as HTMLElement;
const errorSection = document.getElementById("errorSection") as HTMLElement;

const fileName = document.getElementById("fileName") as HTMLElement;
const fileSize = document.getElementById("fileSize") as HTMLElement;
const mimeType = document.getElementById("mimeType") as HTMLElement;
const charCount = document.getElementById("charCount") as HTMLElement;

const extractedContent = document.getElementById("extractedContent") as HTMLElement;
const metadataContent = document.getElementById("metadataContent") as HTMLElement;

const copyButton = document.getElementById("copyButton") as HTMLButtonElement;
const downloadButton = document.getElementById("downloadButton") as HTMLButtonElement;
const closeButton = document.getElementById("closeButton") as HTMLButtonElement;
const errorCloseButton = document.getElementById("errorCloseButton") as HTMLButtonElement;

const contentTab = document.getElementById("contentTab") as HTMLButtonElement;
const metadataTab = document.getElementById("metadataTab") as HTMLButtonElement;
const errorMessage = document.getElementById("errorMessage") as HTMLElement;

// ============================================================================
// State Management
// ============================================================================

interface AppState {
	currentFile: {
		name: string;
		size: number;
		mimeType: string;
		data: Uint8Array;
	} | null;
	results: ExtractionResult | null;
	isProcessing: boolean;
}

const state: AppState = {
	currentFile: null,
	results: null,
	isProcessing: false,
};

// ============================================================================
// Utility Functions
// ============================================================================

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
 * Get MIME type from file extension
 */
function getMimeType(filename: string): string {
	const ext = filename.split(".").pop()?.toLowerCase();
	const mimeTypes: Record<string, string> = {
		pdf: "application/pdf",
		docx: "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
		doc: "application/msword",
		xlsx: "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
		xls: "application/vnd.ms-excel",
		html: "text/html",
		htm: "text/html",
		txt: "text/plain",
		png: "image/png",
		jpg: "image/jpeg",
		jpeg: "image/jpeg",
	};
	return mimeTypes[ext || ""] || "application/octet-stream";
}

/**
 * Show/hide UI sections
 */
function showSection(section: HTMLElement) {
	section.style.display = "block";
}

function hideSection(section: HTMLElement) {
	section.style.display = "none";
}

/**
 * Update progress bar with animation
 */
function updateProgress(percent: number, text: string) {
	progressFill.style.width = `${Math.min(percent, 100)}%`;
	progressText.textContent = text;
}

/**
 * Display extraction results
 */
function displayResults(result: ExtractionResult, fileInfo: AppState["currentFile"]) {
	if (!fileInfo) return;

	state.results = result;

	// Update file info
	fileName.textContent = fileInfo.name;
	fileSize.textContent = formatSize(fileInfo.size);
	mimeType.textContent = fileInfo.mimeType;
	charCount.textContent = result.content.length.toLocaleString();

	// Display content
	extractedContent.textContent = result.content;

	// Display metadata
	const metadata = result.metadata || {};
	metadataContent.textContent = JSON.stringify(metadata, null, 2);

	// Show results section
	showSection(resultsSection);
}

/**
 * Display error message
 */
function displayError(error: Error | string) {
	const message = error instanceof Error ? error.message : String(error);
	errorMessage.textContent = message;
	showSection(errorSection);
	hideSection(progressSection);
	hideSection(resultsSection);
}

/**
 * Reset UI to initial state
 */
function resetUI() {
	hideSection(progressSection);
	hideSection(resultsSection);
	hideSection(errorSection);
	showSection(uploadSection);

	state.currentFile = null;
	state.results = null;
	state.isProcessing = false;

	// Reset tabs
	contentTab.classList.add("active");
	metadataTab.classList.remove("active");
	document.getElementById("contentTab-pane")?.classList.add("active");
	document.getElementById("metadataTab-pane")?.classList.remove("active");
}

// ============================================================================
// File Processing
// ============================================================================

/**
 * Process a file for extraction
 */
async function processFile(file: File) {
	if (state.isProcessing) return;

	try {
		state.isProcessing = true;
		hideSection(uploadSection);
		hideSection(errorSection);
		showSection(progressSection);

		// Read file as buffer
		updateProgress(10, "Reading file...");
		const arrayBuffer = await file.arrayBuffer();
		const fileData = new Uint8Array(arrayBuffer);

		state.currentFile = {
			name: file.name,
			size: fileData.byteLength,
			mimeType: file.type || getMimeType(file.name),
			data: fileData,
		};

		// Extract content
		updateProgress(30, "Initializing extraction...");
		const result = await extractBytes(fileData, state.currentFile.mimeType || "application/octet-stream");

		updateProgress(90, "Processing results...");

		// Display results
		displayResults(result, state.currentFile);

		updateProgress(100, "Complete!");
		hideSection(progressSection);

		state.isProcessing = false;
	} catch (error) {
		state.isProcessing = false;
		displayError(error as Error);
	}
}

/**
 * Load sample PDF from public folder
 */
async function loadSamplePDF() {
	if (state.isProcessing) return;

	try {
		state.isProcessing = true;
		hideSection(uploadSection);
		hideSection(errorSection);
		showSection(progressSection);

		updateProgress(20, "Loading sample document...");

		const response = await fetch("/sample.pdf");
		if (!response.ok) {
			throw new Error(`Failed to load sample PDF: ${response.status} ${response.statusText}`);
		}

		const arrayBuffer = await response.arrayBuffer();
		const fileData = new Uint8Array(arrayBuffer);

		state.currentFile = {
			name: "sample.pdf",
			size: fileData.byteLength,
			mimeType: "application/pdf",
			data: fileData,
		};

		updateProgress(50, "Extracting content...");

		const result = await extractBytes(fileData, "application/pdf");

		updateProgress(90, "Processing results...");
		displayResults(result, state.currentFile);

		updateProgress(100, "Complete!");
		hideSection(progressSection);

		state.isProcessing = false;
	} catch (error) {
		state.isProcessing = false;
		displayError(error as Error);
	}
}

// ============================================================================
// Event Listeners
// ============================================================================

/**
 * Drop zone interactions
 */
dropZone.addEventListener("click", () => fileInput.click());

dropZone.addEventListener("dragover", (e) => {
	e.preventDefault();
	dropZone.classList.add("drag-over");
});

dropZone.addEventListener("dragleave", () => {
	dropZone.classList.remove("drag-over");
});

dropZone.addEventListener("drop", (e) => {
	e.preventDefault();
	dropZone.classList.remove("drag-over");

	const files = e.dataTransfer?.files;
	if (files && files.length > 0) {
		const file = files[0];
		if (file) {
			processFile(file);
		}
	}
});

/**
 * File input change
 */
fileInput.addEventListener("change", (e) => {
	const files = (e.target as HTMLInputElement).files;
	if (files && files.length > 0) {
		const file = files[0];
		if (file) {
			processFile(file);
		}
	}
});

/**
 * Sample button
 */
sampleButton.addEventListener("click", loadSamplePDF);

/**
 * Tab switching
 */
contentTab.addEventListener("click", () => {
	contentTab.classList.add("active");
	metadataTab.classList.remove("active");
	document.getElementById("contentTab-pane")?.classList.add("active");
	document.getElementById("metadataTab-pane")?.classList.remove("active");
});

metadataTab.addEventListener("click", () => {
	contentTab.classList.remove("active");
	metadataTab.classList.add("active");
	document.getElementById("contentTab-pane")?.classList.remove("active");
	document.getElementById("metadataTab-pane")?.classList.add("active");
});

/**
 * Copy to clipboard
 */
copyButton.addEventListener("click", async () => {
	if (!state.results) return;

	try {
		await navigator.clipboard.writeText(state.results.content);
		const originalText = copyButton.textContent;
		copyButton.textContent = "Copied!";
		copyButton.classList.add("copied");

		setTimeout(() => {
			copyButton.textContent = originalText;
			copyButton.classList.remove("copied");
		}, 2000);
	} catch (error) {
		console.error("Failed to copy to clipboard:", error);
	}
});

/**
 * Download as text file
 */
downloadButton.addEventListener("click", () => {
	if (!state.results || !state.currentFile) return;

	const element = document.createElement("a");
	const file = new Blob([state.results.content], { type: "text/plain" });
	element.href = URL.createObjectURL(file);
	element.download = `${state.currentFile.name.replace(/\.[^/.]+$/, "")}.txt`;
	document.body.appendChild(element);
	element.click();
	document.body.removeChild(element);
	URL.revokeObjectURL(element.href);
});

/**
 * Close/Reset buttons
 */
closeButton.addEventListener("click", resetUI);
errorCloseButton.addEventListener("click", resetUI);

// ============================================================================
// Initialization
// ============================================================================

console.log("Kreuzberg WASM Browser Example initialized");
console.log("COOP/COEP headers are configured for SharedArrayBuffer support");
