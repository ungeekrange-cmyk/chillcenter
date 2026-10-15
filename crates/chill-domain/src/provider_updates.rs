//! Provider-update boundary.
//!
//! TMDB is fetched outside SQLite, but the snapshot and final write remain
//! compare-and-swap guarded.  This module never serializes storage paths: the
//! normal `profile_view` projection is used for every returned media item.
use crate::Store;
use chill_catalog::model::{Media, SeasonProvider, SeriesProvider};
use rusqlite::{OptionalExtension, params};

/// Own only a freshly created immutable poster until its database commit.
struct PendingPoster(Option<String>);
impl PendingPoster {
    fn committed(&mut self) {
        self.0 = None;
    }
}
impl Drop for PendingPoster {
    fn drop(&mut self) {
        if let Some(path) = self.0.take() {
            let _ = std::fs::remove_file(path);
        }
    }
}

fn assert_frozen_provider_tx(
    tx: &rusqlite::Transaction<'_>,
    stable: &str,
    revision: u64,
    common: u64,
    members: &[Media],
) -> Result<(), String> {
    let current_common: i64 = tx
        .query_row(
            "SELECT revision FROM cc_common_epoch WHERE id=1",
            [],
            |row| row.get(0),
        )
        .map_err(|_| "Erreur de base locale.")?;
    let current_revision: i64 = tx
        .query_row(
            "SELECT revision FROM cc_media_revision WHERE stable_id=?1",
            [stable],
            |row| row.get(0),
        )
        .map_err(|_| "Vidéo introuvable dans le catalogue.")?;
    if u64::try_from(current_common).ok() != Some(common)
        || u64::try_from(current_revision).ok() != Some(revision)
    {
        return Err(
            "La fiche ou la série a changé pendant la requête. Relancez l’opération.".into(),
        );
    }
    chill_catalog::database::Database::checked_series_members(tx, members)?;
    Ok(())
}

#[derive(Clone)]
pub struct ProviderSnapshot {
    pub media: Media,
    pub revision: u64,
    pub common_revision: u64,
}

#[derive(Clone)]
pub struct SeriesProviderSnapshot {
    pub series: String,
    pub members: Vec<Media>,
    pub anchor_revision: u64,
    pub common_revision: u64,
}

#[derive(Clone)]
pub struct SeasonProviderSnapshot {
    pub media: Media,
    pub members: Vec<Media>,
    pub revision: u64,
    pub common_revision: u64,
}

impl Store {
    /// Reads the raw provider record only for a short-lived server operation.
    /// Callers must return the result through `provider_apply_media`, which
    /// projects it to the safe profile DTO.
    pub fn provider_snapshot(
        &mut self,
        profile: &str,
        stable: &str,
        expected_revision: u64,
    ) -> Result<ProviderSnapshot, String> {
        self.require_profile(profile)?;
        let legacy = self.legacy_id(stable)?;
        let revision = self.media_revision(stable)?;
        if revision != expected_revision {
            return Err("La fiche a changé entre-temps.".into());
        }
        Ok(ProviderSnapshot {
            media: self.catalogue.get(&legacy)?,
            revision,
            common_revision: self.common_revision()?,
        })
    }

    pub fn provider_series_snapshot(
        &mut self,
        profile: &str,
        stable: &str,
        expected_revision: u64,
    ) -> Result<SeriesProviderSnapshot, String> {
        let anchor = self.provider_snapshot(profile, stable, expected_revision)?;
        if anchor.media.category != "serie" {
            return Err("Cette fiche n’est pas un épisode de série.".into());
        }
        let series = anchor
            .media
            .series
            .clone()
            .unwrap_or_else(|| anchor.media.title.clone());
        Ok(SeriesProviderSnapshot {
            members: self.catalogue.series_members_for(&anchor.media)?,
            series,
            anchor_revision: anchor.revision,
            common_revision: anchor.common_revision,
        })
    }

    /// Capture a concrete season.  A season cannot be searched independently:
    /// it is meaningful only below the selected parent TV id.
    pub fn provider_season_snapshot(
        &mut self,
        profile: &str,
        stable: &str,
        expected_revision: u64,
    ) -> Result<SeasonProviderSnapshot, String> {
        let anchor = self.provider_snapshot(profile, stable, expected_revision)?;
        if anchor.media.category != "serie" || anchor.media.season.is_none() {
            return Err("Cette fiche n’appartient pas à une saison de série.".into());
        }
        Ok(SeasonProviderSnapshot {
            members: self.catalogue.series_members_for(&anchor.media)?,
            media: anchor.media,
            revision: anchor.revision,
            common_revision: anchor.common_revision,
        })
    }

