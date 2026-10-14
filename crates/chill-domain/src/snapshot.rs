//! Coherent disk snapshots; never copy a live SQLite file without its WAL.
use rusqlite::{
    Connection, OpenFlags,
    backup::{Backup, StepResult},
};
use std::{
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

pub fn coherent_snapshot(
    source: &Path,
    destination: &Path,
    cancelled: &AtomicBool,
    maximum: u64,
) -> Result<(), String> {
    if destination.exists() {
        return Err("Snapshot déjà présent.".into());
    }
    let source = Connection::open_with_flags(source, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|_| "Base source inaccessible.")?;
    let mut target = Connection::open(destination).map_err(|_| "Snapshot inaccessible.")?;
    let deadline = Instant::now() + Duration::from_secs(10 * 60);
    let backup = Backup::new(&source, &mut target).map_err(|_| "Snapshot impossible.")?;
    loop {
        if cancelled.load(Ordering::Acquire) || Instant::now() > deadline {
            return Err("Snapshot annulé ou délai dépassé.".into());
        }
        let step = backup.step(64).map_err(|_| "Snapshot interrompu.")?;
        if std::fs::metadata(destination)
            .map_err(|_| "Snapshot inaccessible.")?
            .len()
            > maximum
        {
            return Err("Snapshot trop volumineux.".into());
        }
        match step {
            StepResult::Done => break,
            StepResult::More | StepResult::Busy | StepResult::Locked => {
                std::thread::sleep(Duration::from_millis(2))
            }
            _ => return Err("Snapshot interrompu.".into()),
        }
    }
    drop(backup);
    let check: String = target
        .query_row("PRAGMA quick_check", [], |r| r.get(0))
        .map_err(|_| "Snapshot invalide.")?;
    if check != "ok" {
        return Err("Snapshot corrompu.".into());
    }
    target.close().map_err(|_| "Snapshot inaccessible.")?;
    std::fs::File::open(destination)
        .and_then(|file| file.sync_all())
        .map_err(|_| "Snapshot non enregistré.")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn captures_wal_without_mutating_source_and_rejects_future_schema_before_writes() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("live.sqlite");
        let store = crate::Store::open(&source).unwrap();
        let profile = store.create_profile("Avant").unwrap();
        let snapshot = temp.path().join("snapshot.sqlite");
        coherent_snapshot(&source, &snapshot, &AtomicBool::new(false), 1024 * 1024).unwrap();
        store
            .rename_profile(&profile.id, "Après")
            .unwrap();
        assert_eq!(
            crate::Store::open_export_snapshot(&snapshot)
                .unwrap()
                .profiles()
                .unwrap()[0]
                .name,
            "Avant"
        );
        assert_eq!(store.profiles().unwrap()[0].name, "Après");
        let future = temp.path().join("future.sqlite");
        let db = Connection::open(&future).unwrap();
        db.pragma_update(None, "user_version", 99).unwrap();
        drop(db);
        let before = std::fs::read(&future).unwrap();
        assert!(crate::Store::open(&future).is_err());
        assert_eq!(before, std::fs::read(&future).unwrap());
        assert!(!future.with_extension("sqlite-wal").exists());
    }
}
