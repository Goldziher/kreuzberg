//! Security utilities for document extractors.
//!
//! This module provides validation and protection mechanisms against common attacks:
//! - ZIP bomb detection (decompression bombs)
//! - XML entity expansion limits
//! - Nesting depth limits
//! - Input size limits
//! - Entity length validation

use std::io::{Read, Seek};

/// Configuration for security limits across extractors.
///
/// All limits are intentionally conservative to prevent DoS attacks
/// while still supporting legitimate documents.
#[derive(Clone, Debug)]
pub struct SecurityLimits {
    /// Maximum uncompressed size for archives (500 MB)
    pub max_archive_size: usize,

    /// Maximum compression ratio before flagging as potential bomb (100:1)
    pub max_compression_ratio: usize,

    /// Maximum number of files in archive (10,000)
    pub max_files_in_archive: usize,

    /// Maximum nesting depth for structures (100)
    pub max_nesting_depth: usize,

    /// Maximum entity/string length (32)
    pub max_entity_length: usize,

    /// Maximum string growth per document (100 MB)
    pub max_content_size: usize,

    /// Maximum iterations per operation
    pub max_iterations: usize,

    /// Maximum XML depth (100 levels)
    pub max_xml_depth: usize,

    /// Maximum cells per table (100,000)
    pub max_table_cells: usize,
}

impl Default for SecurityLimits {
    fn default() -> Self {
        Self {
            max_archive_size: 500 * 1024 * 1024, // 500 MB
            max_compression_ratio: 100,          // 100:1
            max_files_in_archive: 10_000,        // 10k files
            max_nesting_depth: 100,              // 100 levels
            max_entity_length: 32,               // 32 chars
            max_content_size: 100 * 1024 * 1024, // 100 MB
            max_iterations: 10_000_000,          // 10M iterations
            max_xml_depth: 100,                  // 100 levels
            max_table_cells: 100_000,            // 100k cells
        }
    }
}

/// Security validation errors.
#[derive(Debug, Clone)]
pub enum SecurityError {
    /// Potential ZIP bomb detected
    ZipBombDetected {
        compressed_size: u64,
        uncompressed_size: u64,
        ratio: f64,
    },

    /// Archive exceeds maximum size
    ArchiveTooLarge { size: u64, max: usize },

    /// Archive contains too many files
    TooManyFiles { count: usize, max: usize },

    /// Nesting too deep
    NestingTooDeep { depth: usize, max: usize },

    /// Content exceeds maximum size
    ContentTooLarge { size: usize, max: usize },

    /// Entity/string too long
    EntityTooLong { length: usize, max: usize },

    /// Too many iterations
    TooManyIterations { count: usize, max: usize },

    /// XML depth exceeded
    XmlDepthExceeded { depth: usize, max: usize },

    /// Too many table cells
    TooManyCells { cells: usize, max: usize },
}

impl std::fmt::Display for SecurityError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            SecurityError::ZipBombDetected {
                compressed_size,
                uncompressed_size,
                ratio,
            } => {
                write!(
                    f,
                    "Potential ZIP bomb detected: compressed {}B -> uncompressed {}B (ratio: {:.1}:1)",
                    compressed_size, uncompressed_size, ratio
                )
            }
            SecurityError::ArchiveTooLarge { size, max } => {
                write!(f, "Archive too large: {} bytes (max: {} bytes)", size, max)
            }
            SecurityError::TooManyFiles { count, max } => {
                write!(f, "Archive has too many files: {} (max: {})", count, max)
            }
            SecurityError::NestingTooDeep { depth, max } => {
                write!(f, "Nesting too deep: {} levels (max: {})", depth, max)
            }
            SecurityError::ContentTooLarge { size, max } => {
                write!(f, "Content too large: {} bytes (max: {} bytes)", size, max)
            }
            SecurityError::EntityTooLong { length, max } => {
                write!(f, "Entity too long: {} chars (max: {})", length, max)
            }
            SecurityError::TooManyIterations { count, max } => {
                write!(f, "Too many iterations: {} (max: {})", count, max)
            }
            SecurityError::XmlDepthExceeded { depth, max } => {
                write!(f, "XML depth exceeded: {} (max: {})", depth, max)
            }
            SecurityError::TooManyCells { cells, max } => {
                write!(f, "Too many table cells: {} (max: {})", cells, max)
            }
        }
    }
}

impl std::error::Error for SecurityError {}

/// Helper struct for validating ZIP archives for security issues.
pub struct ZipBombValidator {
    limits: SecurityLimits,
}

