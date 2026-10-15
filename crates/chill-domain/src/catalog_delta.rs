//! Versioned, bounded catalogue-delta vocabulary shared by the HTTP adapter.
//!
//! The revision is deliberately derived only from durable common/personal
//! epochs and a server-side restore generation. It contains no process boot id, so a client cursor survives a
//! server restart.  The server may still return `reset` when its short-lived
//! immutable baseline has expired.
use crate::{SyncMedia, SyncSnapshot};
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub const DELTA_PAGE: usize = 100;
pub const DELTA_MAX_ITEMS: usize = 10_000;
pub const DELTA_MAX_BYTES: usize = 32 * 1024 * 1024;
/// Checkpoints are deliberately much smaller than a catalogue image.  They
/// are enough to decide which current records changed, but can never be used
/// to reconstruct media metadata.
pub const CHECKPOINT_GLOBAL_MAX_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureCheckpoint {
    pub items: BTreeMap<String, String>,
    pub document: String,
}

pub fn item_signature(item: &SyncMedia) -> Result<String, String> {
    // Persist only a fixed signature, never projected metadata/path text.
    // It covers the revision and every field that SyncMedia can expose.
    let canonical = serde_json::to_vec(item).map_err(|_| "SYNC_CHECKPOINT_INVALID".to_owned())?;
    Ok(format!("{:x}", Sha256::digest(canonical)))
}

pub fn checkpoint(snapshot: &SyncSnapshot) -> Result<SignatureCheckpoint, String> {
    let mut items = BTreeMap::new();
    for item in &snapshot.items {
        if items
            .insert(item.media.id.clone(), item_signature(item)?)
            .is_some()
        {
            return Err("SYNC_CHECKPOINT_INVALID".into());
        }
    }
    // Auxiliary documents are always transferred atomically with a changed
    // response; retain a signature for future format evolution without
    // persisting their full payload.
    Ok(SignatureCheckpoint {
        items,
        // The entire auxiliary document is atomically transferred whenever a
        // revision changes; this fixed marker deliberately stores no content.
        document: "current-document".into(),
    })
}

pub fn revision(server: &str, profile: &str, epoch: (u64, u64)) -> String {
    format!("{server}:{profile}:{}:{}", epoch.0, epoch.1)
}

/// This is a wire envelope rather than a database representation.  Keeping it
/// here makes the documented `changes` response easy to validate in clients.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogChanges {
    pub revision: String,
    pub unchanged: bool,
    #[serde(default)]
    pub reset: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot_id: Option<String>,
    #[serde(default)]
    pub total: usize,
    #[serde(default)]
    pub items: Vec<crate::SyncMedia>,
    #[serde(default)]
    pub deleted_ids: Vec<String>,
    #[serde(default)]
    pub sagas: Vec<crate::store::SagaView>,
    #[serde(default)]
    pub playlists: Vec<crate::store::PlaylistView>,
    #[serde(default)]
    pub series_revisions: std::collections::BTreeMap<String, u64>,
    #[serde(default)]
    pub season_metadata: std::collections::BTreeMap<String, chill_catalog::model::SeasonProvider>,
    #[serde(default)]
    pub has_more: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_offset: Option<usize>,
}

pub fn bounded(snapshot: &SyncSnapshot) -> Result<(), &'static str> {
    if snapshot.items.len() > DELTA_MAX_ITEMS || snapshot.serialized_bytes > DELTA_MAX_BYTES {
        return Err("SYNC_CATALOG_TOO_LARGE");
    }
    Ok(())
}

