import { defineConfig } from "vite";

export default defineConfig({
	server: {
		headers: {
			"Cross-Origin-Opener-Policy": "same-origin",
			"Cross-Origin-Embedder-Policy": "require-corp",
		},
		port: 5173,
		open: true,
	},
	build: {
		outDir: "dist",
		sourcemap: true,
	},
});
