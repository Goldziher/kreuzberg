package com.kreuzberg.e2e;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import org.junit.jupiter.api.Test;

import java.util.Arrays;
import java.util.Collections;
import java.util.List;
import java.util.Map;

/** Auto-generated tests for email fixtures. */
public class EmailTest {
    private static final ObjectMapper MAPPER = new ObjectMapper();

    @Test
    public void emailSampleEml() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "email_sample_eml",
            "email/sample_email.eml",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("message/rfc822"));
                E2EHelpers.Assertions.assertMinContentLength(result, 20);
            }
        );
    }

}
