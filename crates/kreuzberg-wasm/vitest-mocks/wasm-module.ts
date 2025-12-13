export const extractBytes = async () => ({
	content: "test content",
	mimeType: "application/pdf",
	metadata: { pageCount: 1 },
	tables: [],
	detectedLanguages: ["en"],
});

export const extractBytesSync = () => ({
	content: "test content",
	mimeType: "application/pdf",
	metadata: { pageCount: 1 },
	tables: [],
	detectedLanguages: ["en"],
});

export const batchExtractBytes = async () => [
	{
		content: "test content 1",
		mimeType: "application/pdf",
		metadata: { pageCount: 1 },
		tables: [],
	},
	{
		content: "test content 2",
		mimeType: "application/pdf",
		metadata: { pageCount: 1 },
		tables: [],
	},
];

export const batchExtractBytesSync = () => [
	{
		content: "test content 1",
		mimeType: "application/pdf",
		metadata: { pageCount: 1 },
		tables: [],
	},
];

export const detectMimeFromBytes = () => "application/pdf";

export const normalizeMimeType = (mime: string) => mime;

export const version = () => "4.0.0";

export default async () => undefined;
