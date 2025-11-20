package dev.kreuzberg;

/**
 * Interface for post-processing extraction results.
 *
 * <p>PostProcessors enrich extraction results by transforming content,
 * adding metadata, or performing additional analysis. They are applied
 * after the core extraction completes.</p>
 *
 * <h2>Usage</h2>
 * <pre>{@code
 * PostProcessor uppercaseProcessor = result -> {
 *     return result.withContent(result.content().toUpperCase());
 * };
 *
 * ExtractionResult result = Kreuzberg.extractFile("document.pdf");
 * ExtractionResult processed = uppercaseProcessor.process(result);
 * }</pre>
 *
 * <h2>Chaining</h2>
 * <pre>{@code
 * ExtractionResult result = Kreuzberg.extractFile("document.pdf");
 * result = processor1.process(result);
 * result = processor2.process(result);
 * }</pre>
 *
 * @since 4.0.0
 */
@FunctionalInterface
public interface PostProcessor {
    /**
     * Process and enrich an extraction result.
     *
     * <p>Implementations may transform the content, add metadata, or
     * perform any other modifications. The original result is not modified;
     * instead, a new result with the changes is returned.</p>
     *
     * @param result the extraction result to process
     * @return the processed result with enrichments applied
     * @throws KreuzbergException if processing fails
     */
    ExtractionResult process(ExtractionResult result) throws KreuzbergException;

    /**
     * Returns a composed post-processor that first applies this processor,
     * then applies the {@code after} processor.
     *
     * @param after the processor to apply after this one
     * @return a composed processor
     * @throws NullPointerException if after is null
     */
    default PostProcessor andThen(PostProcessor after) {
        if (after == null) {
            throw new NullPointerException("after processor must not be null");
        }
        return result -> after.process(this.process(result));
    }

    /**
     * Defines when the processor runs in the pipeline.
     *
     * @return processing stage, defaults to {@link ProcessingStage#MIDDLE}
     */
    default ProcessingStage processingStage() {
        return ProcessingStage.MIDDLE;
    }

    /**
     * Priority within the processing stage. Higher values run first.
     *
     * @return priority value (default 0)
     */
    default int priority() {
        return 0;
    }
}