    fn assert_provider_cas(
        &self,
        stable: &str,
        expected_revision: u64,
        expected_common: u64,
    ) -> Result<(), String> {
        if self.media_revision(stable)? != expected_revision
            || self.common_revision()? != expected_common
        {
            return Err("La fiche ou la série a changé pendant la requête. Vos corrections sont conservées ; relancez l’opération.".into());
        }
        Ok(())
    }

    pub fn provider_apply_media(
        &mut self,
        profile: &str,
        stable: &str,
        snapshot: &ProviderSnapshot,
        downloaded: Media,
        explicit: bool,
    ) -> Result<(Media, u64), String> {
        self.require_profile(profile)?;
        self.assert_provider_cas(stable, snapshot.revision, snapshot.common_revision)?;
        let updated = self
            .catalogue
            .apply_provider(&snapshot.media, downloaded, explicit)?;
        self.personal
            .execute(
                "UPDATE cc_media_revision SET revision=revision+1 WHERE stable_id=?1",
                [stable],
            )
            .map_err(|_| "Erreur de base locale. Vos données n’ont pas été remplacées.")?;
        Ok((self.profile_view(profile, updated)?, snapshot.revision + 1))
    }

    pub fn provider_restore_poster(
        &mut self,
        profile: &str,
        stable: &str,
        snapshot: &ProviderSnapshot,
        poster: String,
    ) -> Result<(Media, u64), String> {
        self.require_profile(profile)?;
        self.assert_provider_cas(stable, snapshot.revision, snapshot.common_revision)?;
        let updated = self
            .catalogue
            .restore_poster(&snapshot.media, Some(poster))?;
        self.personal
            .execute(
                "UPDATE cc_media_revision SET revision=revision+1 WHERE stable_id=?1",
                [stable],
            )
            .map_err(|_| "Erreur de base locale. Vos données n’ont pas été remplacées.")?;
        Ok((self.profile_view(profile, updated)?, snapshot.revision + 1))
    }

    pub fn provider_apply_series(
        &mut self,
        profile: &str,
        stable: &str,
        snapshot: &SeriesProviderSnapshot,
        provider: &SeriesProvider,
        explicit: bool,
    ) -> Result<Vec<Media>, String> {
        self.require_profile(profile)?;
        self.assert_provider_cas(stable, snapshot.anchor_revision, snapshot.common_revision)?;
        let old_key = self.series_identity(stable)?;
        let legacy_unique = self.series(profile, &snapshot.series).is_ok();
        let updated = self.catalogue.apply_series_provider_with(
            &snapshot.series,
            &snapshot.members,
            provider,
            explicit,
            |tx| assert_frozen_provider_tx(tx,stable,snapshot.anchor_revision,snapshot.common_revision,&snapshot.members),
            |tx, updated| {for item in updated {
            let stable: Option<String> = tx
                .query_row(
                    "SELECT stable_id FROM cc_media_identity WHERE legacy_id=?1",
                    [&item.id],
                    |r| r.get(0),
                )
                .optional()
                .map_err(|_| "Erreur de base locale. Vos données n’ont pas été remplacées.")?;
            let stable = stable.ok_or("Vidéo introuvable dans le catalogue.")?;
            tx.execute(
                "UPDATE cc_media_revision SET revision=revision+1 WHERE stable_id=?1",
                params![stable],
            )
            .map_err(|_| "Erreur de base locale. Vos données n’ont pas été remplacées.")?;
        }
        tx.execute("INSERT OR IGNORE INTO cc_series_speed(profile_id,series_key,speed) SELECT profile_id,?1,speed FROM cc_series_speed WHERE series_key=?2",params![format!("tmdb:tv:{}",provider.tmdb_id),old_key]).map_err(|_|"Erreur de base locale.")?;
        if legacy_unique{tx.execute("INSERT OR IGNORE INTO cc_series_speed(profile_id,series_key,speed) SELECT profile_id,?1,speed FROM cc_series_speed WHERE series_key=?2",params![format!("tmdb:tv:{}",provider.tmdb_id),snapshot.series]).map_err(|_|"Erreur de base locale.")?;}
        Ok(()) })?;
        updated
            .into_iter()
            .map(|item| self.profile_view(profile, item))
            .collect()
    }

