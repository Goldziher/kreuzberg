```java
import com.fasterxml.jackson.databind.ObjectMapper;
import java.io.*;
import java.util.Map;

public class McpClient {
    private final Process mcpProcess;
    private final BufferedWriter stdin;
    private final BufferedReader stdout;
    private final ObjectMapper mapper = new ObjectMapper();

    public McpClient() throws IOException {
        ProcessBuilder pb = new ProcessBuilder("kreuzberg", "mcp");
        mcpProcess = pb.start();
        stdin = new BufferedWriter(new OutputStreamWriter(mcpProcess.getOutputStream()));
        stdout = new BufferedReader(new InputStreamReader(mcpProcess.getInputStream()));
    }

    public String extractFile(String path) throws IOException {
        Map<String, Object> request = Map.of(
            "method", "tools/call",
            "params", Map.of(
                "name", "extract_file",
                "arguments", Map.of("path", path, "async", true)
            )
        );

        stdin.write(mapper.writeValueAsString(request));
        stdin.newLine();
        stdin.flush();

        String response = stdout.readLine();
        Map<String, Object> result = mapper.readValue(response, Map.class);
        Map<String, Object> resultData = (Map<String, Object>) result.get("result");
        return (String) resultData.get("content");
    }

    public void close() throws IOException {
        stdin.close();
        stdout.close();
        mcpProcess.destroy();
    }

    public static void main(String[] args) {
        try (McpClient client = new McpClient()) {
            String content = client.extractFile("contract.pdf");
            System.out.println("Extracted content: " + content);
        } catch (IOException e) {
            System.err.println("Error: " + e.getMessage());
        }
    }
}
```
