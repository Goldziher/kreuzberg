import { describe, expect, it } from "vitest";
import { extractBytesSync } from "../../src/index.js";
import type { ExtractionConfig, JsChunkingConfig, JsEmbeddingConfig, JsEmbeddingModelType } from "../../src/types.js";

describe("Embedding Configuration", () => {
	describe("EmbeddingModelType", () => {
		it("should accept preset model configuration", () => {
			const modelType: JsEmbeddingModelType = {
				modelType: "preset",
				value: "balanced",
			};

			const embeddingConfig: JsEmbeddingConfig = {
				model: modelType,
				normalize: true,
				batchSize: 32,
			};

			expect(embeddingConfig.model?.modelType).toBe("preset");
			expect(embeddingConfig.model?.value).toBe("balanced");
		});

		it("should accept all preset model types", () => {
			const presets = ["fast", "balanced", "quality", "multilingual"];

			for (const preset of presets) {
				const modelType: JsEmbeddingModelType = {
					modelType: "preset",
					value: preset,
				};

				expect(modelType.modelType).toBe("preset");
				expect(modelType.value).toBe(preset);
			}
		});

		it("should accept fastembed model configuration", () => {
			const modelType: JsEmbeddingModelType = {
				modelType: "fastembed",
				value: "BGEBaseENV15",
				dimensions: 768,
			};

			const embeddingConfig: JsEmbeddingConfig = {
				model: modelType,
				normalize: true,
			};

			expect(embeddingConfig.model?.modelType).toBe("fastembed");
			expect(embeddingConfig.model?.value).toBe("BGEBaseENV15");
			expect(embeddingConfig.model?.dimensions).toBe(768);
		});

		it("should accept custom model configuration", () => {
			const modelType: JsEmbeddingModelType = {
				modelType: "custom",
				value: "my-custom-model",
				dimensions: 512,
			};

			const embeddingConfig: JsEmbeddingConfig = {
				model: modelType,
				normalize: false,
				batchSize: 16,
			};

			expect(embeddingConfig.model?.modelType).toBe("custom");
			expect(embeddingConfig.model?.value).toBe("my-custom-model");
			expect(embeddingConfig.model?.dimensions).toBe(512);
		});
	});

	describe("EmbeddingConfig", () => {
		it("should accept minimal configuration", () => {
			const config: JsEmbeddingConfig = {
				model: {
					modelType: "preset",
					value: "fast",
				},
			};

			expect(config.model).toBeDefined();
			expect(config.normalize).toBeUndefined();
			expect(config.batchSize).toBeUndefined();
		});

		it("should accept full configuration", () => {
			const config: JsEmbeddingConfig = {
				model: {
					modelType: "preset",
					value: "balanced",
				},
				normalize: true,
				batchSize: 64,
				showDownloadProgress: true,
				cacheDir: "/tmp/models",
			};

			expect(config.model?.modelType).toBe("preset");
			expect(config.normalize).toBe(true);
			expect(config.batchSize).toBe(64);
			expect(config.showDownloadProgress).toBe(true);
			expect(config.cacheDir).toBe("/tmp/models");
		});

		it("should accept normalization flag", () => {
			const configNormalized: JsEmbeddingConfig = {
				model: { modelType: "preset", value: "fast" },
				normalize: true,
			};

			const configNotNormalized: JsEmbeddingConfig = {
				model: { modelType: "preset", value: "fast" },
				normalize: false,
			};

			expect(configNormalized.normalize).toBe(true);
			expect(configNotNormalized.normalize).toBe(false);
		});

		it("should accept various batch sizes", () => {
			const batchSizes = [1, 16, 32, 64, 128];

			for (const batchSize of batchSizes) {
				const config: JsEmbeddingConfig = {
					model: { modelType: "preset", value: "fast" },
					batchSize,
				};

				expect(config.batchSize).toBe(batchSize);
			}
		});

		it("should accept custom cache directory", () => {
			const config: JsEmbeddingConfig = {
				model: { modelType: "preset", value: "balanced" },
				cacheDir: "/custom/cache/path",
			};

			expect(config.cacheDir).toBe("/custom/cache/path");
		});
	});

	describe("ChunkingConfig with embeddings", () => {
		it("should accept chunking config with embedding", () => {
			const chunkingConfig: JsChunkingConfig = {
				maxChars: 1000,
				maxOverlap: 200,
				embedding: {
					model: { modelType: "preset", value: "fast" },
					normalize: true,
					batchSize: 32,
				},
			};

			expect(chunkingConfig.maxChars).toBe(1000);
			expect(chunkingConfig.maxOverlap).toBe(200);
			expect(chunkingConfig.embedding).toBeDefined();
			expect(chunkingConfig.embedding?.model?.value).toBe("fast");
		});

		it("should accept chunking config with preset", () => {
			const chunkingConfig: JsChunkingConfig = {
				preset: "balanced",
			};

			expect(chunkingConfig.preset).toBe("balanced");
		});

		it("should accept chunking config with both preset and embedding", () => {
			const chunkingConfig: JsChunkingConfig = {
				preset: "quality",
				embedding: {
					model: { modelType: "preset", value: "quality" },
					normalize: true,
				},
			};

			expect(chunkingConfig.preset).toBe("quality");
			expect(chunkingConfig.embedding).toBeDefined();
		});

		it("should accept chunking config without embedding", () => {
			const chunkingConfig: JsChunkingConfig = {
				maxChars: 500,
				maxOverlap: 50,
			};

			expect(chunkingConfig.maxChars).toBe(500);
			expect(chunkingConfig.embedding).toBeUndefined();
		});
	});

	describe("Integration with ExtractionConfig", () => {
		it("should accept extraction config with chunking and embeddings", () => {
			const config: ExtractionConfig = {
				chunking: {
					maxChars: 1000,
					maxOverlap: 200,
					embedding: {
						model: { modelType: "preset", value: "fast" },
						normalize: true,
						batchSize: 32,
					},
				},
			};

			expect(config.chunking).toBeDefined();
			expect(config.chunking?.embedding).toBeDefined();
		});

		it("should create extraction config with all preset types", () => {
			const presets = ["fast", "balanced", "quality", "multilingual"];

			for (const preset of presets) {
				const config: ExtractionConfig = {
					chunking: {
						maxChars: 1000,
						embedding: {
							model: { modelType: "preset", value: preset },
							normalize: true,
						},
					},
				};

				expect(config.chunking?.embedding?.model?.value).toBe(preset);
			}
		});

		it("should create extraction config with chunking preset", () => {
			const config: ExtractionConfig = {
				chunking: {
					preset: "balanced",
				},
			};

			expect(config.chunking?.preset).toBe("balanced");
		});
	});
});

