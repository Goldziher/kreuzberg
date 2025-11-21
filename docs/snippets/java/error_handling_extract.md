```java
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.net.URI;
import com.fasterxml.jackson.databind.ObjectMapper;

try {
    HttpClient client = HttpClient.newHttpClient();
    HttpRequest request = HttpRequest.newBuilder()
        .uri(URI.create("http://localhost:8000/extract"))
        .POST(HttpRequest.BodyPublishers.ofString(multipartBody))
        .build();

    HttpResponse<String> response = client.send(request,
        HttpResponse.BodyHandlers.ofString());

    if (response.statusCode() >= 400) {
        ObjectMapper mapper = new ObjectMapper();
        Map<String, Object> error = mapper.readValue(response.body(), Map.class);
        System.err.println("Error: " + error.get("error_type") +
            ": " + error.get("message"));
    } else {
        ObjectMapper mapper = new ObjectMapper();
        Map[] results = mapper.readValue(response.body(), Map[].class);
        // Process results
    }
} catch (IOException | InterruptedException e) {
    System.err.println("Request failed: " + e.getMessage());
}
```
