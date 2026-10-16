//! Content-addressed storage for the private, reconstructible artwork cache.
//!
//! Named artwork files remain where old clients expect them.  They are hard
//! links to private per-server blobs when the filesystem permits it, and plain
//! private copies otherwise.  This deliberately does not use SQLite.
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs::{self, OpenOptions},
    io::{Read, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
};

const RAW_MAX: usize = 8 * 1024 * 1024;
const JPEG_MAX: usize = 512 * 1024;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OptimizeReport {
    pub logical_bytes: u64,
    pub before_unique_allocated_bytes: u64,
    pub after_unique_allocated_bytes: u64,
    pub references: u64,
    pub verified: u64,
}

fn io_error(message: &str) -> std::io::Error {
    std::io::Error::other(message)
}

fn private_dir(path: &Path) -> std::io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(m) if m.is_dir() && !m.file_type().is_symlink() => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => fs::create_dir(path)?,
        _ => return Err(io_error("unsafe artwork cache directory")),
    }
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
}

fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn bounded_regular(path: &Path, max: usize) -> std::io::Result<Vec<u8>> {
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(no_follow())
        .open(path)?;
    let meta = file.metadata()?;
    if !meta.is_file() || meta.len() > max as u64 {
        return Err(io_error("invalid artwork cache file"));
    }
    let mut bytes = Vec::with_capacity(meta.len() as usize);
    file.take(max as u64 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > max || bytes.len() as u64 != meta.len() {
        return Err(io_error("invalid artwork cache file"));
    }
    Ok(bytes)
}

#[cfg(target_os = "macos")]
fn no_follow() -> i32 {
    0x100
}
#[cfg(not(target_os = "macos"))]
fn no_follow() -> i32 {
    0x20000
}

fn temp_near(path: &Path) -> PathBuf {
    path.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()))
}

fn publish_copy(reference: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let temporary = temp_near(reference);
    let result = (|| {
        let mut f = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temporary)?;
        f.write_all(bytes)?;
        f.sync_all()?;
        fs::rename(&temporary, reference)
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}

fn valid_blob(path: &Path, expected: &str, max: usize) -> std::io::Result<bool> {
    match bounded_regular(path, max) {
        Ok(bytes) => Ok(digest(&bytes) == expected),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e),
    }
}

fn publish_with_link<F>(
    pool: &Path,
    reference: &Path,
    bytes: &[u8],
    namespace: &str,
    link: F,
) -> std::io::Result<bool>
where
    F: Fn(&Path, &Path) -> std::io::Result<()>,
{
    let max = if namespace == "jpeg" {
        JPEG_MAX
    } else {
        RAW_MAX
    };
    if bytes.len() > max {
        return Err(io_error("artwork exceeds cache limit"));
    }
    match fs::symlink_metadata(reference) {
        Ok(meta) if meta.is_file() && !meta.file_type().is_symlink() => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        _ => return Err(io_error("unsafe artwork cache reference")),
    }
    let expected = digest(bytes);
    private_dir(pool)?;
    let namespace_dir = pool.join(namespace);
    private_dir(&namespace_dir)?;
    let blob = namespace_dir.join(&expected);
    if !valid_blob(&blob, &expected, max)? {
        // Link, rather than rename, publishes a new blob without ever replacing
        // a concurrently-created blob. The temporary inode is fsynced first.
        let temporary = temp_near(&blob);
        let created = (|| {
            let mut f = OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&temporary)?;
            f.write_all(bytes)?;
            f.sync_all()?;
            link(&temporary, &blob)
        })();
        let _ = fs::remove_file(&temporary);
        match created {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                if !valid_blob(&blob, &expected, max)? {
                    return Err(io_error("existing artwork blob does not match content"));
                }
            }
            // A filesystem without hard links retains exactly the prior cache
            // semantics by falling back to the old atomic named-file publish.
            Err(_) => return publish_copy(reference, bytes).map(|_| false),
        }
    }
    if !valid_blob(&blob, &expected, max)? {
        return Err(io_error("invalid artwork blob"));
    }
    let temporary = temp_near(reference);
    let linked = link(&blob, &temporary);
    match linked {
        Ok(()) => match fs::rename(&temporary, reference) {
            Ok(()) => Ok(true),
            Err(e) => {
                let _ = fs::remove_file(&temporary);
                Err(e)
            }
        },
        Err(_) => {
            let _ = fs::remove_file(&temporary);
            publish_copy(reference, bytes).map(|_| false)
        }
    }
}

