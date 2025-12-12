```java title="Java"
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.nio.file.Files;
import java.nio.file.Path;

public class ExtractWithConfigClient {
    public static void main(String[] args) throws Exception {
        HttpClient client = HttpClient.newHttpClient();
        String boundary = "----WebKitFormBoundary" + System.currentTimeMillis();

        byte[] fileData = Files.readAllBytes(Path.of("scanned.pdf"));
        String configJson = "{\"ocr\":{\"language\":\"eng\"},\"force_ocr\":true}";
        String multipartWithConfig = "--" + boundary + "\r\n"
            + "Content-Disposition: form-data; name=\"files\"; filename=\"scanned.pdf\"\r\n"
            + "Content-Type: application/pdf\r\n\r\n"
            + new String(fileData, java.nio.charset.StandardCharsets.ISO_8859_1) + "\r\n"
            + "--" + boundary + "\r\n"
            + "Content-Disposition: form-data; name=\"config\"\r\n\r\n"
            + configJson + "\r\n"
            + "--" + boundary + "--\r\n";

        HttpRequest request = HttpRequest.newBuilder()
            .uri(URI.create("http://localhost:8000/extract"))
            .header("Content-Type", "multipart/form-data; boundary=" + boundary)
            .POST(HttpRequest.BodyPublishers.ofString(multipartWithConfig))
            .build();

        HttpResponse<String> response = client.send(request, HttpResponse.BodyHandlers.ofString());
        System.out.println(response.body());
    }
}
```
