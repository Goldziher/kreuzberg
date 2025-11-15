package dev.kreuzberg;

import java.util.Objects;

/**
 * Exception thrown when a plugin operation fails.
 *
 * <p>Plugin errors occur in custom plugins (postprocessors, validators, OCR backends), such as:</p>
 * <ul>
 *   <li>Plugin initialization failures</li>
 *   <li>Plugin processing errors</li>
 *   <li>Plugin crashes or timeouts</li>
 *   <li>Invalid plugin configuration</li>
 * </ul>
 *
 * <p>The error message includes the plugin name to help identify which plugin failed.</p>
 *
 * @since 4.0.0
 */
public final class PluginException extends KreuzbergException {
    private final String pluginName;

    /**
     * Constructs a new plugin exception with the specified plugin name and message.
     *
     * @param pluginName the name of the plugin that threw the error (must not be null)
     * @param message the detail message explaining why the plugin failed
     * @throws NullPointerException if pluginName is null
     */
    public PluginException(String pluginName, String message) {
        super("Plugin error in '" + Objects.requireNonNull(pluginName, "pluginName must not be null")
                + "': " + message);
        this.pluginName = pluginName;
    }

    /**
     * Constructs a new plugin exception with the specified plugin name, message and cause.
     *
     * @param pluginName the name of the plugin that threw the error (must not be null)
     * @param message the detail message
     * @param cause the cause of the plugin failure
     * @throws NullPointerException if pluginName is null
     */
    public PluginException(String pluginName, String message, Throwable cause) {
        super("Plugin error in '" + Objects.requireNonNull(pluginName, "pluginName must not be null")
                + "': " + message, cause);
        this.pluginName = pluginName;
    }

    /**
     * Returns the name of the plugin that threw this error.
     *
     * @return the plugin name
     */
    public String getPluginName() {
        return pluginName;
    }
}
