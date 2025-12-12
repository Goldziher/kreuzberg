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

/** Auto-generated tests for office fixtures. */
public class OfficeTest {
    private static final ObjectMapper MAPPER = new ObjectMapper();

    @Test
    public void officeDocLegacy() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "office_doc_legacy",
            "legacy_office/unit_test_lists.doc",
            config,
            Arrays.asList("libreoffice", "libreoffice"),
            "LibreOffice must be installed for conversion.",
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/msword"));
                E2EHelpers.Assertions.assertMinContentLength(result, 20);
            }
        );
    }

    @Test
    public void officeDocxBasic() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "office_docx_basic",
            "office/document.docx",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/vnd.openxmlformats-officedocument.wordprocessingml.document"));
                E2EHelpers.Assertions.assertMinContentLength(result, 10);
            }
        );
    }

    @Test
    public void officeDocxEquations() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "office_docx_equations",
            "documents/equations.docx",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/vnd.openxmlformats-officedocument.wordprocessingml.document"));
                E2EHelpers.Assertions.assertMinContentLength(result, 20);
            }
        );
    }

    @Test
    public void officeDocxFake() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "office_docx_fake",
            "documents/fake.docx",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/vnd.openxmlformats-officedocument.wordprocessingml.document"));
                E2EHelpers.Assertions.assertMinContentLength(result, 20);
            }
        );
    }

    @Test
    public void officeDocxFormatting() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "office_docx_formatting",
            "documents/unit_test_formatting.docx",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/vnd.openxmlformats-officedocument.wordprocessingml.document"));
                E2EHelpers.Assertions.assertMinContentLength(result, 20);
            }
        );
    }

    @Test
    public void officeDocxHeaders() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "office_docx_headers",
            "documents/unit_test_headers.docx",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/vnd.openxmlformats-officedocument.wordprocessingml.document"));
                E2EHelpers.Assertions.assertMinContentLength(result, 20);
            }
        );
    }

    @Test
    public void officeDocxLists() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "office_docx_lists",
            "documents/unit_test_lists.docx",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/vnd.openxmlformats-officedocument.wordprocessingml.document"));
                E2EHelpers.Assertions.assertMinContentLength(result, 20);
            }
        );
    }

    @Test
    public void officeDocxTables() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "office_docx_tables",
            "documents/docx_tables.docx",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/vnd.openxmlformats-officedocument.wordprocessingml.document"));
                E2EHelpers.Assertions.assertMinContentLength(result, 50);
                E2EHelpers.Assertions.assertContentContainsAll(result, Arrays.asList("Simple uniform table", "Nested Table", "merged cells", "Header Col"));
                E2EHelpers.Assertions.assertTableCount(result, 1, null);
            }
        );
    }

    @Test
    public void officePptLegacy() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "office_ppt_legacy",
            "legacy_office/simple.ppt",
            config,
            Arrays.asList("libreoffice", "libreoffice"),
            "Skip if LibreOffice conversion is unavailable.",
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/vnd.ms-powerpoint"));
                E2EHelpers.Assertions.assertMinContentLength(result, 10);
            }
        );
    }

    @Test
    public void officePptxBasic() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "office_pptx_basic",
            "presentations/simple.pptx",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/vnd.openxmlformats-officedocument.presentationml.presentation"));
                E2EHelpers.Assertions.assertMinContentLength(result, 50);
            }
        );
    }

    @Test
    public void officePptxImages() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "office_pptx_images",
            "presentations/powerpoint_with_image.pptx",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/vnd.openxmlformats-officedocument.presentationml.presentation"));
                E2EHelpers.Assertions.assertMinContentLength(result, 20);
            }
        );
    }

    @Test
    public void officePptxPitchDeck() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "office_pptx_pitch_deck",
            "presentations/pitch_deck_presentation.pptx",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/vnd.openxmlformats-officedocument.presentationml.presentation"));
                E2EHelpers.Assertions.assertMinContentLength(result, 100);
            }
        );
    }

    @Test
    public void officeXlsLegacy() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "office_xls_legacy",
            "spreadsheets/test_excel.xls",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/vnd.ms-excel"));
                E2EHelpers.Assertions.assertMinContentLength(result, 10);
            }
        );
    }

    @Test
    public void officeXlsxBasic() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "office_xlsx_basic",
            "spreadsheets/stanley_cups.xlsx",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"));
                E2EHelpers.Assertions.assertMinContentLength(result, 100);
                E2EHelpers.Assertions.assertContentContainsAll(result, Arrays.asList("Team", "Location", "Stanley Cups"));
                E2EHelpers.Assertions.assertTableCount(result, 1, null);
                E2EHelpers.Assertions.assertMetadataExpectation(result, "sheet_count", Map.of("gte", 2));
                E2EHelpers.Assertions.assertMetadataExpectation(result, "sheet_names", Map.of("contains", Arrays.asList("Stanley Cups")));
            }
        );
    }

    @Test
    public void officeXlsxMultiSheet() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "office_xlsx_multi_sheet",
            "spreadsheets/excel_multi_sheet.xlsx",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"));
                E2EHelpers.Assertions.assertMinContentLength(result, 20);
                E2EHelpers.Assertions.assertMetadataExpectation(result, "sheet_count", Map.of("gte", 2));
            }
        );
    }

    @Test
    public void officeXlsxOfficeExample() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "office_xlsx_office_example",
            "office/excel.xlsx",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"));
                E2EHelpers.Assertions.assertMinContentLength(result, 10);
            }
        );
    }

}
