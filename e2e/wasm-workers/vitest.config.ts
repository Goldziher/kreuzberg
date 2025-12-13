import { defineConfig } from "vitest/config";
import miniflare from "miniflare/vitest";

export default defineConfig({
	plugins: [miniflare.getPlugin()],
	test: {
		globals: true,
		environment: "miniflare",
		environmentOptions: {
			modules: true,
			scriptPath: new URL("./tests/index.ts", import.meta.url),
			bindings: {},
		},
		testTimeout: 60000,
	},
});
