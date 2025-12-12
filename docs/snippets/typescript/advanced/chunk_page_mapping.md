import { extractFileSync } from '@kreuzberg/node';

const result = extractFileSync('document.pdf', null, { chunking: { maxChars: 500, maxOverlap: 50 }, pages: { extractPages: true } });

if (result.chunks) {
  for (const chunk of result.chunks) {
    if (chunk.metadata.firstPage) {
      const pageRange = chunk.metadata.firstPage === chunk.metadata.lastPage
        ? `Page ${chunk.metadata.firstPage}`
        : `Pages ${chunk.metadata.firstPage}-${chunk.metadata.lastPage}`;

      console.log(`Chunk: ${chunk.content.substring(0, 50)}... (${pageRange})`);
    }
  }
}