    /// Persist selected season information as a distinct provider record and
    /// return the refreshed profile-facing members.  Episode-specific fields
    /// remain untouched; the UI can display season synopsis/artwork from the
    /// returned `SeasonProvider` DTO.
    pub fn provider_apply_season(
        &mut self,
        profile: &str,
        stable: &str,
        snapshot: &SeasonProviderSnapshot,
        provider: &SeasonProvider,
    ) -> Result<(SeasonProvider, Vec<Media>), String> {
        self.require_profile(profile)?;
        self.assert_provider_cas(stable, snapshot.revision, snapshot.common_revision)?;
        let season = snapshot.media.season.ok_or("Saison introuvable.")?;
        if provider.tv_id != snapshot.media.tmdb_id.unwrap_or_default() || provider.season != season
        {
            return Err("La saison ne correspond pas à la série sélectionnée.".into());
        }
        let data =
            serde_json::to_string(provider).map_err(|_| "Métadonnées de saison invalides.")?;
        let tx = rusqlite::Transaction::new_unchecked(
            &self.personal,
            rusqlite::TransactionBehavior::Immediate,
        )
        .map_err(|_| "Erreur de base locale. Vos données n’ont pas été remplacées.")?;
        assert_frozen_provider_tx(
            &tx,
            stable,
            snapshot.revision,
            snapshot.common_revision,
            &snapshot.members,
        )?;
        tx.execute(
            "INSERT INTO cc_season_provider(tv_id,season,data,updated_at) VALUES(?1,?2,?3,strftime('%s','now')) ON CONFLICT(tv_id,season) DO UPDATE SET data=excluded.data,updated_at=excluded.updated_at",
            params![provider.tv_id, provider.season, data],
        ).map_err(|_| "Erreur de base locale. Vos données n’ont pas été remplacées.")?;
        for item in &snapshot.members {
            if item.season != Some(season) {
                continue;
            }
            let stable_id: String = tx
                .query_row(
                    "SELECT stable_id FROM cc_media_identity WHERE legacy_id=?1",
                    [&item.id],
                    |r| r.get(0),
                )
                .map_err(|_| "Vidéo introuvable dans le catalogue.")?;
            tx.execute(
                "UPDATE cc_media_revision SET revision=revision+1 WHERE stable_id=?1",
                [stable_id],
            )
            .map_err(|_| "Erreur de base locale. Vos données n’ont pas été remplacées.")?;
        }
        tx.commit()
            .map_err(|_| "Erreur de base locale. Vos données n’ont pas été remplacées.")?;
        let items = snapshot
            .members
            .clone()
            .into_iter()
            .filter(|item| item.season == Some(season))
            .map(|item| self.profile_view(profile, item))
            .collect::<Result<Vec<_>, _>>()?;
        Ok((provider.clone(), items))
    }

    pub fn season_provider(
        &self,
        tv_id: i64,
        season: i32,
    ) -> Result<Option<SeasonProvider>, String> {
        self.personal
            .query_row(
                "SELECT data FROM cc_season_provider WHERE tv_id=?1 AND season=?2",
                params![tv_id, season],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|_| "Erreur de base locale. Vos données n’ont pas été remplacées.")?
            .map(|data| {
                serde_json::from_str(&data).map_err(|_| "Métadonnées de saison invalides.".into())
            })
            .transpose()
    }

    pub fn season_artwork_path(
        &self,
        profile: &str,
        tv_id: i64,
        season: i32,
    ) -> Result<std::path::PathBuf, String> {
        self.require_profile(profile)?;
        let provider = self
            .season_provider(tv_id, season)?
            .ok_or("Affiche introuvable.")?;
        let poster = provider.poster.ok_or("Affiche introuvable.")?;
        if !std::path::Path::new(&poster).is_absolute() || poster.starts_with("file:") {
            return Err("Affiche introuvable.".into());
        }
        Ok(std::path::PathBuf::from(poster))
    }

