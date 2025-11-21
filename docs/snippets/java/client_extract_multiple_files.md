```java
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.nio.file.Files;
import java.nio.file.Path;

public class ExtractMultipleFilesClient {
    public static void main(String[] args) throws Exception {
        HttpClient client = HttpClient.newHttpClient();
        String boundary = "----WebKitFormBoundary" + System.currentTimeMillis();

        byte[] fileData1 = Files.readAllBytes(Path.of("doc1.pdf"));
        byte[] fileData2 = Files.readAllBytes(Path.of("doc2.docx"));

        String multipartBody = "--" + boundary + "\r\n"
            + "Content-Disposition: form-data; name=\"files\"; filename=\"doc1.pdf\"\r\n"
            + "Content-Type: application/pdf\r\n\r\n"
            + new String(fileData1, java.nio.charset.StandardCharsets.ISO_8859_1) + "\r\n"
            + "--" + boundary + "\r\n"
            + "Content-Disposition: form-data; name=\"files\"; filename=\"doc2.docx\"\r\n"
            + "Content-Type: application/vnd.openxmlformats-officedocument.wordprocessingml.document\r\n\r\n"
            + new String(fileData2, java.nio.charset.StandardCharsets.ISO_8859_1) + "\r\n"
            + "--" + boundary + "--\r\n";

        HttpRequest request = HttpRequest.newBuilder()
            .uri(URI.create("http://localhost:8000/extract"))
            .header("Content-Type", "multipart/form-data; boundary=" + boundary)
            .POST(HttpRequest.BodyPublishers.ofString(multipartBody))
            .build();

        HttpResponse<String> response = client.send(request, HttpResponse.BodyHandlers.ofString());
        System.out.println(response.body());
    }
}
```
