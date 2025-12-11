```java
import java.io.BufferedReader;
import java.io.IOException;
import java.io.InputStreamReader;

public final class BasicCli {
  private BasicCli() {}

  public static String extractWithCli(String filePath, String outputFormat) throws IOException, InterruptedException {
    ProcessBuilder pb = new ProcessBuilder("kreuzberg", "extract", filePath, "--format", outputFormat);
    pb.redirectErrorStream(true);

    Process process = pb.start();

    StringBuilder output = new StringBuilder();
    try (BufferedReader reader = new BufferedReader(new InputStreamReader(process.getInputStream()))) {
      String line;
      while ((line = reader.readLine()) != null) {
        output.append(line).append("\n");
      }
    }

    int exitCode = process.waitFor();
    if (exitCode != 0) {
      throw new RuntimeException("CLI exited with code " + exitCode + ": " + output);
    }

    return output.toString().trim();
  }

  public static void main(String[] args) throws IOException, InterruptedException {
    String document = "document.pdf";

    String textOutput = extractWithCli(document, "text");
    System.out.println("Extracted: " + textOutput.length() + " characters");

    String jsonOutput = extractWithCli(document, "json");
    System.out.println("JSON output received: " + jsonOutput.length() + " bytes");
  }
}
```
