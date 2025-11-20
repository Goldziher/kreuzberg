package dev.kreuzberg;

/**
 * Processing stage for post-processors.
 *
 * <p>Controls when a post-processor is applied in the extraction pipeline.</p>
 */
public enum ProcessingStage {
    /**
     * Runs before built-in enrichments.
     */
    EARLY("early"),

    /**
     * Runs after core extraction (default).
     */
    MIDDLE("middle"),

    /**
     * Runs after other processors have completed.
     */
    LATE("late");

    private final String wireName;

    ProcessingStage(String wireName) {
        this.wireName = wireName;
    }

    String wireName() {
        return wireName;
    }
}