impl crate::Store {
    fn clear_legacy_delta_checkpoints(&self) -> Result<(), String> {
        // Only a disposable generated cache from the unreleased prototype;
        // never catalogue, profiles, personal history or media files.
        let exists:bool=self.personal.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='cc_catalog_delta_checkpoint')",[],|row|row.get(0)).map_err(|_|"SYNC_CHECKPOINT_UNAVAILABLE")?;
        if exists {
            self.personal
                .execute("DELETE FROM cc_catalog_delta_checkpoint", [])
                .map_err(|_| "SYNC_CHECKPOINT_UNAVAILABLE")?;
        }
        Ok(())
    }
    /// Durable, bounded baselines let a client cursor survive a server restart.
    /// They contain only fixed hashes and opaque ids, never media metadata,
    /// video bytes or SQLite files. Four per profile, 16 MiB globally.
    pub fn catalog_delta_checkpoint(
        &self,
        profile: &str,
        revision: &str,
    ) -> Result<Option<SignatureCheckpoint>, String> {
        self.sync_revision(profile)?; // profile ownership/liveness
        self.clear_legacy_delta_checkpoints()?;
        if revision.len() > 255 {
            return Ok(None);
        }
        self.personal
            .execute_batch("CREATE TABLE IF NOT EXISTS cc_catalog_delta_checkpoint_v2 (profile_id TEXT NOT NULL REFERENCES cc_profiles(id) ON DELETE CASCADE, revision TEXT NOT NULL, signatures TEXT NOT NULL, created_at INTEGER NOT NULL DEFAULT(unixepoch()), PRIMARY KEY(profile_id,revision));")
            .map_err(|_| "SYNC_CHECKPOINT_UNAVAILABLE".to_owned())?;
        let raw: Option<String> = self
            .personal
            .query_row(
                "SELECT signatures FROM cc_catalog_delta_checkpoint_v2 WHERE profile_id=?1 AND revision=?2",
                params![profile, revision],
                |row| row.get(0),
            )
            .optional()
            .map_err(|_| "SYNC_CHECKPOINT_UNAVAILABLE".to_owned())?;
        raw.map(|raw| serde_json::from_str(&raw).map_err(|_| "SYNC_CHECKPOINT_INVALID".to_owned()))
            .transpose()
    }
    pub fn save_catalog_delta_checkpoint(
        &mut self,
        profile: &str,
        revision: &str,
        snapshot: &SyncSnapshot,
    ) -> Result<(), String> {
        self.sync_revision(profile)?;
        self.clear_legacy_delta_checkpoints()?;
        bounded(snapshot).map_err(str::to_owned)?;
        if revision.len() > 255 {
            return Err("SYNC_CHECKPOINT_INVALID".into());
        }
        let encoded = serde_json::to_string(&checkpoint(snapshot)?)
            .map_err(|_| "SYNC_CHECKPOINT_INVALID".to_owned())?;
        if encoded.len() > CHECKPOINT_GLOBAL_MAX_BYTES {
            return Err("SYNC_CATALOG_TOO_LARGE".into());
        }
        let transaction = self
            .personal
            .unchecked_transaction()
            .map_err(|_| "SYNC_CHECKPOINT_UNAVAILABLE".to_owned())?;
        transaction.execute_batch("CREATE TABLE IF NOT EXISTS cc_catalog_delta_checkpoint_v2 (profile_id TEXT NOT NULL REFERENCES cc_profiles(id) ON DELETE CASCADE, revision TEXT NOT NULL, signatures TEXT NOT NULL, created_at INTEGER NOT NULL DEFAULT(unixepoch()), PRIMARY KEY(profile_id,revision));")
            .map_err(|_| "SYNC_CHECKPOINT_UNAVAILABLE".to_owned())?;
        transaction.execute("INSERT INTO cc_catalog_delta_checkpoint_v2(profile_id,revision,signatures,created_at) VALUES(?1,?2,?3,unixepoch()) ON CONFLICT(profile_id,revision) DO UPDATE SET signatures=excluded.signatures,created_at=excluded.created_at",params![profile,revision,encoded])
            .map_err(|_| "SYNC_CHECKPOINT_UNAVAILABLE".to_owned())?;
        transaction.execute("DELETE FROM cc_catalog_delta_checkpoint_v2 WHERE profile_id=?1 AND rowid NOT IN (SELECT rowid FROM cc_catalog_delta_checkpoint_v2 WHERE profile_id=?1 ORDER BY created_at DESC, rowid DESC LIMIT 4)",[profile])
            .map_err(|_| "SYNC_CHECKPOINT_UNAVAILABLE".to_owned())?;
        // Four is a count ceiling; this is the companion byte ceiling. Large
        // snapshots retain only the newest usable baseline instead of turning
        // a client-cache feature into unbounded server disk growth.
        loop {
            let bytes: i64 = transaction
                .query_row(
                    "SELECT COALESCE(SUM(length(CAST(signatures AS BLOB))),0) FROM cc_catalog_delta_checkpoint_v2",
                    [],
                    |row| row.get(0),
                )
                .map_err(|_| "SYNC_CHECKPOINT_UNAVAILABLE".to_owned())?;
            if bytes <= CHECKPOINT_GLOBAL_MAX_BYTES as i64 {
                break;
            }
            let removed = transaction.execute(
                "DELETE FROM cc_catalog_delta_checkpoint_v2 WHERE rowid=(SELECT rowid FROM cc_catalog_delta_checkpoint_v2 ORDER BY created_at ASC, rowid ASC LIMIT 1)",
                [],
            ).map_err(|_| "SYNC_CHECKPOINT_UNAVAILABLE".to_owned())?;
            if removed == 0 {
                break;
            }
        }
        transaction
            .commit()
            .map_err(|_| "SYNC_CHECKPOINT_UNAVAILABLE".to_owned())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn legacy_generated_snapshots_are_not_retained_outside_signature_budget() {
        let directory = tempfile::tempdir().unwrap();
        let store = crate::Store::open(&directory.path().join("catalog.sqlite")).unwrap();
        let profile = store.create_profile("Old cache").unwrap();
        store.personal.execute_batch("CREATE TABLE cc_catalog_delta_checkpoint(profile_id TEXT,revision TEXT,snapshot TEXT); INSERT INTO cc_catalog_delta_checkpoint VALUES('old','r0','generated metadata');").unwrap();
        assert!(
            store
                .catalog_delta_checkpoint(&profile.id, "r0")
                .unwrap()
                .is_none()
        );
        assert_eq!(
            store
                .personal
                .query_row(
                    "SELECT COUNT(*) FROM cc_catalog_delta_checkpoint",
                    [],
                    |row| row.get::<_, i64>(0)
                )
                .unwrap(),
            0
        );
        assert_eq!(store.profiles().unwrap().len(), 1);
    }
    #[test]
    fn checkpoint_budget_is_global_across_profiles() {
        let directory = tempfile::tempdir().unwrap();
        let mut store = crate::Store::open(&directory.path().join("catalog.sqlite")).unwrap();
        let a = store.create_profile("A").unwrap();
        let b = store.create_profile("B").unwrap();
        let c = store.create_profile("C").unwrap();
        let snapshot = store.sync_snapshot(&c.id).unwrap();
        store
            .save_catalog_delta_checkpoint(&c.id, "latest", &snapshot)
            .unwrap();
        let payload = "x".repeat(super::CHECKPOINT_GLOBAL_MAX_BYTES / 2 + 1);
        for profile in [&a.id, &b.id] {
            store.personal.execute("INSERT INTO cc_catalog_delta_checkpoint_v2(profile_id,revision,signatures,created_at) VALUES(?1,'old',?2,0)",rusqlite::params![profile,payload]).unwrap();
        }
        store
            .save_catalog_delta_checkpoint(&c.id, "latest", &snapshot)
            .unwrap();
        let bytes: i64 = store
            .personal
            .query_row(
                "SELECT SUM(length(CAST(signatures AS BLOB))) FROM cc_catalog_delta_checkpoint_v2",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(bytes <= super::CHECKPOINT_GLOBAL_MAX_BYTES as i64);
        assert!(
            store
                .catalog_delta_checkpoint(&a.id, "old")
                .unwrap()
                .is_none()
        );
        assert!(
            store
                .catalog_delta_checkpoint(&c.id, "latest")
                .unwrap()
                .is_some()
        );
    }
    #[test]
    fn checkpoint_survives_reopen_and_keeps_four_revisions() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("catalog.sqlite");
        let mut store = crate::Store::open(&path).unwrap();
        let profile = store.create_profile("Cache").unwrap();
        let media = serde_json::from_value(serde_json::json!({
            "id":"one.mp4","path":"/one.mp4","title":"One","category":"film",
            "year":null,"series":null,"season":null,"episode":null,"genres":[],"overview":"",
            "poster":null,"saga":null,"watched":false,"rating":null,"available":true,"added_at":1,"tmdb_id":null
        })).unwrap();
        store.snapshot(vec![media]).unwrap();
        let snapshot = store.sync_snapshot(&profile.id).unwrap();
        for n in 0..5 {
            store
                .save_catalog_delta_checkpoint(&profile.id, &format!("server:{}", n), &snapshot)
                .unwrap();
        }
        drop(store);
        let store = crate::Store::open(&path).unwrap();
        assert!(
            store
                .catalog_delta_checkpoint(&profile.id, "server:0")
                .unwrap()
                .is_none()
        );
        assert_eq!(
            store
                .catalog_delta_checkpoint(&profile.id, "server:4")
                .unwrap()
                .unwrap()
                .items
                .len(),
            1
        );
        let rows: i64 = store
            .personal
            .query_row(
                "SELECT COUNT(*) FROM cc_catalog_delta_checkpoint_v2 WHERE profile_id=?1",
                [&profile.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(rows, 4);
        store.delete_profile(&profile.id).unwrap();
        let rows: i64 = store
            .personal
            .query_row(
                "SELECT COUNT(*) FROM cc_catalog_delta_checkpoint_v2 WHERE profile_id=?1",
                [&profile.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            rows, 0,
            "checkpoint signatures cascade with profile deletion"
        );
    }
}
