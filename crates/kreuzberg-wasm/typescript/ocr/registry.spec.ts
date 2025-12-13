import type { OcrBackendProtocol } from "@kreuzberg/core";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
	clearOcrBackends,
	getOcrBackend,
	listOcrBackends,
	registerOcrBackend,
	unregisterOcrBackend,
} from "./registry.js";

describe("OCR Registry", () => {
	beforeEach(async () => {
		await clearOcrBackends();
	});

	const createMockBackend = (name: string): OcrBackendProtocol => ({
		name: () => name,
		supportedLanguages: () => ["eng", "deu", "fra"],
		processImage: vi.fn(async () => ({ text: "test" })),
		shutdown: vi.fn(async () => undefined),
	});

	describe("registerOcrBackend", () => {
		it("should register a valid backend", () => {
			const backend = createMockBackend("test-backend");
			registerOcrBackend(backend);

			const registered = getOcrBackend("test-backend");
			expect(registered).toBe(backend);
		});

		it("should throw if backend is null", () => {
			expect(() => registerOcrBackend(null as any)).toThrow("Backend cannot be null or undefined");
		});

		it("should throw if backend is undefined", () => {
			expect(() => registerOcrBackend(undefined as any)).toThrow("Backend cannot be null or undefined");
		});

		it("should throw if backend missing name method", () => {
			const invalid = {
				supportedLanguages: () => [],
				processImage: async () => ({}),
			};

			expect(() => registerOcrBackend(invalid as any)).toThrow("must implement name() method");
		});

		it("should throw if backend missing supportedLanguages method", () => {
			const invalid = {
				name: () => "test",
				processImage: async () => ({}),
			};

			expect(() => registerOcrBackend(invalid as any)).toThrow("must implement supportedLanguages() method");
		});

		it("should throw if backend missing processImage method", () => {
			const invalid = {
				name: () => "test",
				supportedLanguages: () => [],
			};

			expect(() => registerOcrBackend(invalid as any)).toThrow("must implement processImage() method");
		});

		it("should throw if backend name is empty string", () => {
			const backend = {
				name: () => "",
				supportedLanguages: () => [],
				processImage: async () => ({}),
			};

			expect(() => registerOcrBackend(backend as any)).toThrow("Backend name must be a non-empty string");
		});

		it("should throw if backend name is not a string", () => {
			const backend = {
				name: () => 123,
				supportedLanguages: () => [],
				processImage: async () => ({}),
			};

			expect(() => registerOcrBackend(backend as any)).toThrow("Backend name must be a non-empty string");
		});

		it("should allow overwriting existing backend", () => {
			const backend1 = createMockBackend("test");
			const backend2 = createMockBackend("test");

			registerOcrBackend(backend1);
			expect(() => registerOcrBackend(backend2)).not.toThrow();

			const registered = getOcrBackend("test");
			expect(registered).toBe(backend2);
		});

		it("should warn when overwriting backend", () => {
			const warnSpy = vi.spyOn(console, "warn");
			const backend1 = createMockBackend("test");
			const backend2 = createMockBackend("test");

			registerOcrBackend(backend1);
			registerOcrBackend(backend2);

			expect(warnSpy).toHaveBeenCalledWith(expect.stringContaining("already registered"));

			warnSpy.mockRestore();
		});

		it("should register multiple different backends", () => {
			const backend1 = createMockBackend("backend1");
			const backend2 = createMockBackend("backend2");

			registerOcrBackend(backend1);
			registerOcrBackend(backend2);

			expect(getOcrBackend("backend1")).toBe(backend1);
			expect(getOcrBackend("backend2")).toBe(backend2);
		});
	});

	describe("getOcrBackend", () => {
		it("should return undefined for unregistered backend", () => {
			const backend = getOcrBackend("nonexistent");
			expect(backend).toBeUndefined();
		});

		it("should return registered backend", () => {
			const original = createMockBackend("test");
			registerOcrBackend(original);

			const retrieved = getOcrBackend("test");
			expect(retrieved).toBe(original);
		});

		it("should return correct backend when multiple registered", () => {
			const backend1 = createMockBackend("backend1");
			const backend2 = createMockBackend("backend2");

			registerOcrBackend(backend1);
			registerOcrBackend(backend2);

			expect(getOcrBackend("backend1")).toBe(backend1);
			expect(getOcrBackend("backend2")).toBe(backend2);
			expect(getOcrBackend("backend3")).toBeUndefined();
		});

		it("should be case-sensitive", () => {
			const backend = createMockBackend("Test");
			registerOcrBackend(backend);

			expect(getOcrBackend("test")).toBeUndefined();
			expect(getOcrBackend("Test")).toBe(backend);
		});
	});

	describe("listOcrBackends", () => {
		it("should return empty array when no backends registered", () => {
			const backends = listOcrBackends();
			expect(backends).toEqual([]);
		});

		it("should return single backend name", () => {
			const backend = createMockBackend("tesseract");
			registerOcrBackend(backend);

			const backends = listOcrBackends();
			expect(backends).toContain("tesseract");
			expect(backends.length).toBe(1);
		});

		it("should return multiple backend names", () => {
			const backend1 = createMockBackend("backend1");
			const backend2 = createMockBackend("backend2");
			const backend3 = createMockBackend("backend3");

			registerOcrBackend(backend1);
			registerOcrBackend(backend2);
			registerOcrBackend(backend3);

			const backends = listOcrBackends();
			expect(backends).toHaveLength(3);
			expect(backends).toContain("backend1");
			expect(backends).toContain("backend2");
			expect(backends).toContain("backend3");
		});

		it("should return backends as array", () => {
			const backend = createMockBackend("test");
			registerOcrBackend(backend);

			const backends = listOcrBackends();
			expect(Array.isArray(backends)).toBe(true);
		});

		it("should not include unregistered backends", () => {
			const backend = createMockBackend("test");
			registerOcrBackend(backend);

			const backends = listOcrBackends();
			expect(backends).not.toContain("other");
		});
	});

	describe("unregisterOcrBackend", () => {
		it("should unregister a registered backend", async () => {
			const backend = createMockBackend("test");
			registerOcrBackend(backend);

			expect(getOcrBackend("test")).toBe(backend);

			await unregisterOcrBackend("test");

			expect(getOcrBackend("test")).toBeUndefined();
		});

		it("should throw if backend not found", async () => {
			await expect(unregisterOcrBackend("nonexistent")).rejects.toThrow('OCR backend "nonexistent" is not registered');
		});

		it("should call shutdown method if available", async () => {
			const backend = createMockBackend("test");
			const shutdownSpy = vi.spyOn(backend, "shutdown");

			registerOcrBackend(backend);
			await unregisterOcrBackend("test");

			expect(shutdownSpy).toHaveBeenCalled();
		});

		it("should not throw if shutdown fails", async () => {
			const backend = createMockBackend("test");
			backend.shutdown = vi.fn(async () => {
				throw new Error("Shutdown failed");
			});

			registerOcrBackend(backend);

			expect(async () => {
				await unregisterOcrBackend("test");
			}).not.toThrow();
		});

		it("should remove backend even if shutdown fails", async () => {
			const backend = createMockBackend("test");
			backend.shutdown = vi.fn(async () => {
				throw new Error("Shutdown error");
			});

			registerOcrBackend(backend);
			await unregisterOcrBackend("test");

			expect(getOcrBackend("test")).toBeUndefined();
		});

		it("should be case-sensitive", async () => {
			const backend = createMockBackend("Test");
			registerOcrBackend(backend);

			await expect(unregisterOcrBackend("test")).rejects.toThrow("is not registered");

			expect(getOcrBackend("Test")).toBe(backend);
		});

		it("should throw error that lists available backends", async () => {
			const backend = createMockBackend("available");
			registerOcrBackend(backend);

			try {
				await unregisterOcrBackend("nonexistent");
			} catch (error) {
				if (error instanceof Error) {
					expect(error.message).toContain("available");
				}
			}
		});
	});

	describe("clearOcrBackends", () => {
		it("should clear all backends", async () => {
			const backend1 = createMockBackend("backend1");
			const backend2 = createMockBackend("backend2");

			registerOcrBackend(backend1);
			registerOcrBackend(backend2);

			expect(listOcrBackends()).toHaveLength(2);

			await clearOcrBackends();

			expect(listOcrBackends()).toEqual([]);
		});

		it("should call shutdown on all backends", async () => {
			const backend1 = createMockBackend("backend1");
			const backend2 = createMockBackend("backend2");

			const shutdown1 = vi.spyOn(backend1, "shutdown");
			const shutdown2 = vi.spyOn(backend2, "shutdown");

			registerOcrBackend(backend1);
			registerOcrBackend(backend2);

			await clearOcrBackends();

			expect(shutdown1).toHaveBeenCalled();
			expect(shutdown2).toHaveBeenCalled();
		});

		it("should not throw if shutdown fails on any backend", async () => {
			const backend1 = createMockBackend("backend1");
			const backend2 = createMockBackend("backend2");

			backend1.shutdown = vi.fn(async () => {
				throw new Error("Error 1");
			});
			backend2.shutdown = vi.fn(async () => {
				throw new Error("Error 2");
			});

			registerOcrBackend(backend1);
			registerOcrBackend(backend2);

			expect(async () => {
				await clearOcrBackends();
			}).not.toThrow();
		});

		it("should clear even if shutdowns fail", async () => {
			const backend = createMockBackend("test");
			backend.shutdown = vi.fn(async () => {
				throw new Error("Shutdown error");
			});

			registerOcrBackend(backend);
			await clearOcrBackends();

			expect(listOcrBackends()).toEqual([]);
		});

		it("should work when no backends registered", async () => {
			expect(async () => {
				await clearOcrBackends();
			}).not.toThrow();

			expect(listOcrBackends()).toEqual([]);
		});
	});

	describe("Integration scenarios", () => {
		it("should support registering, listing, and unregistering", async () => {
			const backend1 = createMockBackend("ocr1");
			const backend2 = createMockBackend("ocr2");

			registerOcrBackend(backend1);
			registerOcrBackend(backend2);

			const backends = listOcrBackends();
			expect(backends).toContain("ocr1");
			expect(backends).toContain("ocr2");

			await unregisterOcrBackend("ocr1");

			const remaining = listOcrBackends();
			expect(remaining).not.toContain("ocr1");
			expect(remaining).toContain("ocr2");
		});

		it("should support re-registering after unregister", async () => {
			const backend = createMockBackend("test");

			registerOcrBackend(backend);
			await unregisterOcrBackend("test");

			registerOcrBackend(backend);

			expect(getOcrBackend("test")).toBe(backend);
		});

		it("should handle clear and re-register", async () => {
			const backend1 = createMockBackend("backend1");
			const backend2 = createMockBackend("backend2");

			registerOcrBackend(backend1);
			registerOcrBackend(backend2);

			await clearOcrBackends();
			expect(listOcrBackends()).toEqual([]);

			const newBackend = createMockBackend("backend3");
			registerOcrBackend(newBackend);

			expect(listOcrBackends()).toEqual(["backend3"]);
		});

		it("should maintain separate registry instances", () => {
			const backend = createMockBackend("test");
			registerOcrBackend(backend);

			const retrieved1 = getOcrBackend("test");
			const retrieved2 = getOcrBackend("test");

			expect(retrieved1).toBe(retrieved2);
			expect(retrieved1).toBe(backend);
		});
	});
});
