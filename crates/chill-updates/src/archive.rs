use crate::{Result, UpdateError};
use std::collections::BTreeSet;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read};
use std::path::{Component as PathComponent, Path, PathBuf};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArchiveKind {
    Raw,
    Zip,
    Tar,
}

#[derive(Clone, Debug)]
pub struct ExtractionLimits {
    pub max_entries: usize,
    pub max_file_bytes: u64,
    pub max_total_bytes: u64,
    pub max_compression_ratio: u64,
}

impl Default for ExtractionLimits {
    fn default() -> Self {
        Self {
            max_entries: 10_000,
            max_file_bytes: 512 * 1024 * 1024,
            max_total_bytes: 1024 * 1024 * 1024,
            max_compression_ratio: 200,
        }
    }
}

/// Extract into a brand-new directory. Existing targets are never merged or
/// overwritten, and only regular files/directories are accepted.
pub fn extract_archive(
    archive: &Path,
    destination: &Path,
    kind: ArchiveKind,
    limits: &ExtractionLimits,
) -> Result<()> {
    if fs::symlink_metadata(destination).is_ok() {
        return Err(UpdateError::Security(
            "staging destination already exists".into(),
        ));
    }
    let parent = destination
        .parent()
        .ok_or_else(|| UpdateError::Invalid("staging path has no parent".into()))?;
    reject_symlink_path(parent)?;
    fs::create_dir(destination)?;
    let result = match kind {
        ArchiveKind::Raw => extract_raw(archive, destination, limits),
        ArchiveKind::Zip => extract_zip(archive, destination, limits),
        ArchiveKind::Tar => extract_tar(archive, destination, limits),
    };
    if result.is_err() {
        // This directory was created by this call and has not been activated.
        let _ = fs::remove_dir_all(destination);
    }
    result
}

fn extract_raw(source: &Path, destination: &Path, limits: &ExtractionLimits) -> Result<()> {
    let metadata = source.metadata()?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > limits.max_file_bytes {
        return Err(UpdateError::Archive(
            "raw payload size is outside bounds".into(),
        ));
    }
    let mut input = File::open(source)?;
    let mut output = create_new_file(&destination.join("payload"))?;
    copy_exact_bounded(
        &mut input,
        &mut output,
        metadata.len(),
        limits.max_file_bytes,
    )?;
    output.sync_all()?;
    Ok(())
}

fn extract_zip(source: &Path, destination: &Path, limits: &ExtractionLimits) -> Result<()> {
    let file = File::open(source)?;
    let mut zip =
        zip::ZipArchive::new(file).map_err(|error| UpdateError::Archive(error.to_string()))?;
    if zip.is_empty() || zip.len() > limits.max_entries {
        return Err(UpdateError::Archive(
            "ZIP entry count is outside bounds".into(),
        ));
    }
    let mut total = 0u64;
    let mut seen = BTreeSet::new();
    for index in 0..zip.len() {
        let mut entry = zip
            .by_index(index)
            .map_err(|error| UpdateError::Archive(error.to_string()))?;
        let relative = entry
            .enclosed_name()
            .ok_or_else(|| UpdateError::Security("ZIP traversal path".into()))?;
        validate_relative_path(&relative)?;
        if !seen.insert(relative.to_path_buf()) {
            return Err(UpdateError::Security("duplicate archive path".into()));
        }
        if let Some(mode) = entry.unix_mode() {
            let file_type = mode & 0o170000;
            if file_type != 0 && file_type != 0o040000 && file_type != 0o100000 {
                return Err(UpdateError::Security(
                    "ZIP link or special file refused".into(),
                ));
            }
        }
        let output_path = destination.join(relative);
        if entry.is_dir() {
            create_dir_checked(destination, &output_path)?;
            continue;
        }
        let size = entry.size();
        if size > limits.max_file_bytes {
            return Err(UpdateError::Archive("ZIP file is too large".into()));
        }
        let compressed = entry.compressed_size();
        if compressed == 0 && size > 0
            || compressed > 0 && size / compressed.max(1) > limits.max_compression_ratio
        {
            return Err(UpdateError::Archive(
                "ZIP compression ratio is unsafe".into(),
            ));
        }
        total = total
            .checked_add(size)
            .ok_or_else(|| UpdateError::Archive("ZIP size overflow".into()))?;
        if total > limits.max_total_bytes {
            return Err(UpdateError::Archive(
                "ZIP expanded size exceeds limit".into(),
            ));
        }
        if let Some(parent) = output_path.parent() {
            create_dir_checked(destination, parent)?;
        }
        let mut output = create_new_file(&output_path)?;
        copy_exact_bounded(&mut entry, &mut output, size, limits.max_file_bytes)?;
        output.sync_all()?;
    }
    sync_tree_root(destination)
}

