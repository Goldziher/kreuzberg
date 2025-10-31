import type { ExtractionResult, PostProcessorProtocol, ProcessingStage } from "../../../src/index.js";

type WrappedProcessor = {
	name: () => string;
	processingStage?: () => string;
	process: (...args: unknown[]) => string | Promise<string>;
	__original?: PostProcessorProtocol;
	__stage?: ProcessingStage | string;
};

type RegisteredProcessor = {
	wrapped: WrappedProcessor;
	name: string;
	stage: string;
	original?: PostProcessorProtocol;
	order: number;
};

const textDecoder = new TextDecoder();
const STAGE_PRIORITY = new Map<string, number>([
	["early", 0],
	["middle", 1],
	["late", 2],
]);

const toWire = (result: ExtractionResult) => ({
	content: result.content,
	mime_type: result.mimeType,
	metadata: result.metadata,
	tables: result.tables,
	detected_languages: result.detectedLanguages,
	chunks: result.chunks ?? undefined,
});

const fromWire = (wire: {
	content: string;
	mime_type: string;
	metadata: Record<string, unknown> | string;
	tables?: unknown[];
	detected_languages?: string[] | null;
	chunks?: unknown[] | null;
}): ExtractionResult => ({
	content: wire.content,
	mimeType: wire.mime_type,
	metadata: typeof wire.metadata === "string" ? JSON.parse(wire.metadata) : (wire.metadata ?? {}),
	tables: Array.isArray(wire.tables) ? wire.tables : [],
	detectedLanguages: wire.detected_languages ?? null,
	chunks: (wire.chunks ?? null) as ExtractionResult["chunks"],
});

const createInitialResult = (data: Uint8Array | Buffer, mimeType: string | null): ExtractionResult => ({
	content: textDecoder.decode(data),
	mimeType: mimeType ?? "text/plain",
	metadata: {},
	tables: [],
	detectedLanguages: null,
	chunks: null,
});

const waitForPromise = <T>(promise: Promise<T>): T => {
	const sab = new SharedArrayBuffer(4);
	const view = new Int32Array(sab);
	let value: T | undefined;
	let error: unknown;

	promise
		.then((resolved) => {
			value = resolved;
			Atomics.store(view, 0, 1);
			Atomics.notify(view, 0);
		})
		.catch((err) => {
			error = err;
			Atomics.store(view, 0, 1);
			Atomics.notify(view, 0);
		});

	Atomics.wait(view, 0, 0);

	if (error) {
		throw error;
	}

	return value as T;
};

export type MockExtractionBinding = ReturnType<typeof createMockExtractionBinding>;

export function createMockExtractionBinding() {
	let registrationCounter = 0;
	const processors: RegisteredProcessor[] = [];

	const buildProcessorList = (config: Record<string, unknown> | null) => {
		const postConfig = (config?.postprocessor ?? null) as {
			enabled?: boolean;
			enabledProcessors?: string[];
			disabledProcessors?: string[];
		} | null;

		if (postConfig && postConfig.enabled === false) {
			return [] as RegisteredProcessor[];
		}

		const enabledSet = postConfig?.enabledProcessors ? new Set(postConfig.enabledProcessors) : null;
		const disabledSet = postConfig?.disabledProcessors ? new Set(postConfig.disabledProcessors) : null;

		return processors
			.filter((proc) => {
				if (enabledSet && !enabledSet.has(proc.name)) {
					return false;
				}
				if (disabledSet && disabledSet.has(proc.name)) {
					return false;
				}
				return true;
			})
			.slice()
			.sort((a, b) => {
				const stageA = STAGE_PRIORITY.get(a.stage) ?? 1;
				const stageB = STAGE_PRIORITY.get(b.stage) ?? 1;
				if (stageA !== stageB) {
					return stageA - stageB;
				}
				return a.order - b.order;
			});
	};

	const runPipelineAsync = async (
		result: ExtractionResult,
		config: Record<string, unknown> | null,
	): Promise<ExtractionResult> => {
		let current = result;
		for (const processor of buildProcessorList(config)) {
			const wireInput = JSON.stringify(toWire(current));
			try {
				const response = await processor.wrapped.process([wireInput]);
				const wireUpdated = JSON.parse(response) as ReturnType<typeof toWire>;
				current = fromWire(wireUpdated);
			} catch (error) {
				const metadata = {
					...current.metadata,
					[`${processor.name}_error`]: error instanceof Error ? error.message : "unknown error",
				};
				current = { ...current, metadata };
			}
		}
		return current;
	};

	const runPipelineSync = (result: ExtractionResult, config: Record<string, unknown> | null): ExtractionResult => {
		let current = result;
		for (const processor of buildProcessorList(config)) {
			const wireInput = JSON.stringify(toWire(current));
			try {
				const response = processor.wrapped.process([wireInput]);
				const resolved = response instanceof Promise ? waitForPromise(response) : response;
				const wireUpdated = JSON.parse(resolved) as ReturnType<typeof toWire>;
				current = fromWire(wireUpdated);
			} catch (error) {
				const metadata = {
					...current.metadata,
					[`${processor.name}_error`]: error instanceof Error ? error.message : "unknown error",
				};
				current = { ...current, metadata };
			}
		}
		return current;
	};

	return {
		registerPostProcessor(processor: WrappedProcessor) {
			const stage = processor.__stage ?? processor.processingStage?.() ?? "middle";
			const entry: RegisteredProcessor = {
				wrapped: processor,
				name: processor.name(),
				stage,
				original: processor.__original,
				order: registrationCounter++,
			};
			entry.original?.initialize?.();
			processors.push(entry);
		},
		clearPostProcessors() {
			for (const proc of processors) {
				proc.original?.shutdown?.();
			}
			processors.length = 0;
		},
		unregisterPostProcessor(name: string) {
			const index = processors.findIndex((proc) => proc.name === name);
			if (index >= 0) {
				processors[index].original?.shutdown?.();
				processors.splice(index, 1);
			}
		},
		extractBytesSync(data: Buffer, mimeType: string | null, config: Record<string, unknown> | null = null) {
			const result = createInitialResult(data, mimeType);
			const processed = runPipelineSync(result, config);
			return {
				content: processed.content,
				mimeType: processed.mimeType,
				metadata: processed.metadata,
				tables: processed.tables,
				detectedLanguages: processed.detectedLanguages,
				chunks: processed.chunks,
			};
		},
		async extractBytes(data: Buffer, mimeType: string | null, config: Record<string, unknown> | null = null) {
			const result = createInitialResult(data, mimeType);
			const processed = await runPipelineAsync(result, config);
			return {
				content: processed.content,
				mimeType: processed.mimeType,
				metadata: processed.metadata,
				tables: processed.tables,
				detectedLanguages: processed.detectedLanguages,
				chunks: processed.chunks,
			};
		},
	};
}
