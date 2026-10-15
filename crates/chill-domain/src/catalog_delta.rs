//! Versioned, bounded catalogue-delta vocabulary shared by the HTTP adapter.
//!
//! The revision is deliberately derived only from durable common/personal
//! epochs.  It contains no process boot id, so a client cursor survives a
//! server restart.  The server may still return `reset` when its short-lived
//! immutable baseline has expired.
use crate::SyncSnapshot;
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};

pub const DELTA_PAGE: usize = 100;
pub const DELTA_MAX_ITEMS: usize = 10_000;
pub const DELTA_MAX_BYTES: usize = 32 * 1024 * 1024;

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
    /// Durable, bounded baselines let a client cursor survive a server restart.
    /// They contain only the already bounded sync representation, never media
    /// bytes or SQLite files. The newest four revisions per profile are kept.
    pub fn catalog_delta_checkpoint(
        &self,
        profile: &str,
        revision: &str,
    ) -> Result<Option<SyncSnapshot>, String> {
        self.sync_revision(profile)?; // profile ownership/liveness
        if revision.len() > 255 {
            return Ok(None);
        }
        self.personal
            .execute_batch("CREATE TABLE IF NOT EXISTS cc_catalog_delta_checkpoint (profile_id TEXT NOT NULL REFERENCES cc_profiles(id) ON DELETE CASCADE, revision TEXT NOT NULL, snapshot TEXT NOT NULL, PRIMARY KEY(profile_id,revision));")
            .map_err(|_| "SYNC_CHECKPOINT_UNAVAILABLE".to_owned())?;
        let raw: Option<String> = self
            .personal
            .query_row(
                "SELECT snapshot FROM cc_catalog_delta_checkpoint WHERE profile_id=?1 AND revision=?2",
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
        bounded(snapshot).map_err(str::to_owned)?;
        if revision.len() > 255 {
            return Err("SYNC_CHECKPOINT_INVALID".into());
        }
        let encoded =
            serde_json::to_string(snapshot).map_err(|_| "SYNC_CHECKPOINT_INVALID".to_owned())?;
        if encoded.len() > DELTA_MAX_BYTES {
            return Err("SYNC_CATALOG_TOO_LARGE".into());
        }
        let transaction = self
            .personal
            .unchecked_transaction()
            .map_err(|_| "SYNC_CHECKPOINT_UNAVAILABLE".to_owned())?;
        transaction.execute_batch("CREATE TABLE IF NOT EXISTS cc_catalog_delta_checkpoint (profile_id TEXT NOT NULL REFERENCES cc_profiles(id) ON DELETE CASCADE, revision TEXT NOT NULL, snapshot TEXT NOT NULL, PRIMARY KEY(profile_id,revision));")
            .map_err(|_| "SYNC_CHECKPOINT_UNAVAILABLE".to_owned())?;
        transaction.execute("INSERT INTO cc_catalog_delta_checkpoint(profile_id,revision,snapshot) VALUES(?1,?2,?3) ON CONFLICT(profile_id,revision) DO UPDATE SET snapshot=excluded.snapshot",params![profile,revision,encoded])
            .map_err(|_| "SYNC_CHECKPOINT_UNAVAILABLE".to_owned())?;
        transaction.execute("DELETE FROM cc_catalog_delta_checkpoint WHERE profile_id=?1 AND rowid NOT IN (SELECT rowid FROM cc_catalog_delta_checkpoint WHERE profile_id=?1 ORDER BY rowid DESC LIMIT 4)",[profile])
            .map_err(|_| "SYNC_CHECKPOINT_UNAVAILABLE".to_owned())?;
        // Four is a count ceiling; this is the companion byte ceiling. Large
        // snapshots retain only the newest usable baseline instead of turning
        // a client-cache feature into unbounded server disk growth.
        loop {
            let bytes: i64 = transaction
                .query_row(
                    "SELECT COALESCE(SUM(length(snapshot)),0) FROM cc_catalog_delta_checkpoint WHERE profile_id=?1",
                    [profile],
                    |row| row.get(0),
                )
                .map_err(|_| "SYNC_CHECKPOINT_UNAVAILABLE".to_owned())?;
            if bytes <= DELTA_MAX_BYTES as i64 {
                break;
            }
            let removed = transaction.execute(
                "DELETE FROM cc_catalog_delta_checkpoint WHERE rowid=(SELECT rowid FROM cc_catalog_delta_checkpoint WHERE profile_id=?1 ORDER BY rowid ASC LIMIT 1)",
                [profile],
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
    }
}
