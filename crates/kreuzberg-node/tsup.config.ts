import { defineConfig } from "tsup";

export default defineConfig({
	entry: ["typescript/index.ts", "typescript/cli.ts", "typescript/errors.ts", "typescript/ocr/guten-ocr.ts"],
	format: ["esm", "cjs"],
	bundle: false,
	dts: {
		compilerOptions: {
			skipLibCheck: true,
			skipDefaultLibCheck: true,
		},
	},
	splitting: false,
	sourcemap: true,
	clean: true,
	shims: false,
	platform: "node",
	target: "node22",
	external: ["sharp", "@gutenye/ocr-node", /\.node$/, /@kreuzberg\/node-.*/, "./index.js", "../index.js"],
});
