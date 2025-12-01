package dev.kreuzberg;

import com.fasterxml.jackson.annotation.JsonProperty;
import com.fasterxml.jackson.databind.ObjectMapper;

/**
 * Immutable record representing panic context information from the native library.
 *
 * <p>This record holds information about panics that occurred in the Kreuzberg
 * native library, including the source location, function name, message, and timestamp.
 * The panic context is deserialized from a JSON string returned by the FFI layer.</p>
 *
 * @param file the source file where the panic occurred
 * @param line the line number in the source file
 * @param function the name of the function where the panic occurred
 * @param message the panic message
 * @param timestampSecs the Unix timestamp (seconds since epoch) when the panic occurred
 * @since 4.0.0
 */
public record PanicContext(
    @JsonProperty("file") String file,
    @JsonProperty("line") int line,
    @JsonProperty("function") String function,
    @JsonProperty("message") String message,
    @JsonProperty("timestamp_secs") long timestampSecs
) {

    private static final ObjectMapper OBJECT_MAPPER = new ObjectMapper();

    /**
     * Parses a PanicContext from a JSON string.
     *
     * @param jsonString the JSON string containing panic context information
     * @return a PanicContext parsed from the JSON
     * @throws IllegalArgumentException if the JSON cannot be parsed
     */
    public static PanicContext fromJson(String jsonString) {
        if (jsonString == null || jsonString.isEmpty()) {
            throw new IllegalArgumentException("JSON string cannot be null or empty");
        }
        try {
            return OBJECT_MAPPER.readValue(jsonString, PanicContext.class);
        } catch (Exception e) {
            throw new IllegalArgumentException("Failed to parse panic context from JSON: " + e.getMessage(), e);
        }
    }

    /**
     * Returns a formatted string representation of the panic context.
     *
     * <p>The format is: "Panic at file:line in function(): message"</p>
     *
     * @return a formatted string
     */
    @Override
    public String toString() {
        return String.format("Panic at %s:%d in %s(): %s", file, line, function, message);
    }
}
