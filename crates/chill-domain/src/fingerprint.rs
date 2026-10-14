//! Private media evidence used to propose or prove stable-identity matches.
//!
//! Hashes and physical paths never leave the server/domain boundary.
use crate::{Store, reassociation::ReassociationConfirmation};
use rusqlite::{OptionalExtension, params};
use serde::Serialize;
use std::path::PathBuf;

fn db(_: rusqlite::Error) -> String {
    "FINGERPRINT_INVALID".into()
}
fn sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FingerprintKind {
    Partial,
    Full,
}

impl FingerprintKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Partial => "partial",
            Self::Full => "full",
        }
    }
}

#[derive(Clone, Debug)]
pub struct FingerprintTarget {
    pub media_id: String,
    pub path: PathBuf,
    pub media_revision: u64,
    pub observed_file_version_token: Option<String>,
    pub evidence_current: bool,
}

#[derive(Clone, Debug)]
pub struct FingerprintEvidence {
    pub byte_size: u64,
    pub partial_sha256: String,
    pub full_sha256: Option<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct FingerprintStatus {
    pub media_id: String,
    pub state: String,
    pub strength: Option<String>,
}

impl Store {
    pub fn fingerprint_target(&self, media_id: &str) -> Result<FingerprintTarget, String> {
        let legacy = self.legacy_id(media_id)?;
        let media = self.catalogue.get(&legacy)?;
        if !media.available {
            return Err("FINGERPRINT_UNAVAILABLE".into());
        }
        let path = PathBuf::from(media.path);
        let (observed_file_version_token, evidence_current) = self
            .personal
            .query_row(
                "SELECT file_version_token,
                    state='complete' AND source_path=?2 AND partial_sha256 IS NOT NULL
                 FROM cc_media_fingerprint WHERE stable_id=?1",
                params![media_id, path.to_string_lossy()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(db)?
            .unwrap_or((None, false));
        Ok(FingerprintTarget {
            media_id: media_id.to_owned(),
            path,
            media_revision: self.media_revision(media_id)?,
            observed_file_version_token,
            evidence_current,
        })
    }

    /// Returns a small deterministic page. Repeated scans advance naturally as
    /// completed rows no longer satisfy the query; no catalogue-wide full hash.
    pub fn partial_fingerprint_targets(
        &self,
        limit: usize,
    ) -> Result<Vec<FingerprintTarget>, String> {
        if limit == 0 || limit > 32 {
            return Err("FINGERPRINT_INVALID".into());
        }
        let mut query = self
            .personal
            .prepare(
                "SELECT i.stable_id,COALESCE(r.revision,1),m.data,f.file_version_token,
                    COALESCE(f.state='complete'
                      AND f.source_path=json_extract(m.data,'$.path')
                      AND f.partial_sha256 IS NOT NULL,0)
             FROM cc_media_identity i
             JOIN media m ON m.id=i.legacy_id
             LEFT JOIN cc_media_revision r ON r.stable_id=i.stable_id
             LEFT JOIN cc_media_fingerprint f ON f.stable_id=i.stable_id
             WHERE json_extract(m.data,'$.available')=1
               AND (f.stable_id IS NULL OR f.partial_sha256 IS NULL
                    OR f.source_path IS NULL OR f.source_path<>json_extract(m.data,'$.path'))
             ORDER BY i.stable_id LIMIT ?1",
            )
            .map_err(db)?;
        query
            .query_map([limit as i64], |row| {
                let media: chill_catalog::model::Media =
                    serde_json::from_str(&row.get::<_, String>(2)?)
                        .map_err(|_| rusqlite::Error::InvalidQuery)?;
                Ok(FingerprintTarget {
                    media_id: row.get(0)?,
                    media_revision: row.get::<_, i64>(1)? as u64,
                    path: PathBuf::from(media.path),
                    observed_file_version_token: row.get(3)?,
                    evidence_current: row.get(4)?,
                })
            })
            .map_err(db)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(db)
    }

    /// Returns one deterministic bounded page for the background stat/partial
    /// evidence pump. A caller-provided cursor guarantees that failed rows do
    /// not starve later identities within one scan pass.
    pub fn fingerprint_scan_targets_after(
        &self,
        after: &str,
        limit: usize,
    ) -> Result<Vec<FingerprintTarget>, String> {
        if limit == 0 || limit > 32 {
            return Err("FINGERPRINT_INVALID".into());
        }
        let mut query = self
            .personal
            .prepare(
                "SELECT i.stable_id,COALESCE(r.revision,1),m.data,f.file_version_token,
                    COALESCE(f.state='complete'
                      AND f.source_path=json_extract(m.data,'$.path')
                      AND f.partial_sha256 IS NOT NULL,0)
                 FROM cc_media_identity i
                 JOIN media m ON m.id=i.legacy_id
                 LEFT JOIN cc_media_revision r ON r.stable_id=i.stable_id
                 LEFT JOIN cc_media_fingerprint f ON f.stable_id=i.stable_id
                 WHERE json_extract(m.data,'$.available')=1 AND i.stable_id>?1
                   AND (f.stable_id IS NULL OR f.state NOT IN ('queued','running'))
                 ORDER BY i.stable_id LIMIT ?2",
            )
            .map_err(db)?;
        query
            .query_map(params![after, limit as i64], |row| {
                let media: chill_catalog::model::Media =
                    serde_json::from_str(&row.get::<_, String>(2)?)
                        .map_err(|_| rusqlite::Error::InvalidQuery)?;
                Ok(FingerprintTarget {
                    media_id: row.get(0)?,
                    media_revision: row.get::<_, i64>(1)? as u64,
                    path: PathBuf::from(media.path),
                    observed_file_version_token: row.get(3)?,
                    evidence_current: row.get(4)?,
                })
            })
            .map_err(db)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(db)
    }

    pub fn mark_fingerprint_target_queued(
        &self,
        target: &FingerprintTarget,
        kind: FingerprintKind,
    ) -> Result<(), String> {
        if self.media_revision(&target.media_id)? != target.media_revision
            || self.media_path(&target.media_id)? != target.path
        {
            return Err("FINGERPRINT_STALE".into());
        }
        self.mark_fingerprint_queued(&target.media_id, kind)
    }

    pub fn mark_fingerprint_queued(
        &self,
        media_id: &str,
        kind: FingerprintKind,
    ) -> Result<(), String> {
        self.legacy_id(media_id)?;
        if self.personal.execute(
            "INSERT INTO cc_media_fingerprint(stable_id,state,requested_kind)
             VALUES(?1,'queued',?2)
             ON CONFLICT(stable_id) DO UPDATE SET state='queued',requested_kind=excluded.requested_kind
             WHERE cc_media_fingerprint.state NOT IN ('queued','running')",
            params![media_id, kind.as_str()],
        ).map_err(db)? != 1 {
            return Err("FINGERPRINT_BUSY".into());
        }
        Ok(())
    }

    pub fn mark_fingerprint_running(
        &self,
        media_id: &str,
        expected_revision: u64,
    ) -> Result<(), String> {
        if self.media_revision(media_id)? != expected_revision {
            return Err("FINGERPRINT_STALE".into());
        }
        if self.personal.execute(
            "UPDATE cc_media_fingerprint SET state='running' WHERE stable_id=?1 AND state='queued'",
            [media_id],
        ).map_err(db)? != 1 {
            return Err("FINGERPRINT_STALE".into());
        }
        Ok(())
    }

    pub fn mark_fingerprint_unchanged(
        &self,
        target: &FingerprintTarget,
        file_version_token: &str,
    ) -> Result<(), String> {
        if !sha256(file_version_token)
            || self.media_revision(&target.media_id)? != target.media_revision
            || self.media_path(&target.media_id)? != target.path
        {
            return Err("FINGERPRINT_STALE".into());
        }
        if self
            .personal
            .execute(
                "UPDATE cc_media_fingerprint SET state='complete'
                 WHERE stable_id=?1 AND state='running' AND source_path=?2
                   AND file_version_token=?3 AND partial_sha256 IS NOT NULL",
                params![
                    target.media_id,
                    target.path.to_string_lossy(),
                    file_version_token
                ],
            )
            .map_err(db)?
            != 1
        {
            return Err("FINGERPRINT_STALE".into());
        }
        Ok(())
    }

    pub fn mark_fingerprint_failed(&self, media_id: &str) -> Result<(), String> {
        self.personal
            .execute(
                "UPDATE cc_media_fingerprint SET state='failed' WHERE stable_id=?1",
                [media_id],
            )
            .map_err(db)?;
        Ok(())
    }

    /// CAS-publishes evidence only while the stable identity still names the
    /// same catalogue revision. A completed full proof may reuse the existing
    /// protected reassociation transaction, but only for one unique old/new pair.
    pub fn save_fingerprint_versioned(
        &self,
        target: &FingerprintTarget,
        evidence: &FingerprintEvidence,
        file_version_token: &str,
    ) -> Result<Option<String>, String> {
        if evidence.byte_size == 0
            || !sha256(&evidence.partial_sha256)
            || !sha256(file_version_token)
            || evidence
                .full_sha256
                .as_ref()
                .is_some_and(|value| !sha256(value))
            || self.media_revision(&target.media_id)? != target.media_revision
            || self.media_path(&target.media_id)? != target.path
        {
            return Err("FINGERPRINT_STALE".into());
        }
        let byte_size = i64::try_from(evidence.byte_size).map_err(|_| "FINGERPRINT_INVALID")?;
        self.personal.execute(
            "INSERT INTO cc_media_fingerprint(stable_id,byte_size,partial_sha256,full_sha256,source_path,file_version_token,state,requested_kind)
             VALUES(?1,?2,?3,?4,?5,?6,'complete',CASE WHEN ?4 IS NULL THEN 'partial' ELSE 'full' END)
             ON CONFLICT(stable_id) DO UPDATE SET byte_size=excluded.byte_size,
               partial_sha256=excluded.partial_sha256,
               full_sha256=CASE
                 WHEN excluded.full_sha256 IS NOT NULL THEN excluded.full_sha256
                 WHEN cc_media_fingerprint.source_path=excluded.source_path
                  AND cc_media_fingerprint.byte_size=excluded.byte_size
                  AND cc_media_fingerprint.partial_sha256=excluded.partial_sha256
                  AND cc_media_fingerprint.file_version_token=excluded.file_version_token
                 THEN cc_media_fingerprint.full_sha256 ELSE NULL END,
               source_path=excluded.source_path,
               file_version_token=excluded.file_version_token,
               state='complete',requested_kind=excluded.requested_kind",
            params![target.media_id, byte_size, evidence.partial_sha256, evidence.full_sha256, target.path.to_string_lossy(), file_version_token],
        ).map_err(db)?;
        let Some(full) = evidence.full_sha256.as_deref() else {
            return Ok(None);
        };
        let mut candidates = self
            .personal
            .prepare(
                "SELECT f.stable_id,json_extract(m.data,'$.available'),COALESCE(r.revision,1)
             FROM cc_media_fingerprint f
             JOIN cc_media_identity i ON i.stable_id=f.stable_id
             JOIN media m ON m.id=i.legacy_id
             LEFT JOIN cc_media_revision r ON r.stable_id=f.stable_id
             WHERE f.byte_size=?1 AND f.full_sha256=?2 AND f.state='complete'
               AND f.file_version_token IS NOT NULL
               AND f.source_path=json_extract(m.data,'$.path') ORDER BY f.stable_id",
            )
            .map_err(db)?;
        let rows = candidates
            .query_map(params![byte_size, full], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, bool>(1)?,
                    row.get::<_, i64>(2)? as u64,
                ))
            })
            .map_err(db)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(db)?;
        if rows.len() != 2 {
            return Ok(None);
        }
        let Some((source, _, source_revision)) = rows.iter().find(|(_, available, _)| !available)
        else {
            return Ok(None);
        };
        let Some((destination, _, destination_revision)) = rows
            .iter()
            .find(|(id, available, _)| *available && id == &target.media_id)
        else {
            return Ok(None);
        };
        let confirmation = ReassociationConfirmation {
            source_id: source.clone(),
            target_id: destination.clone(),
            expected_source_revision: *source_revision,
            expected_target_revision: *destination_revision,
            expected_epoch: self.catalog_revision()?,
        };
        self.apply_reassociation(&confirmation)?;
        Ok(Some(source.clone()))
    }