/// Atomically publish a legacy named artwork path, deduplicating by bytes when
/// supported. `pool` is private to one server; profiles never share a database.
pub fn publish(pool: &Path, reference: &Path, bytes: &[u8], jpeg: bool) -> std::io::Result<bool> {
    publish_with_link(
        pool,
        reference,
        bytes,
        if jpeg { "jpeg" } else { "raw" },
        |source, destination| fs::hard_link(source, destination),
    )
}

fn legacy_name(name: &str) -> Option<(bool, usize)> {
    let (key, suffix) = name.rsplit_once('.')?;
    if key.len() != 64 || !key.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    match suffix {
        "image" => Some((false, RAW_MAX)),
        "jpg" => Some((true, JPEG_MAX)),
        _ => None,
    }
}

fn safe_directory(path: &Path) -> std::io::Result<()> {
    let meta = fs::symlink_metadata(path)?;
    if meta.is_dir() && !meta.file_type().is_symlink() {
        Ok(())
    } else {
        Err(io_error("unsafe artwork cache directory"))
    }
}
fn for_each_legacy<F>(base: &Path, mut visit: F) -> std::io::Result<()>
where
    F: FnMut(&Path, &Path, bool, usize) -> std::io::Result<()>,
{
    for server in fs::read_dir(base)? {
        let server = server?;
        let sid = server.file_name();
        let Some(sid) = sid.to_str() else { continue };
        if uuid::Uuid::parse_str(sid).is_err() {
            continue;
        }
        safe_directory(&server.path())?;
        let pool = server.path().join(".artwork-blobs");
        for (profile_count, profile) in fs::read_dir(server.path())?.enumerate() {
            if profile_count >= 30_000 {
                return Err(io_error("too many cache profiles"));
            }
            let profile = profile?;
            let pid = profile.file_name();
            let Some(pid) = pid.to_str() else { continue };
            if uuid::Uuid::parse_str(pid).is_err() {
                continue;
            }
            safe_directory(&profile.path())?;
            let images = profile.path().join("images");
            safe_directory(&images)?;
            for (count, entry) in fs::read_dir(images)?.enumerate() {
                if count >= 30_000 {
                    return Err(io_error("too many artwork cache files"));
                }
                let entry = entry?;
                let name = entry.file_name();
                let Some(name) = name.to_str() else { continue };
                let Some((jpeg, max)) = legacy_name(name) else {
                    continue;
                };
                let path = entry.path();
                let meta = fs::symlink_metadata(&path)?;
                if meta.is_file() && !meta.file_type().is_symlink() && meta.len() <= max as u64 {
                    visit(&pool, &path, jpeg, max)?;
                }
            }
        }
    }
    Ok(())
}
fn measure(base: &Path) -> std::io::Result<(u64, u64, u64)> {
    let mut logical = 0;
    let mut refs = 0;
    let mut allocated = 0;
    let mut inodes = HashSet::new();
    for_each_legacy(base, |_, path, _, _| {
        let m = fs::symlink_metadata(path)?;
        logical += m.len();
        refs += 1;
        if inodes.insert((m.dev(), m.ino())) {
            allocated += m.blocks().saturating_mul(512);
        }
        Ok(())
    })?;
    for server in fs::read_dir(base)? {
        let server = server?;
        let id = server.file_name();
        let Some(id) = id.to_str() else { continue };
        if uuid::Uuid::parse_str(id).is_err() {
            continue;
        }
        safe_directory(&server.path())?;
        let pool = server.path().join(".artwork-blobs");
        let Ok(_) = fs::symlink_metadata(&pool) else {
            continue;
        };
        safe_directory(&pool)?;
        for namespace in ["raw", "jpeg"] {
            let dir = pool.join(namespace);
            let Ok(_) = fs::symlink_metadata(&dir) else {
                continue;
            };
            safe_directory(&dir)?;
            for (count, entry) in fs::read_dir(dir)?.enumerate() {
                if count >= 30_000 {
                    return Err(io_error("too many artwork blobs"));
                }
                let m = fs::symlink_metadata(entry?.path())?;
                if m.is_file() && !m.file_type().is_symlink() && inodes.insert((m.dev(), m.ino())) {
                    allocated += m.blocks().saturating_mul(512);
                }
            }
        }
    }
    Ok((logical, allocated, refs))
}
fn require_same_reference(before: &fs::Metadata, after: &fs::Metadata) -> std::io::Result<()> {
    if !before.is_file()
        || before.file_type().is_symlink()
        || !after.is_file()
        || after.file_type().is_symlink()
        || (before.dev(), before.ino(), before.len(), before.modified()?)
            != (after.dev(), after.ino(), after.len(), after.modified()?)
    {
        return Err(io_error(
            "artwork reference changed; close all clients before retrying",
        ));
    }
    Ok(())
}

