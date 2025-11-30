import { defineConfig } from "tsup";

export default defineConfig({
	entry: ["typescript/index.ts", "typescript/cli.ts"],
	format: ["esm", "cjs"],
	dts: {
		compilerOptions: {
			skipLibCheck: true,
			skipDefaultLibCheck: true,
		},
	},
	splitting: false,
	sourcemap: true,
	clean: true,
	shims: true,
	external: ["sharp", "@gutenye/ocr-node"],
});