    /// Apply a local image to the frozen series member set.  A series artwork
    /// import is common data, so it is deliberately not a one-episode edit.
    pub fn set_custom_series_poster(
        &mut self,
        profile: &str,
        stable: &str,
        expected: u64,
        bytes: &[u8],
        cache: &std::path::Path,
    ) -> Result<Vec<Media>, String> {
        let snapshot = self.provider_series_snapshot(profile, stable, expected)?;
        self.assert_provider_cas(stable, snapshot.anchor_revision, snapshot.common_revision)?;
        let poster = chill_catalog::poster::store(bytes, cache)?;
        let mut pending = PendingPoster(Some(poster.clone()));
        let tx = rusqlite::Transaction::new_unchecked(
            &self.personal,
            rusqlite::TransactionBehavior::Immediate,
        )
        .map_err(|_| "Erreur de base locale. Vos données n’ont pas été remplacées.")?;
        assert_frozen_provider_tx(
            &tx,
            stable,
            snapshot.anchor_revision,
            snapshot.common_revision,
            &snapshot.members,
        )?;
        for item in &snapshot.members {
            let data: String = tx
                .query_row("SELECT data FROM media WHERE id=?1", [&item.id], |r| {
                    r.get(0)
                })
                .map_err(|_| "Vidéo introuvable dans le catalogue.")?;
            let mut updated: Media = serde_json::from_str(&data)
                .map_err(|_| "Catalogue local endommagé : aucune donnée remplacée.")?;
            if updated.tmdb_id != item.tmdb_id || updated.series != item.series {
                return Err(
                    "La série a changé pendant l’import. Votre image est conservée.".into(),
                );
            }
            updated.poster = Some(poster.clone());
            updated.poster_custom = true;
            tx.execute(
                "UPDATE media SET data=?1 WHERE id=?2",
                params![
                    serde_json::to_string(&updated).map_err(|_| "Métadonnées invalides.")?,
                    updated.id
                ],
            )
            .map_err(|_| "Erreur de base locale. Vos données n’ont pas été remplacées.")?;
            let stable_id: String = tx
                .query_row(
                    "SELECT stable_id FROM cc_media_identity WHERE legacy_id=?1",
                    [&item.id],
                    |r| r.get(0),
                )
                .map_err(|_| "Vidéo introuvable dans le catalogue.")?;
            tx.execute(
                "UPDATE cc_media_revision SET revision=revision+1 WHERE stable_id=?1",
                [stable_id],
            )
            .map_err(|_| "Erreur de base locale. Vos données n’ont pas été remplacées.")?;
        }
        tx.commit()
            .map_err(|_| "Erreur de base locale. Vos données n’ont pas été remplacées.")?;
        pending.committed();
        snapshot
            .members
            .into_iter()
            .map(|item| {
                let mut updated = item;
                updated.poster = Some(poster.clone());
                updated.poster_custom = true;
                self.profile_view(profile, updated)
            })
            .collect()
    }