fn require_complete(
    logical: u64,
    refs: u64,
    verified: u64,
    after: (u64, u64, u64),
) -> std::io::Result<()> {
    if verified != refs || after.0 != logical || after.2 != refs {
        return Err(io_error(
            "artwork migration incomplete; close all clients before retrying",
        ));
    }
    Ok(())
}

/// Explicit closed-cache migration. It only examines legacy image leaves and
/// does not open, create, or alter SQLite/WAL/SHM files. New pool orphans are
/// intentionally retained until a future closed maintenance run.
pub fn optimize_closed_cache(root: &Path) -> std::io::Result<OptimizeReport> {
    safe_directory(root)?;
    let base = root.join("catalog-cache");
    safe_directory(&base)?;
    let (logical, before, refs) = measure(&base)?;
    let mut verified = 0;
    for_each_legacy(&base, |pool, path, jpeg, max| {
        let before_file = fs::symlink_metadata(path)?;
        let bytes = bounded_regular(path, max)?;
        let after = fs::symlink_metadata(path)?;
        // Never turn a detected concurrent change into a silent partial success.
        // Already-linked references retain their bytes and are safe to retry.
        require_same_reference(&before_file, &after)?;
        publish(pool, path, &bytes, jpeg)?;
        verified += 1;
        Ok(())
    })?;
    let after_measure = measure(&base)?;
    require_complete(logical, refs, verified, after_measure)?;
    let after = after_measure.1;
    Ok(OptimizeReport {
        logical_bytes: logical,
        before_unique_allocated_bytes: before,
        after_unique_allocated_bytes: after,
        references: refs,
        verified,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn leaf(dir: &Path, suffix: &str) -> PathBuf {
        dir.join(format!("{}.{suffix}", "a".repeat(64)))
    }
    #[test]
    fn changed_reference_and_partial_migration_cannot_report_success() {
        let d = tempfile::tempdir().unwrap();
        let a = d.path().join("before");
        let b = d.path().join("replacement");
        publish_copy(&a, b"same").unwrap();
        publish_copy(&b, b"same").unwrap();
        let before = fs::symlink_metadata(&a).unwrap();
        assert!(require_same_reference(&before, &before).is_ok());
        assert!(require_same_reference(&before, &fs::symlink_metadata(&b).unwrap()).is_err());
        assert!(require_complete(8, 2, 1, (8, 4096, 2)).is_err());
        assert!(require_complete(8, 2, 2, (9, 4096, 2)).is_err());
        assert!(require_complete(8, 2, 2, (8, 4096, 1)).is_err());
        assert!(require_complete(8, 2, 2, (8, 4096, 2)).is_ok());
    }
    #[test]
    fn identical_bytes_share_blob_and_distinct_do_not() {
        let d = tempfile::tempdir().unwrap();
        let pool = d.path().join("pool");
        let a = leaf(d.path(), "image");
        let b = d.path().join(format!("{}.image", "b".repeat(64)));
        publish(&pool, &a, b"same", false).unwrap();
        publish(&pool, &b, b"same", false).unwrap();
        assert_eq!(
            fs::metadata(&a).unwrap().ino(),
            fs::metadata(&b).unwrap().ino()
        );
        publish(&pool, &b, b"other", false).unwrap();
        assert_ne!(
            fs::metadata(&a).unwrap().ino(),
            fs::metadata(&b).unwrap().ino()
        );
    }
    #[test]
    fn link_failure_falls_back_to_the_legacy_atomic_copy() {
        let d = tempfile::tempdir().unwrap();
        let pool = d.path().join("pool");
        let a = leaf(d.path(), "jpg");
        publish_copy(&a, b"old").unwrap();
        let result =
            publish_with_link(&pool, &a, b"new", "jpeg", |_, _| Err(io_error("no links"))).unwrap();
        assert!(!result);
        assert_eq!(bounded_regular(&a, JPEG_MAX).unwrap(), b"new");
        let old = bounded_regular(&a, JPEG_MAX).unwrap();
        let failed =
            publish_with_link(&pool, &a, b"later", "jpeg", |_, _| Err(io_error("fail"))).unwrap();
        assert!(!failed);
        assert_ne!(old, bounded_regular(&a, JPEG_MAX).unwrap());
    }
    #[test]
    fn wrong_blob_and_symlink_are_rejected() {
        let d = tempfile::tempdir().unwrap();
        let pool = d.path().join("pool");
        private_dir(&pool).unwrap();
        private_dir(&pool.join("raw")).unwrap();
        let key = digest(b"wanted");
        publish_copy(&pool.join("raw").join(key), b"wrong").unwrap();
        let reference = leaf(d.path(), "image");
        publish_copy(&reference, b"old").unwrap();
        assert!(publish(&pool, &reference, b"wanted", false).is_err());
        assert_eq!(fs::read(&reference).unwrap(), b"old");
        let target = d.path().join("target");
        publish_copy(&target, b"x").unwrap();
        fs::remove_dir_all(pool.join("raw")).unwrap();
        std::os::unix::fs::symlink(&target, pool.join("raw")).unwrap();
        assert!(publish(&pool, &leaf(d.path(), "image"), b"wanted", false).is_err());
    }
    #[test]
    fn profile_removal_does_not_remove_shared_blob_pool() {
        let d = tempfile::tempdir().unwrap();
        let server = d.path().join("server");
        let images = server.join("profile").join("images");
        fs::create_dir_all(&images).unwrap();
        let reference = leaf(&images, "image");
        let pool = server.join(".artwork-blobs");
        publish(&pool, &reference, b"shared", false).unwrap();
        fs::remove_dir_all(images.parent().unwrap()).unwrap();
        assert!(pool.join("raw").read_dir().unwrap().next().is_some());
    }
    #[test]
    fn publishing_does_not_scan_or_collect_pool_orphans() {
        let d = tempfile::tempdir().unwrap();
        let pool = d.path().join("pool");
        publish(&pool, &leaf(d.path(), "image"), b"orphan", false).unwrap();
        fs::remove_file(leaf(d.path(), "image")).unwrap();
        let orphan = pool.join("raw").join(digest(b"orphan"));
        publish(
            &pool,
            &d.path().join(format!("{}.image", "b".repeat(64))),
            b"live",
            false,
        )
        .unwrap();
        assert!(orphan.exists());
    }
    #[test]
    fn migration_keeps_legacy_paths_and_never_touches_sqlite_sidecars() {
        let d = tempfile::tempdir().unwrap();
        let server = uuid::Uuid::new_v4().to_string();
        let a = uuid::Uuid::new_v4().to_string();
        let b = uuid::Uuid::new_v4().to_string();
        let ia = d
            .path()
            .join("catalog-cache")
            .join(&server)
            .join(&a)
            .join("images");
        let ib = d
            .path()
            .join("catalog-cache")
            .join(&server)
            .join(&b)
            .join("images");
        fs::create_dir_all(&ia).unwrap();
        fs::create_dir_all(&ib).unwrap();
        let la = leaf(&ia, "image");
        let lb = leaf(&ib, "image");
        publish_copy(&la, b"same").unwrap();
        publish_copy(&lb, b"same").unwrap();
        let sql = ia.parent().unwrap().join("catalog.sqlite-wal");
        publish_copy(&sql, b"untouched").unwrap();
        let before = fs::read(&sql).unwrap();
        let report = optimize_closed_cache(d.path()).unwrap();
        assert_eq!(report.references, 2);
        assert_eq!(fs::read(&la).unwrap(), b"same");
        assert_eq!(fs::read(&lb).unwrap(), b"same");
        assert_eq!(
            fs::metadata(&la).unwrap().ino(),
            fs::metadata(&lb).unwrap().ino()
        );
        assert_eq!(fs::read(sql).unwrap(), before);
        let repeat = optimize_closed_cache(d.path()).unwrap();
        assert_eq!(
            report.after_unique_allocated_bytes,
            repeat.before_unique_allocated_bytes
        );
        assert_eq!(
            report.after_unique_allocated_bytes,
            repeat.after_unique_allocated_bytes
        );
    }
    #[test]
    fn migration_rejects_a_symlink_profile_ancestor() {
        let d = tempfile::tempdir().unwrap();
        let server = uuid::Uuid::new_v4().to_string();
        let profile = uuid::Uuid::new_v4().to_string();
        let outside = tempfile::tempdir().unwrap();
        fs::create_dir_all(outside.path().join("images")).unwrap();
        fs::create_dir_all(d.path().join("catalog-cache").join(&server)).unwrap();
        std::os::unix::fs::symlink(
            outside.path(),
            d.path().join("catalog-cache").join(server).join(profile),
        )
        .unwrap();
        assert!(optimize_closed_cache(d.path()).is_err());
    }
}
