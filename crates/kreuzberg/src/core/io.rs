//! File I/O utilities.
//!
//! This module provides async and sync file reading utilities with proper error handling.

use crate::{KreuzbergError, Result};
use std::path::Path;
use tokio::fs;

/// Read a file asynchronously.
///
/// # Arguments
///
/// * `path` - Path to the file to read
///
/// # Returns
///
/// The file contents as bytes.
///
/// # Errors
///
/// Returns `KreuzbergError::Io` for I/O errors (these always bubble up).
pub async fn read_file_async(path: impl AsRef<Path>) -> Result<Vec<u8>> {
    fs::read(path.as_ref())
        .await
        .map_err(KreuzbergError::Io)
}

/// Read a file synchronously.
///
/// # Arguments
///
/// * `path` - Path to the file to read
///
/// # Returns
///
/// The file contents as bytes.
///
/// # Errors
///
/// Returns `KreuzbergError::Io` for I/O errors (these always bubble up).
pub fn read_file_sync(path: impl AsRef<Path>) -> Result<Vec<u8>> {
    std::fs::read(path.as_ref())
        .map_err(KreuzbergError::Io)
}

/// Check if a file exists.
///
/// # Arguments
///
/// * `path` - Path to check
///
/// # Returns
///
/// `true` if the file exists, `false` otherwise.
pub fn file_exists(path: impl AsRef<Path>) -> bool {
    path.as_ref().exists()
}

/// Validate that a file exists.
///
/// # Arguments
///
/// * `path` - Path to validate
///
/// # Errors
///
/// Returns `KreuzbergError::Validation` if file doesn't exist.
pub fn validate_file_exists(path: impl AsRef<Path>) -> Result<()> {
    if !file_exists(&path) {
        return Err(KreuzbergError::Validation(format!("File does not exist: {}", path.as_ref().display())));
    }
    Ok(())
}

/// Traverse a directory and return all file paths matching a pattern.
///
/// # Arguments
///
/// * `dir` - Directory to traverse
/// * `recursive` - Whether to recursively traverse subdirectories
/// * `filter` - Optional filter function to match files
///
/// # Returns
///
/// Vector of file paths that match the criteria.
///
/// # Errors
///
/// Returns `KreuzbergError::Io` for I/O errors.
pub fn traverse_directory<F>(
    dir: impl AsRef<Path>,
    recursive: bool,
    filter: Option<F>,
) -> Result<Vec<std::path::PathBuf>>
where
    F: Fn(&Path) -> bool,
{
    let dir = dir.as_ref();
    let mut files = Vec::new();

    if !dir.is_dir() {
        return Err(KreuzbergError::Validation(
            format!("Path is not a directory: {}", dir.display())
        ));
    }

    traverse_directory_impl(dir, recursive, &filter, &mut files)?;
    Ok(files)
}

fn traverse_directory_impl<F>(
    dir: &Path,
    recursive: bool,
    filter: &Option<F>,
    files: &mut Vec<std::path::PathBuf>,
) -> Result<()>
where
    F: Fn(&Path) -> bool,
{
    let entries = std::fs::read_dir(dir)
        .map_err(KreuzbergError::Io)?;

    for entry in entries {
        let entry = entry.map_err(KreuzbergError::Io)?;
        let path = entry.path();

        if path.is_file() {
            // Apply filter if provided
            let should_include = match filter {
                Some(f) => f(&path),
                None => true,
            };

            if should_include {
                files.push(path);
            }
        } else if path.is_dir() && recursive {
            // Recursively traverse subdirectories
            traverse_directory_impl(&path, recursive, filter, files)?;
        }
    }

    Ok(())
}

