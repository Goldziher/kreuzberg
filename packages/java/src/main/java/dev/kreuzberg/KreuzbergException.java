package dev.kreuzberg;

/**
 * Exception thrown when Kreuzberg extraction operations fail.
 *
 * <p>This exception wraps errors from the native Kreuzberg library,
 * including parsing errors, unsupported formats, and internal errors.</p>
 */
public class KreuzbergException extends Exception {
    /**
     * Creates a new KreuzbergException with the specified message.
     *
     * @param message the error message
     */
    public KreuzbergException(String message) {
        super(message);
    }

    /**
     * Creates a new KreuzbergException with the specified message and cause.
     *
     * @param message the error message
     * @param cause the underlying cause
     */
    public KreuzbergException(String message, Throwable cause) {
        super(message, cause);
    }

    /**
     * Exception thrown when a required dependency is missing.
     */
    public static class MissingDependency extends KreuzbergException {
        private final String dependency;

        /**
         * Creates a new MissingDependency exception.
         *
         * @param message the error message
         * @param dependency the name of the missing dependency
         */
        public MissingDependency(String message, String dependency) {
            super(message);
            this.dependency = dependency;
        }

        /**
         * Gets the name of the missing dependency.
         *
         * @return the dependency name
         */
        public String getDependency() {
            return dependency;
        }
    }

    /**
     * Exception thrown when document parsing fails.
     */
    public static class ParsingError extends KreuzbergException {
        /**
         * Creates a new ParsingError exception.
         *
         * @param message the error message
         */
        public ParsingError(String message) {
            super(message);
        }

        /**
         * Creates a new ParsingError exception with a cause.
         *
         * @param message the error message
         * @param cause the underlying cause
         */
        public ParsingError(String message, Throwable cause) {
            super(message, cause);
        }
    }

    /**
     * Exception thrown when OCR processing fails.
     */
    public static class OcrError extends KreuzbergException {
        /**
         * Creates a new OcrError exception.
         *
         * @param message the error message
         */
        public OcrError(String message) {
            super(message);
        }

        /**
         * Creates a new OcrError exception with a cause.
         *
         * @param message the error message
         * @param cause the underlying cause
         */
        public OcrError(String message, Throwable cause) {
            super(message, cause);
        }
    }

    /**
     * Exception thrown when a plugin operation fails.
     */
    public static class PluginError extends KreuzbergException {
        /**
         * Creates a new PluginError exception.
         *
         * @param message the error message
         */
        public PluginError(String message) {
            super(message);
        }

        /**
         * Creates a new PluginError exception with a cause.
         *
         * @param message the error message
         * @param cause the underlying cause
         */
        public PluginError(String message, Throwable cause) {
            super(message, cause);
        }
    }
}
