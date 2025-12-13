import { defineConfig } from "tsup";

export default defineConfig({
	entry: [
		"src/index.ts",
		"src/types/index.ts",
		"src/types/config.ts",
		"src/types/errors.ts",
		"src/types/metadata.ts",
		"src/types/protocols.ts",
		"src/types/results.ts",
		"src/utils/index.ts",
		"src/constants/index.ts",
	],
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
	platform: "neutral",
});
