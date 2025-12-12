import dev.kreuzberg.*;

var config = ExtractionConfig.builder()
    .chunking(ChunkingConfig.builder()
        .chunkSize(500)
        .overlap(50)
        .build())
    .pages(PageConfig.builder()
        .extractPages(true)
        .build())
    .build();

var result = Kreuzberg.extractFileSync("document.pdf", config);

if (result.chunks() != null) {
    for (var chunk : result.chunks()) {
        if (chunk.metadata().firstPage() != null) {
            var pageRange = chunk.metadata().firstPage().equals(chunk.metadata().lastPage())
                ? "Page " + chunk.metadata().firstPage()
                : "Pages " + chunk.metadata().firstPage() + "-" + chunk.metadata().lastPage();

            System.out.println("Chunk: " + chunk.text().substring(0, 50) +
                               "... (" + pageRange + ")");
        }
    }
}