fn extract_tar(source: &Path, destination: &Path, limits: &ExtractionLimits) -> Result<()> {
    let mut input = File::open(source)?;
    let archive_length = input.metadata()?.len();
    if archive_length == 0 || archive_length > limits.max_total_bytes.saturating_mul(2) {
        return Err(UpdateError::Archive("TAR size is outside bounds".into()));
    }
    let mut header = [0u8; 512];
    let mut entries = 0usize;
    let mut total = 0u64;
    let mut seen = BTreeSet::new();
    loop {
        input.read_exact(&mut header).map_err(|error| {
            if error.kind() == io::ErrorKind::UnexpectedEof {
                UpdateError::Archive("truncated TAR header".into())
            } else {
                error.into()
            }
        })?;
        if header.iter().all(|byte| *byte == 0) {
            break;
        }
        verify_tar_checksum(&header)?;
        entries += 1;
        if entries > limits.max_entries {
            return Err(UpdateError::Archive("TAR has too many entries".into()));
        }
        let name = tar_field(&header[0..100])?;
        let prefix = tar_field(&header[345..500])?;
        let joined = if prefix.is_empty() {
            name
        } else {
            format!("{prefix}/{name}")
        };
        let relative = PathBuf::from(joined);
        validate_relative_path(&relative)?;
        if !seen.insert(relative.clone()) {
            return Err(UpdateError::Security("duplicate archive path".into()));
        }
        let size = parse_tar_octal(&header[124..136])?;
        let kind = header[156];
        let output = destination.join(&relative);
        match kind {
            0 | b'0' => {
                if size > limits.max_file_bytes {
                    return Err(UpdateError::Archive("TAR file is too large".into()));
                }
                total = total
                    .checked_add(size)
                    .ok_or_else(|| UpdateError::Archive("TAR size overflow".into()))?;
                if total > limits.max_total_bytes {
                    return Err(UpdateError::Archive(
                        "TAR expanded size exceeds limit".into(),
                    ));
                }
                if let Some(parent) = output.parent() {
                    create_dir_checked(destination, parent)?;
                }
                let mut target = create_new_file(&output)?;
                copy_exact_bounded(
                    &mut input.by_ref(),
                    &mut target,
                    size,
                    limits.max_file_bytes,
                )?;
                target.sync_all()?;
            }
            b'5' => {
                if size != 0 {
                    return Err(UpdateError::Archive("TAR directory has payload".into()));
                }
                create_dir_checked(destination, &output)?;
            }
            _ => {
                return Err(UpdateError::Security(
                    "TAR link, extension, or special file refused".into(),
                ));
            }
        }
        let padding = (512 - (size % 512)) % 512;
        if padding > 0 {
            io::copy(&mut input.by_ref().take(padding), &mut io::sink())?;
        }
    }
    if entries == 0 {
        return Err(UpdateError::Archive("empty TAR refused".into()));
    }
    sync_tree_root(destination)
}

fn validate_relative_path(path: &Path) -> Result<()> {
    if path.as_os_str().is_empty() || path.is_absolute() || path.to_string_lossy().contains('\\') {
        return Err(UpdateError::Security("unsafe archive path".into()));
    }
    if path
        .components()
        .any(|part| !matches!(part, PathComponent::Normal(_)))
    {
        return Err(UpdateError::Security("archive traversal path".into()));
    }
    Ok(())
}

