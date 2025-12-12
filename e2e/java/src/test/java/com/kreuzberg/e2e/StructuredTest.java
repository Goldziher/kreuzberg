package com.kreuzberg.e2e;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import org.junit.jupiter.api.Test;

import java.util.Arrays;
import java.util.Collections;
import java.util.List;
import java.util.Map;

/** Auto-generated tests for structured fixtures. */
public class StructuredTest {
    private static final ObjectMapper MAPPER = new ObjectMapper();

    @Test
    public void structuredJsonBasic() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "structured_json_basic",
            "json/sample_document.json",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/json"));
                E2EHelpers.Assertions.assertMinContentLength(result, 20);
                E2EHelpers.Assertions.assertContentContainsAny(result, Arrays.asList("Sample Document", "Test Author"));
            }
        );
    }

    @Test
    public void structuredJsonSimple() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "structured_json_simple",
            "data_formats/simple.json",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/json"));
                E2EHelpers.Assertions.assertMinContentLength(result, 10);
                E2EHelpers.Assertions.assertContentContainsAny(result, Arrays.asList("{", "name"));
            }
        );
    }

    @Test
    public void structuredYamlSimple() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "structured_yaml_simple",
            "data_formats/simple.yaml",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/x-yaml"));
                E2EHelpers.Assertions.assertMinContentLength(result, 10);
            }
        );
    }

}