    /// Store a manual season artwork independently of the show/episodes.
    pub fn set_custom_season_poster(
        &mut self,
        profile: &str,
        stable: &str,
        expected: u64,
        bytes: &[u8],
        cache: &std::path::Path,
    ) -> Result<SeasonProvider, String> {
        let snapshot = self.provider_season_snapshot(profile, stable, expected)?;
        self.assert_provider_cas(stable, snapshot.revision, snapshot.common_revision)?;
        let tv_id = snapshot
            .media
            .tmdb_id
            .ok_or("Sélectionnez d’abord la série TMDB.")?;
        let season = snapshot.media.season.ok_or("Saison introuvable.")?;
        let poster = chill_catalog::poster::store(bytes, cache)?;
        let mut pending = PendingPoster(Some(poster.clone()));
        let mut provider = self
            .season_provider(tv_id, season)?
            .unwrap_or(SeasonProvider {
                tv_id,
                season,
                name: format!("Saison {season}"),
                overview: String::new(),
                poster: None,
            });
        provider.poster = Some(poster);
        let tx = rusqlite::Transaction::new_unchecked(
            &self.personal,
            rusqlite::TransactionBehavior::Immediate,
        )
        .map_err(|_| "Erreur de base locale. Vos données n’ont pas été remplacées.")?;
        assert_frozen_provider_tx(
            &tx,
            stable,
            snapshot.revision,
            snapshot.common_revision,
            &snapshot.members,
        )?;
        tx.execute(
            "INSERT INTO cc_season_provider(tv_id,season,data,updated_at) VALUES(?1,?2,?3,strftime('%s','now')) ON CONFLICT(tv_id,season) DO UPDATE SET data=excluded.data,updated_at=excluded.updated_at",
            params![tv_id, season, serde_json::to_string(&provider).map_err(|_| "Métadonnées de saison invalides.")?],
        ).map_err(|_| "Erreur de base locale. Vos données n’ont pas été remplacées.")?;
        for item in snapshot
            .members
            .iter()
            .filter(|item| item.season == Some(season))
        {
            let stable_id: String = tx
                .query_row(
                    "SELECT stable_id FROM cc_media_identity WHERE legacy_id=?1",
                    [&item.id],
                    |r| r.get(0),
                )
                .map_err(|_| "Vidéo introuvable dans le catalogue.")?;
            tx.execute(
                "UPDATE cc_media_revision SET revision=revision+1 WHERE stable_id=?1",
                [stable_id],
            )
            .map_err(|_| "Erreur de base locale. Vos données n’ont pas été remplacées.")?;
        }
        tx.commit()
            .map_err(|_| "Erreur de base locale. Vos données n’ont pas été remplacées.")?;
        pending.committed();
        Ok(provider)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chill_catalog::model::{Media, SeriesPatch};
    use tempfile::tempdir;

    #[test]
    fn final_transaction_rejects_scan_committed_after_early_cas() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("catalog.sqlite");
        let mut store = Store::open(&path).unwrap();
        let profile = store.create_profile("Viewer").unwrap();
        let mut item = media("Show/Season 1/E1.mkv", "Pilot");
        item.category = "serie".into();
        item.series = Some("Show".into());
        item.season = Some(1);
        item.episode = Some(1);
        store.snapshot(vec![item.clone()]).unwrap();
        let id = store.catalog(&profile.id).unwrap()[0].id.clone();
        let snapshot = store.provider_series_snapshot(&profile.id, &id, 1).unwrap();
        store
            .assert_provider_cas(&id, snapshot.anchor_revision, snapshot.common_revision)
            .unwrap();
        let mut scanner = Store::open(&path).unwrap();
        let mut second = item.clone();
        second.id = "Show/Season 1/E2.mkv".into();
        second.path = format!("/nas/{}", second.id);
        second.episode = Some(2);
        scanner.snapshot(vec![item, second]).unwrap();
        let tx = rusqlite::Transaction::new_unchecked(
            &store.personal,
            rusqlite::TransactionBehavior::Immediate,
        )
        .unwrap();
        assert!(
            assert_frozen_provider_tx(
                &tx,
                &id,
                snapshot.anchor_revision,
                snapshot.common_revision,
                &snapshot.members
            )
            .is_err()
        );
        drop(tx);
        assert_eq!(store.catalog(&profile.id).unwrap().len(), 2);
        assert!(
            store
                .catalog(&profile.id)
                .unwrap()
                .iter()
                .all(|item| item.tmdb_id.is_none())
        );
    }

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
    fn unknown_homonyms_keep_two_complete_source_groups_and_scoped_edits() {
        let mut items = Vec::new();
        for year in [2005, 2023] {
            for season in 1..=2 {
                for episode in 1..=2 {
                    let mut item = media(
                        &format!("Doctor Who ({year})/Season {season}/E{episode}.mkv"),
                        &format!("Episode {episode}"),
                    );
                    item.category = "serie".into();
                    item.series = Some("Doctor Who".into());
                    item.season = Some(season);
                    item.episode = Some(episode);
                    items.push(item);
                }
            }
        }
        let (mut store, profile) = store(items);
        let query = crate::catalog_browser::BrowserQuery {
            grouped: true,
            ..Default::default()
        };
        let page = store.browse_catalog(&profile, &query).unwrap();
        assert_eq!(page.items.len(), 2);
        assert!(store.series(&profile, "Doctor Who").is_err());
        for counter in page.series_counters.values() {
            assert_eq!(counter.episodes, 4);
            assert_eq!(counter.seasons, 2);
            assert_eq!(
                store.series(&profile, &counter.series_key).unwrap().len(),
                4
            );
        }
        let first = &page.items[0];
        let key = page.series_counters[&first.id].series_key.clone();
        let other = page.items[1].id.clone();
        store.set_series_speed(&profile, "Doctor Who", 2.0).unwrap();
        assert_eq!(
            store.series_playback_speed(&profile, &first.id).unwrap(),
            1.0,
            "ambiguous legacy speed must not leak across shows"
        );
        store
            .set_series_speed(&profile, &store.series_identity(&first.id).unwrap(), 1.5)
            .unwrap();
        store
            .set_series_speed(&profile, &store.series_identity(&other).unwrap(), 0.75)
            .unwrap();
        store
            .set_rating(&profile, &first.id, Some(4), "series")
            .unwrap();
        assert_eq!(store.media(&profile, &other).unwrap().rating, None);
        let snapshot_document = store.sync_snapshot(&profile).unwrap();
        assert!(
            snapshot_document
                .series_revisions
                .contains_key(&format!("media:{}", first.id))
        );
        let epoch = store.common_revision().unwrap();
        store
            .update_shared_series(
                &profile,
                &key,
                epoch,
                &SeriesPatch {
                    name: "Doctor Who".into(),
                    year: 2005,
                    genres: vec![],
                    overview: "Selected only".into(),
                    cast: vec![],
                    directors: vec![],
                },
            )
            .unwrap();
        assert!(
            store
                .series(&profile, &key)
                .unwrap()
                .iter()
                .all(|m| m.series_overview == "Selected only")
        );
        assert!(
            store
                .media(&profile, &other)
                .unwrap()
                .series_overview
                .is_empty()
        );
        let revision = store.media_revision(&first.id).unwrap();
        let snapshot = store
            .provider_series_snapshot(&profile, &first.id, revision)
            .unwrap();
        let provider = SeriesProvider {
            tmdb_id: 57243,
            name: "Doctor Who".into(),
            metadata: snapshot.members[0].series_metadata.clone().unwrap(),
            poster: None,
        };
        store
            .provider_apply_series(&profile, &first.id, &snapshot, &provider, true)
            .unwrap();
        assert_eq!(
            store.series_playback_speed(&profile, &first.id).unwrap(),
            1.5
        );
        assert_eq!(store.series_playback_speed(&profile, &other).unwrap(), 0.75);
        let identified = store.series(&profile, "tmdb:tv:57243").unwrap();
        assert_eq!(identified.len(), 4);
        store
            .update_shared_series(
                &profile,
                "tmdb:tv:57243",
                store.common_revision().unwrap(),
                &SeriesPatch {
                    name: "Doctor Who".into(),
                    year: 2005,
                    genres: vec![],
                    overview: "Identified edit".into(),
                    cast: vec![],
                    directors: vec![],
                },
            )
            .unwrap();
        assert_eq!(
            store.series_playback_speed(&profile, &first.id).unwrap(),
            1.5,
            "renaming/edit must preserve the identity preference"
        );
        assert_eq!(
            store
                .browse_catalog(
                    &profile,
                    &crate::catalog_browser::BrowserQuery {
                        grouped: true,
                        q: Some("Doctor Who".into()),
                        sort: Some("title_asc".into()),
                        ..Default::default()
                    }
                )
                .unwrap()
                .items
                .len(),
            2
        );
        // A second unchanged page must not rewrite/reparse the grouping table.
        store.personal.execute_batch("CREATE TEMP TRIGGER no_group_rebuild BEFORE DELETE ON cc_series_group BEGIN SELECT RAISE(ABORT,'unnecessary rebuild');END;").unwrap();
        assert_eq!(
            store.browse_catalog(&profile, &query).unwrap().items.len(),
            2
        );
    }
    #[test]
    fn series_provider_revision_failure_rolls_back_metadata_and_revisions_together() {
        let mut a = media("Show/Season 1/E1.mkv", "Pilot");
        a.category = "serie".into();
        a.series = Some("Show".into());
        a.season = Some(1);
        a.episode = Some(1);
        let mut b = a.clone();
        b.id = "Show/Season 1/E2.mkv".into();
        b.path = format!("/nas/{}", b.id);
        b.episode = Some(2);
        let (mut store, profile) = store(vec![a, b]);
        let members = store.catalog(&profile).unwrap();
        let id = members[0].id.clone();
        let before = store.common_revision().unwrap();
        let snapshot = store.provider_series_snapshot(&profile, &id, 1).unwrap();
        let metadata = SeriesPatch {
            name: "Show".into(),
            year: 2026,
            genres: vec![],
            overview: "Downloaded".into(),
            cast: vec![],
            directors: vec![],
        }
        .validate(&snapshot.members[0])
        .unwrap();
        let provider = SeriesProvider {
            tmdb_id: 42,
            name: "Show".into(),
            metadata,
            poster: Some("/fixture/fresh-owned.jpg".into()),
        };
        store.personal.execute_batch("CREATE TRIGGER reject_series_revision BEFORE UPDATE ON cc_media_revision BEGIN SELECT RAISE(ABORT,'simulated revision failure');END;").unwrap();
        assert!(
            store
                .provider_apply_series(&profile, &id, &snapshot, &provider, true)
                .is_err()
        );
        assert_eq!(store.common_revision().unwrap(), before);
        for member in members {
            let now = store.media(&profile, &member.id).unwrap();
            assert_eq!(now.tmdb_id, None);
            assert_eq!(now.poster, None);
            assert_eq!(store.media_revision(&member.id).unwrap(), 1);
        }
    }
    #[test]
    fn personal_activity_does_not_cancel_a_provider_match() {
        let item = media("film.mkv", "Film");
        let (mut store, profile) = store(vec![item]);
        let id = store.catalog(&profile).unwrap()[0].id.clone();
        let snapshot = store.provider_snapshot(&profile, &id, 1).unwrap();
        store.set_watched(&profile, &id, true).unwrap();
        store.set_rating(&profile, &id, Some(5), "video").unwrap();
        let mut downloaded = snapshot.media.clone();
        downloaded.title = "Matched".into();
        downloaded.tmdb_id = Some(42);
        let (result, _) = store
            .provider_apply_media(&profile, &id, &snapshot, downloaded, true)
            .unwrap();
        assert!(result.watched);
        assert_eq!(result.rating, Some(5));
        assert_eq!(result.title, "Matched");
    }
    #[test]
    fn failed_custom_series_and_season_imports_remove_only_the_new_owned_image() {
        let mut item = media("Show/Season 1/E1.mkv", "Pilot");
        item.category = "serie".into();
        item.series = Some("Show".into());
        item.season = Some(1);
        item.episode = Some(1);
        item.tmdb_id = Some(42);
        let (mut store, profile) = store(vec![item]);
        let id = store.catalog(&profile).unwrap()[0].id.clone();
        let cache = tempdir().unwrap();
        let bytes = include_bytes!("../../../icons/branding/vector/default.png");
        store.personal.execute_batch("CREATE TEMP TRIGGER reject_custom_series BEFORE UPDATE ON media BEGIN SELECT RAISE(ABORT,'simulated DB failure');END;").unwrap();
        assert!(
            store
                .set_custom_series_poster(&profile, &id, 1, bytes, cache.path())
                .is_err()
        );
        assert_eq!(std::fs::read_dir(cache.path()).unwrap().count(), 0);
        store.personal.execute_batch("DROP TRIGGER reject_custom_series;CREATE TEMP TRIGGER reject_custom_season BEFORE INSERT ON cc_season_provider BEGIN SELECT RAISE(ABORT,'simulated DB failure');END;").unwrap();
        assert!(
            store
                .set_custom_season_poster(&profile, &id, 1, bytes, cache.path())
                .is_err()
        );
        assert_eq!(std::fs::read_dir(cache.path()).unwrap().count(), 0);
        store
            .personal
            .execute_batch("DROP TRIGGER reject_custom_season;")
            .unwrap();
        store
            .set_custom_season_poster(&profile, &id, 1, bytes, cache.path())
            .unwrap();
        assert_eq!(store.media_revision(&id).unwrap(), 2);
        assert_eq!(std::fs::read_dir(cache.path()).unwrap().count(), 1);
    }
    fn store(items: Vec<Media>) -> (Store, String) {
        let path = tempdir().unwrap().keep().join("catalog.sqlite");
        let mut store = Store::open(&path).unwrap();
        store.snapshot(items).unwrap();
        let profile = store.create_profile("Admin").unwrap();
        (store, profile.id)
    }

