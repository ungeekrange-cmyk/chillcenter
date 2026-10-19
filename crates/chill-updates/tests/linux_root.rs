//! Root-only regression coverage for the on-disk anti-rollback ledger.
//!
//! This is deliberately opt-in: `AntiRollbackStore::open_root` checks real
//! root ownership on Linux, which ordinary developer/CI test processes cannot
//! honestly emulate.  The disposable systemd VM harness enables it.

#![cfg(target_os = "linux")]

use chill_updates::{AntiRollbackStore, Component};
use semver::Version;
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::Path;

const ENABLE: &str = "CHILL_UPDATE_DISPOSABLE_VM";
const DIGEST: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn enabled() -> bool {
    std::env::var_os(ENABLE).as_deref() == Some(std::ffi::OsStr::new("1"))
        && unsafe { libc::geteuid() } == 0
}

fn root() -> tempfile::TempDir {
    assert!(enabled(), "Linux root test requires {ENABLE}=1 and uid 0");
    // `/tmp` is intentionally world-writable, so it must never be an ancestor
    // of a path passed to the production root-owned-path validator.
    let base = std::env::var_os("CHILL_UPDATE_ROOT_TEST_BASE")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| "/var/lib/chillcenter-update".into());
    assert!(
        base.is_absolute() && base.is_dir(),
        "missing safe test base: {base:?}"
    );
    let temp = tempfile::Builder::new()
        .prefix("updater-ledger-test-")
        .tempdir_in(base)
        .unwrap();
    let path = temp.path().join("trust");
    fs::create_dir(&path).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
    temp
}

fn trust(path: &Path) -> AntiRollbackStore {
    AntiRollbackStore::open_root(path).unwrap()
}

#[test]
#[ignore = "requires CHILL_UPDATE_DISPOSABLE_VM=1 and real Linux root ownership"]
fn persisted_pending_is_exact_and_survives_reopen() {
    let temp = root();
    let root = temp.path().join("trust");
    let store = trust(&root);
    let version = Version::parse("0.7.1").unwrap();
    store
        .accept_pending(Component::Server, 7, DIGEST, &version)
        .unwrap();
    drop(store);

    let reopened = trust(&root);
    assert_eq!(reopened.highest(Component::Server).unwrap(), 7);
    assert_eq!(
        reopened
            .verification_floor(Component::Server, DIGEST)
            .unwrap()
            .0,
        6
    );
    assert!(
        reopened
            .verification_floor(Component::Server, &"f".repeat(64))
            .unwrap()
            .1
            .is_none()
    );
    reopened
        .commit_pending(Component::Server, 7, DIGEST)
        .unwrap();
    assert_eq!(
        reopened
            .verification_floor(Component::Server, DIGEST)
            .unwrap()
            .0,
        7
    );
}

#[test]
#[ignore = "requires CHILL_UPDATE_DISPOSABLE_VM=1 and real Linux root ownership"]
fn corrupt_or_symlinked_ledger_is_refused() {
    let temp = root();
    let root = temp.path().join("trust");
    fs::write(root.join("sequences.json"), b"not-json").unwrap();
    fs::set_permissions(
        root.join("sequences.json"),
        fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    assert!(trust(&root).highest(Component::Server).is_err());

    fs::remove_file(root.join("sequences.json")).unwrap();
    symlink("/etc/passwd", root.join("sequences.json")).unwrap();
    assert!(trust(&root).highest(Component::Server).is_err());
}
