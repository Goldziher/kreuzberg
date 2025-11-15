package com.kreuzberg.e2e;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.kreuzberg.ExtractionResult;
import com.kreuzberg.Kreuzberg;
import com.kreuzberg.KreuzbergException;
import com.kreuzberg.Table;
import org.junit.jupiter.api.Assumptions;

import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.List;
import java.util.Map;

import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.junit.jupiter.api.Assertions.fail;

/**
 * Helper utilities for E2E tests.
 */
public final class E2EHelpers {
    private static final Path WORKSPACE_ROOT =
            Paths.get("").toAbsolutePath().getParent().getParent().getParent();
    private static final Path TEST_DOCUMENTS = WORKSPACE_ROOT.resolve("test_documents");
    private static final ObjectMapper MAPPER = new ObjectMapper();

    private E2EHelpers() { }

    public static Path resolveDocument(String relativePath) {
        return TEST_DOCUMENTS.resolve(relativePath);
    }

    public static Map<String, Object> buildConfig(JsonNode configNode) {
        if (configNode == null || configNode.isNull() || !configNode.isObject()) {
            return null;
        }
        try {
            @SuppressWarnings("unchecked")
            Map<String, Object> result = MAPPER.convertValue(configNode, Map.class);
            return result;
        } catch (Exception e) {
            throw new RuntimeException("Failed to parse config", e);
        }
    }

    public static String skipReasonFor(
            Exception error,
            String fixtureId,
            List<String> requirements,
            String notes
    ) {
        String message = error.getMessage() != null ? error.getMessage() : "";
        String lowered = message.toLowerCase();
        boolean requirementHit = requirements.stream()
                .anyMatch(req -> lowered.contains(req.toLowerCase()));
        boolean missingDependency = error instanceof KreuzbergException.MissingDependency
                || lowered.contains("missing dependency");
        boolean unsupportedFormat = lowered.contains("unsupported format");

        if (!missingDependency && !unsupportedFormat && !requirementHit) {
            return null;
        }

        String reason;
        if (missingDependency) {
            if (error instanceof KreuzbergException.MissingDependency) {
                KreuzbergException.MissingDependency md = (KreuzbergException.MissingDependency) error;
                String dep = md.getDependency();
                reason = dep != null && !dep.isEmpty()
                        ? "missing dependency " + dep
                        : "missing dependency";
            } else {
                reason = "missing dependency";
            }
        } else if (unsupportedFormat) {
            reason = "unsupported format";
        } else {
            reason = "requires " + String.join(", ", requirements);
        }

        String details = String.format(
                "Skipping %s: %s. %s: %s",
                fixtureId,
                reason,
                error.getClass().getSimpleName(),
                message
        );
        if (notes != null && !notes.isEmpty()) {
            details += " Notes: " + notes;
        }
        System.err.println(details);
        return details;
    }

    public static void runFixture(
            String fixtureId,
            String relativePath,
            JsonNode configNode,
            List<String> requirements,
            String notes,
            boolean skipIfMissing,
            TestCallback callback
    ) throws Exception {
        Path documentPath = resolveDocument(relativePath);

        if (skipIfMissing && !Files.exists(documentPath)) {
            String msg = String.format("Skipping %s: missing document at %s", fixtureId, documentPath);
            System.err.println(msg);
            Assumptions.assumeTrue(false, msg);
            return;
        }

        Map<String, Object> config = buildConfig(configNode);
        ExtractionResult result;
        try {
            result = Kreuzberg.extractFile(documentPath.toString(), config);
        } catch (Exception e) {
            String skipReason = skipReasonFor(e, fixtureId, requirements, notes);
            if (skipReason != null) {
                Assumptions.assumeTrue(false, skipReason);
                return;
            }
            throw e;
        }

        callback.run(result);
    }

    @FunctionalInterface
    public interface TestCallback {
        void run(ExtractionResult result) throws Exception;
    }

    /**
     * Assertion utilities for E2E tests.
     */
    public static final class Assertions {
        private Assertions() { }

        public static void assertExpectedMime(ExtractionResult result, List<String> expected) {
            if (expected.isEmpty()) {
                return;
            }
            String mimeType = result.getMimeType();
            boolean matches = expected.stream()
                    .anyMatch(token -> mimeType != null && mimeType.contains(token));
            assertTrue(matches,
                    String.format("Expected mime type to contain one of %s, got %s", expected, mimeType));
        }

        public static void assertMinContentLength(ExtractionResult result, int minimum) {
            String content = result.getContent();
            int length = content != null ? content.length() : 0;
            assertTrue(length >= minimum,
                    String.format("Expected content length >= %d, got %d", minimum, length));
        }

        public static void assertMaxContentLength(ExtractionResult result, int maximum) {
            String content = result.getContent();
            int length = content != null ? content.length() : 0;
            assertTrue(length <= maximum,
                    String.format("Expected content length <= %d, got %d", maximum, length));
        }

        public static void assertContentContainsAny(ExtractionResult result, List<String> snippets) {
            if (snippets.isEmpty()) {
                return;
            }
            String content = result.getContent();
            String lowered = content != null ? content.toLowerCase() : "";
            boolean matches = snippets.stream()
                    .anyMatch(snippet -> lowered.contains(snippet.toLowerCase()));
            assertTrue(matches,
                    String.format("Expected content to contain any of %s", snippets));
        }

