```java title="Java"
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionResult;
import dev.kreuzberg.config.ExtractionConfig;
import dev.kreuzberg.config.ChunkingConfig;
import dev.kreuzberg.config.EmbeddingConfig;
import dev.kreuzberg.config.EmbeddingModelType;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

public class VectorDatabaseIntegration {
    public static class VectorRecord {
        public String id;
        public float[] embedding;
        public String content;
        public Map<String, String> metadata;
    }

    public static List<VectorRecord> extractAndVectorize(String documentPath, String documentId) throws Exception {
        ExtractionConfig config = ExtractionConfig.builder()
            .chunking(ChunkingConfig.builder()
                .maxChars(512)
                .maxOverlap(50)
                .embedding(EmbeddingConfig.builder()
                    .model(EmbeddingModelType.preset("balanced"))
                    .normalize(true)
                    .batchSize(32)
                    .build())
                .build())
            .build();

        ExtractionResult result = Kreuzberg.extractFile(documentPath, config);
        List<Object> chunks = result.getChunks() != null ? result.getChunks() : List.of();

        List<VectorRecord> vectorRecords = new java.util.ArrayList<>();
        for (int index = 0; index < chunks.size(); index++) {
            VectorRecord record = new VectorRecord();
            record.id = documentId + "_chunk_" + index;
            record.metadata = new HashMap<>();
            record.metadata.put("document_id", documentId);
            record.metadata.put("chunk_index", String.valueOf(index));

            if (chunk instanceof java.util.Map) {
                Map<String, Object> chunkMap = (Map<String, Object>) chunks.get(index);
                record.content = (String) chunkMap.get("content");
                record.embedding = (float[]) chunkMap.get("embedding");
                record.metadata.put("content_length", String.valueOf(record.content.length()));
            }

            vectorRecords.add(record);
        }

        storeInVectorDatabase(vectorRecords);
        return vectorRecords;
    }

    private static void storeInVectorDatabase(List<VectorRecord> records) {
        for (VectorRecord record : records) {
            if (record.embedding != null && record.embedding.length > 0) {
                System.out.println("Storing " + record.id + ": " + record.content.length()
                    + " chars, " + record.embedding.length + " dims");
            }
        }
    }
}
```
