//! Common catalogue edits shared by the CAS and AI transaction paths.
use crate::store::db_error;
use chill_catalog::model::{Media, MediaPatch};
use rusqlite::{OptionalExtension, Transaction, params};

/// Applies only common media fields.  The caller owns the stable-media revision
/// bump, which lets a larger plan validate every revision before its first write.
pub(crate) fn apply_common_patch(
    tx: &Transaction<'_>,
    legacy: &str,
    patch: &MediaPatch,
) -> Result<Media, String> {
    if patch.watched.is_some() || patch.rating.is_some() {
        return Err("La note et le vu sont propres au profil.".into());
    }
    let data: String = tx
        .query_row("SELECT data FROM media WHERE id=?1", [legacy], |row| {
            row.get(0)
        })
        .map_err(db_error)?;
    let original: Media = serde_json::from_str(&data)
        .map_err(|_| "Catalogue local endommagé : aucune donnée remplacée.".to_string())?;
    let mut updated = original.clone();
    patch.apply(&mut updated)?;

    if updated.category == "serie"
        && (updated.series != original.series || original.category != "serie")
    {
        let key = updated.series.as_deref().unwrap_or(&updated.title);
        let sibling: Option<String> = tx
            .query_row(
                "SELECT data FROM media WHERE id<>?1 AND json_extract(data,'$.category')='serie' AND COALESCE(json_extract(data,'$.series'),json_extract(data,'$.title'))=?2 AND json_extract(data,'$.series_metadata') IS NOT NULL ORDER BY id LIMIT 1",
                params![legacy, key],
                |row| row.get(0),
            )
            .optional()
            .map_err(db_error)?;
        if let Some(data) = sibling {
            let sibling: Media = serde_json::from_str(&data)
                .map_err(|_| "Catalogue local endommagé : aucune donnée remplacée.".to_string())?;
            updated.series_metadata = sibling.series_metadata;
            updated.series_overview = sibling.series_overview;
        }
    }

    if updated.category != "film" {
        // Never leave an episode in a film-only common saga.  Clearing both the
        // legacy marker and its import receipt prevents a later catalogue read
        // from resurrecting the association.
        updated.saga = None;
        tx.execute(
            "UPDATE collection_revisions SET revision=revision+1 WHERE collection_id IN (SELECT cm.collection_id FROM collection_members cm JOIN collections c ON c.id=cm.collection_id WHERE cm.media_id=?1 AND c.kind='saga')",
            [legacy],
        ).map_err(db_error)?;
        tx.execute(
            "DELETE FROM collection_members WHERE media_id=?1 AND collection_id IN (SELECT id FROM collections WHERE kind='saga')",
            [legacy],
        ).map_err(db_error)?;
        tx.execute("DELETE FROM imported_sagas WHERE media_id=?1", [legacy])
            .map_err(db_error)?;
    }
    tx.execute(
        "UPDATE media SET data=?1 WHERE id=?2",
        params![
            serde_json::to_string(&updated).map_err(|_| "Métadonnées invalides.")?,
            legacy
        ],
    )
    .map_err(db_error)?;
    Ok(updated)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Store;
    use chill_catalog::model::{Media, SeriesMetadata};
    use rusqlite::TransactionBehavior;

    fn media(id: &str, category: &str) -> Media {
        Media {
            id: id.into(),
            path: format!("/nas/{id}"),
            title: id.into(),
            category: category.into(),
            year: Some(2020),
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
            available: true,
            added_at: 0,
            tmdb_id: None,
        }
    }
    #[test]
    fn inherits_series_metadata_and_removes_film_only_saga_without_personal_leak() {
        let d = tempfile::tempdir().unwrap();
        let mut store = Store::open(&d.path().join("db.sqlite")).unwrap();
        let profile = store.create_profile("A").unwrap();
        let mut sibling = media("series.mkv", "serie");
        sibling.series = Some("Needle".into());
        sibling.series_metadata = Some(SeriesMetadata {
            year: Some(2001),
            genres: vec!["Sci-fi".into()],
            overview: "shared".into(),
            cast: vec!["A".into()],
            directors: vec!["B".into()],
        });
        sibling.series_overview = "shared overview".into();
        let mut film = media("film.mkv", "film");
        film.saga = Some("Legacy".into());
        store.snapshot(vec![sibling, film]).unwrap();
        let items = store.catalog(&profile.id).unwrap();
        let film_id = items
            .iter()
            .find(|m| m.title == "film.mkv")
            .unwrap()
            .id
            .clone();
        store
            .set_rating(&profile.id, &film_id, Some(4), "video")
            .unwrap();
        let saga = store.create_saga("Legacy").unwrap();
        store
            .saga_members_at_revision(&saga.id, &[film_id.clone()], saga.revision, true)
            .unwrap();
        let legacy: String = store
            .personal
            .query_row(
                "SELECT legacy_id FROM cc_media_identity WHERE stable_id=?1",
                [&film_id],
                |r| r.get(0),
            )
            .unwrap();
        let tx =
            rusqlite::Transaction::new_unchecked(&store.personal, TransactionBehavior::Immediate)
                .unwrap();
        let changed = apply_common_patch(
            &tx,
            &legacy,
            &MediaPatch {
                category: Some("serie".into()),
                series: Some("Needle".into()),
                ..Default::default()
            },
        )
        .unwrap();
        tx.commit().unwrap();
        assert_eq!(changed.series_metadata.as_ref().unwrap().overview, "shared");
        assert_eq!(changed.saga, None);
        assert_eq!(
            store.personal_state(&profile.id, &film_id).unwrap().rating,
            Some(4)
        );
        assert_eq!(
            store
                .personal
                .query_row::<i64, _, _>(
                    "SELECT COUNT(*) FROM collection_members WHERE media_id=?1",
                    [&legacy],
                    |r| r.get(0)
                )
                .unwrap(),
            0
        );
        assert_eq!(
            store
                .personal
                .query_row::<i64, _, _>(
                    "SELECT COUNT(*) FROM imported_sagas WHERE media_id=?1",
                    [&legacy],
                    |r| r.get(0)
                )
                .unwrap(),
            0
        );
        assert!(
            store.export_json().unwrap()["sagas"]
                .as_array()
                .unwrap()
                .iter()
                .all(|s| !s["media_ids"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|id| id == &film_id))
        );
    }
}
