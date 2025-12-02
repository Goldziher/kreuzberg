import { defineConfig } from "vitest/config";

export default defineConfig({
	test: {
		globals: true,
		environment: "node",
		// Use single-threaded execution to avoid process.chdir() issues in workers
		singleThread: true,
		testTimeout: 60000,
		hookTimeout: 10000,
	},
});