    #[test]
    fn provider_media_cas_preserves_custom_poster_and_rejects_concurrent_change() {
        let mut item = media("film.mkv", "Original");
        item.poster = Some("/cache/custom.png".into());
        item.poster_custom = true;
        let (mut store, profile) = store(vec![item]);
        let id = store.catalog(&profile).unwrap()[0].id.clone();
        let snapshot = store.provider_snapshot(&profile, &id, 1).unwrap();
        let mut downloaded = snapshot.media.clone();
        downloaded.title = "TMDB title".into();
        downloaded.tmdb_id = Some(42);
        downloaded.poster = Some("/cache/provider.jpg".into());
        let (updated, revision) = store
            .provider_apply_media(&profile, &id, &snapshot, downloaded.clone(), true)
            .unwrap();
        assert_eq!(revision, 2);
        assert!(updated.poster_custom);
        let artwork = format!("/api/v1/profiles/{profile}/media/{id}/artwork");
        assert_eq!(updated.poster.as_deref(), Some(artwork.as_str()));

        let snapshot = store.provider_snapshot(&profile, &id, revision).unwrap();
        store
            .update_media_at_revision(
                &profile,
                &id,
                revision,
                &chill_catalog::model::MediaPatch {
                    title: Some("Local correction".into()),
                    ..Default::default()
                },
            )
            .unwrap();
        assert!(
            store
                .provider_apply_media(&profile, &id, &snapshot, downloaded, false)
                .is_err()
        );
        assert_eq!(
            store.media(&profile, &id).unwrap().title,
            "Local correction"
        );
    }

