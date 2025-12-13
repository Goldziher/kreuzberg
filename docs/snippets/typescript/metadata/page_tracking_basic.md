import { extractFileSync } from '@kreuzberg/node';

const result = extractFileSync('document.pdf', null, { pages: { extractPages: true } });

if (result.pages) {
  for (const page of result.pages) {
    console.log(`Page ${page.pageNumber}:`);
    console.log(`  Content: ${page.content.length} chars`);
    console.log(`  Tables: ${page.tables.length}`);
    console.log(`  Images: ${page.images.length}`);
  }
}
