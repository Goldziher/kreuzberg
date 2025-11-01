import { defineConfig } from "vitest/config";

export default defineConfig({
	test: {
		globals: true,
		environment: "node",
		pool: "forks",
		poolOptions: {
			forks: {
				singleFork: true,
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
				"tests/unit/helpers/**",
			],
			thresholds: {
				lines: 88,
				functions: 94,
				branches: 73,
				statements: 88,
			},
		},
		testTimeout: 30000,
		hookTimeout: 10000,
	},
});