impl ZipBombValidator {
    /// Create a new ZIP bomb validator.
    pub fn new(limits: SecurityLimits) -> Self {
        Self { limits }
    }

    /// Validate a ZIP archive for security issues.
    ///
    /// # Arguments
    /// * `archive` - Mutable ZIP archive to validate
    ///
    /// # Returns
    /// * `Ok(())` if archive is safe
    /// * `Err(SecurityError)` if security limit violated
    pub fn validate<R: Read + Seek>(&self, archive: &mut zip::ZipArchive<R>) -> Result<(), SecurityError> {
        let file_count = archive.len();

        // Check file count
        if file_count > self.limits.max_files_in_archive {
            return Err(SecurityError::TooManyFiles {
                count: file_count,
                max: self.limits.max_files_in_archive,
            });
        }

        let mut total_uncompressed: u64 = 0;
        let mut total_compressed: u64 = 0;

        // Validate each file
        for i in 0..file_count {
            if let Ok(file) = archive.by_index(i) {
                let compressed_size = file.compressed_size();
                let uncompressed_size = file.size();

                total_uncompressed += uncompressed_size;
                total_compressed += compressed_size;

                // Check for individual file compression ratio
                if compressed_size > 0 && uncompressed_size > 0 {
                    let ratio = uncompressed_size as f64 / compressed_size as f64;
                    if ratio > self.limits.max_compression_ratio as f64 {
                        return Err(SecurityError::ZipBombDetected {
                            compressed_size,
                            uncompressed_size,
                            ratio,
                        });
                    }
                }
            }
        }

        // Check total uncompressed size
        if total_uncompressed > self.limits.max_archive_size as u64 {
            return Err(SecurityError::ArchiveTooLarge {
                size: total_uncompressed,
                max: self.limits.max_archive_size,
            });
        }

        // Check overall compression ratio
        if total_compressed > 0 {
            let ratio = total_uncompressed as f64 / total_compressed as f64;
            if ratio > self.limits.max_compression_ratio as f64 {
                return Err(SecurityError::ZipBombDetected {
                    compressed_size: total_compressed,
                    uncompressed_size: total_uncompressed,
                    ratio,
                });
            }
        }

        Ok(())
    }
}

/// Helper struct for tracking and validating string growth.
pub struct StringGrowthValidator {
    max_size: usize,
    current_size: usize,
}

impl StringGrowthValidator {
    /// Create a new string growth validator.
    pub fn new(max_size: usize) -> Self {
        Self {
            max_size,
            current_size: 0,
        }
    }

    /// Validate and update size after appending.
    ///
    /// # Returns
    /// * `Ok(())` if size is within limits
    /// * `Err(SecurityError)` if size exceeds limit
    pub fn check_append(&mut self, len: usize) -> Result<(), SecurityError> {
        self.current_size = self.current_size.saturating_add(len);
        if self.current_size > self.max_size {
            Err(SecurityError::ContentTooLarge {
                size: self.current_size,
                max: self.max_size,
            })
        } else {
            Ok(())
        }
    }

    /// Get current size.
    pub fn current_size(&self) -> usize {
        self.current_size
    }
}

/// Helper struct for validating iteration counts.
pub struct IterationValidator {
    max_iterations: usize,
    current_count: usize,
}

impl IterationValidator {
    /// Create a new iteration validator.
    pub fn new(max_iterations: usize) -> Self {
        Self {
            max_iterations,
            current_count: 0,
        }
    }

    /// Validate and increment iteration count.
    ///
    /// # Returns
    /// * `Ok(())` if count is within limits
    /// * `Err(SecurityError)` if count exceeds limit
    pub fn check_iteration(&mut self) -> Result<(), SecurityError> {
        self.current_count += 1;
        if self.current_count > self.max_iterations {
            Err(SecurityError::TooManyIterations {
                count: self.current_count,
                max: self.max_iterations,
            })
        } else {
            Ok(())
        }
    }

    /// Get current iteration count.
    pub fn current_count(&self) -> usize {
        self.current_count
    }
}

/// Helper struct for validating nesting depth.
pub struct DepthValidator {
    max_depth: usize,
    current_depth: usize,
}

impl DepthValidator {
    /// Create a new depth validator.
    pub fn new(max_depth: usize) -> Self {
        Self {
            max_depth,
            current_depth: 0,
        }
    }

