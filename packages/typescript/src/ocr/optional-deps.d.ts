/**
 * Type declarations for optional OCR dependencies.
 * These are optionalDependencies and may not be installed.
 */

declare module "sharp" {
	const sharp: any;
	export default sharp;
}

declare module "@gutenye/ocr-node" {
	const GutenOcr: any;
	export default GutenOcr;
}