        public static void assertContentContainsAll(ExtractionResult result, List<String> snippets) {
            if (snippets.isEmpty()) {
                return;
            }
            String content = result.getContent();
            String lowered = content != null ? content.toLowerCase() : "";
            boolean allMatch = snippets.stream()
                    .allMatch(snippet -> lowered.contains(snippet.toLowerCase()));
            assertTrue(allMatch,
                    String.format("Expected content to contain all of %s", snippets));
        }

        public static void assertTableCount(
                ExtractionResult result,
                Integer minimum,
                Integer maximum
        ) {
            List<Table> tables = result.getTables();
            int count = tables != null ? tables.size() : 0;
            if (minimum != null) {
                assertTrue(count >= minimum,
                        String.format("Expected table count >= %d, got %d", minimum, count));
            }
            if (maximum != null) {
                assertTrue(count <= maximum,
                        String.format("Expected table count <= %d, got %d", maximum, count));
            }
        }

        public static void assertDetectedLanguages(
                ExtractionResult result,
                List<String> expected,
                Double minConfidence
        ) {
            if (expected.isEmpty()) {
                return;
            }
            List<String> languages = result.getDetectedLanguages();
            assertNotNull(languages, "Expected detected languages to be present");
            boolean allFound = expected.stream()
                    .allMatch(lang -> languages.contains(lang));
            assertTrue(allFound,
                    String.format("Expected languages %s to be in %s", expected, languages));

            if (minConfidence != null) {
                Map<String, Object> metadata = result.getMetadata();
                if (metadata != null && metadata.containsKey("confidence")) {
                    Object confObj = metadata.get("confidence");
                    double confidence = confObj instanceof Number
                            ? ((Number) confObj).doubleValue()
                            : 0.0;
                    assertTrue(confidence >= minConfidence,
                            String.format("Expected confidence >= %f, got %f", minConfidence, confidence));
                }
            }
        }

        public static void assertMetadataExpectation(
                ExtractionResult result,
                String path,
                Map<String, Object> expectation
        ) {
            Map<String, Object> metadata = result.getMetadata();
            Object value = fetchMetadataValue(metadata, path);
            assertNotNull(value, String.format("Metadata path '%s' missing", path));

            if (expectation.containsKey("eq")) {
                Object expected = expectation.get("eq");
                assertTrue(valuesEqual(value, expected),
                        String.format("Expected %s to equal %s", value, expected));
            }

            if (expectation.containsKey("gte")) {
                Object expected = expectation.get("gte");
                double actualNum = convertNumeric(value);
                double expectedNum = convertNumeric(expected);
                assertTrue(actualNum >= expectedNum,
                        String.format("Expected %f >= %f", actualNum, expectedNum));
            }

            if (expectation.containsKey("lte")) {
                Object expected = expectation.get("lte");
                double actualNum = convertNumeric(value);
                double expectedNum = convertNumeric(expected);
                assertTrue(actualNum <= expectedNum,
                        String.format("Expected %f <= %f", actualNum, expectedNum));
            }

            if (expectation.containsKey("contains")) {
                Object contains = expectation.get("contains");
                if (value instanceof String && contains instanceof String) {
                    assertTrue(((String) value).contains((String) contains),
                            String.format("Expected '%s' to contain '%s'", value, contains));
                } else if (value instanceof List && contains instanceof List) {
                    @SuppressWarnings("unchecked")
                    List<Object> valueList = (List<Object>) value;
                    @SuppressWarnings("unchecked")
                    List<Object> containsList = (List<Object>) contains;
                    boolean allContained = containsList.stream()
                            .allMatch(valueList::contains);
                    assertTrue(allContained,
                            String.format("Expected %s to contain all of %s", value, contains));
                } else {
                    fail(String.format("Unsupported contains expectation for path '%s'", path));
                }
            }
        }

        private static Object fetchMetadataValue(Map<String, Object> metadata, String path) {
            if (metadata == null) {
                return null;
            }
            Object current = metadata;
            for (String segment : path.split("\\.")) {
                if (!(current instanceof Map)) {
                    return null;
                }
                @SuppressWarnings("unchecked")
                Map<String, Object> map = (Map<String, Object>) current;
                current = map.get(segment);
            }
            return current;
        }

        private static boolean valuesEqual(Object lhs, Object rhs) {
            if (lhs == null && rhs == null) {
                return true;
            }
            if (lhs == null || rhs == null) {
                return false;
            }
            if (lhs instanceof String && rhs instanceof String) {
                return lhs.equals(rhs);
            }
            if (isNumericLike(lhs) && isNumericLike(rhs)) {
                return convertNumeric(lhs) == convertNumeric(rhs);
            }
            return lhs.equals(rhs);
        }

        private static boolean isNumericLike(Object value) {
            return value instanceof Number;
        }

        private static double convertNumeric(Object value) {
            if (value instanceof Number) {
                return ((Number) value).doubleValue();
            }
            throw new IllegalArgumentException("Cannot convert to numeric: " + value);
        }
    }
}
