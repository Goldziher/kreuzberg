//! Base plugin trait definition.
//!
//! All plugins must implement the `Plugin` trait, which provides basic lifecycle
//! management and metadata methods.

use crate::Result;

/// Base trait that all plugins must implement.
///
/// This trait provides common functionality for plugin lifecycle management,
/// identification, and metadata.
///
/// # Thread Safety
///
/// All plugins must be `Send + Sync` to support concurrent usage across threads.
///
/// # Example
///
/// ```rust
/// use kreuzberg::plugins::Plugin;
/// use kreuzberg::Result;
///
/// struct MyPlugin {
///     initialized: bool,
/// }
///
/// impl Plugin for MyPlugin {
///     fn name(&self) -> &str {
///         "my-plugin"
///     }
///
///     fn version(&self) -> &str {
///         "1.0.0"
///     }
///
///     fn initialize(&mut self) -> Result<()> {
///         self.initialized = true;
///         println!("Plugin initialized!");
///         Ok(())
///     }
///
///     fn shutdown(&mut self) -> Result<()> {
///         self.initialized = false;
///         println!("Plugin shutdown!");
///         Ok(())
///     }
/// }
/// ```
pub trait Plugin: Send + Sync {
    /// Returns the unique name/identifier for this plugin.
    ///
    /// The name should be:
    /// - Unique across all plugins
    /// - Lowercase with hyphens (e.g., "my-custom-plugin")
    /// - URL-safe characters only
    ///
    /// # Example
    ///
    /// ```rust
    /// # use kreuzberg::plugins::Plugin;
    /// # use kreuzberg::Result;
    /// # struct MyPlugin;
    /// # impl Plugin for MyPlugin {
    /// #     fn version(&self) -> &str { "1.0.0" }
    /// #     fn initialize(&mut self) -> Result<()> { Ok(()) }
    /// #     fn shutdown(&mut self) -> Result<()> { Ok(()) }
    /// fn name(&self) -> &str {
    ///     "pdf-extractor"
    /// }
    /// # }
    /// ```
    fn name(&self) -> &str;

    /// Returns the semantic version of this plugin.
    ///
    /// Should follow semver format: `MAJOR.MINOR.PATCH`
    ///
    /// # Example
    ///
    /// ```rust
    /// # use kreuzberg::plugins::Plugin;
    /// # use kreuzberg::Result;
    /// # struct MyPlugin;
    /// # impl Plugin for MyPlugin {
    /// #     fn name(&self) -> &str { "my-plugin" }
    /// #     fn initialize(&mut self) -> Result<()> { Ok(()) }
    /// #     fn shutdown(&mut self) -> Result<()> { Ok(()) }
    /// fn version(&self) -> &str {
    ///     "1.2.3"
    /// }
    /// # }
    /// ```
    fn version(&self) -> &str;

    /// Initialize the plugin.
    ///
    /// Called once when the plugin is registered. Use this to:
    /// - Load configuration
    /// - Initialize resources (connections, caches, etc.)
    /// - Validate dependencies
    ///
    /// # Errors
    ///
    /// Should return an error if initialization fails. The plugin will not be
    /// registered if this method returns an error.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use kreuzberg::plugins::Plugin;
    /// # use kreuzberg::{Result, KreuzbergError};
    /// # struct MyPlugin { config: Option<String> }
    /// # impl Plugin for MyPlugin {
    /// #     fn name(&self) -> &str { "my-plugin" }
    /// #     fn version(&self) -> &str { "1.0.0" }
    /// #     fn shutdown(&mut self) -> Result<()> { Ok(()) }
    /// fn initialize(&mut self) -> Result<()> {
    ///     // Load configuration
    ///     self.config = Some("loaded".to_string());
    ///
    ///     // Validate dependencies
    ///     if !self.check_dependencies() {
    ///         return Err(KreuzbergError::MissingDependency(
    ///             "Required dependency not found".to_string()
    ///         ));
    ///     }
    ///
    ///     Ok(())
    /// }
    /// # fn check_dependencies(&self) -> bool { true }
    /// # }
    /// ```
    fn initialize(&mut self) -> Result<()>;

    /// Shutdown the plugin.
    ///
    /// Called when the plugin is being unregistered or the application is shutting down.
    /// Use this to:
    /// - Close connections
    /// - Flush caches
    /// - Release resources
    ///
    /// # Errors
    ///
    /// Errors during shutdown are logged but don't prevent the shutdown process.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use kreuzberg::plugins::Plugin;
    /// # use kreuzberg::Result;
    /// # struct MyPlugin { cache: Option<Vec<String>> }
    /// # impl Plugin for MyPlugin {
    /// #     fn name(&self) -> &str { "my-plugin" }
    /// #     fn version(&self) -> &str { "1.0.0" }
    /// #     fn initialize(&mut self) -> Result<()> { Ok(()) }
    /// fn shutdown(&mut self) -> Result<()> {
    ///     // Flush caches
    ///     if let Some(cache) = &self.cache {
    ///         // Persist cache to disk
    ///     }
    ///
    ///     // Clear resources
    ///     self.cache = None;
    ///
    ///     Ok(())
    /// }
    /// # }
    /// ```
    fn shutdown(&mut self) -> Result<()>;

    /// Optional plugin description for debugging and logging.
    ///
    /// Defaults to empty string if not overridden.
    fn description(&self) -> &str {
        ""
    }

    /// Optional plugin author information.
    ///
    /// Defaults to empty string if not overridden.
    fn author(&self) -> &str {
        ""
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestPlugin {
        initialized: bool,
    }

    impl Plugin for TestPlugin {
        fn name(&self) -> &str {
            "test-plugin"
        }

        fn version(&self) -> &str {
            "1.0.0"
        }

        fn initialize(&mut self) -> Result<()> {
            self.initialized = true;
            Ok(())
        }

        fn shutdown(&mut self) -> Result<()> {
            self.initialized = false;
            Ok(())
        }

        fn description(&self) -> &str {
            "A test plugin"
        }

        fn author(&self) -> &str {
            "Test Author"
        }
    }

    #[test]
    fn test_plugin_metadata() {
        let plugin = TestPlugin { initialized: false };
        assert_eq!(plugin.name(), "test-plugin");
        assert_eq!(plugin.version(), "1.0.0");
        assert_eq!(plugin.description(), "A test plugin");
        assert_eq!(plugin.author(), "Test Author");
    }

    #[test]
    fn test_plugin_lifecycle() {
        let mut plugin = TestPlugin { initialized: false };

        assert!(!plugin.initialized);

        plugin.initialize().unwrap();
        assert!(plugin.initialized);

        plugin.shutdown().unwrap();
        assert!(!plugin.initialized);
    }
}
