import { extractBytes, initWasm } from "@kreuzberg/wasm";

interface DocumentSummary {
  fileName: string;
  title: string | undefined;
  author: string | undefined;
  pageCount: number | undefined;
  language: string;
}

async function filterAndSummarizeMetadata(files: string[]): Promise<DocumentSummary[]> {
  await initWasm();

  const summaries: DocumentSummary[] = [];

  for (const fileName of files) {
    const bytes = new Uint8Array(await fetch(fileName).then((r) => r.arrayBuffer()));

    const result = await extractBytes(bytes, "application/pdf");

    summaries.push({
      fileName,
      title: result.metadata.title,
      author: result.metadata.author,
      pageCount: result.metadata.pageCount,
      language: result.detectedLanguages?.[0] ?? "unknown",
    });
  }

  return summaries;
}

filterAndSummarizeMetadata(["doc1.pdf", "doc2.pdf", "doc3.pdf"])
  .then((summaries) => console.table(summaries))
  .catch(console.error);