    /// Push a level (increase depth).
    ///
    /// # Returns
    /// * `Ok(())` if depth is within limits
    /// * `Err(SecurityError)` if depth exceeds limit
    pub fn push(&mut self) -> Result<(), SecurityError> {
        self.current_depth += 1;
        if self.current_depth > self.max_depth {
            Err(SecurityError::NestingTooDeep {
                depth: self.current_depth,
                max: self.max_depth,
            })
        } else {
            Ok(())
        }
    }

    /// Pop a level (decrease depth).
    pub fn pop(&mut self) {
        if self.current_depth > 0 {
            self.current_depth -= 1;
        }
    }

    /// Get current depth.
    pub fn current_depth(&self) -> usize {
        self.current_depth
    }
}

/// Helper struct for validating entity/string length.
pub struct EntityValidator {
    max_length: usize,
}

impl EntityValidator {
    /// Create a new entity validator.
    pub fn new(max_length: usize) -> Self {
        Self { max_length }
    }

    /// Validate entity length.
    ///
    /// # Returns
    /// * `Ok(())` if length is within limits
    /// * `Err(SecurityError)` if length exceeds limit
    pub fn validate(&self, content: &str) -> Result<(), SecurityError> {
        if content.len() > self.max_length {
            Err(SecurityError::EntityTooLong {
                length: content.len(),
                max: self.max_length,
            })
        } else {
            Ok(())
        }
    }
}

/// Helper struct for validating table cell counts.
pub struct TableValidator {
    max_cells: usize,
    current_cells: usize,
}

impl TableValidator {
    /// Create a new table validator.
    pub fn new(max_cells: usize) -> Self {
        Self {
            max_cells,
            current_cells: 0,
        }
    }

    /// Add cells to table and validate.
    ///
    /// # Returns
    /// * `Ok(())` if cell count is within limits
    /// * `Err(SecurityError)` if cell count exceeds limit
    pub fn add_cells(&mut self, count: usize) -> Result<(), SecurityError> {
        self.current_cells = self.current_cells.saturating_add(count);
        if self.current_cells > self.max_cells {
            Err(SecurityError::TooManyCells {
                cells: self.current_cells,
                max: self.max_cells,
            })
        } else {
            Ok(())
        }
    }

    /// Get current cell count.
    pub fn current_cells(&self) -> usize {
        self.current_cells
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_depth_validator() {
        let mut validator = DepthValidator::new(3);

        // First push should succeed
        assert!(validator.push().is_ok());
        assert_eq!(validator.current_depth(), 1);

        // Second push should succeed
        assert!(validator.push().is_ok());
        assert_eq!(validator.current_depth(), 2);

        // Third push should succeed
        assert!(validator.push().is_ok());
        assert_eq!(validator.current_depth(), 3);

        // Fourth push should fail
        assert!(validator.push().is_err());
        assert_eq!(validator.current_depth(), 4);

        // Pop should decrease
        validator.pop();
        assert_eq!(validator.current_depth(), 3);
    }

    #[test]
    fn test_entity_validator() {
        let validator = EntityValidator::new(10);

        // Valid entity
        assert!(validator.validate("short").is_ok());

        // Boundary case
        assert!(validator.validate("0123456789").is_ok());

        // Too long
        assert!(validator.validate("01234567890").is_err());
    }

    #[test]
    fn test_string_growth_validator() {
        let mut validator = StringGrowthValidator::new(100);

        assert!(validator.check_append(50).is_ok());
        assert_eq!(validator.current_size(), 50);

        assert!(validator.check_append(50).is_ok());
        assert_eq!(validator.current_size(), 100);

        assert!(validator.check_append(1).is_err());
    }

    #[test]
    fn test_iteration_validator() {
        let mut validator = IterationValidator::new(3);

        assert!(validator.check_iteration().is_ok());
        assert!(validator.check_iteration().is_ok());
        assert!(validator.check_iteration().is_ok());
        assert!(validator.check_iteration().is_err());
    }

    #[test]
    fn test_table_validator() {
        let mut validator = TableValidator::new(10);

        assert!(validator.add_cells(5).is_ok());
        assert_eq!(validator.current_cells(), 5);

        assert!(validator.add_cells(5).is_ok());
        assert_eq!(validator.current_cells(), 10);

        assert!(validator.add_cells(1).is_err());
    }

    #[test]
    fn test_default_limits() {
        let limits = SecurityLimits::default();
        assert_eq!(limits.max_archive_size, 500 * 1024 * 1024);
        assert_eq!(limits.max_nesting_depth, 100);
        assert_eq!(limits.max_entity_length, 32);
    }
}
