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

/** Auto-generated tests for html fixtures. */
public class HtmlTest {
    private static final ObjectMapper MAPPER = new ObjectMapper();

    @Test
    public void htmlComplexLayout() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "html_complex_layout",
            "web/taylor_swift.html",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("text/html"));
                E2EHelpers.Assertions.assertMinContentLength(result, 1000);
            }
        );
    }

    @Test
    public void htmlSimpleTable() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "html_simple_table",
            "web/simple_table.html",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("text/html"));
                E2EHelpers.Assertions.assertMinContentLength(result, 100);
                E2EHelpers.Assertions.assertContentContainsAll(result, Arrays.asList("Product", "Category", "Price", "Stock", "Laptop", "Electronics", "Sample Data Table"));
                E2EHelpers.Assertions.assertTableCount(result, 1, null);
            }
        );
    }

}
