package com.kreuzberg.e2e;

// CHECKSTYLE.OFF: UnusedImports - generated code
// CHECKSTYLE.OFF: LineLength - generated code
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import org.junit.jupiter.api.Test;

import java.util.Arrays;
import java.util.Collections;
import java.util.List;
import java.util.Map;
// CHECKSTYLE.ON: UnusedImports
// CHECKSTYLE.ON: LineLength

/** Auto-generated tests for smoke fixtures. */
public class SmokeTest {
    private static final ObjectMapper MAPPER = new ObjectMapper();

    @Test
    public void smokeDocxBasic() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "smoke_docx_basic",
            "documents/fake.docx",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/vnd.openxmlformats-officedocument.wordprocessingml.document"));
                E2EHelpers.Assertions.assertMinContentLength(result, 20);
                E2EHelpers.Assertions.assertContentContainsAny(result, Arrays.asList("Lorem", "ipsum", "document", "text"));
            }
        );
    }

    @Test
    public void smokeHtmlBasic() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "smoke_html_basic",
            "web/simple_table.html",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("text/html"));
                E2EHelpers.Assertions.assertMinContentLength(result, 10);
                E2EHelpers.Assertions.assertContentContainsAny(result, Arrays.asList("#", "**", "simple", "HTML"));
            }
        );
    }

    @Test
    public void smokeImagePng() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "smoke_image_png",
            "images/sample.png",
            config,
            Collections.emptyList(),
            "Image extraction requires image processing dependencies",
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("image/png"));
                E2EHelpers.Assertions.assertMetadataExpectation(result, "format", Map.of("eq", "PNG"));
            }
        );
    }

    @Test
    public void smokeJsonBasic() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "smoke_json_basic",
            "data_formats/simple.json",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/json"));
                E2EHelpers.Assertions.assertMinContentLength(result, 5);
            }
        );
    }

    @Test
    public void smokePdfBasic() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "smoke_pdf_basic",
            "pdfs/fake_memo.pdf",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/pdf"));
                E2EHelpers.Assertions.assertMinContentLength(result, 50);
                E2EHelpers.Assertions.assertContentContainsAny(result, Arrays.asList("May 5, 2023", "To Whom it May Concern"));
            }
        );
    }

    @Test
    public void smokeTxtBasic() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "smoke_txt_basic",
            "text/report.txt",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("text/plain"));
                E2EHelpers.Assertions.assertMinContentLength(result, 5);
            }
        );
    }

    @Test
    public void smokeXlsxBasic() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "smoke_xlsx_basic",
            "spreadsheets/stanley_cups.xlsx",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"));
                E2EHelpers.Assertions.assertMinContentLength(result, 100);
                E2EHelpers.Assertions.assertContentContainsAll(result, Arrays.asList("Team", "Location", "Stanley Cups", "Blues", "Flyers", "Maple Leafs", "STL", "PHI", "TOR"));
                E2EHelpers.Assertions.assertTableCount(result, 1, null);
                E2EHelpers.Assertions.assertMetadataExpectation(result, "sheet_count", Map.of("gte", 2));
                E2EHelpers.Assertions.assertMetadataExpectation(result, "sheet_names", Map.of("contains", Arrays.asList("Stanley Cups")));
            }
        );
    }

}
