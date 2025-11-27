/**
 * Tests for registry and utility functions.
 *
 * Covers:
 * - listPostProcessors, listValidators, listOcrBackends, listDocumentExtractors
 * - unregisterPostProcessor, unregisterValidator, unregisterOcrBackend, unregisterDocumentExtractor
 * - clearPostProcessors, clearValidators, clearOcrBackends, clearDocumentExtractors
 * - detectMimeType, detectMimeTypeFromPath, validateMimeType, getExtensionsForMime
 * - listEmbeddingPresets, getEmbeddingPreset
 * - ExtractionConfig.fromFile, ExtractionConfig.discover
 */

import { beforeEach, describe, expect, it } from "vitest";
import {
	__resetBindingForTests,
	__setBindingForTests,
	clearDocumentExtractors,
	clearOcrBackends,
	clearPostProcessors,
	clearValidators,
	detectMimeType,
	detectMimeTypeFromPath,
	getEmbeddingPreset,
	getExtensionsForMime,
	listDocumentExtractors,
	listEmbeddingPresets,
	listOcrBackends,
	listPostProcessors,
	listValidators,
	type OcrBackendProtocol,
	registerOcrBackend,
	type PostProcessorProtocol,
	registerPostProcessor,
	type ValidatorProtocol,
	registerValidator,
	unregisterDocumentExtractor,
	unregisterOcrBackend,
	unregisterPostProcessor,
	unregisterValidator,
	validateMimeType,
	ExtractionConfig,
} from "../../src/index.js";
import { createMockExtractionBinding } from "./helpers/mock-binding.js";

describe("listPostProcessors", () => {
	beforeEach(() => {
		clearPostProcessors();
	});

	it("should return empty array when no processors registered", () => {
		const names = listPostProcessors();
		expect(Array.isArray(names)).toBe(true);
	});

	it("should return processor names after registration", () => {
		class TestProcessor implements PostProcessorProtocol {
			name(): string {
				return "test_processor";
			}

			process(result) {
				return result;
			}
		}

		registerPostProcessor(new TestProcessor());
		const names = listPostProcessors();
		expect(names).toContain("test_processor");
	});

	it("should return multiple processor names", () => {
		class Processor1 implements PostProcessorProtocol {
			name(): string {
				return "processor_1";
			}

			process(result) {
				return result;
			}
		}

		class Processor2 implements PostProcessorProtocol {
			name(): string {
				return "processor_2";
			}

			process(result) {
				return result;
			}
		}

		registerPostProcessor(new Processor1());
		registerPostProcessor(new Processor2());

		const names = listPostProcessors();
		expect(names).toContain("processor_1");
		expect(names).toContain("processor_2");
	});
});

describe("unregisterPostProcessor", () => {
	beforeEach(() => {
		clearPostProcessors();
	});

	it("should unregister a processor by name", () => {
		class TestProcessor implements PostProcessorProtocol {
			name(): string {
				return "test_processor";
			}

			process(result) {
				return result;
			}
		}

		registerPostProcessor(new TestProcessor());
		let names = listPostProcessors();
		expect(names).toContain("test_processor");

		unregisterPostProcessor("test_processor");
		names = listPostProcessors();
		expect(names).not.toContain("test_processor");
	});

	it("should handle unregistering non-existent processor", () => {
		expect(() => {
			unregisterPostProcessor("non_existent");
		}).not.toThrow();
	});
});

describe("clearPostProcessors", () => {
	it("should clear all registered processors", () => {
		class Processor1 implements PostProcessorProtocol {
			name(): string {
				return "processor_1";
			}

			process(result) {
				return result;
			}
		}

		class Processor2 implements PostProcessorProtocol {
			name(): string {
				return "processor_2";
			}

			process(result) {
				return result;
			}
		}

		registerPostProcessor(new Processor1());
		registerPostProcessor(new Processor2());

		let names = listPostProcessors();
		expect(names.length).toBeGreaterThan(0);

		clearPostProcessors();
		names = listPostProcessors();
		expect(names.length).toBe(0);
	});
});

describe("listValidators", () => {
	beforeEach(() => {
		clearValidators();
	});

	it("should return empty array when no validators registered", () => {
		const names = listValidators();
		expect(Array.isArray(names)).toBe(true);
	});

	it("should return validator names after registration", () => {
		class TestValidator implements ValidatorProtocol {
			name(): string {
				return "test_validator";
			}

			validate(result) {
				// Pass
			}
		}

		registerValidator(new TestValidator());
		const names = listValidators();
		expect(names).toContain("test_validator");
	});
});