describe("Embedding Generation Integration", () => {
	const testText = "This is a test document for embedding generation. ".repeat(20);
	const textBytes = new TextEncoder().encode(testText);

	it("should extract text with embedding configuration", () => {
		const config: ExtractionConfig = {
			chunking: {
				maxChars: 100,
				maxOverlap: 20,
				embedding: {
					model: { modelType: "preset", value: "fast" },
					normalize: true,
					batchSize: 32,
				},
			},
		};

		const result = extractBytesSync(textBytes, "text/plain", config);

		expect(result.content).toBeTruthy();
		expect(result.chunks).toBeDefined();
	});

	it("should handle different preset configurations", () => {
		const presets: Array<"fast" | "balanced" | "quality" | "multilingual"> = [
			"fast",
			"balanced",
			"quality",
			"multilingual",
		];

		for (const preset of presets) {
			const config: ExtractionConfig = {
				chunking: {
					maxChars: 100,
					maxOverlap: 20,
					embedding: {
						model: { modelType: "preset", value: preset },
						normalize: true,
					},
				},
			};

			expect(config.chunking?.embedding?.model?.value).toBe(preset);
		}
	});

	it("should handle fastembed model configuration", () => {
		const config: ExtractionConfig = {
			chunking: {
				maxChars: 100,
				maxOverlap: 20,
				embedding: {
					model: {
						modelType: "fastembed",
						value: "BGEBaseENV15",
						dimensions: 768,
					},
					normalize: true,
					batchSize: 32,
				},
			},
		};

		expect(config.chunking?.embedding?.model?.modelType).toBe("fastembed");
		expect(config.chunking?.embedding?.model?.value).toBe("BGEBaseENV15");
		expect(config.chunking?.embedding?.model?.dimensions).toBe(768);
	});

	it("should handle normalization settings", () => {
		const configNormalized: ExtractionConfig = {
			chunking: {
				maxChars: 100,
				maxOverlap: 20,
				embedding: {
					model: { modelType: "preset", value: "fast" },
					normalize: true,
				},
			},
		};

		const configNotNormalized: ExtractionConfig = {
			chunking: {
				maxChars: 100,
				maxOverlap: 20,
				embedding: {
					model: { modelType: "preset", value: "fast" },
					normalize: false,
				},
			},
		};

		expect(configNormalized.chunking?.embedding?.normalize).toBe(true);
		expect(configNotNormalized.chunking?.embedding?.normalize).toBe(false);
	});

	it("should handle various batch sizes", () => {
		const batchSizes = [16, 32, 64, 128];

		for (const batchSize of batchSizes) {
			const config: ExtractionConfig = {
				chunking: {
					maxChars: 100,
					maxOverlap: 20,
					embedding: {
						model: { modelType: "preset", value: "fast" },
						batchSize,
					},
				},
			};

			expect(config.chunking?.embedding?.batchSize).toBe(batchSize);
		}
	});

	it("should handle extraction without embeddings", () => {
		const config: ExtractionConfig = {
			chunking: {
				maxChars: 100,
				maxOverlap: 20,
			},
		};

		const result = extractBytesSync(textBytes, "text/plain", config);

		expect(result.content).toBeTruthy();
		expect(result.chunks).toBeDefined();
	});

	it("should handle chunking preset without explicit embedding", () => {
		const config: ExtractionConfig = {
			chunking: {
				preset: "balanced",
			},
		};

		const result = extractBytesSync(textBytes, "text/plain", config);

		expect(result.content).toBeTruthy();
	});

	it("should handle custom cache directory", () => {
		const config: ExtractionConfig = {
			chunking: {
				maxChars: 100,
				maxOverlap: 20,
				embedding: {
					model: { modelType: "preset", value: "fast" },
					cacheDir: "/tmp/kreuzberg_cache",
				},
			},
		};

		expect(config.chunking?.embedding?.cacheDir).toBe("/tmp/kreuzberg_cache");
	});

	it("should handle show download progress flag", () => {
		const config: ExtractionConfig = {
			chunking: {
				maxChars: 100,
				maxOverlap: 20,
				embedding: {
					model: { modelType: "preset", value: "fast" },
					showDownloadProgress: true,
				},
			},
		};

		expect(config.chunking?.embedding?.showDownloadProgress).toBe(true);
	});
});

