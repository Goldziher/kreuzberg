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

/** Auto-generated tests for xml fixtures. */
public class XmlTest {
    private static final ObjectMapper MAPPER = new ObjectMapper();

    @Test
    public void xmlPlantCatalog() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "xml_plant_catalog",
            "xml/plant_catalog.xml",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/xml"));
                E2EHelpers.Assertions.assertMinContentLength(result, 100);
                E2EHelpers.Assertions.assertMetadataExpectation(result, "element_count", Map.of("gte", 1));
            }
        );
    }

}
