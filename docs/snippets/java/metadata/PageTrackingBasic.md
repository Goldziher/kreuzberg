import dev.kreuzberg.*;

var config = ExtractionConfig.builder()
    .pages(PageConfig.builder()
        .extractPages(true)
        .build())
    .build();

var result = Kreuzberg.extractFileSync("document.pdf", config);

if (result.pages() != null) {
    for (var page : result.pages()) {
        System.out.println("Page " + page.pageNumber() + ":");
        System.out.println("  Content: " + page.content().length() + " chars");
        System.out.println("  Tables: " + page.tables().size());
        System.out.println("  Images: " + page.images().size());
    }
}
