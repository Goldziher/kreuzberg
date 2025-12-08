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

/** Auto-generated tests for image fixtures. */
public class ImageTest {
    private static final ObjectMapper MAPPER = new ObjectMapper();

    @Test
    public void imageMetadataOnly() throws Exception {
        JsonNode config = MAPPER.readTree("{\"ocr\":null}");
        E2EHelpers.runFixture(
            "image_metadata_only",
            "images/example.jpg",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("image/jpeg"));
                E2EHelpers.Assertions.assertMaxContentLength(result, 100);
            }
        );
    }

}
