import { defineConfig } from "vitest/config";

export default defineConfig({
	test: {
		globals: true,
		environment: "node",
		pool: "threads",
		poolOptions: {
			threads: {
				singleThread: true,
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
				"**/types.ts",
				"**/cli.ts",
				"tests/**/helpers/**",
				"tests/binding/helpers/**",
			],
			thresholds: {
				lines: 85,
				functions: 90,
				branches: 70,
				statements: 86,
			},
		},
		testTimeout: 30000,
		hookTimeout: 10000,
	},
});