    #[cfg(test)]
    pub fn save_fingerprint(
        &self,
        target: &FingerprintTarget,
        evidence: &FingerprintEvidence,
    ) -> Result<Option<String>, String> {
        self.save_fingerprint_versioned(target, evidence, &"0".repeat(64))
    }

    pub fn fingerprint_status(&self, media_id: &str) -> Result<FingerprintStatus, String> {
        self.legacy_id(media_id)?;
        self.personal
            .query_row(
                "SELECT
               CASE WHEN f.state='failed' THEN 'failed'
                    WHEN f.source_path=json_extract(m.data,'$.path') THEN f.state ELSE 'stale' END,
               CASE WHEN f.state='failed' OR f.source_path<>json_extract(m.data,'$.path') THEN NULL
                    WHEN f.full_sha256 IS NOT NULL AND f.file_version_token IS NOT NULL THEN 'full'
                    WHEN f.partial_sha256 IS NOT NULL THEN 'partial' ELSE NULL END
             FROM cc_media_fingerprint f
             JOIN cc_media_identity i ON i.stable_id=f.stable_id
             JOIN media m ON m.id=i.legacy_id
             WHERE f.stable_id=?1",
                [media_id],
                |row| {
                    Ok(FingerprintStatus {
                        media_id: media_id.to_owned(),
                        state: row.get(0)?,
                        strength: row.get(1)?,
                    })
                },
            )
            .optional()
            .map_err(db)?
            .ok_or_else(|| "FINGERPRINT_NOT_REQUESTED".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chill_catalog::model::Media;

    fn media(path: &str, available: bool) -> Media {
        Media {
            id: path.into(),
            path: format!("/fixture/{path}"),
            title: "same title".into(),
            category: "film".into(),
            year: None,
            series: None,
            season: None,
            episode: None,
            genres: vec![],
            overview: String::new(),
            poster: None,
            poster_custom: false,
            runtime_minutes: None,
            cast: vec![],
            directors: vec![],
            series_overview: String::new(),
            series_metadata: None,
            series_manual: false,
            rating: None,
            saga: None,
            watched: false,
            available,
            added_at: 1,
            tmdb_id: None,
        }
    }

    fn evidence(full: Option<&str>) -> FingerprintEvidence {
        FingerprintEvidence {
            byte_size: 7,
            partial_sha256: "a".repeat(64),
            full_sha256: full.map(str::to_owned),
        }
    }

    #[test]
    fn unique_full_proof_reassociates_all_profiles_and_playlists_atomically() {
        let temp = tempfile::tempdir().unwrap();
        let mut store = Store::open(&temp.path().join("db.sqlite")).unwrap();
        store.snapshot(vec![media("old.mp4", true)]).unwrap();
        let a = store.create_profile("A").unwrap();
        let b = store.create_profile("B").unwrap();
        let source = store.catalog(&a.id).unwrap()[0].id.clone();
        store.set_rating(&a.id, &source, Some(5), "video").unwrap();
        store.set_watched(&b.id, &source, true).unwrap();
        let list = store.create_collection(&a.id, "List", "playlist").unwrap();
        store
            .collection_member_add_bulk(&a.id, &list.id, std::slice::from_ref(&source))
            .unwrap();
        let old = store.fingerprint_target(&source).unwrap();
        store
            .save_fingerprint_versioned(&old, &evidence(Some(&"b".repeat(64))), &"1".repeat(64))
            .unwrap();
        store.snapshot(vec![media("new.mp4", true)]).unwrap();
        let target = store
            .catalog(&a.id)
            .unwrap()
            .into_iter()
            .find(|item| item.path == "new.mp4")
            .unwrap()
            .id;
        let capture = store.fingerprint_target(&target).unwrap();
        assert_eq!(
            store
                .save_fingerprint_versioned(
                    &capture,
                    &evidence(Some(&"b".repeat(64))),
                    &"2".repeat(64),
                )
                .unwrap(),
            Some(source.clone())
        );
        assert_eq!(store.media(&a.id, &source).unwrap().rating, Some(5));
        assert!(store.media(&b.id, &source).unwrap().watched);
        assert_eq!(
            store.playlist(&a.id, &list.id).unwrap().media_ids,
            vec![source.clone()]
        );
        assert_eq!(
            store
                .fingerprint_status(&source)
                .unwrap()
                .strength
                .as_deref(),
            Some("full")
        );
        let transferred_token: String = store
            .personal
            .query_row(
                "SELECT file_version_token FROM cc_media_fingerprint WHERE stable_id=?1",
                [&source],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(transferred_token, "2".repeat(64));
        let portable = store.export_json().unwrap().to_string();
        assert!(!portable.contains("partial_sha256"));
        assert!(!portable.contains(&"b".repeat(64)));
    }

    #[test]
    fn ambiguous_full_proof_and_partial_proof_never_merge() {
        let temp = tempfile::tempdir().unwrap();
        let mut store = Store::open(&temp.path().join("db.sqlite")).unwrap();
        store
            .snapshot(vec![media("old-a.mp4", true), media("old-b.mp4", true)])
            .unwrap();
        let profile = store.create_profile("A").unwrap();
        let old = store.catalog(&profile.id).unwrap();
        for item in &old {
            let target = store.fingerprint_target(&item.id).unwrap();
            store
                .save_fingerprint(&target, &evidence(Some(&"c".repeat(64))))
                .unwrap();
        }
        store
            .snapshot(vec![
                media("new.mp4", true),
                media("different-size.mp4", true),
            ])
            .unwrap();
        let target = store
            .catalog(&profile.id)
            .unwrap()
            .into_iter()
            .find(|item| item.path == "new.mp4")
            .unwrap();
        let capture = store.fingerprint_target(&target.id).unwrap();
        assert_eq!(
            store.save_fingerprint(&capture, &evidence(None)).unwrap(),
            None
        );
        assert_eq!(
            store
                .save_fingerprint(&capture, &evidence(Some(&"c".repeat(64))))
                .unwrap(),
            None
        );
        let different = store
            .catalog(&profile.id)
            .unwrap()
            .into_iter()
            .find(|item| item.path == "different-size.mp4")
            .unwrap();
        let different_target = store.fingerprint_target(&different.id).unwrap();
        let mut different_evidence = evidence(Some(&"c".repeat(64)));
        different_evidence.byte_size = 8;
        assert_eq!(
            store
                .save_fingerprint(&different_target, &different_evidence)
                .unwrap(),
            None
        );
        assert_eq!(store.catalog(&profile.id).unwrap().len(), 4);
    }

    #[test]
    fn automatic_reassociation_rolls_back_on_mid_transaction_fault_and_retries() {
        let temp = tempfile::tempdir().unwrap();
        let mut store = Store::open(&temp.path().join("db.sqlite")).unwrap();
        store.snapshot(vec![media("old.mp4", true)]).unwrap();
        let profile = store.create_profile("A").unwrap();
        let source = store.catalog(&profile.id).unwrap()[0].id.clone();
        store
            .set_rating(&profile.id, &source, Some(4), "video")
            .unwrap();
        let old = store.fingerprint_target(&source).unwrap();
        store
            .save_fingerprint(&old, &evidence(Some(&"d".repeat(64))))
            .unwrap();
        store.snapshot(vec![media("new.mp4", true)]).unwrap();
        let target = store
            .catalog(&profile.id)
            .unwrap()
            .into_iter()
            .find(|item| item.path == "new.mp4")
            .unwrap()
            .id;
        let capture = store.fingerprint_target(&target).unwrap();
        store
            .personal
            .execute_batch(&format!(
                "CREATE TRIGGER fail_fingerprint_reassociation BEFORE UPDATE OF legacy_id ON cc_media_identity WHEN OLD.stable_id='{source}' BEGIN SELECT RAISE(ABORT,'fault'); END;"
            ))
            .unwrap();
        assert!(
            store
                .save_fingerprint(&capture, &evidence(Some(&"d".repeat(64))))
                .is_err()
        );
        assert_eq!(store.catalog(&profile.id).unwrap().len(), 2);
        assert_eq!(store.media(&profile.id, &source).unwrap().rating, Some(4));
        store
            .personal
            .execute_batch("DROP TRIGGER fail_fingerprint_reassociation")
            .unwrap();
        assert_eq!(
            store
                .save_fingerprint(&capture, &evidence(Some(&"d".repeat(64))))
                .unwrap(),
            Some(source.clone())
        );
        assert_eq!(store.catalog(&profile.id).unwrap().len(), 1);
        assert_eq!(store.media(&profile.id, &source).unwrap().rating, Some(4));
    }

    #[test]
    fn changed_catalogue_location_makes_old_evidence_stale() {
        let temp = tempfile::tempdir().unwrap();
        let mut store = Store::open(&temp.path().join("db.sqlite")).unwrap();
        let original = media("same.mp4", true);
        store.snapshot(vec![original.clone()]).unwrap();
        let profile = store.create_profile("A").unwrap();
        let id = store.catalog(&profile.id).unwrap()[0].id.clone();
        let target = store.fingerprint_target(&id).unwrap();
        store
            .save_fingerprint(&target, &evidence(Some(&"f".repeat(64))))
            .unwrap();
        assert!(store.partial_fingerprint_targets(16).unwrap().is_empty());
        let mut moved = original;
        moved.path = "/other-root/same.mp4".into();
        store.snapshot(vec![moved]).unwrap();
        let status = store.fingerprint_status(&id).unwrap();
        assert_eq!(status.state, "stale");
        assert_eq!(status.strength, None);
        assert_eq!(
            store.partial_fingerprint_targets(16).unwrap()[0].media_id,
            id
        );
        let moved_target = store.fingerprint_target(&id).unwrap();
        store
            .save_fingerprint(&moved_target, &evidence(None))
            .unwrap();
        assert_eq!(
            store.fingerprint_status(&id).unwrap().strength.as_deref(),
            Some("partial")
        );
    }

    #[test]
    fn automatic_proof_never_overwrites_a_protected_target_identity() {
        let temp = tempfile::tempdir().unwrap();
        let mut store = Store::open(&temp.path().join("db.sqlite")).unwrap();
        store.snapshot(vec![media("old.mp4", true)]).unwrap();
        let profile = store.create_profile("A").unwrap();
        let source = store.catalog(&profile.id).unwrap()[0].id.clone();
        let old = store.fingerprint_target(&source).unwrap();
        store
            .save_fingerprint(&old, &evidence(Some(&"e".repeat(64))))
            .unwrap();
        store.snapshot(vec![media("new.mp4", true)]).unwrap();
        let target = store
            .catalog(&profile.id)
            .unwrap()
            .into_iter()
            .find(|item| item.path == "new.mp4")
            .unwrap()
            .id;
        store
            .set_rating(&profile.id, &target, Some(2), "video")
            .unwrap();
        let capture = store.fingerprint_target(&target).unwrap();
        assert!(
            store
                .save_fingerprint(&capture, &evidence(Some(&"e".repeat(64))))
                .is_err()
        );
        assert_eq!(store.catalog(&profile.id).unwrap().len(), 2);
        assert_eq!(store.media(&profile.id, &target).unwrap().rating, Some(2));
    }

    #[test]
    fn migration_conservatively_drops_unversioned_full_strength() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("db.sqlite");
        let mut store = Store::open(&path).unwrap();
        store.snapshot(vec![media("old.mp4", true)]).unwrap();
        let profile = store.create_profile("A").unwrap();
        let id = store.catalog(&profile.id).unwrap()[0].id.clone();
        let target = store.fingerprint_target(&id).unwrap();
        store
            .save_fingerprint(&target, &evidence(Some(&"f".repeat(64))))
            .unwrap();
        store
            .personal
            .execute(
                "UPDATE cc_media_fingerprint SET file_version_token=NULL WHERE stable_id=?1",
                [&id],
            )
            .unwrap();
        drop(store);

        let reopened = Store::open(&path).unwrap();
        let status = reopened.fingerprint_status(&id).unwrap();
        assert_eq!(status.state, "complete");
        assert_eq!(status.strength.as_deref(), Some("partial"));
        let full: Option<String> = reopened
            .personal
            .query_row(
                "SELECT full_sha256 FROM cc_media_fingerprint WHERE stable_id=?1",
                [&id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(full, None);
    }
}