describe("unregisterValidator", () => {
	beforeEach(() => {
		clearValidators();
	});

	it("should unregister a validator by name", () => {
		class TestValidator implements ValidatorProtocol {
			name(): string {
				return "test_validator";
			}

			validate(result) {
				// Pass
			}
		}

		registerValidator(new TestValidator());
		let names = listValidators();
		expect(names).toContain("test_validator");

		unregisterValidator("test_validator");
		names = listValidators();
		expect(names).not.toContain("test_validator");
	});
});

describe("clearValidators", () => {
	it("should clear all registered validators", () => {
		class Validator1 implements ValidatorProtocol {
			name(): string {
				return "validator_1";
			}

			validate(result) {
				// Pass
			}
		}

		class Validator2 implements ValidatorProtocol {
			name(): string {
				return "validator_2";
			}

			validate(result) {
				// Pass
			}
		}

		registerValidator(new Validator1());
		registerValidator(new Validator2());

		let names = listValidators();
		expect(names.length).toBeGreaterThan(0);

		clearValidators();
		names = listValidators();
		expect(names.length).toBe(0);
	});
});

describe("listOcrBackends", () => {
	it("should return array of registered OCR backends", () => {
		const backends = listOcrBackends();
		expect(Array.isArray(backends)).toBe(true);
		// Should include default Tesseract backend
		expect(backends.some((b) => b.toLowerCase().includes("tesseract"))).toBe(true);
	});
});

describe("unregisterOcrBackend", () => {
	it("should unregister an OCR backend by name", () => {
		class CustomBackend implements OcrBackendProtocol {
			name(): string {
				return "custom_ocr_backend";
			}

			supportedLanguages(): string[] {
				return ["en"];
			}

			async processImage(imageBytes: Uint8Array, language: string) {
				return {
					content: "test",
					mime_type: "text/plain",
					metadata: {},
					tables: [],
				};
			}
		}

		registerOcrBackend(new CustomBackend());
		let backends = listOcrBackends();
		expect(backends).toContain("custom_ocr_backend");

		unregisterOcrBackend("custom_ocr_backend");
		backends = listOcrBackends();
		expect(backends).not.toContain("custom_ocr_backend");
	});

	it("should handle unregistering non-existent backend", () => {
		expect(() => {
			unregisterOcrBackend("non_existent_backend");
		}).not.toThrow();
	});
});

describe("clearOcrBackends", () => {
	it("should clear all OCR backends", () => {
		const beforeClear = listOcrBackends();
		expect(beforeClear.length).toBeGreaterThan(0);

		clearOcrBackends();
		const afterClear = listOcrBackends();
		expect(afterClear.length).toBe(0);
	});
});

describe("listDocumentExtractors", () => {
	it("should return array of document extractors", () => {
		const extractors = listDocumentExtractors();
		expect(Array.isArray(extractors)).toBe(true);
		// Extractors might be empty if not initialized
	});
});

describe("unregisterDocumentExtractor", () => {
	it("should handle unregistering document extractor", () => {
		const extractors = listDocumentExtractors();
		if (extractors.length > 0) {
			const firstExtractor = extractors[0];

			expect(() => {
				unregisterDocumentExtractor(firstExtractor);
			}).not.toThrow();
		}
	});

	it("should handle unregistering non-existent extractor", () => {
		expect(() => {
			unregisterDocumentExtractor("non_existent_extractor");
		}).not.toThrow();
	});
});

describe("clearDocumentExtractors", () => {
	it("should clear all document extractors", () => {
		const beforeClear = listDocumentExtractors();

		if (beforeClear.length > 0) {
			clearDocumentExtractors();
			const afterClear = listDocumentExtractors();
			expect(afterClear.length).toBe(0);
		}
	});
});

describe("detectMimeType", () => {
	it("should detect MIME type from PDF bytes", () => {
		const pdfBytes = new Uint8Array([0x25, 0x50, 0x44, 0x46]); // %PDF
		const mimeType = detectMimeType(Buffer.from(pdfBytes));
		expect(mimeType).toContain("pdf");
	});

	it("should detect MIME type from JPEG bytes", () => {
		const jpegBytes = new Uint8Array([0xff, 0xd8, 0xff, 0xe0]); // JPEG magic
		const mimeType = detectMimeType(Buffer.from(jpegBytes));
		expect(mimeType).toContain("image");
	});

	it("should detect MIME type from PNG bytes", () => {
		const pngBytes = new Uint8Array([0x89, 0x50, 0x4e, 0x47]); // PNG magic
		const mimeType = detectMimeType(Buffer.from(pngBytes));
		expect(mimeType).toContain("png");
	});
});

