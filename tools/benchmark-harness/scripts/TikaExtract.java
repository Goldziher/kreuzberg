/*
 * CHECKSTYLE:OFF
 * Single-file benchmark helper executed via `java <file>`.
 */
import org.apache.tika.parser.AutoDetectParser;
import org.apache.tika.sax.BodyContentHandler;
import org.apache.tika.metadata.Metadata;

import java.io.File;
import java.io.FileInputStream;
import java.io.InputStream;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;

public final class TikaExtract {
    private static final double NANOS_IN_MILLISECOND = 1_000_000.0;

    private TikaExtract() {
    }

    public static void main(String[] args) {
        if (args.length < 2) {
            System.err.println("Usage: TikaExtract <mode> <file1> [file2] ...");
            System.err.println("Modes: sync, batch");
            System.exit(1);
        }

        String mode = args[0];
        if (!"sync".equals(mode) && !"batch".equals(mode)) {
            System.err.printf("Unsupported mode '%s'%n", mode);
            System.exit(1);
        }

        // Enable debug logging if TIKA_BENCHMARK_DEBUG is set
        boolean debug = "true".equalsIgnoreCase(System.getenv("TIKA_BENCHMARK_DEBUG"));

        if (debug) {
            debugLog("java.version", System.getProperty("java.version"));
            debugLog("os.name", System.getProperty("os.name"));
            debugLog("os.arch", System.getProperty("os.arch"));
            debugLog("Mode", mode);
            debugLog("Files to process", String.valueOf(args.length - 1));
        }

        try {
            if ("sync".equals(mode)) {
                if (args.length < 2) {
                    System.err.println("Sync mode requires exactly one file");
                    System.exit(1);
                }
                processSyncMode(args[1], debug);
            } else {
                processBatchMode(args, debug);
            }
        } catch (Exception e) {
            if (debug) {
                debugLog("Processing failed with exception", e.getClass().getName());
                e.printStackTrace(System.err);
            } else {
                e.printStackTrace(System.err);
            }
            System.exit(1);
        }
    }

    private static void processSyncMode(String filePath, boolean debug) throws Exception {
        if (debug) {
            debugLog("Input file", filePath);
        }

        Path path = Path.of(filePath);
        ExtractionData data;
        long start = System.nanoTime();

        try {
            if (debug) {
                debugLog("Starting extraction", "");
            }
            data = extractFile(path.toFile(), debug);
            if (debug) {
                debugLog("Extraction completed", "");
            }
        } catch (Exception e) {
            if (debug) {
                debugLog("Extraction failed", e.getClass().getName());
                e.printStackTrace(System.err);
            }
            throw e;
        }

        double elapsedMs = (System.nanoTime() - start) / NANOS_IN_MILLISECOND;
        String json = toJson(data, elapsedMs);
        System.out.print(json);
    }

    private static void processBatchMode(String[] args, boolean debug) throws Exception {
        List<String> jsonResults = new ArrayList<>();

        for (int i = 1; i < args.length; i++) {
            String filePath = args[i];

            if (debug) {
                debugLog("Processing file", filePath);
            }

            try {
                Path path = Path.of(filePath);
                long start = System.nanoTime();
                ExtractionData data = extractFile(path.toFile(), debug);
                double elapsedMs = (System.nanoTime() - start) / NANOS_IN_MILLISECOND;

                String json = toJson(data, elapsedMs);
                jsonResults.add(json);

                if (debug) {
                    debugLog("File processed", filePath);
                }
            } catch (Exception e) {
                if (debug) {
                    debugLog("Failed to process file", filePath);
                    debugLog("Exception", e.getClass().getName());
                    e.printStackTrace(System.err);
                } else {
                    System.err.printf("Error processing %s: %s%n", filePath, e.getMessage());
                }
                // Continue with next file in batch mode
            }
        }

        if (jsonResults.isEmpty()) {
            System.err.println("No files were successfully processed");
            System.exit(1);
            return;
        }

        // Output results, one per line
        for (String json : jsonResults) {
            System.out.println(json);
        }
    }

    private static ExtractionData extractFile(File file, boolean debug) throws Exception {
        if (!file.exists()) {
            throw new IllegalArgumentException("File does not exist: " + file.getAbsolutePath());
        }

        AutoDetectParser parser = new AutoDetectParser();
        BodyContentHandler handler = new BodyContentHandler(-1); // No limit on content length
        Metadata metadata = new Metadata();

        try (InputStream stream = new FileInputStream(file)) {
            parser.parse(stream, handler, metadata);
        }

        String content = handler.toString();
        String mimeType = metadata.get(Metadata.CONTENT_TYPE);

        if (mimeType == null) {
            mimeType = "application/octet-stream";
        }

        return new ExtractionData(content, mimeType);
    }

    private static String toJson(ExtractionData data, double elapsedMs) {
        StringBuilder builder = new StringBuilder();
        builder.append('{');
        builder.append("\"content\":").append(quote(data.getContent())).append(',');
        builder.append("\"metadata\":{");
        builder.append("\"mimeType\":").append(quote(data.getMimeType()));
        builder.append("},\"_extraction_time_ms\":").append(String.format("%.3f", elapsedMs));
        builder.append('}');
        return builder.toString();
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

    private static void debugLog(String key, String value) {
        if (value == null) {
            value = "(null)";
        }
        System.err.printf("[BENCHMARK_DEBUG] %-30s = %s%n", key, value);
    }

    private static class ExtractionData {
        private final String content;
        private final String mimeType;

        ExtractionData(String content, String mimeType) {
            this.content = content;
            this.mimeType = mimeType;
        }

        String getContent() {
            return content;
        }

        String getMimeType() {
            return mimeType;
        }
    }
}
/* CHECKSTYLE:ON */
