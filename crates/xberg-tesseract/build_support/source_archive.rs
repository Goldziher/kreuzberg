use std::fs;
use std::io::{self, Read};
use std::path::Path;
use zip::ZipArchive;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ArchiveLimits {
    pub(crate) max_entries: usize,
    pub(crate) max_uncompressed_bytes: u64,
}

pub(crate) fn extract_source_archive(
    bytes: &[u8],
    destination: &Path,
    expected_root: &str,
    limits: ArchiveLimits,
) -> io::Result<()> {
    let reader = std::io::Cursor::new(bytes);
    let mut archive = ZipArchive::new(reader).map_err(invalid_archive)?;
    if archive.len() > limits.max_entries {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "source archive contains {} entries, exceeding limit {}",
                archive.len(),
                limits.max_entries
            ),
        ));
    }

    ensure_existing_directory(destination)?;
    let mut uncompressed_bytes = 0_u64;
    for index in 0..archive.len() {
        let mut file = archive.by_index(index).map_err(invalid_archive)?;
        extract_entry(
            &mut file,
            destination,
            expected_root,
            &mut uncompressed_bytes,
            limits.max_uncompressed_bytes,
        )?;
    }

    Ok(())
}

fn extract_entry<R: Read + ?Sized>(
    file: &mut zip::read::ZipFile<'_, R>,
    destination: &Path,
    expected_root: &str,
    uncompressed_bytes: &mut u64,
    max_uncompressed_bytes: u64,
) -> io::Result<()> {
    let name = file.name().map_err(invalid_archive)?.into_owned();
    reject_symlink(file, &name)?;
    let Some(target_path) = archive_target_path(file, destination, expected_root, &name)? else {
        return Ok(());
    };
    if file.is_dir() {
        fs::create_dir_all(target_path)?;
        return Ok(());
    }

    let remaining = max_uncompressed_bytes - *uncompressed_bytes;
    if file.size() > remaining {
        return Err(expanded_size_error(max_uncompressed_bytes));
    }
    let expected_size = file.size();
    let written = copy_archive_file(file, &target_path, remaining)?;
    if written != expected_size {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("source archive entry {name} produced {written} bytes, expected {expected_size}"),
        ));
    }
    *uncompressed_bytes = uncompressed_bytes
        .checked_add(written)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "source archive size overflow"))?;
    Ok(())
}

fn reject_symlink<R: Read + ?Sized>(file: &zip::read::ZipFile<'_, R>, name: &str) -> io::Result<()> {
    if file.unix_mode().is_some_and(|mode| mode & 0o170_000 == 0o120_000) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("source archive contains a symbolic link: {name}"),
        ));
    }
    Ok(())
}

fn archive_target_path<R: Read + ?Sized>(
    file: &zip::read::ZipFile<'_, R>,
    destination: &Path,
    expected_root: &str,
    name: &str,
) -> io::Result<Option<std::path::PathBuf>> {
    let enclosed_path = file.enclosed_name().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("source archive contains an unsafe path: {name}"),
        )
    })?;
    let relative_path = enclosed_path.strip_prefix(expected_root).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "source archive entry {} is outside expected root {expected_root}",
                enclosed_path.display()
            ),
        )
    })?;
    if relative_path.as_os_str().is_empty() {
        return Ok(None);
    }
    Ok(Some(destination.join(relative_path)))
}

fn copy_archive_file(file: &mut impl Read, target_path: &Path, remaining: u64) -> io::Result<u64> {
    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut output = fs::OpenOptions::new().write(true).create_new(true).open(target_path)?;
    let mut bounded = file.take(remaining.saturating_add(1));
    let written = io::copy(&mut bounded, &mut output)?;
    if written > remaining {
        return Err(expanded_size_error(remaining));
    }
    Ok(written)
}

fn ensure_existing_directory(path: &Path) -> io::Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_dir() && !metadata.file_type().is_symlink() {
        return Ok(());
    }
    Err(io::Error::new(
        io::ErrorKind::InvalidData,
        format!("source destination is not a regular directory: {}", path.display()),
    ))
}

fn expanded_size_error(limit: u64) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("source archive expands beyond {limit} bytes"),
    )
}

fn invalid_archive(error: zip::result::ZipError) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, format!("invalid source archive: {error}"))
}