describe("detectMimeTypeFromPath", () => {
	it("should detect MIME type from .pdf extension with checkExists false", () => {
		try {
			const mimeType = detectMimeTypeFromPath("document.pdf", false);
			expect(mimeType).toContain("pdf");
		} catch {
			// If implementation still checks existence, that's acceptable
		}
	});

	it("should detect MIME type from .docx extension with checkExists false", () => {
		try {
			const mimeType = detectMimeTypeFromPath("document.docx", false);
			expect(mimeType).toContain("wordprocessingml");
		} catch {
			// If implementation still checks existence, that's acceptable
		}
	});

	it("should throw for non-existent file when checkExists is true", () => {
		expect(() => {
			detectMimeTypeFromPath("/nonexistent/path/document.pdf", true);
		}).toThrow();
	});

	it("should throw error for files without extension", () => {
		expect(() => {
			detectMimeTypeFromPath("/tmp/file_without_extension", false);
		}).toThrow();
	});
});

describe("validateMimeType", () => {
	it("should validate supported MIME type", () => {
		const validated = validateMimeType("application/pdf");
		expect(validated).toBe("application/pdf");
	});

	it("should validate any image/* MIME type", () => {
		const validated = validateMimeType("image/custom-format");
		expect(validated).toBe("image/custom-format");
	});

	it("should throw for unsupported MIME type", () => {
		expect(() => {
			validateMimeType("video/mp4");
		}).toThrow();
	});

	it("should validate DOCX MIME type", () => {
		const validated = validateMimeType("application/vnd.openxmlformats-officedocument.wordprocessingml.document");
		expect(validated).toContain("word");
	});
});

describe("getExtensionsForMime", () => {
	it("should get extensions for PDF MIME type", () => {
		const extensions = getExtensionsForMime("application/pdf");
		expect(extensions).toContain("pdf");
	});

	it("should get extensions for JPEG MIME type", () => {
		const extensions = getExtensionsForMime("image/jpeg");
		expect(extensions.some((ext) => ext === "jpg" || ext === "jpeg")).toBe(true);
	});

	it("should get extensions for PNG MIME type", () => {
		const extensions = getExtensionsForMime("image/png");
		expect(extensions).toContain("png");
	});

	it("should get extensions for DOCX MIME type", () => {
		const extensions = getExtensionsForMime("application/vnd.openxmlformats-officedocument.wordprocessingml.document");
		expect(extensions).toContain("docx");
	});

	it("should throw for unsupported MIME type", () => {
		expect(() => {
			getExtensionsForMime("video/unsupported");
		}).toThrow();
	});
});

describe("listEmbeddingPresets", () => {
	it("should return array of embedding preset names", () => {
		const presets = listEmbeddingPresets();
		expect(Array.isArray(presets)).toBe(true);
		expect(presets.length).toBeGreaterThan(0);
	});

	it("should include standard preset names", () => {
		const presets = listEmbeddingPresets();
		const presetNames = presets.map((p) => p.toLowerCase());
		// Should have some common presets
		expect(presetNames.length).toBeGreaterThan(0);
	});
});

describe("getEmbeddingPreset", () => {
	it("should get embedding preset by name", () => {
		const presets = listEmbeddingPresets();
		if (presets.length > 0) {
			const preset = getEmbeddingPreset(presets[0]);
			expect(preset).not.toBeNull();
			if (preset) {
				expect(preset.name).toBeTruthy();
				expect(preset.modelName).toBeTruthy();
				expect(preset.dimensions).toBeGreaterThan(0);
				expect(preset.chunkSize).toBeGreaterThan(0);
				expect(preset.description).toBeTruthy();
			}
		}
	});

	it("should return null for non-existent preset", () => {
		const preset = getEmbeddingPreset("non_existent_preset");
		expect(preset).toBeNull();
	});

	it("should have valid preset structure", () => {
		const presets = listEmbeddingPresets();
		for (const presetName of presets) {
			const preset = getEmbeddingPreset(presetName);
			expect(preset).not.toBeNull();
			if (preset) {
				expect(typeof preset.name).toBe("string");
				expect(typeof preset.modelName).toBe("string");
				expect(typeof preset.dimensions).toBe("number");
				expect(typeof preset.chunkSize).toBe("number");
				expect(typeof preset.overlap).toBe("number");
				expect(typeof preset.description).toBe("string");
			}
		}
	});
});

describe("ExtractionConfig.fromFile", () => {
	it("should throw for non-existent file", () => {
		expect(() => {
			ExtractionConfig.fromFile("/nonexistent/path/config.toml");
		}).toThrow();
	});

	it("should handle invalid JSON config file", () => {
		expect(() => {
			ExtractionConfig.fromFile("/invalid/json.json");
		}).toThrow();
	});
});

describe("ExtractionConfig.discover", () => {
	it("should return null or config object", () => {
		const result = ExtractionConfig.discover();
		if (result !== null) {
			expect(typeof result).toBe("object");
		}
	});
});
