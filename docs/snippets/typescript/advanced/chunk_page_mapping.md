import { extractFileSync } from 'kreuzberg';

const result = extractFileSync('document.pdf', {
  chunking: { chunkSize: 500, overlap: 50 },
  pages: { extractPages: true }
});

if (result.chunks) {
  for (const chunk of result.chunks) {
    if (chunk.metadata.firstPage) {
      const pageRange = chunk.metadata.firstPage === chunk.metadata.lastPage
        ? `Page ${chunk.metadata.firstPage}`
        : `Pages ${chunk.metadata.firstPage}-${chunk.metadata.lastPage}`;

      console.log(`Chunk: ${chunk.text.substring(0, 50)}... (${pageRange})`);
    }
  }
}
