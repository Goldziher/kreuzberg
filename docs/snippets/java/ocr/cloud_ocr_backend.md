```java title="Java"
import dev.kreuzberg.*;
import java.lang.foreign.Arena;
import java.lang.foreign.MemorySegment;
import java.lang.foreign.ValueLayout;
import java.net.http.*;
import java.net.URI;

public class CloudOcrExample {
    public static void main(String[] args) {
        Arena callbackArena = Arena.ofAuto();
        String apiKey = "your-api-key";

        OcrBackend cloudOcr = (imageBytes, imageLength, configJson) -> {
            try {
                // Read image bytes from native memory
                byte[] image = imageBytes.reinterpret(imageLength)
                    .toArray(ValueLayout.JAVA_BYTE);

                // Read config JSON
                String config = configJson.reinterpret(Long.MAX_VALUE)
                    .getString(0);

                // Call cloud OCR API
                HttpClient client = HttpClient.newHttpClient();
                HttpRequest request = HttpRequest.newBuilder()
                    .uri(URI.create("https://api.example.com/ocr"))
                    .header("Authorization", "Bearer " + apiKey)
                    .POST(HttpRequest.BodyPublishers.ofByteArray(image))
                    .build();

                HttpResponse<String> response = client.send(request,
                    HttpResponse.BodyHandlers.ofString());

                String text = parseTextFromResponse(response.body());

                // Return result as C string
                return callbackArena.allocateFrom(text);
            } catch (Exception e) {
                return MemorySegment.NULL;
            }
        };

        try (Arena arena = Arena.ofConfined()) {
            Kreuzberg.registerOcrBackend("cloud-ocr", cloudOcr, arena);

            // Use custom OCR backend in extraction
            // Note: Requires ExtractionConfig with OCR enabled
            ExtractionResult result = Kreuzberg.extractFileSync("scanned.pdf");
        } catch (Exception e) {
            e.printStackTrace();
        }
    }

    private static String parseTextFromResponse(String json) {
        // Parse JSON response and extract text field
        return json; // Simplified
    }
}
```