    #[test]
    fn series_provider_renames_atomically_but_keeps_episode_fields() {
        let mut first = media("s01e01.mkv", "Episode one");
        first.category = "serie".into();
        first.series = Some("Old name".into());
        first.season = Some(1);
        first.episode = Some(1);
        let mut second = media("s01e02.mkv", "Episode two");
        second.category = "serie".into();
        second.series = Some("Old name".into());
        second.season = Some(1);
        second.episode = Some(2);
        let (mut store, profile) = store(vec![first, second]);
        let ids = store
            .catalog(&profile)
            .unwrap()
            .into_iter()
            .map(|m| m.id)
            .collect::<Vec<_>>();
        let snapshot = store
            .provider_series_snapshot(&profile, &ids[0], 1)
            .unwrap();
        let provider = SeriesProvider {
            name: "New name".into(),
            metadata: SeriesPatch {
                name: "New name".into(),
                year: 2024,
                genres: vec!["SF".into()],
                overview: "Shared".into(),
                cast: vec![],
                directors: vec![],
            }
            .validate(&snapshot.members[0])
            .unwrap(),
            tmdb_id: 7,
            poster: Some("/cache/tv.jpg".into()),
        };
        let changed = store
            .provider_apply_series(&profile, &ids[0], &snapshot, &provider, true)
            .unwrap();
        assert_eq!(changed.len(), 2);
        assert!(
            changed
                .iter()
                .all(|m| m.series.as_deref() == Some("New name"))
        );
        assert_eq!(
            changed.iter().find(|m| m.id == ids[1]).unwrap().title,
            "Episode two"
        );

        let snapshot = store
            .provider_series_snapshot(&profile, &ids[0], 2)
            .unwrap();
        store
            .update_media_at_revision(
                &profile,
                &ids[1],
                2,
                &chill_catalog::model::MediaPatch {
                    overview: Some("Corrected episode".into()),
                    ..Default::default()
                },
            )
            .unwrap();
        assert!(
            store
                .provider_apply_series(&profile, &ids[0], &snapshot, &provider, false)
                .is_err()
        );
        assert_eq!(
            store.media(&profile, &ids[1]).unwrap().overview,
            "Corrected episode"
        );
    }

