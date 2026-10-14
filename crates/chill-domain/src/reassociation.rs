//! Explicit, conservative media reassociation.  It never probes the NAS.
use crate::Store;
use chill_catalog::model::Media;
use rusqlite::{TransactionBehavior, params};
use serde::Serialize;

fn db(_: rusqlite::Error) -> String {
    "REASSOCIATION_INVALID".into()
}
fn relative(value: &str) -> bool {
    crate::import::relative(value, false)
}
fn decode(value: String) -> Result<Media, String> {
    serde_json::from_str(&value).map_err(|_| "REASSOCIATION_INVALID".into())
}
fn encode(value: &Media) -> Result<String, String> {
    serde_json::to_string(value).map_err(|_| "REASSOCIATION_INVALID".into())
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::fingerprint::{FingerprintEvidence, FingerprintKind};
    use chill_catalog::model::Media;
    fn media(id: &str, title: &str) -> Media {
        Media {
            id: id.into(),
            path: format!("/nas/{id}"),
            title: title.into(),
            category: "film".into(),
            year: Some(2000),
            series: None,
            season: None,
            episode: None,
            genres: vec![],
            overview: String::new(),
            poster: Some("/cache/custom.png".into()),
            poster_custom: true,
            runtime_minutes: None,
            cast: vec![],
            directors: vec![],
            series_overview: String::new(),
            series_metadata: None,
            series_manual: false,
            rating: None,
            saga: None,
            watched: false,
            available: true,
            added_at: 1,
            tmdb_id: None,
        }
    }
    #[test]
    fn confirmation_moves_stable_history_to_scanned_target_and_cas_protects_it() {
        let temp = tempfile::tempdir().unwrap();
        let mut store = Store::open(&temp.path().join("db.sqlite")).unwrap();
        store
            .snapshot(vec![media("Films/Old.mkv", "Source correction")])
            .unwrap();
        let profile_a = store.create_profile("A").unwrap();
        let profile_b = store.create_profile("B").unwrap();
        let source = store.catalog(&profile_a.id).unwrap()[0].id.clone();
        store
            .set_rating(&profile_a.id, &source, Some(5), "video")
            .unwrap();
        store.set_watched(&profile_b.id, &source, true).unwrap();
        let list = store
            .create_collection(&profile_a.id, "Liste", "playlist")
            .unwrap();
        store
            .collection_member_add_bulk(&profile_a.id, &list.id, std::slice::from_ref(&source))
            .unwrap();
        let old_target = store.fingerprint_target(&source).unwrap();
        store
            .mark_fingerprint_queued(&source, FingerprintKind::Partial)
            .unwrap();
        store
            .save_fingerprint(
                &old_target,
                &FingerprintEvidence {
                    byte_size: 42,
                    partial_sha256: "a".repeat(64),
                    full_sha256: None,
                },
            )
            .unwrap();
        store
            .snapshot(vec![media("Films/Old Renamed.mkv", "New metadata")])
            .unwrap();
        // Exact normalized names are intentionally conservative; use the explicit
        // source/target operation below to verify the server-side confirmation.
        let target = store
            .catalog(&profile_a.id)
            .unwrap()
            .into_iter()
            .find(|m| m.path == "Films/Old Renamed.mkv")
            .unwrap()
            .id;
        let new_target = store.fingerprint_target(&target).unwrap();
        store
            .save_fingerprint(
                &new_target,
                &FingerprintEvidence {
                    byte_size: 42,
                    partial_sha256: "a".repeat(64),
                    full_sha256: None,
                },
            )
            .unwrap();
        let page = store.reassociation_proposals(0, 48).unwrap();
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].label, "probable");
        let request = ReassociationConfirmation {
            source_id: source.clone(),
            target_id: target.clone(),
            expected_source_revision: store.media_revision(&source).unwrap(),
            expected_target_revision: store.media_revision(&target).unwrap(),
            expected_epoch: page.epoch,
        };
        store.apply_reassociation(&request).unwrap();
        let restored = store.media(&profile_a.id, &source).unwrap();
        assert_eq!(restored.title, "Source correction");
        assert_eq!(restored.path, "Films/Old Renamed.mkv");
        assert_eq!(restored.rating, Some(5));
        assert!(store.apply_reassociation(&request).is_err());
        let target_again = store.catalog(&profile_a.id).unwrap();
        assert_eq!(target_again.len(), 1);
        assert!(store.media(&profile_b.id, &source).unwrap().watched);
    }
    #[test]
    fn duplicate_heavy_proposals_are_paged_by_pair_with_relative_paths() {
        use crate::fingerprint::FingerprintEvidence;
        let temp = tempfile::tempdir().unwrap();
        let mut store = Store::open(&temp.path().join("db.sqlite")).unwrap();
        let profile = store.create_profile("A").unwrap();
        store
            .snapshot(vec![media("Films/source.mkv", "Source")])
            .unwrap();
        let source = store.catalog(&profile.id).unwrap()[0].id.clone();
        let evidence = FingerprintEvidence {
            byte_size: 42,
            partial_sha256: "a".repeat(64),
            full_sha256: None,
        };
        store
            .save_fingerprint(&store.fingerprint_target(&source).unwrap(), &evidence)
            .unwrap();
        store
            .snapshot(
                (0..100)
                    .map(|i| media(&format!("Copies/{i:03}.mkv"), "Copy"))
                    .collect(),
            )
            .unwrap();
        let targets = store.catalog(&profile.id).unwrap();
        for m in targets.iter().filter(|m| m.available) {
            store
                .save_fingerprint(&store.fingerprint_target(&m.id).unwrap(), &evidence)
                .unwrap();
        }
        let mut seen = std::collections::HashSet::new();
        for (offset, count, more) in [(0, 48, true), (48, 48, true), (96, 4, false)] {
            let page = store.reassociation_proposals(offset, 48).unwrap();
            assert_eq!(page.items.len(), count);
            assert_eq!(page.has_more, more);
            for item in page.items {
                assert_eq!(item.source_relative_path, "Films/source.mkv");
                assert!(item.target_relative_path.starts_with("Copies/"));
                assert!(seen.insert(item.target_id));
            }
        }
        assert_eq!(seen.len(), 100);
    }
}
#[derive(Serialize, Clone)]
pub struct ReassociationProposal {
    pub source_id: String,
    pub source_relative_path: String,
    pub target_id: String,
    pub source_revision: u64,
    pub target_revision: u64,
    pub expected_catalogue_epoch: u64,
    pub target_relative_path: String,
    pub label: String,
    pub reason: String,
}
#[derive(Serialize)]
pub struct ReassociationPage {
    pub items: Vec<ReassociationProposal>,
    pub has_more: bool,
    pub epoch: u64,
}
#[derive(Clone)]
pub struct ReassociationConfirmation {
    pub source_id: String,
    pub target_id: String,
    pub expected_source_revision: u64,
    pub expected_target_revision: u64,
    pub expected_epoch: u64,
}