fn create_dir_checked(root: &Path, path: &Path) -> Result<()> {
    let relative = path
        .strip_prefix(root)
        .map_err(|_| UpdateError::Security("path escaped staging root".into()))?;
    let mut current = root.to_path_buf();
    for component in relative.components() {
        let PathComponent::Normal(name) = component else {
            return Err(UpdateError::Security("unsafe directory component".into()));
        };
        current.push(name);
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                return Err(UpdateError::Security(
                    "non-directory in staging path".into(),
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => fs::create_dir(&current)?,
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

fn create_new_file(path: &Path) -> Result<File> {
    Ok(OpenOptions::new().write(true).create_new(true).open(path)?)
}

fn copy_exact_bounded<R: Read, W: io::Write>(
    reader: &mut R,
    writer: &mut W,
    expected: u64,
    maximum: u64,
) -> Result<()> {
    if expected > maximum {
        return Err(UpdateError::Archive("file exceeds extraction limit".into()));
    }
    let copied = io::copy(&mut reader.take(expected), writer)?;
    if copied != expected {
        return Err(UpdateError::Archive("archive entry length mismatch".into()));
    }
    Ok(())
}

fn reject_symlink_path(path: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(UpdateError::Security(
            "staging parent is not a real directory".into(),
        ));
    }
    Ok(())
}

fn tar_field(bytes: &[u8]) -> Result<String> {
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    let text = std::str::from_utf8(&bytes[..end])
        .map_err(|_| UpdateError::Archive("non-UTF-8 TAR path".into()))?;
    if text.len() > 240 {
        return Err(UpdateError::Archive("TAR path is too long".into()));
    }
    Ok(text.to_owned())
}

fn parse_tar_octal(bytes: &[u8]) -> Result<u64> {
    let raw = std::str::from_utf8(bytes)
        .map_err(|_| UpdateError::Archive("invalid TAR number".into()))?;
    let trimmed = raw.trim_matches(|character| character == '\0' || character == ' ');
    if trimmed.is_empty() {
        return Ok(0);
    }
    u64::from_str_radix(trimmed, 8)
        .map_err(|_| UpdateError::Archive("invalid TAR octal number".into()))
}

fn verify_tar_checksum(header: &[u8; 512]) -> Result<()> {
    let expected = parse_tar_octal(&header[148..156])?;
    let actual: u64 = header
        .iter()
        .enumerate()
        .map(|(index, byte)| {
            if (148..156).contains(&index) {
                b' ' as u64
            } else {
                *byte as u64
            }
        })
        .sum();
    if actual != expected {
        return Err(UpdateError::Archive("TAR checksum mismatch".into()));
    }
    Ok(())
}

fn sync_tree_root(root: &Path) -> Result<()> {
    File::open(root)?.sync_all()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn tar_header(name: &str, size: usize, kind: u8) -> [u8; 512] {
        let mut header = [0u8; 512];
        header[..name.len()].copy_from_slice(name.as_bytes());
        header[100..108].copy_from_slice(b"0000600\0");
        header[108..116].copy_from_slice(b"0000000\0");
        header[116..124].copy_from_slice(b"0000000\0");
        let size_text = format!("{size:011o}\0");
        header[124..136].copy_from_slice(size_text.as_bytes());
        header[136..148].copy_from_slice(b"00000000000\0");
        header[148..156].fill(b' ');
        header[156] = kind;
        header[257..263].copy_from_slice(b"ustar\0");
        let checksum: u64 = header.iter().map(|byte| *byte as u64).sum();
        let checksum_text = format!("{checksum:06o}\0 ");
        header[148..156].copy_from_slice(checksum_text.as_bytes());
        header
    }

    #[test]
    fn extracts_regular_tar_and_rejects_traversal() {
        let temp = tempfile::tempdir().unwrap();
        let good = temp.path().join("good.tar");
        let mut file = File::create(&good).unwrap();
        file.write_all(&tar_header("bin/chill", 2, b'0')).unwrap();
        file.write_all(b"ok").unwrap();
        file.write_all(&vec![0; 510]).unwrap();
        file.write_all(&[0; 1024]).unwrap();
        extract_archive(
            &good,
            &temp.path().join("out"),
            ArchiveKind::Tar,
            &ExtractionLimits::default(),
        )
        .unwrap();
        assert_eq!(fs::read(temp.path().join("out/bin/chill")).unwrap(), b"ok");

        let bad = temp.path().join("bad.tar");
        let mut file = File::create(&bad).unwrap();
        file.write_all(&tar_header("../escape", 1, b'0')).unwrap();
        file.write_all(b"x").unwrap();
        file.write_all(&vec![0; 511]).unwrap();
        file.write_all(&[0; 1024]).unwrap();
        assert!(
            extract_archive(
                &bad,
                &temp.path().join("bad-out"),
                ArchiveKind::Tar,
                &ExtractionLimits::default()
            )
            .is_err()
        );
        assert!(!temp.path().join("escape").exists());
    }
}
