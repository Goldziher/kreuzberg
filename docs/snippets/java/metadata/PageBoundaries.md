import dev.kreuzberg.*;
import java.nio.charset.StandardCharsets;

var result = Kreuzberg.extractFileSync("document.pdf");

if (result.metadata().pages() != null &&
    result.metadata().pages().boundaries() != null) {

    var contentBytes = result.content().getBytes(StandardCharsets.UTF_8);

    for (var boundary : result.metadata().pages().boundaries().subList(0, 3)) {
        var pageBytes = Arrays.copyOfRange(
            contentBytes,
            boundary.byteStart(),
            boundary.byteEnd()
        );
        var pageText = new String(pageBytes, StandardCharsets.UTF_8);

        System.out.println("Page " + boundary.pageNumber() + ":");
        System.out.println("  Byte range: " + boundary.byteStart() +
                           "-" + boundary.byteEnd());
        System.out.println("  Preview: " + pageText.substring(0, 100) + "...");
    }
}
