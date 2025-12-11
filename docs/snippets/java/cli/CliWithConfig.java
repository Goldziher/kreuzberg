```java
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.io.BufferedReader;
import java.io.IOException;
import java.io.InputStreamReader;

public final class CliWithConfig {
  private static final ObjectMapper MAPPER = new ObjectMapper();

  private CliWithConfig() {}

  public static JsonNode extractWithConfig(String filePath, String configPath)
      throws IOException, InterruptedException {
    ProcessBuilder pb = new ProcessBuilder(
        "kreuzberg",
        "extract",
        filePath,
        "--config",
        configPath,
        "--format",
        "json");
    pb.redirectErrorStream(true);

    Process process = pb.start();

    StringBuilder output = new StringBuilder();
    try (BufferedReader reader = new BufferedReader(new InputStreamReader(process.getInputStream()))) {
      String line;
      while ((line = reader.readLine()) != null) {
        output.append(line);
      }
    }

    int exitCode = process.waitFor();
    if (exitCode != 0) {
      throw new RuntimeException("CLI exited with code " + exitCode + ": " + output);
    }

    return MAPPER.readTree(output.toString());
  }

  public static void main(String[] args) throws IOException, InterruptedException {
    String configFile = "kreuzberg.toml";
    String document = "document.pdf";

    System.out.println("Extracting " + document + " with config " + configFile);
    JsonNode result = extractWithConfig(document, configFile);

    String content = result.get("content").asText();
    System.out.println("Content length: " + content.length());
    System.out.println("Format: " + result.get("format").asText());
    System.out.println("Languages: " + result.get("languages").toString());
  }
}
```
