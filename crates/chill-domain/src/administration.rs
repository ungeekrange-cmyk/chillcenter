//! Common-library edits under one transaction. No filesystem mutations.
use crate::Store;
use chill_catalog::model::{Media, SeriesPatch};
use rusqlite::{TransactionBehavior, params};
use serde::Serialize;
fn db(_: rusqlite::Error) -> String {
    "Erreur de base locale. Vos données n’ont pas été remplacées.".into()
}
fn encode(media: &Media) -> Result<String, String> {
    serde_json::to_string(media).map_err(|_| "Métadonnées invalides.".into())
}
fn decode(text: String) -> Result<Media, String> {
    serde_json::from_str(&text).map_err(|_| "Catalogue local endommagé.".into())
}
#[derive(Serialize)]
pub struct UnavailableReference {
    pub id: String,
    pub title: String,
    pub relative_path: String,
    pub revision: u64,
}
#[derive(Serialize)]
pub struct UnavailablePage {
    pub items: Vec<UnavailableReference>,
    pub has_more: bool,
    pub epoch: u64,
}
impl Store {
    pub(crate) fn saga_order(&self, saga: &str) -> Result<Vec<String>, String> {
        let id = saga.parse::<i64>().map_err(|_| "Saga invalide.")?;
        let mut stmt=self.personal.prepare("SELECT m.media_id FROM collection_members m JOIN collections c ON c.id=m.collection_id JOIN media d ON d.id=m.media_id LEFT JOIN collection_positions p ON p.collection_id=m.collection_id AND p.media_id=m.media_id WHERE c.kind='saga' AND c.id=?1 ORDER BY p.position IS NULL,p.position,json_extract(d.data,'$.year') IS NULL,json_extract(d.data,'$.year'),json_extract(d.data,'$.title') COLLATE NOCASE,m.media_id").map_err(db)?;
        stmt.query_map([id], |r| r.get(0))
            .map_err(db)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(db)
    }
    pub(crate) fn reorder_common_saga(
        &self,
        saga: &str,
        revision: u64,
        expected: &[String],
        desired: &[String],
    ) -> Result<(), String> {
        let id = saga.parse::<i64>().map_err(|_| "Saga invalide.")?;
        if expected.len() > 1000 || desired.len() > 1000 {
            return Err("Liste trop volumineuse.".into());
        }
        let tx =
            rusqlite::Transaction::new_unchecked(&self.personal, TransactionBehavior::Immediate)
                .map_err(db)?;
        let current:i64=tx.query_row("SELECT r.revision FROM collection_revisions r JOIN collections c ON c.id=r.collection_id WHERE c.id=?1 AND c.kind='saga'",[id],|r|r.get(0)).map_err(db)?;
        if current as u64 != revision {
            return Err("La saga a changé entre-temps.".into());
        }
        let ids = self
            .saga_order(saga)?
            .iter()
            .map(|legacy| self.stable_id(legacy))
            .collect::<Result<Vec<_>, _>>()?;
        let unique = desired.iter().collect::<std::collections::BTreeSet<_>>();
        if ids != expected {
            return Err("La saga a changé entre-temps.".into());
        }
        if unique.len() != ids.len()
            || desired.len() != ids.len()
            || ids.iter().any(|id| !unique.contains(id))
        {
            return Err("Ordre invalide.".into());
        }
        tx.execute(
            "DELETE FROM collection_positions WHERE collection_id=?1",
            [id],
        )
        .map_err(db)?;
        for (position, stable) in desired.iter().enumerate() {
            tx.execute("INSERT INTO collection_positions(collection_id,media_id,position) VALUES(?1,?2,?3)",params![id,self.legacy_id(stable)?,position as i64]).map_err(db)?;
        }
        tx.execute(
            "UPDATE collection_revisions SET revision=revision+1 WHERE collection_id=?1",
            [id],
        )
        .map_err(db)?;
        tx.commit().map_err(db)
    }
    pub fn common_revision(&self) -> Result<u64, String> {
        self.personal
            .query_row("SELECT revision FROM cc_common_epoch WHERE id=1", [], |r| {
                r.get::<_, i64>(0)
            })
            .map(|n| n as u64)
            .map_err(db)
    }
    /// Coarse catalogue epoch deliberately includes episode changes: a shared
    /// form never silently overwrites a concurrently changed grouping.
    pub fn update_shared_series(
        &self,
        profile: &str,
        series: &str,
        expected: u64,
        patch: &SeriesPatch,
    ) -> Result<(), String> {
        self.require_profile(profile)?;
        let members = if let Some(anchor) = series.strip_prefix("media:") {
            let raw = self.catalogue.get(&self.legacy_id(anchor)?)?;
            self.catalogue.series_members_for(&raw)?
        } else if let Some(raw) = series.strip_prefix("tmdb:tv:") {
            let id: i64 = raw.parse().map_err(|_| "Série invalide.")?;
            self.catalogue
                .list()?
                .into_iter()
                .filter(|m| m.category == "serie" && m.tmdb_id == Some(id))
                .collect()
        } else {
            let all: Vec<_> = self
                .catalogue
                .list()?
                .into_iter()
                .filter(|m| {
                    m.category == "serie" && m.series.as_deref().unwrap_or(&m.title) == series
                })
                .collect();
            let groups = all
                .iter()
                .filter_map(chill_catalog::database::series_source_key)
                .collect::<std::collections::HashSet<_>>();
            if groups.len() != 1 {
                return Err(
                    "Plusieurs séries portent ce nom. Ouvrez-la depuis sa vignette.".into(),
                );
            }
            all
        };
        let tx =
            rusqlite::Transaction::new_unchecked(&self.personal, TransactionBehavior::Immediate)
                .map_err(db)?;
        if self.common_revision()? != expected {
            return Err("La fiche a changé entre-temps.".into());
        }
        if series.is_empty() || series.len() > 500 {
            return Err("Série invalide.".into());
        }
        let metadata = patch.validate(members.first().ok_or("Série introuvable.")?)?;
        for mut media in members {
            let stable = self.stable_id(&media.id)?;
            media.series = Some(patch.name.trim().into());
            media.series_manual = true;
            media.series_overview = metadata.overview.clone();
            media.series_metadata = Some(metadata.clone());
            tx.execute(
                "UPDATE media SET data=?1 WHERE id=?2",
                params![encode(&media)?, media.id],
            )
            .map_err(db)?;
            tx.execute("INSERT INTO cc_media_revision(stable_id,revision) VALUES(?1,1) ON CONFLICT(stable_id) DO UPDATE SET revision=revision+1",[stable]).map_err(db)?;
        }
        if patch.name.trim() != series {
            tx.execute("INSERT OR IGNORE INTO cc_series_speed(profile_id,series_key,speed) SELECT profile_id,?1,speed FROM cc_series_speed WHERE series_key=?2",params![patch.name.trim(),series]).map_err(db)?;
            tx.execute("DELETE FROM cc_series_speed WHERE series_key=?1", [series])
                .map_err(db)?;
        }
        tx.commit().map_err(db)
    }
    pub fn unavailable_page(&self, offset: usize, limit: usize) -> Result<UnavailablePage, String> {
        if limit == 0 || limit > 100 || offset > i64::MAX as usize {
            return Err("Page invalide.".into());
        }
        let mut stmt=self.personal.prepare("SELECT i.stable_id,i.relative_path,m.data,COALESCE(r.revision,1) FROM media m JOIN cc_media_identity i ON i.legacy_id=m.id LEFT JOIN cc_media_revision r ON r.stable_id=i.stable_id WHERE json_extract(m.data,'$.available')=0 ORDER BY i.relative_path LIMIT ?1 OFFSET ?2").map_err(db)?;
        let mut items = stmt
            .query_map(params![limit as i64 + 1, offset as i64], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, i64>(3)?,
                ))
            })
            .map_err(db)?
            .map(|r| {
                let (id, relative_path, text, revision) = r.map_err(db)?;
                Ok(UnavailableReference {
                    id,
                    title: decode(text)?.title,
                    relative_path,
                    revision: revision as u64,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let has_more = items.len() > limit;
        items.truncate(limit);
        Ok(UnavailablePage {
            items,
            has_more,
            epoch: self.catalog_revision()?,
        })
    }
    pub fn forget_unavailable(&self, ids: &[String], expected: u64) -> Result<(), String> {
        if ids.is_empty()
            || ids.len() > 1000
            || ids.iter().collect::<std::collections::BTreeSet<_>>().len() != ids.len()
        {
            return Err("Sélection invalide (1 à 1000 références).".into());
        }
        let tx =
            rusqlite::Transaction::new_unchecked(&self.personal, TransactionBehavior::Immediate)
                .map_err(db)?;
        if self.catalog_revision()? != expected {
            return Err("IMPORT_PLAN_STALE".into());
        }
        for id in ids {
            let legacy = self.legacy_id(id)?;
            let media = decode(
                tx.query_row("SELECT data FROM media WHERE id=?1", [&legacy], |r| {
                    r.get(0)
                })
                .map_err(db)?,
            )?;
            if media.available {
                return Err(
                    "Cette vidéo est redevenue disponible. Actualisez avant de retirer.".into(),
                );
            }
        }
        for id in ids {
            let legacy = self.legacy_id(id)?;
            tx.execute("UPDATE cc_playlists SET revision=revision+1 WHERE id IN (SELECT playlist_id FROM cc_playlist_members WHERE stable_media_id=?1)",[id]).map_err(db)?;
            tx.execute("UPDATE collection_revisions SET revision=revision+1 WHERE collection_id IN (SELECT collection_id FROM collection_members WHERE media_id=?1)",[&legacy]).map_err(db)?;
            tx.execute(
                "DELETE FROM cc_playlist_members WHERE stable_media_id=?1",
                [id],
            )
            .map_err(db)?;
            tx.execute(
                "DELETE FROM cc_profile_state WHERE stable_media_id=?1",
                [id],
            )
            .map_err(db)?;
            tx.execute(
                "DELETE FROM collection_members WHERE media_id=?1",
                [&legacy],
            )
            .map_err(db)?;
            tx.execute("DELETE FROM imported_sagas WHERE media_id=?1", [&legacy])
                .map_err(db)?;
            tx.execute("DELETE FROM media WHERE id=?1", [&legacy])
                .map_err(db)?;
            tx.execute("DELETE FROM cc_media_identity WHERE stable_id=?1", [id])
                .map_err(db)?;
        }
        // Relative order remains stable; holes in positions are legal and later
        // explicit order edits normalise them without touching any other list.
        tx.commit().map_err(db)
    }
    pub fn set_saga_artwork_at_revision(
        &self,
        profile: &str,
        saga: &str,
        expected: u64,
        path: &str,
    ) -> Result<(), String> {
        self.require_profile(profile)?;
        let id = saga.parse::<i64>().map_err(|_| "Saga invalide.")?;
        let tx =
            rusqlite::Transaction::new_unchecked(&self.personal, TransactionBehavior::Immediate)
                .map_err(db)?;
        let current:i64=tx.query_row("SELECT r.revision FROM collections c JOIN collection_revisions r ON r.collection_id=c.id WHERE c.id=?1 AND c.kind='saga'",[id],|r|r.get(0)).map_err(db)?;
        if current as u64 != expected {
            return Err("La saga a changé entre-temps.".into());
        }
        tx.execute("INSERT INTO collection_artwork(collection_id,poster) VALUES(?1,?2) ON CONFLICT(collection_id) DO UPDATE SET poster=excluded.poster",params![id,path]).map_err(db)?;
        tx.execute(
            "UPDATE collection_revisions SET revision=revision+1 WHERE collection_id=?1",
            [id],
        )
        .map_err(db)?;
        tx.commit().map_err(db)
    }
    pub fn saga_artwork_path(
        &self,
        profile: &str,
        saga: &str,
    ) -> Result<std::path::PathBuf, String> {
        self.require_profile(profile)?;
        self.common_saga_artwork_path(saga)
    }
    pub fn common_saga_artwork_path(&self, saga: &str) -> Result<std::path::PathBuf, String> {
        let id = saga.parse::<i64>().map_err(|_| "Saga invalide.")?;
        self.personal.query_row("SELECT a.poster FROM collection_artwork a JOIN collections c ON c.id=a.collection_id WHERE c.kind='saga' AND c.id=?1",[id],|r|r.get::<_,String>(0)).map(std::path::PathBuf::from).map_err(db)
    }
    pub fn related(&mut self, profile: &str, id: &str) -> Result<Vec<Media>, String> {
        let source = self.media(profile, id)?;
        // Limit both results and materialised candidates. Shared saga members
        // rank first, then genre intersection and nearby years.
        let legacy = self.legacy_id(id)?;
        let mut stmt=self.personal.prepare("SELECT DISTINCT m.data FROM media m LEFT JOIN collection_members cm ON cm.media_id=m.id WHERE m.id<>?1 AND json_extract(m.data,'$.available')=1 AND (cm.collection_id IN (SELECT collection_id FROM collection_members WHERE media_id=?1) OR EXISTS(SELECT 1 FROM json_each(json_extract(m.data,'$.genres')) g WHERE g.value IN (SELECT value FROM json_each(?2))) OR (?3 IS NOT NULL AND ABS(json_extract(m.data,'$.year')-?3)<=2)) ORDER BY CASE WHEN cm.collection_id IN (SELECT collection_id FROM collection_members WHERE media_id=?1) THEN 0 ELSE 1 END,json_extract(m.data,'$.title'),m.id LIMIT 12").map_err(db)?;
        stmt.query_map(
            params![
                legacy,
                serde_json::to_string(&source.genres).map_err(|_| "Genres invalides.")?,
                source.year
            ],
            |r| r.get::<_, String>(0),
        )
        .map_err(db)?
        .map(|r| self.profile_view(profile, decode(r.map_err(db)?)?))
        .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (tempfile::TempDir, Store, String, String, String) {
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::open(&dir.path().join("db.sqlite")).unwrap();
        let profile = store.create_profile("A").unwrap().id;
        let root = std::path::Path::new("/fixture");
        let mut a = chill_catalog::scanner::parse_file(root, &root.join("One.S01E01.mkv"));
        let mut b = chill_catalog::scanner::parse_file(root, &root.join("One.S02E01.mkv"));
        for m in [&mut a, &mut b] {
            m.category = "serie".into();
            m.series = Some("One".into());
        }
        store.snapshot(vec![a, b]).unwrap();
        let ids = store.catalog(&profile).unwrap();
        (dir, store, profile, ids[0].id.clone(), ids[1].id.clone())
    }
    #[test]
    fn shared_series_cas_preserves_episode_personal_and_speed() {
        let (_dir, mut store, p, a, _b) = fixture();
        store.set_rating(&p, &a, Some(4), "video").unwrap();
        store.set_series_speed(&p, "One", 1.5).unwrap();
        let before = store.media(&p, &a).unwrap();
        let epoch = store.common_revision().unwrap();
        store.set_rating(&p, &a, Some(4), "video").unwrap();
        let patch = SeriesPatch {
            name: "Renamed".into(),
            year: 2026,
            genres: vec!["Comédie".into()],
            overview: "Commun".into(),
            cast: vec![],
            directors: vec![],
        };
        store
            .update_shared_series(&p, "One", epoch, &patch)
            .unwrap();
        let after = store.media(&p, &a).unwrap();
        assert_eq!(
            (after.title, after.overview, after.rating),
            (before.title, before.overview, Some(4))
        );
        assert_eq!(after.series_metadata.unwrap().overview, "Commun");
        assert_eq!(store.series_speed(&p, "Renamed").unwrap(), 1.5);
        assert!(
            store
                .update_shared_series(&p, "Renamed", epoch, &patch)
                .is_err()
        );
    }
    #[test]
    fn unavailable_batch_is_atomic_and_protects_rediscovered_video() {
        let (_dir, mut store, p, a, b) = fixture();
        store.set_rating(&p, &a, Some(4), "video").unwrap();
        let legacy = store.legacy_id(&a).unwrap();
        let mut media = decode(
            store
                .personal
                .query_row("SELECT data FROM media WHERE id=?1", [&legacy], |r| {
                    r.get(0)
                })
                .unwrap(),
        )
        .unwrap();
        media.available = false;
        store
            .personal
            .execute(
                "UPDATE media SET data=?1 WHERE id=?2",
                params![encode(&media).unwrap(), legacy],
            )
            .unwrap();
        let epoch = store.catalog_revision().unwrap();
        assert!(store.forget_unavailable(&[a.clone(), b], epoch).is_err());
        assert_eq!(store.media(&p, &a).unwrap().rating, Some(4));
        store
            .forget_unavailable(std::slice::from_ref(&a), epoch)
            .unwrap();
        assert!(store.media(&p, &a).is_err());
        assert!(store.unavailable_page(0, 48).unwrap().items.is_empty());
    }
}
