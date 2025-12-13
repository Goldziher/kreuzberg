import { extractBytes, initWasm } from "@kreuzberg/wasm";

/**
 * Type definitions for the API responses
 */
interface ExtractedData {
	text: string;
	metadata?: Record<string, unknown>;
	tables?: Array<{
		cells: string[][];
		markdown: string;
		pageNumber: number;
	}>;
}

interface HealthResponse {
	status: "healthy" | "unhealthy";
	timestamp: string;
	version: string;
}

interface ErrorResponse {
	error: string;
	code: string;
	details?: string;
}

/**
 * CORS headers for browser clients
 */
const CORS_HEADERS = {
	"Access-Control-Allow-Origin": "*",
	"Access-Control-Allow-Methods": "GET, POST, OPTIONS",
	"Access-Control-Allow-Headers": "Content-Type, Authorization",
	"Access-Control-Max-Age": "86400",
};

/**
 * Create JSON response with proper headers
 */
function createJsonResponse<T>(data: T, status: number = 200, additionalHeaders: HeadersInit = {}): Response {
	return new Response(JSON.stringify(data), {
		status,
		headers: {
			"Content-Type": "application/json; charset=utf-8",
			...CORS_HEADERS,
			...additionalHeaders,
		},
	});
}

/**
 * Create error response
 */
function createErrorResponse(error: string, code: string, status: number = 400, details?: string): Response {
	const errorResponse: ErrorResponse = {
		error,
		code,
		...(details && { details }),
	};
	return createJsonResponse(errorResponse, status);
}

/**
 * Handle CORS preflight requests
 */
function handleCors(request: Request): Response | null {
	if (request.method === "OPTIONS") {
		return new Response(null, {
			status: 204,
			headers: CORS_HEADERS,
		});
	}
	return null;
}

/**
 * Extract file type from Content-Type header or filename
 */
function getFileType(filename: string): string {
	const ext = filename.split(".").pop()?.toLowerCase() || "";
	const typeMap: Record<string, string> = {
		pdf: "pdf",
		doc: "docx",
		docx: "docx",
		xls: "xlsx",
		xlsx: "xlsx",
		ppt: "pptx",
		pptx: "pptx",
		txt: "text",
		html: "html",
		htm: "html",
		jpg: "image",
		jpeg: "image",
		png: "image",
		gif: "image",
		webp: "image",
	};
	return typeMap[ext] || "unknown";
}

/**
 * Handle file extraction endpoint
 */
async function handleExtract(request: Request): Promise<Response> {
	try {
		// Parse form data
		const formData = await request.formData();
		const fileData = formData.get("file");

		if (!fileData || typeof fileData === "string") {
			return createErrorResponse(
				"No file provided",
				"MISSING_FILE",
				400,
				'Please provide a file in the "file" form field',
			);
		}
		const file: File = fileData;

		// Validate file size (100MB limit)
		const MAX_FILE_SIZE = 100 * 1024 * 1024;
		if (file.size > MAX_FILE_SIZE) {
			return createErrorResponse(
				"File too large",
				"FILE_TOO_LARGE",
				413,
				`File size must be less than 100MB, got ${(file.size / 1024 / 1024).toFixed(2)}MB`,
			);
		}

		// Get file type
		const fileType = getFileType(file.name);
		if (fileType === "unknown") {
			return createErrorResponse(
				"Unsupported file type",
				"UNSUPPORTED_FILE_TYPE",
				415,
				`File extension not recognized: ${file.name}`,
			);
		}

		// Convert file to ArrayBuffer
		const arrayBuffer = await file.arrayBuffer();
		const uint8Array = new Uint8Array(arrayBuffer);

		// Initialize WASM module and extract data
		await initWasm();
		const extractedData = await extractBytes(uint8Array, `application/${fileType}`);

		// Prepare response
		const response: ExtractedData = {
			text: extractedData.content || "",
			...(extractedData.metadata && { metadata: extractedData.metadata }),
			...(extractedData.tables && extractedData.tables.length > 0 && { tables: extractedData.tables }),
		};

		return createJsonResponse(response, 200, {
			"X-Processing-Time": `${Date.now()}ms`,
			"X-File-Type": fileType,
			"X-File-Name": file.name,
		});
	} catch (error) {
		console.error("Extract error:", error);

		const errorMessage = error instanceof Error ? error.message : "Unknown error";
		const isTimeout = errorMessage.includes("timeout");
		const isMemory = errorMessage.includes("memory");

		if (isTimeout) {
			return createErrorResponse(
				"Processing timeout",
				"PROCESSING_TIMEOUT",
				504,
				"File processing took too long. Try with a smaller file.",
			);
		}

		if (isMemory) {
			return createErrorResponse(
				"Insufficient memory",
				"INSUFFICIENT_MEMORY",
				507,
				"Not enough memory to process this file",
			);
		}

		return createErrorResponse("Processing failed", "PROCESSING_ERROR", 500, errorMessage);
	}
}

/**
 * Handle health check endpoint
 */
function handleHealth(): Response {
	const healthResponse: HealthResponse = {
		status: "healthy",
		timestamp: new Date().toISOString(),
		version: "1.0.0",
	};
	return createJsonResponse(healthResponse);
}

/**
 * Handle 404 Not Found
 */
function handleNotFound(): Response {
	return createErrorResponse("Not found", "NOT_FOUND", 404, "The requested endpoint does not exist");
}

/**
 * Main request handler
 */
export default {
	async fetch(request: Request): Promise<Response> {
		const url = new URL(request.url);
		const path = url.pathname;
		const method = request.method;

		// Handle CORS preflight
		const corsResponse = handleCors(request);
		if (corsResponse) {
			return corsResponse;
		}

		// Health check endpoint
		if (path === "/health" && method === "GET") {
			return handleHealth();
		}

		// File extraction endpoint
		if (path === "/extract" && method === "POST") {
			return await handleExtract(request);
		}

		// Handle API documentation
		if (path === "/" && method === "GET") {
			const docs = {
				name: "Kreuzberg WASM API",
				version: "1.0.0",
				description: "Document intelligence API powered by Kreuzberg and Cloudflare Workers",
				endpoints: {
					extract: {
						method: "POST",
						path: "/extract",
						description: "Extract text and data from documents",
						accepts: "multipart/form-data",
						parameters: {
							file: {
								type: "File",
								description: "Document file (PDF, DOCX, XLSX, PPTX, HTML, Image)",
								required: true,
							},
						},
						example: 'curl -X POST -F "file=@document.pdf" https://api.example.com/extract',
					},
					health: {
						method: "GET",
						path: "/health",
						description: "Health check endpoint",
						example: "curl https://api.example.com/health",
					},
				},
				supportedFormats: ["pdf", "docx", "xlsx", "pptx", "html", "image"],
			};
			return createJsonResponse(docs);
		}

		// 404 for unknown routes
		return handleNotFound();
	},
} satisfies ExportedHandler<Record<string, never>>;
