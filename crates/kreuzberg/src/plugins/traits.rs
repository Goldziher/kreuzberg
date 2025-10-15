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
/// use std::sync::atomic::{AtomicBool, Ordering};
///
/// struct MyPlugin {
///     initialized: AtomicBool,
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
///     fn initialize(&self) -> Result<()> {
///         self.initialized.store(true, Ordering::Release);
///         println!("Plugin initialized!");
///         Ok(())
///     }
///
///     fn shutdown(&self) -> Result<()> {
///         self.initialized.store(false, Ordering::Release);
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
    /// #     fn initialize(&self) -> Result<()> { Ok(()) }
    /// #     fn shutdown(&self) -> Result<()> { Ok(()) }
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
    /// #     fn initialize(&self) -> Result<()> { Ok(()) }
    /// #     fn shutdown(&self) -> Result<()> { Ok(()) }
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
    /// # Thread Safety
    ///
    /// This method takes `&self` instead of `&mut self` to work with `Arc<dyn Plugin>`.
    /// Plugins needing mutable state during initialization should use interior mutability
    /// patterns (Mutex, RwLock, OnceCell, etc.).
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
    /// # use kreuzberg::Result;
    /// # use std::sync::Mutex;
    /// # struct MyPlugin { config: Mutex<Option<String>> }
    /// # impl Plugin for MyPlugin {
    /// #     fn name(&self) -> &str { "my-plugin" }
    /// #     fn version(&self) -> &str { "1.0.0" }
    /// #     fn shutdown(&self) -> Result<()> { Ok(()) }
    /// fn initialize(&self) -> Result<()> {
    ///     // Load configuration using interior mutability
    ///     let mut config = self.config.lock().unwrap();
    ///     *config = Some("loaded".to_string());
    ///
    ///     // Perform any initialization work
    ///     println!("Plugin initialized successfully");
    ///
    ///     Ok(())
    /// }
    /// # }
    /// ```
    fn initialize(&self) -> Result<()>;

    /// Shutdown the plugin.
    ///
    /// Called when the plugin is being unregistered or the application is shutting down.
    /// Use this to:
    /// - Close connections
    /// - Flush caches
    /// - Release resources
    ///
    /// # Thread Safety
    ///
    /// This method takes `&self` instead of `&mut self` to work with `Arc<dyn Plugin>`.
    /// Plugins needing mutable state during shutdown should use interior mutability
    /// patterns (Mutex, RwLock, etc.).
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
    /// # use std::sync::Mutex;
    /// # struct MyPlugin { cache: Mutex<Option<Vec<String>>> }
    /// # impl Plugin for MyPlugin {
    /// #     fn name(&self) -> &str { "my-plugin" }
    /// #     fn version(&self) -> &str { "1.0.0" }
    /// #     fn initialize(&self) -> Result<()> { Ok(()) }
    /// fn shutdown(&self) -> Result<()> {
    ///     // Flush caches using interior mutability
    ///     let mut cache = self.cache.lock().unwrap();
    ///     if let Some(data) = cache.take() {
    ///         // Persist cache to disk
    ///     }
    ///
    ///     Ok(())
    /// }
    /// # }
    /// ```
    fn shutdown(&self) -> Result<()>;

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
    use std::sync::atomic::{AtomicBool, Ordering};

    struct TestPlugin {
        initialized: AtomicBool,
    }

    impl Plugin for TestPlugin {
        fn name(&self) -> &str {
            "test-plugin"
        }

        fn version(&self) -> &str {
            "1.0.0"
        }

        fn initialize(&self) -> Result<()> {
            self.initialized.store(true, Ordering::Release);
            Ok(())
        }

        fn shutdown(&self) -> Result<()> {
            self.initialized.store(false, Ordering::Release);
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
        let plugin = TestPlugin {
            initialized: AtomicBool::new(false),
        };
        assert_eq!(plugin.name(), "test-plugin");
        assert_eq!(plugin.version(), "1.0.0");
        assert_eq!(plugin.description(), "A test plugin");
        assert_eq!(plugin.author(), "Test Author");
    }

    #[test]
    fn test_plugin_lifecycle() {
        let plugin = TestPlugin {
            initialized: AtomicBool::new(false),
        };

        assert!(!plugin.initialized.load(Ordering::Acquire));

        plugin.initialize().unwrap();
        assert!(plugin.initialized.load(Ordering::Acquire));

        plugin.shutdown().unwrap();
        assert!(!plugin.initialized.load(Ordering::Acquire));
    }
}