impl Store {
    pub fn reassociation_proposals(
        &self,
        offset: usize,
        limit: usize,
    ) -> Result<ReassociationPage, String> {
        if limit == 0 || limit > 48 || offset > i64::MAX as usize {
            return Err("REASSOCIATION_INVALID".into());
        }
        let epoch = self.catalog_revision()?;
        // Page the proposed pairs, not source rows: a duplicate-heavy library
        // cannot produce an unbounded response from one 48-source page.
        let mut query = self
            .personal
            .prepare(
                "SELECT si.stable_id,si.relative_path,COALESCE(sr.revision,1),
                    ti.stable_id,ti.relative_path,COALESCE(tr.revision,1),
                    sf.full_sha256,tf.full_sha256
             FROM cc_media_fingerprint sf
             JOIN cc_media_identity si ON si.stable_id=sf.stable_id
             JOIN media sm ON sm.id=si.legacy_id
             LEFT JOIN cc_media_revision sr ON sr.stable_id=si.stable_id
             JOIN cc_media_fingerprint tf ON tf.byte_size=sf.byte_size
                AND tf.partial_sha256=sf.partial_sha256
             JOIN cc_media_identity ti ON ti.stable_id=tf.stable_id
             JOIN media tm ON tm.id=ti.legacy_id
             LEFT JOIN cc_media_revision tr ON tr.stable_id=ti.stable_id
             WHERE json_extract(sm.data,'$.available')=0
                AND json_extract(tm.data,'$.available')=1
                AND sf.state='complete' AND tf.state='complete'
                AND sf.source_path=json_extract(sm.data,'$.path')
                AND tf.source_path=json_extract(tm.data,'$.path')
                AND sf.byte_size IS NOT NULL AND sf.partial_sha256 IS NOT NULL
             ORDER BY si.relative_path,ti.relative_path
             LIMIT ?1 OFFSET ?2",
            )
            .map_err(db)?;
        let rows = query
            .query_map(params![limit as i64 + 1, offset as i64], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, i64>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, i64>(5)?,
                    r.get::<_, Option<String>>(6)?,
                    r.get::<_, Option<String>>(7)?,
                ))
            })
            .map_err(db)?;
        let mut items = Vec::with_capacity(limit + 1);
        for row in rows {
            let (
                source_id,
                source_relative_path,
                source_rev,
                target_id,
                target_relative_path,
                target_rev,
                full,
                target_full,
            ) = row.map_err(db)?;
            let full_matches = full
                .as_ref()
                .zip(target_full.as_ref())
                .is_some_and(|(a, b)| a == b);
            items.push(ReassociationProposal{source_id,source_relative_path,target_id,
                source_revision:source_rev as u64,target_revision:target_rev as u64,
                expected_catalogue_epoch:epoch,target_relative_path,label:"probable".into(),
                reason:if full_matches{"Empreinte complète identique mais réassociation automatique non applicable ; confirmation obligatoire."}else{"Taille et échantillons identiques ; confirmation obligatoire."}.into()});
        }
        let has_more = items.len() > limit;
        items.truncate(limit);
        Ok(ReassociationPage {
            items,
            has_more,
            epoch,
        })
    }
    pub fn apply_reassociation(&self, request: &ReassociationConfirmation) -> Result<(), String> {
        if request.source_id == request.target_id {
            return Err("REASSOCIATION_INVALID".into());
        }
        let tx =
            rusqlite::Transaction::new_unchecked(&self.personal, TransactionBehavior::Immediate)
                .map_err(db)?;
        let epoch: i64 = tx
            .query_row(
                "SELECT revision FROM cc_catalog_epoch WHERE id=1",
                [],
                |r| r.get(0),
            )
            .map_err(db)?;
        if epoch as u64 != request.expected_epoch {
            return Err("IMPORT_PLAN_STALE".into());
        }
        let source:(String,String,i64,String)=tx.query_row("SELECT i.legacy_id,i.relative_path,COALESCE(r.revision,1),m.data FROM cc_media_identity i JOIN media m ON m.id=i.legacy_id LEFT JOIN cc_media_revision r ON r.stable_id=i.stable_id WHERE i.stable_id=?1",[&request.source_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).map_err(db)?;
        let target:(String,String,i64,String)=tx.query_row("SELECT i.legacy_id,i.relative_path,COALESCE(r.revision,1),m.data FROM cc_media_identity i JOIN media m ON m.id=i.legacy_id LEFT JOIN cc_media_revision r ON r.stable_id=i.stable_id WHERE i.stable_id=?1",[&request.target_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).map_err(db)?;
        if source.2 as u64 != request.expected_source_revision
            || target.2 as u64 != request.expected_target_revision
        {
            return Err("IMPORT_PLAN_STALE".into());
        }
        let old = decode(source.3)?;
        let new = decode(target.3)?;
        if old.available || !new.available || !relative(&source.1) || !relative(&target.1) {
            return Err("REASSOCIATION_INVALID".into());
        }
        let protected:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM cc_profile_state WHERE stable_media_id=?1) OR EXISTS(SELECT 1 FROM cc_playlist_members WHERE stable_media_id=?1) OR EXISTS(SELECT 1 FROM collection_members cm JOIN collections c ON c.id=cm.collection_id WHERE cm.media_id=?2 AND c.kind='saga') OR EXISTS(SELECT 1 FROM cc_media_revision WHERE stable_id=?1 AND revision>1)",params![request.target_id,target.0],|r|r.get(0)).map_err(db)?;
        if protected {
            return Err("REASSOCIATION_TARGET_PROTECTED".into());
        }
        let mut rewritten = old;
        rewritten.id = target.0.clone();
        rewritten.path = new.path;
        rewritten.available = true;
        tx.execute(
            "UPDATE collection_members SET media_id=?1 WHERE media_id=?2",
            params![target.0, source.0],
        )
        .map_err(db)?;
        tx.execute(
            "UPDATE collection_positions SET media_id=?1 WHERE media_id=?2",
            params![target.0, source.0],
        )
        .map_err(db)?;
        tx.execute(
            "UPDATE imported_sagas SET media_id=?1 WHERE media_id=?2",
            params![target.0, source.0],
        )
        .map_err(db)?;
        tx.execute(
            "DELETE FROM cc_media_revision WHERE stable_id=?1",
            [&request.target_id],
        )
        .map_err(db)?;
        tx.execute(
            "UPDATE cc_media_fingerprint SET
               byte_size=(SELECT byte_size FROM cc_media_fingerprint WHERE stable_id=?1),
               partial_sha256=(SELECT partial_sha256 FROM cc_media_fingerprint WHERE stable_id=?1),
               full_sha256=(SELECT full_sha256 FROM cc_media_fingerprint WHERE stable_id=?1),
               source_path=(SELECT source_path FROM cc_media_fingerprint WHERE stable_id=?1),
               state=(SELECT state FROM cc_media_fingerprint WHERE stable_id=?1),
               requested_kind=(SELECT requested_kind FROM cc_media_fingerprint WHERE stable_id=?1)
             WHERE stable_id=?2 AND EXISTS(SELECT 1 FROM cc_media_fingerprint WHERE stable_id=?1)",
            params![request.target_id, request.source_id],
        )
        .map_err(db)?;
        tx.execute(
            "DELETE FROM cc_media_identity WHERE stable_id=?1",
            [&request.target_id],
        )
        .map_err(db)?;
        tx.execute(
            "UPDATE cc_media_identity SET legacy_id=?1,relative_path=?1 WHERE stable_id=?2",
            params![target.0, request.source_id],
        )
        .map_err(db)?;
        tx.execute("DELETE FROM media WHERE id=?1", [source.0])
            .map_err(db)?;
        tx.execute(
            "UPDATE media SET data=?1 WHERE id=?2",
            params![encode(&rewritten)?, target.0],
        )
        .map_err(db)?;
        tx.execute(
            "UPDATE cc_media_revision SET revision=revision+1 WHERE stable_id=?1",
            [&request.source_id],
        )
        .map_err(db)?;
        tx.commit().map_err(db)
    }
}