describe("Embedding Configuration Edge Cases", () => {
	it("should handle minimum configuration", () => {
		const config: ExtractionConfig = {
			chunking: {
				embedding: {
					model: { modelType: "preset", value: "fast" },
				},
			},
		};

		expect(config.chunking?.embedding?.model).toBeDefined();
	});

	it("should handle all optional fields set", () => {
		const config: ExtractionConfig = {
			chunking: {
				maxChars: 1000,
				maxOverlap: 200,
				preset: "balanced",
				embedding: {
					model: { modelType: "preset", value: "quality" },
					normalize: true,
					batchSize: 64,
					showDownloadProgress: false,
					cacheDir: "/custom/path",
				},
			},
		};

		expect(config.chunking?.maxChars).toBe(1000);
		expect(config.chunking?.maxOverlap).toBe(200);
		expect(config.chunking?.preset).toBe("balanced");
		expect(config.chunking?.embedding?.model?.value).toBe("quality");
		expect(config.chunking?.embedding?.normalize).toBe(true);
		expect(config.chunking?.embedding?.batchSize).toBe(64);
		expect(config.chunking?.embedding?.showDownloadProgress).toBe(false);
		expect(config.chunking?.embedding?.cacheDir).toBe("/custom/path");
	});

	it("should handle different chunk sizes with embeddings", () => {
		const chunkSizes = [50, 100, 500, 1000, 2000];

		for (const maxChars of chunkSizes) {
			const maxOverlap = Math.min(Math.floor(maxChars * 0.2), maxChars - 1);
			const config: ExtractionConfig = {
				chunking: {
					maxChars,
					maxOverlap,
					embedding: {
						model: { modelType: "preset", value: "fast" },
					},
				},
			};

			expect(config.chunking?.maxChars).toBe(maxChars);
		}
	});
});
