package dev.kreuzberg;

import java.util.Objects;
import java.util.Optional;

/**
 * Metadata describing where a chunk appears within the original document.
 */
public final class ChunkMetadata {
    private final int charStart;
    private final int charEnd;
    private final Integer tokenCount;
    private final int chunkIndex;
    private final int totalChunks;

    public ChunkMetadata(int charStart, int charEnd, Integer tokenCount, int chunkIndex, int totalChunks) {
        if (charStart < 0 || charEnd < charStart) {
            throw new IllegalArgumentException("Invalid chunk character range: " + charStart + "-" + charEnd);
        }
        if (chunkIndex < 0) {
            throw new IllegalArgumentException("chunkIndex must be non-negative");
        }
        if (totalChunks < 1) {
            throw new IllegalArgumentException("totalChunks must be positive");
        }
        this.charStart = charStart;
        this.charEnd = charEnd;
        this.tokenCount = tokenCount;
        this.chunkIndex = chunkIndex;
        this.totalChunks = totalChunks;
    }

    public int getCharStart() {
        return charStart;
    }

    public int getCharEnd() {
        return charEnd;
    }

    public Optional<Integer> getTokenCount() {
        return Optional.ofNullable(tokenCount);
    }

    public int getChunkIndex() {
        return chunkIndex;
    }

    public int getTotalChunks() {
        return totalChunks;
    }

    @Override
    public boolean equals(Object obj) {
        if (this == obj) {
            return true;
        }
        if (!(obj instanceof ChunkMetadata)) {
            return false;
        }
        ChunkMetadata other = (ChunkMetadata) obj;
        return charStart == other.charStart
            && charEnd == other.charEnd
            && Objects.equals(tokenCount, other.tokenCount)
            && chunkIndex == other.chunkIndex
            && totalChunks == other.totalChunks;
    }

    @Override
    public int hashCode() {
        return Objects.hash(charStart, charEnd, tokenCount, chunkIndex, totalChunks);
    }

    @Override
    public String toString() {
        return "ChunkMetadata{"
            + "charStart=" + charStart
            + ", charEnd=" + charEnd
            + ", tokenCount=" + tokenCount
            + ", chunkIndex=" + chunkIndex
            + ", totalChunks=" + totalChunks
            + '}';
    }
}
