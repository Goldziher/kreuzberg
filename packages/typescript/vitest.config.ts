import { defineConfig } from "vitest/config";

export default defineConfig({
	test: {
		globals: true,
		environment: "node",
		pool: "threads",
		poolOptions: {
			threads: {
				singleThread: true, // Run in single thread - NAPI-RS has concurrency issues
			},
		},
		coverage: {
			provider: "v8",
			reporter: ["text", "json", "html", "lcov"],
			exclude: [
				"node_modules",
				"dist",
				"*.config.*",
				"**/*.spec.ts",
				"**/types.ts", // Type definitions only
				"**/cli.ts", // CLI wrapper - thin subprocess proxy
				"tests/helpers/**", // Test infrastructure
			],
			thresholds: {
				lines: 90,
				functions: 95,
				branches: 85,
				statements: 90,
			},
		},
		testTimeout: 30000, // 30s for OCR tests
		hookTimeout: 10000,
	},
});