    #[test]
    fn season_provider_is_keyed_by_tv_id_and_does_not_touch_episode_details() {
        let mut episode = media("doctor/s01/e01.mkv", "Pilot");
        episode.category = "serie".into();
        episode.series = Some("Doctor Who".into());
        episode.season = Some(1);
        episode.episode = Some(1);
        episode.tmdb_id = Some(57243);
        episode.overview = "Episode synopsis".into();
        let (mut store, profile) = store(vec![episode]);
        let id = store.catalog(&profile).unwrap()[0].id.clone();
        let snapshot = store.provider_season_snapshot(&profile, &id, 1).unwrap();
        let season = SeasonProvider {
            tv_id: 57243,
            season: 1,
            name: "Season 1".into(),
            overview: "Season synopsis".into(),
            poster: Some("/cache/season.jpg".into()),
        };
        let (saved, episodes) = store
            .provider_apply_season(&profile, &id, &snapshot, &season)
            .unwrap();
        assert_eq!(saved.overview, "Season synopsis");
        assert_eq!(episodes.len(), 1);
        assert_eq!(episodes[0].overview, "Episode synopsis");
        assert_eq!(
            store.season_provider(57243, 1).unwrap().unwrap().poster,
            Some("/cache/season.jpg".into())
        );
    }
}
