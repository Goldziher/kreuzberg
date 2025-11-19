/*
 * CHECKSTYLE:OFF
 * Single-file benchmark helper executed via `java <file>`.
 */
import dev.kreuzberg.ExtractionResult;
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.KreuzbergException;

import java.nio.file.Path;

public final class KreuzbergExtractJava {
    private static final double NANOS_IN_MILLISECOND = 1_000_000.0;

    private KreuzbergExtractJava() { }

    public static void main(String[] args) {
        if (args.length < 2) {
            System.err.println("Usage: KreuzbergExtractJava <mode> <file_path>");
            System.err.println("Modes: sync");
            System.exit(1);
        }

        String mode = args[0];
        if (!"sync".equals(mode)) {
            System.err.printf("Unsupported mode '%s'%n", mode);
            System.exit(1);
        }

        Path path = Path.of(args[1]);
        ExtractionResult result;
        long start = System.nanoTime();
        try {
            result = Kreuzberg.extractFile(path);
        } catch (KreuzbergException | RuntimeException e) {
            e.printStackTrace(System.err);
            System.exit(1);
            return;
        }
        double elapsedMs = (System.nanoTime() - start) / NANOS_IN_MILLISECOND;

        String json = toJson(result, elapsedMs);
        System.out.print(json);
    }

    private static String toJson(ExtractionResult result, double elapsedMs) {
        StringBuilder builder = new StringBuilder();
        builder.append('{');
        builder.append("\"content\":").append(quote(result.content())).append(',');
        builder.append("\"metadata\":{");
        builder.append("\"mimeType\":").append(quote(result.mimeType())).append(',');
        builder.append("\"language\":").append(optionalToJson(result.language())).append(',');
        builder.append("\"date\":").append(optionalToJson(result.date())).append(',');
        builder.append("\"subject\":").append(optionalToJson(result.subject()));
        builder.append("},\"_extraction_time_ms\":").append(String.format("%.3f", elapsedMs));
        builder.append('}');
        return builder.toString();
    }

    private static String optionalToJson(java.util.Optional<String> value) {
        return value.isPresent() ? quote(value.get()) : "null";
    }

    private static String quote(String value) {
        if (value == null) {
            return "null";
        }
        String escaped = value
                .replace("\\", "\\\\")
                .replace("\"", "\\\"")
                .replace("\n", "\\n")
                .replace("\r", "\\r");
        return "\"" + escaped + "\"";
    }
}
/* CHECKSTYLE:ON */