/// Get all files in a directory with a specific extension.
///
/// # Arguments
///
/// * `dir` - Directory to search
/// * `extension` - File extension to match (without the dot)
/// * `recursive` - Whether to recursively search subdirectories
///
/// # Returns
///
/// Vector of file paths with the specified extension.
///
/// # Errors
///
/// Returns `KreuzbergError::Io` for I/O errors.
pub fn find_files_by_extension(
    dir: impl AsRef<Path>,
    extension: &str,
    recursive: bool,
) -> Result<Vec<std::path::PathBuf>> {
    let ext = extension.to_lowercase();
    traverse_directory(dir, recursive, Some(|path: &Path| {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase() == ext)
            .unwrap_or(false)
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_read_file_async() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"test content").unwrap();

        let content = read_file_async(&file_path).await.unwrap();
        assert_eq!(content, b"test content");
    }

    #[test]
    fn test_read_file_sync() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"test content").unwrap();

        let content = read_file_sync(&file_path).unwrap();
        assert_eq!(content, b"test content");
    }

    #[test]
    fn test_file_exists() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        File::create(&file_path).unwrap();

        assert!(file_exists(&file_path));
        assert!(!file_exists(dir.path().join("nonexistent.txt")));
    }

    #[test]
    fn test_validate_file_exists() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        File::create(&file_path).unwrap();

        assert!(validate_file_exists(&file_path).is_ok());
        assert!(validate_file_exists(dir.path().join("nonexistent.txt")).is_err());
    }

    #[test]
    fn test_traverse_directory_non_recursive() {
        let dir = tempdir().unwrap();

        // Create test files
        File::create(dir.path().join("file1.txt")).unwrap();
        File::create(dir.path().join("file2.pdf")).unwrap();
        File::create(dir.path().join("file3.txt")).unwrap();

        // Create subdirectory with files (should be ignored in non-recursive mode)
        std::fs::create_dir(dir.path().join("subdir")).unwrap();
        File::create(dir.path().join("subdir").join("file4.txt")).unwrap();

        let files = traverse_directory(dir.path(), false, None::<fn(&Path) -> bool>).unwrap();
        assert_eq!(files.len(), 3);
    }

    #[test]
    fn test_traverse_directory_recursive() {
        let dir = tempdir().unwrap();

        // Create test files
        File::create(dir.path().join("file1.txt")).unwrap();
        File::create(dir.path().join("file2.pdf")).unwrap();

        // Create subdirectory with files
        std::fs::create_dir(dir.path().join("subdir")).unwrap();
        File::create(dir.path().join("subdir").join("file3.txt")).unwrap();
        File::create(dir.path().join("subdir").join("file4.pdf")).unwrap();

        let files = traverse_directory(dir.path(), true, None::<fn(&Path) -> bool>).unwrap();
        assert_eq!(files.len(), 4);
    }

    #[test]
    fn test_traverse_directory_with_filter() {
        let dir = tempdir().unwrap();

        // Create test files
        File::create(dir.path().join("file1.txt")).unwrap();
        File::create(dir.path().join("file2.pdf")).unwrap();
        File::create(dir.path().join("file3.txt")).unwrap();

        // Filter for .txt files only
        let files = traverse_directory(dir.path(), false, Some(|path: &Path| {
            path.extension()
                .and_then(|e| e.to_str())
                .map(|e| e == "txt")
                .unwrap_or(false)
        })).unwrap();

        assert_eq!(files.len(), 2);
        assert!(files.iter().all(|p| p.extension().unwrap() == "txt"));
    }

    #[test]
    fn test_find_files_by_extension() {
        let dir = tempdir().unwrap();

        // Create test files
        File::create(dir.path().join("file1.txt")).unwrap();
        File::create(dir.path().join("file2.pdf")).unwrap();
        File::create(dir.path().join("file3.TXT")).unwrap(); // Test case insensitivity

        // Create subdirectory with files
        std::fs::create_dir(dir.path().join("subdir")).unwrap();
        File::create(dir.path().join("subdir").join("file4.txt")).unwrap();

        // Non-recursive
        let files = find_files_by_extension(dir.path(), "txt", false).unwrap();
        assert_eq!(files.len(), 2);

        // Recursive
        let files_recursive = find_files_by_extension(dir.path(), "txt", true).unwrap();
        assert_eq!(files_recursive.len(), 3);
    }

    #[test]
    fn test_traverse_directory_invalid_path() {
        let result = traverse_directory("/nonexistent/directory", false, None::<fn(&Path) -> bool>);
        assert!(result.is_err());
    }

    #[test]
    fn test_traverse_directory_file_not_dir() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        File::create(&file_path).unwrap();

        let result = traverse_directory(&file_path, false, None::<fn(&Path) -> bool>);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_read_file_async_io_error() {
        let result = read_file_async("/nonexistent/file.txt").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KreuzbergError::Io(_)));
    }

    #[test]
    fn test_read_file_sync_io_error() {
        let result = read_file_sync("/nonexistent/file.txt");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KreuzbergError::Io(_)));
    }
}
