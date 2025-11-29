import { defineConfig } from "vitest/config";
import path from "path";

export default defineConfig({
	resolve: {
		alias: {
			kreuzberg: path.resolve(__dirname, "../../packages/typescript/dist/index.mjs"),
		},
	},
	test: {
		globals: true,
		environment: "node",
		// Use single-threaded execution to avoid process.chdir() issues in workers
		singleThread: true,
		testTimeout: 60000,
		hookTimeout: 10000,
	},
});
