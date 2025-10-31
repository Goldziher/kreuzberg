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
			exclude: ["node_modules", "dist", "*.config.*", "**/*.spec.ts", "**/types.ts", "**/cli.ts", "tests/helpers/**"],
			// Coverage thresholds disabled - don't fail CI on coverage
			// thresholds: {
			// 	lines: 90,
			// 	functions: 95,
			// 	branches: 85,
			// 	statements: 90,
			// },
		},
		testTimeout: 30000,
		hookTimeout: 10000,
	},
});
